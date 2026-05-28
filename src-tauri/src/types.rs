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
// OCR 関連
// ---------------------------------------------------------------------------

/// OCR で読み取った生テキスト
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcrText {
    pub text: String,
    pub confidence: f32,
}

/// 1試合分の抽出データ (OCR 生出力)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedMatchData {
    pub result: String,
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
    /// Phase 1: 「バトルを開始します！」ROI の OCR 生テキスト
    pub battle_start_text: String,
    /// Phase 1: バトル開始テキストが見つかったか (OCR または暗巻物)
    pub battle_start_found: bool,
    /// Phase 1: 暗い巻物ピクセルで検出したか
    pub dark_scroll_found: bool,
    /// Phase 2: リザルト画面判定 — WIN 側グレー行数 (4行中)
    pub win_grey_rows: u32,
    /// Phase 2: リザルト画面判定 — LOSE 側グレー行数 (4行中)
    pub lose_grey_rows: u32,
    /// Phase 2 参考: WIN_ROI OCR テキスト (大文字)
    pub win_roi_text: String,
    /// Phase 2 参考: WIN テキストが見つかったか
    pub win_text_found: bool,
    /// Phase 2: WIN パネル側の黄色矢印ピクセル数
    pub yellow_win_px: u32,
    /// Phase 2: LOSE パネル側の黄色矢印ピクセル数
    pub yellow_lose_px: u32,
    /// Phase 2: 黄色ピクセルの y 重心 (0.0–1.0)
    pub centroid_y: f32,
    /// Phase 2: 黄色ピクセルの y スプレッド (参考値 px)
    pub y_spread: u32,
    /// 判定サマリー文字列
    pub detection_summary: String,
}
