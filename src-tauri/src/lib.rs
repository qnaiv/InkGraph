/// InkGraph — Tauri アプリのエントリポイント

pub mod capture;
pub mod capture_loop;
pub mod cascade;
pub mod commands;
pub mod db;
pub mod detector;
pub mod extractor;
pub mod ocr;
pub mod preprocess;
pub mod screen_state;
pub mod state;
pub mod types;

use state::AppState;
use commands::{
    debug_capture,
    debug_full,
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
                    tauri_plugin_sql::Migration {
                        version: 2,
                        description: "add mode column",
                        sql: include_str!("../migrations/002_add_mode.sql"),
                        kind: tauri_plugin_sql::MigrationKind::Up,
                    },
                    tauri_plugin_sql::Migration {
                        version: 3,
                        description: "add special_count column",
                        sql: include_str!("../migrations/003_add_special_count.sql"),
                        kind: tauri_plugin_sql::MigrationKind::Up,
                    },
                    tauri_plugin_sql::Migration {
                        version: 4,
                        description: "add special_count column",
                        sql: include_str!("../migrations/004_add_special_count.sql"),
                        kind: tauri_plugin_sql::MigrationKind::Up,
                    },
                    tauri_plugin_sql::Migration {
                        version: 5,
                        description: "add gold_award_count column",
                        sql: include_str!("../migrations/005_add_gold_award.sql"),
                        kind: tauri_plugin_sql::MigrationKind::Up,
                    },
                    tauri_plugin_sql::Migration {
                        version: 6,
                        description: "add paint_count column",
                        sql: include_str!("../migrations/006_add_paint_count.sql"),
                        kind: tauri_plugin_sql::MigrationKind::Up,
                    },
                    tauri_plugin_sql::Migration {
                        version: 7,
                        description: "fix result check constraint to allow in_progress and draw",
                        sql: include_str!("../migrations/007_fix_result_constraint.sql"),
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
            debug_full,
        ])
        .run(tauri::generate_context!())
        .expect("error while running InkGraph");
}
