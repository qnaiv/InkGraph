/// IkaVision XP — グローバルアプリ状態
use std::sync::Arc;
use tokio::sync::Mutex;

/// Tauri の managed state として登録するアプリ状態
#[derive(Debug, Clone)]
pub struct AppState {
    /// キャプチャ中かどうか
    pub is_capturing: Arc<Mutex<bool>>,
    /// 最後に選択されたウィンドウタイトル
    pub target_window: Arc<Mutex<Option<String>>>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            is_capturing: Arc::new(Mutex::new(false)),
            target_window: Arc::new(Mutex::new(None)),
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}
