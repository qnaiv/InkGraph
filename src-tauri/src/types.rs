/// InkGraph — 共有型定義
use serde::{Deserialize, Serialize};

// YOLO 検出結果型を detector.rs から再エクスポート
pub use crate::detector::{BBox, Detection, YoloClass};

// ---------------------------------------------------------------------------
// Match (試合記録)
// ---------------------------------------------------------------------------

/// SQLite に保存する試合データ
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Match {
    pub id: String,
    pub played_at: String,
    pub mode: Option<String>,  // "Xマッチ" / "バンカラマッチ(チャレンジ)" / "ナワバリバトル" 等
    pub rule: Option<String>,
    pub stage: Option<String>,
    pub weapon: Option<String>,
    pub result: String, // "win" | "lose" | "in_progress"
    pub kill_count: Option<i64>,
    pub assist_count: Option<i64>,
    pub death_count: Option<i64>,
    pub xp_after: Option<f64>,
    pub tags: Option<String>, // JSON 配列文字列
    pub note: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

// ---------------------------------------------------------------------------
// OCR / 抽出 関連
// ---------------------------------------------------------------------------

/// OCR で読み取った生テキスト
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcrText {
    pub text: String,
    pub confidence: f32,
}

/// 1試合分の抽出データ
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedMatchData {
    pub result: String,
    pub mode: Option<String>,
    pub kill_count: Option<i64>,
    pub assist_count: Option<i64>,
    pub death_count: Option<i64>,
    pub xp_after: Option<f64>,
    pub rule: Option<String>,
    pub stage: Option<String>,
}

// ---------------------------------------------------------------------------
// イベント ペイロード
// ---------------------------------------------------------------------------

/// Rust → React: リザルト検知イベント
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchDetectedPayload {
    pub match_data: Match,
    pub ocr_confidence: f32,
}

/// Rust → React: キャプチャ状態
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaptureStatusPayload {
    pub active: bool,
    pub fps: f32,
    pub window_title: Option<String>,
}

/// Rust → React: OCR デバッグ情報
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcrDebugPayload {
    pub region: String,
    pub raw_text: String,
    pub parsed_value: Option<String>,
}

// ---------------------------------------------------------------------------
// コマンド引数 / 戻り値
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, Deserialize)]
pub struct WindowInfo {
    pub hwnd: u64,
    pub title: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OcrTestResult {
    pub raw_text: String,
    pub lines: Vec<String>,
}

/// debug_capture コマンドの診断結果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaptureDebugResult {
    pub frame_w: u32,
    pub frame_h: u32,
    pub battle_start_text: String,
    pub battle_start_found: bool,
    pub dark_scroll_found: bool,
    pub win_grey_rows: u32,
    pub lose_grey_rows: u32,
    pub win_roi_text: String,
    pub win_text_found: bool,
    pub yellow_win_px: u32,
    pub yellow_lose_px: u32,
    pub centroid_y: f32,
    pub y_spread: u32,
    pub detection_summary: String,
}
