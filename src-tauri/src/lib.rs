/// IkaVision XP — Tauri アプリのエントリポイント

pub mod capture;
pub mod capture_loop;
pub mod commands;
pub mod db;
pub mod detector;
pub mod extractor;
pub mod ocr;
pub mod state;
pub mod types;

use state::AppState;
use commands::{
    debug_capture,
    list_windows,
    start_capture,
    stop_capture,
    test_ocr,
};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        // ── プラグイン ──────────────────────────────────────────────────
        .plugin(tauri_plugin_log::Builder::default()
            .level(log::LevelFilter::Info)
            .build())
        .plugin(tauri_plugin_sql::Builder::default()
            .add_migrations(
                db::DB_URL,
                vec![
                    tauri_plugin_sql::Migration {
                        version: 1,
                        description: "initial schema",
                        sql: include_str!("../migrations/001_initial.sql"),
                        kind: tauri_plugin_sql::MigrationKind::Up,
                    },
                ],
            )
            .build())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        // ── グローバル状態 ──────────────────────────────────────────────
        .manage(AppState::new())
        // ── Tauri コマンド ──────────────────────────────────────────────
        .invoke_handler(tauri::generate_handler![
            test_ocr,
            list_windows,
            start_capture,
            stop_capture,
            debug_capture,
        ])
        .run(tauri::generate_context!())
        .expect("error while running IkaVision XP");
}
