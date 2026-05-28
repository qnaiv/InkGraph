/// IkaVision XP — グローバルアプリ状態
use std::sync::Arc;
use tauri::async_runtime::JoinHandle;
use tokio::sync::Mutex;

#[derive(Debug, Clone)]
pub struct AppState {
    /// キャプチャ中フラグ (ループの継続判定に使う)
    pub is_capturing: Arc<Mutex<bool>>,
    /// 現在実行中のキャプチャタスクハンドル
    /// stop や再起動時に abort() で即座に終了させる
    pub capture_task: Arc<Mutex<Option<JoinHandle<()>>>>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            is_capturing:  Arc::new(Mutex::new(false)),
            capture_task:  Arc::new(Mutex::new(None)),
        }
    }
}

impl Default for AppState {
    fn default() -> Self { Self::new() }
}
