/// InkGraph — Tauri コマンド定義

use tauri::{AppHandle, Emitter, State};
use crate::{
    ocr::ocr_from_file,
    state::AppState,
    types::{CaptureDebugResult, CaptureStatusPayload, OcrTestResult, WindowInfo,
            YoloDebugDetection, FullDebugResult, HeaderDebugResult},
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

/// Model 1 (YOLO) + Model 2 (カスケード) を 1 回のコマンドで実行する統合デバッグ。
/// 信頼度 0.10 以上の全検出を返すため、通常の閾値以下の候補も確認できる。
#[tauri::command]
pub async fn debug_full(hwnd: u64) -> Result<FullDebugResult, String> {
    #[cfg(target_os = "windows")]
    {
        use crate::{
            cascade::StatsDetector,
            detector::{YoloDetector, YoloClass},
        };

        let result_model_path = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|d| d.join("assets/models/yolo_result.onnx")))
            .unwrap_or_else(|| std::path::PathBuf::from("assets/models/yolo_result.onnx"));
        let stats_model_path = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|d| d.join("assets/models/yolo_stats.onnx")))
            .unwrap_or_else(|| std::path::PathBuf::from("assets/models/yolo_stats.onnx"));

        let mut yolo  = YoloDetector::new(&result_model_path);
        let mut stats = StatsDetector::new(&stats_model_path);

        if let Err(e) = tokio::task::block_in_place(|| yolo.load()) {
            return Ok(FullDebugResult {
                frame_w: 0, frame_h: 0,
                model1_loaded: false, model2_loaded: false,
                detections: vec![], ocr: None,
                arrow_found: false,
                crop_x: 0, crop_y: 0, crop_w: 0, crop_h: 0,
                crop_image_base64: None,
                cascade_detections: vec![],
                kill_anchor_x: None, death_anchor_x: None, special_anchor_x: None,
                paint: None, kill: None, death: None, special: None,
                error: Some(format!("yolo_result.onnx ロード失敗: {e}")),
                header: HeaderDebugResult {
                    frame_w: 0, frame_h: 0,
                    crop_x: 0, crop_y: 0, crop_w: 0, crop_h: 0,
                    crop_image_base64: None,
                    detections: vec![],
                    mode: None, rule: None, stage: None,
                    error: None,
                },
            });
        }

        let model2_loaded = tokio::task::block_in_place(|| stats.load()).is_ok();

        let session = tokio::task::block_in_place(|| crate::capture::WindowCaptureSession::new(hwnd))
            .map_err(|e| format!("WGC session failed: {e}"))?;
        let frame = get_frame_with_retry(&session).await
            .map_err(|e| format!("get_frame failed: {e}"))?;

        let (fw, fh) = (frame.width, frame.height);

        // Model 1: 閾値 0.10 で全候補取得 + OCR
        let m1_dets = tokio::task::block_in_place(|| yolo.detect_debug(&frame))
            .map_err(|e| format!("YOLO detect failed: {e}"))?;

        let ocr = tokio::task::block_in_place(|| crate::extractor::extract_debug_ocr(&frame, &m1_dets));

        let detections: Vec<YoloDebugDetection> = m1_dets.iter().map(|d| YoloDebugDetection {
            class_name: d.class_name.clone(),
            class_id:   d.class_id,
            confidence: d.confidence,
            x1: d.bbox.x1, y1: d.bbox.y1,
            x2: d.bbox.x2, y2: d.bbox.y2,
        }).collect();

        // Model 2: カスケード (MyArrow 検出 → クロップ → stats 推論)
        let arrow = YoloDetector::best_detection(&m1_dets, YoloClass::MyArrow);
        let cascade = tokio::task::block_in_place(|| stats.run_cascade_debug(&frame, arrow));

        // Model 2: ヘッダーカスケード (モード/ルール/ステージ)
        let header = tokio::task::block_in_place(|| stats.run_header_cascade_debug(&frame));

        Ok(FullDebugResult {
            frame_w: fw, frame_h: fh,
            model1_loaded: true,
            model2_loaded,
            detections,
            ocr: Some(ocr),
            arrow_found: cascade.arrow_found,
            crop_x: cascade.crop_x,
            crop_y: cascade.crop_y,
            crop_w: cascade.crop_w,
            crop_h: cascade.crop_h,
            crop_image_base64: cascade.crop_image_base64,
            cascade_detections: cascade.detections,
            kill_anchor_x: cascade.kill_anchor_x,
            death_anchor_x: cascade.death_anchor_x,
            special_anchor_x: cascade.special_anchor_x,
            paint: cascade.paint,
            kill:  cascade.kill,
            death: cascade.death,
            special: cascade.special,
            error: cascade.error,
            header,
        })
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = hwnd;
        Err("debug_full は Windows 専用です".to_string())
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
