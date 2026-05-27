/// IkaVision XP — Tauri コマンド定義

use tauri::{AppHandle, State};
use crate::{
    ocr::ocr_from_file,
    state::AppState,
    types::{OcrTestResult, WindowInfo},
};

// ---------------------------------------------------------------------------
// OCR テストコマンド
// ---------------------------------------------------------------------------

/// 画像ファイルから OCR を実行するテストコマンド
///
/// フロントエンドから invoke("test_ocr", { imagePath: "C:/..." }) で呼び出す
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

    Ok(OcrTestResult {
        raw_text: result.text,
        lines,
    })
}

// ---------------------------------------------------------------------------
// キャプチャ制御コマンド
// ---------------------------------------------------------------------------

/// 利用可能なウィンドウ一覧を返す
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

/// キャプチャを開始する
#[tauri::command]
pub async fn start_capture(
    app: AppHandle,
    state: State<'_, AppState>,
    window_title: String,
) -> Result<(), String> {
    let mut capturing = state.is_capturing.lock().await;
    if *capturing {
        return Err("Already capturing".to_string());
    }
    *capturing = true;
    drop(capturing);

    log::info!("[commands] start_capture: window_title={window_title}");

    let state_clone = state.inner().clone();
    let app_clone = app.clone();
    tauri::async_runtime::spawn(async move {
        crate::capture_loop::run(app_clone, state_clone, window_title).await;
    });

    Ok(())
}

/// キャプチャを停止する
#[tauri::command]
pub async fn stop_capture(state: State<'_, AppState>) -> Result<(), String> {
    let mut capturing = state.is_capturing.lock().await;
    *capturing = false;
    log::info!("[commands] stop_capture");
    Ok(())
}
