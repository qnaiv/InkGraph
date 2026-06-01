/// InkGraph — Tauri コマンド定義

use tauri::{AppHandle, Emitter, State};
use crate::{
    ocr::ocr_from_file,
    state::AppState,
    types::{CaptureDebugResult, CaptureStatusPayload, OcrTestResult, WindowInfo, YoloDebugResult, YoloDebugDetection},
};

// ---------------------------------------------------------------------------
// OCR テストコマンド
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn test_ocr(image_path: String) -> Result<OcrTestResult, String> {
    let result = ocr_from_file(&image_path, None)
        .map_err(|e| format!("OCR failed: {e}"))?;

    let lines: Vec<String> = result
        .text
        .lines()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
        .collect();

    Ok(OcrTestResult { raw_text: result.text, lines })
}

// ---------------------------------------------------------------------------
// キャプチャ制御コマンド
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn list_windows() -> Result<Vec<WindowInfo>, String> {
    #[cfg(target_os = "windows")]
    {
        crate::capture::list_capturable_windows()
            .map_err(|e| format!("list_windows failed: {e}"))
    }
    #[cfg(not(target_os = "windows"))]
    {
        Ok(vec![WindowInfo {
            hwnd: 0,
            title: "[stub] Non-Windows platform".to_string(),
        }])
    }
}

/// キャプチャを開始する。
/// 既存のタスクがあれば abort() してから新しいタスクを起動する。
/// `hwnd` でウィンドウを直接指定することでタイトル曖昧マッチを排除。
#[tauri::command]
pub async fn start_capture(
    app: AppHandle,
    state: State<'_, AppState>,
    hwnd: u64,
) -> Result<(), String> {
    // 既存タスクを abort（再起動時のレースコンディション防止）
    {
        let mut task = state.capture_task.lock().await;
        if let Some(handle) = task.take() {
            handle.abort();
        }
    }

    *state.is_capturing.lock().await = true;
    log::info!("[commands] start_capture: hwnd={hwnd}");

    let state_clone = state.inner().clone();
    let app_clone   = app.clone();
    let handle = tauri::async_runtime::spawn(async move {
        crate::capture_loop::run(app_clone, state_clone, hwnd).await;
    });

    *state.capture_task.lock().await = Some(handle);
    Ok(())
}

// ---------------------------------------------------------------------------
// デバッグキャプチャコマンド
// ---------------------------------------------------------------------------

/// 指定ウィンドウから 1 フレームだけ取得して検知パイプラインの診断情報を返す。
/// リザルト画面を表示した状態で呼ぶと各ステップの通過状況が確認できる。
#[tauri::command]
pub async fn debug_capture(hwnd: u64) -> Result<CaptureDebugResult, String> {
    #[cfg(target_os = "windows")]
    {
        use crate::{capture::WindowCaptureSession, detector::debug_detect_frame};
        let session = tokio::task::block_in_place(|| WindowCaptureSession::new(hwnd))
            .map_err(|e| format!("WGC session failed: {e}"))?;
        // WGC が最初のフレームを届けるまでリトライ (静止画面では 500ms 超えることがある)
        let frame = get_frame_with_retry(&session).await
            .map_err(|e| format!("get_frame failed: {e}"))?;
        debug_detect_frame(&frame).map_err(|e| format!("debug_detect failed: {e}"))
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = hwnd;
        Err("debug_capture は Windows 専用です".to_string())
    }
}

/// YOLO モデルの生検出結果を診断する。
/// 信頼度 0.10 以上の全検出を返すため、通常の閾値 (0.70) 以下の候補も確認できる。
#[tauri::command]
pub async fn debug_yolo(hwnd: u64) -> Result<YoloDebugResult, String> {
    #[cfg(target_os = "windows")]
    {
        use crate::{capture::WindowCaptureSession, detector::YoloDetector};

        let model_path = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|d| d.join("assets/models/yolo_result.onnx")))
            .unwrap_or_else(|| std::path::PathBuf::from("assets/models/yolo_result.onnx"));

        let mut yolo = YoloDetector::new(&model_path);
        let load_result = tokio::task::block_in_place(|| yolo.load());

        if let Err(e) = load_result {
            return Ok(YoloDebugResult {
                frame_w: 0, frame_h: 0,
                model_loaded: false,
                detections: vec![],
                ocr: None,
                error: Some(format!("モデルロード失敗: {e}")),
            });
        }

        let session = tokio::task::block_in_place(|| WindowCaptureSession::new(hwnd))
            .map_err(|e| format!("WGC session failed: {e}"))?;
        let frame = get_frame_with_retry(&session).await
            .map_err(|e| format!("get_frame failed: {e}"))?;

        let (fw, fh) = (frame.width, frame.height);
        let dets = tokio::task::block_in_place(|| yolo.detect_debug(&frame))
            .map_err(|e| format!("YOLO detect failed: {e}"))?;

        // 全フィールドの OCR デバッグ情報を収集
        let ocr = tokio::task::block_in_place(|| crate::extractor::extract_debug_ocr(&frame, &dets));

        let detections = dets.into_iter().map(|d| YoloDebugDetection {
            class_name: d.class_name,
            class_id:   d.class_id,
            confidence: d.confidence,
            x1: d.bbox.x1, y1: d.bbox.y1,
            x2: d.bbox.x2, y2: d.bbox.y2,
        }).collect();

        Ok(YoloDebugResult {
            frame_w: fw, frame_h: fh,
            model_loaded: true,
            detections,
            ocr: Some(ocr),
            error: None,
        })
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = hwnd;
        Err("debug_yolo は Windows 専用です".to_string())
    }
}

/// WGC フレームをリトライ付きで取得する (最大 3秒)。
/// 静止した画面では最初のフレームが届くまで時間がかかることがある。
#[cfg(target_os = "windows")]
async fn get_frame_with_retry(
    session: &crate::capture::WindowCaptureSession,
) -> anyhow::Result<crate::capture::CapturedFrame> {
    for attempt in 0..6 {
        match tokio::task::block_in_place(|| session.get_frame()) {
            Ok(f)  => return Ok(f),
            Err(e) => {
                if attempt == 5 { return Err(e); }
                tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
            }
        }
    }
    unreachable!()
}

/// キャプチャを停止する。
/// タスクを abort() して即座に終了させ、inactive をフロントエンドに通知する。
#[tauri::command]
pub async fn stop_capture(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    {
        let mut task = state.capture_task.lock().await;
        if let Some(handle) = task.take() {
            handle.abort();
        }
    }

    *state.is_capturing.lock().await = false;

    // abort でタスクが強制終了するため run() 末尾が動かない → ここで通知
    let _ = app.emit("capture_status", CaptureStatusPayload {
        active: false,
        fps: 0.0,
        window_title: None,
        yolo_loaded: false,
    });

    log::info!("[commands] stop_capture");
    Ok(())
}
