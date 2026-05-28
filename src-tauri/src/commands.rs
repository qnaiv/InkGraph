/// IkaVision XP — Tauri コマンド定義

use tauri::{AppHandle, Emitter, State};
use crate::{
    ocr::ocr_from_file,
    state::AppState,
    types::{CaptureStatusPayload, OcrTestResult, WindowInfo},
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
