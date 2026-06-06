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
    pub death_count: Option<i64>,
    pub special_count: Option<i64>,
    pub paint_count: Option<i64>,
    pub xp_after: Option<f64>,
    pub gold_award_count: Option<i64>,
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
    pub death_count: Option<i64>,
    pub special_count: Option<i64>,
    pub paint_count: Option<i64>,
    pub xp_after: Option<f64>,
    pub rule: Option<String>,
    pub stage: Option<String>,
    pub gold_award_count: Option<i64>,
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
    pub yolo_loaded: bool,
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
    pub rule_ocr_text: String,
    pub rule_normalized: Option<String>,
    pub stage_ocr_text: String,
    pub stage_normalized: Option<String>,
    pub win_roi_text: String,
    pub win_text_found: bool,
    pub yellow_win_px: u32,
    pub yellow_lose_px: u32,
    pub centroid_y: f32,
    pub y_spread: u32,
    pub detection_summary: String,
}

/// debug_yolo コマンドの診断結果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YoloDebugDetection {
    pub class_name: String,
    pub class_id: usize,
    pub confidence: f32,
    pub x1: f32,
    pub y1: f32,
    pub x2: f32,
    pub y2: f32,
}

/// OCR デバッグ: 1フィールド分の生テキスト + 正規化後
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcrDebugField {
    pub raw: String,
    pub normalized: Option<String>,
}

/// カスケードデバッグ: Model 2 の1検出エントリ（グループ割り当て付き）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CascadeDebugDetection {
    pub class_name:  String,
    pub confidence:  f32,
    pub x_center:    f32,
    /// "paint" | "kill" | "death" | "special" | "anchor_kill" | "anchor_death" | "anchor_special" | "ignored"
    pub group:       String,
}

/// カスケードデバッグコマンドの戻り値
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CascadeDebugResult {
    pub frame_w:            u32,
    pub frame_h:            u32,
    pub stats_model_loaded: bool,
    pub arrow_found:        bool,
    pub crop_x:             u32,
    pub crop_y:             u32,
    pub crop_w:             u32,
    pub crop_h:             u32,
    /// クロップ画像の base64 PNG (フロントエンド表示用)
    pub crop_image_base64:  Option<String>,
    /// Model 2 の全検出 (x_center 昇順)
    pub detections:         Vec<CascadeDebugDetection>,
    pub kill_anchor_x:      Option<f32>,
    pub death_anchor_x:     Option<f32>,
    pub special_anchor_x:   Option<f32>,
    pub paint:              Option<i64>,
    pub kill:               Option<i64>,
    pub death:              Option<i64>,
    pub special:            Option<i64>,
    pub error:              Option<String>,
}

/// ヘッダーカスケードデバッグ: Model 2 の1検出エントリ（モード/ルール/ステージ判定用、グループなし）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeaderDebugDetection {
    pub class_name: String,
    pub confidence: f32,
    pub x_center:   f32,
}

/// ヘッダーカスケードデバッグコマンドの戻り値
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeaderDebugResult {
    pub frame_w:           u32,
    pub frame_h:           u32,
    pub crop_x:            u32,
    pub crop_y:            u32,
    pub crop_w:            u32,
    pub crop_h:            u32,
    /// クロップ画像の base64 PNG (フロントエンド表示用)
    pub crop_image_base64: Option<String>,
    /// Model 2 の全検出 (確信度降順)
    pub detections:        Vec<HeaderDebugDetection>,
    pub mode:              Option<String>,
    pub rule:              Option<String>,
    pub stage:             Option<String>,
    pub error:             Option<String>,
}

/// YOLO + カスケード統合デバッグコマンドの戻り値
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FullDebugResult {
    pub frame_w:            u32,
    pub frame_h:            u32,
    // Model 1 (yolo_result.onnx)
    pub model1_loaded:      bool,
    pub detections:         Vec<YoloDebugDetection>,
    pub ocr:                Option<OcrDebugResult>,
    // Cascade / Model 2 (yolo_stats.onnx) — KDA カスケード
    pub model2_loaded:      bool,
    pub arrow_found:        bool,
    pub crop_x:             u32,
    pub crop_y:             u32,
    pub crop_w:             u32,
    pub crop_h:             u32,
    pub crop_image_base64:  Option<String>,
    pub cascade_detections: Vec<CascadeDebugDetection>,
    pub kill_anchor_x:      Option<f32>,
    pub death_anchor_x:     Option<f32>,
    pub special_anchor_x:   Option<f32>,
    pub paint:              Option<i64>,
    pub kill:               Option<i64>,
    pub death:              Option<i64>,
    pub special:            Option<i64>,
    pub error:              Option<String>,
    // ヘッダーカスケード (モード/ルール/ステージ)
    pub header:             HeaderDebugResult,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcrDebugResult {
    pub rule:    OcrDebugField,
    pub stage:   OcrDebugField,
    pub mode:    OcrDebugField,
    pub kill:    OcrDebugField,
    pub death:   OcrDebugField,
    pub special: OcrDebugField,
    pub arrow_y: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YoloDebugResult {
    pub frame_w: u32,
    pub frame_h: u32,
    pub model_loaded: bool,
    /// 信頼度 0.10 以上の全検出 (通常の閾値 0.60 より低い)
    pub detections: Vec<YoloDebugDetection>,
    /// YOLO 検出領域の OCR 結果
    pub ocr: Option<OcrDebugResult>,
    pub error: Option<String>,
}
