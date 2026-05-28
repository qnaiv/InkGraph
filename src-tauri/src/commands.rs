/// IkaVision XP — Tauri コマンド定義

use tauri::{AppHandle, Emitter, State};
use crate::{
    ocr::ocr_from_file,
    state::AppState,
    types::{CaptureDebugResult, CaptureStatusPayload, OcrTestResult, WindowInfo},
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
        let frame = tokio::task::block_in_place(|| session.get_frame())
            .map_err(|e| format!("get_frame failed: {e}"))?;
        debug_detect_frame(&frame).map_err(|e| format!("debug_detect failed: {e}"))
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = hwnd;
        Err("debug_capture は Windows 専用です".to_string())
    }
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
    });

    log::info!("[commands] stop_capture");
    Ok(())
}
