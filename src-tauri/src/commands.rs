/// IkaVision XP — Tauri コマンド定義

use tauri::{AppHandle, State};
use serde::Deserialize;
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
/// フロントエンドから invoke("test_ocr", { image_path: "C:/..." }) で呼び出す
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

    // キャプチャループを別タスクで起動
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

// ---------------------------------------------------------------------------
// 試合データ CRUD コマンド
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct GetMatchesParams {
    pub limit: Option<u32>,
    pub rule: Option<String>,
}

/// 試合一覧を取得する
///
/// DB 操作は tauri-plugin-sql が JS 側で担うため、ここでは SQL 文字列を返す
/// (フロントエンドが直接 SQLite を呼ぶ設計)
#[tauri::command]
pub async fn get_xp_history(rule: Option<String>) -> Result<String, String> {
    Ok(crate::db::select_xp_history_sql(rule.is_some()))
}

/// ブキを更新する
#[tauri::command]
pub async fn update_weapon(id: String, weapon: String) -> Result<(), String> {
    log::info!("[commands] update_weapon: id={id}, weapon={weapon}");
    // 実際の DB 更新は フロントエンドの tauri-plugin-sql 経由で行う
    Ok(())
}

/// タグを更新する
#[tauri::command]
pub async fn update_tags(id: String, tags: Vec<String>) -> Result<(), String> {
    let tags_json = serde_json::to_string(&tags)
        .map_err(|e| format!("JSON serialize failed: {e}"))?;
    log::info!("[commands] update_tags: id={id}, tags={tags_json}");
    Ok(())
}

/// メモを更新する
#[tauri::command]
pub async fn update_note(id: String, _note: String) -> Result<(), String> {
    log::info!("[commands] update_note: id={id}");
    Ok(())
}
