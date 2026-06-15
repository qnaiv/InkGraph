/// InkGraph — グローバルアプリ状態
use std::sync::Arc;
use tauri::async_runtime::JoinHandle;
use tokio::sync::Mutex;
use crate::capture::CapturedFrame;

#[derive(Debug, Clone)]
pub struct AppState {
    /// キャプチャ中フラグ (ループの継続判定に使う)
    pub is_capturing: Arc<Mutex<bool>>,
    /// 現在実行中のキャプチャタスクハンドル
    /// stop や再起動時に abort() で即座に終了させる
    pub capture_task: Arc<Mutex<Option<JoinHandle<()>>>>,
    /// キャプチャループが取得した最新フレーム。
    /// debug_full が同一 hwnd に 2 つ目の WGC セッションを作る問題を回避するため
    /// ループ側が更新し、デバッグコマンド側が読み取る。
    pub last_frame: Arc<Mutex<Option<CapturedFrame>>>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            is_capturing: Arc::new(Mutex::new(false)),
            capture_task: Arc::new(Mutex::new(None)),
            last_frame:   Arc::new(Mutex::new(None)),
        }
    }
}

impl Default for AppState {
    fn default() -> Self { Self::new() }
}
