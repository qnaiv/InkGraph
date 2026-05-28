/// InkGraph — リザルト検知モジュール
///
/// 検知方式 (2フェーズ):
///   Phase 1 (WaitingForBattle):
///     「バトルを開始します！」テキストを OCR で検知 → Phase 2 へ移行
///     ロビー・試合中など、バトル開始前の誤検知をここで排除する
///   Phase 2 (BattleInProgress):
///     黄色プレイヤー矢印 (▶) のピクセルスキャンで WIN/LOSE 判定 → Phase 1 へ戻る
///     バトルが確定した後のみリザルト検知を行う

use crate::{
    capture::CapturedFrame,
    ocr::ocr_from_bgra,
};
use anyhow::Result;

// ---------------------------------------------------------------------------
// ROI 定義 (16:9 フレームに対する比率)
// ---------------------------------------------------------------------------

/// 「バトルを開始します！」テキスト領域
/// 白テキストが黒い巻物背景に表示される — OCR で確実に読める
const BATTLE_START_ROI: Roi = Roi {
    x_ratio: 0.25,
    y_ratio: 0.36,
    w_ratio: 0.50,
    h_ratio: 0.20,
};

/// WIN! バナー領域 — debug_detect_frame の診断用のみ
/// WinRT OCR はスタイル化フォントを認識できないため detect() では使用しない
const WIN_ROI: Roi = Roi {
    x_ratio: 0.455,
    y_ratio: 0.267,
    w_ratio: 0.190,
    h_ratio: 0.065,
};

/// プレイヤー矢印スキャン領域 (黄色 ▶ が表示される x 帯)
const ARROW_X_START: f32 = 0.455;
const ARROW_X_END:   f32 = 0.505;
const ARROW_Y_START: f32 = 0.270;
const ARROW_Y_END:   f32 = 0.940;

/// WIN パネルと LOSE パネルの境界 y 比率
const PANEL_BOUNDARY_Y: f32 = 0.630;

/// 黄色矢印と判定する最小ピクセル数 (リザルト画面では 350+ px)
const MIN_YELLOW_PIXELS: u32 = 100;

/// 黄色ピクセルの y 方向スプレッド上限 (フレーム高さに対する比率)
/// 矢印 (▶) は集中した小領域のはずなので 12% 超なら別物と判断する
const MAX_Y_SPREAD_RATIO: f32 = 0.12;

// ---------------------------------------------------------------------------
// Roi 構造体
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy)]
pub struct Roi {
    pub x_ratio: f32,
    pub y_ratio: f32,
    pub w_ratio: f32,
    pub h_ratio: f32,
}

impl Roi {
    pub fn to_pixels(&self, frame_w: u32, frame_h: u32) -> (u32, u32, u32, u32) {
        let x = (self.x_ratio * frame_w as f32) as u32;
        let y = (self.y_ratio * frame_h as f32) as u32;
        let w = (self.w_ratio * frame_w as f32) as u32;
        let h = (self.h_ratio * frame_h as f32) as u32;
        (x, y, w, h)
    }
}

// ---------------------------------------------------------------------------
// 検知エンジン
// ---------------------------------------------------------------------------

#[derive(Debug, PartialEq)]
enum DetectorPhase {
    WaitingForBattle,
    BattleInProgress,
}

pub struct ResultDetector {
    phase: DetectorPhase,
}

/// 検知結果。Win/Lose には矢印の y 重心比率を持たせ、
/// extractor がプレイヤー行を動的に特定できるようにする。
#[derive(Debug, Clone)]
pub enum DetectionResult {
    Win  { arrow_y_ratio: f32 },
    Lose { arrow_y_ratio: f32 },
    NotDetected,
}

impl DetectionResult {
    pub fn result_str(&self) -> Option<&'static str> {
        match self {
            Self::Win  { .. } => Some("win"),
            Self::Lose { .. } => Some("lose"),
            Self::NotDetected => None,
        }
    }

    pub fn arrow_y_ratio(&self) -> Option<f32> {
        match self {
            Self::Win  { arrow_y_ratio } | Self::Lose { arrow_y_ratio } => Some(*arrow_y_ratio),
            Self::NotDetected => None,
        }
    }
}

impl ResultDetector {
    pub fn new(_cooldown_secs: u64) -> Self {
        Self { phase: DetectorPhase::WaitingForBattle }
    }

    /// 2フェーズ検知。detect() は blocking OCR を含むため、
    /// 呼び出し元は tokio::task::block_in_place でラップすること。
    pub fn detect(&mut self, frame: &CapturedFrame) -> Result<DetectionResult> {
        match self.phase {
            DetectorPhase::WaitingForBattle => {
                // 「バトルを開始します！」が見えたら試合中フラグを立てる
                let text = ocr_roi_raw(frame, &BATTLE_START_ROI, "ja-JP")
                    .unwrap_or_default();
                if text.contains("バトル") || text.contains("開始") {
                    self.phase = DetectorPhase::BattleInProgress;
                    log::info!("[detector] battle start detected → BattleInProgress");
                }
                Ok(DetectionResult::NotDetected)
            }

            DetectorPhase::BattleInProgress => {
                let (win_px, lose_px, centroid_y, y_spread) = count_yellow_arrow_pixels(frame);
                let max_spread = (frame.height as f32 * MAX_Y_SPREAD_RATIO) as u32;

                // 黄色矢印が見えなければ待機継続
                if (win_px < MIN_YELLOW_PIXELS && lose_px < MIN_YELLOW_PIXELS)
                    || y_spread > max_spread
                {
                    return Ok(DetectionResult::NotDetected);
                }

                // リザルト検知 → Phase 1 へ戻す (次の試合まで再検知しない)
                self.phase = DetectorPhase::WaitingForBattle;
                let result = if win_px >= lose_px {
                    DetectionResult::Win  { arrow_y_ratio: centroid_y }
                } else {
                    DetectionResult::Lose { arrow_y_ratio: centroid_y }
                };
                log::info!(
                    "[detector] {:?} detected (win_px={win_px} lose_px={lose_px} spread={y_spread} arrow_y={centroid_y:.3})",
                    result.result_str()
                );
                Ok(result)
            }
        }
    }
}

// ---------------------------------------------------------------------------
// デバッグ診断 (1フレーム one-shot)
// ---------------------------------------------------------------------------

/// フレームに対して両フェーズの診断情報をまとめて返す。状態変更なし。
pub fn debug_detect_frame(frame: &CapturedFrame) -> Result<crate::types::CaptureDebugResult> {
    // Phase 1: バトル開始テキスト
    let battle_start_text = ocr_roi_raw(frame, &BATTLE_START_ROI, "ja-JP")
        .unwrap_or_else(|e| format!("OCR_ERROR: {e}"));
    let battle_start_found =
        battle_start_text.contains("バトル") || battle_start_text.contains("開始");

    // Phase 2: 黄色矢印 (参考情報)
    let win_roi_text = ocr_roi_raw(frame, &WIN_ROI, "en-US")
        .map(|t| t.to_uppercase())
        .unwrap_or_else(|e| format!("OCR_ERROR: {e}"));
    let win_text_found = win_roi_text.contains("WIN");

    let (yellow_win_px, yellow_lose_px, centroid_y, y_spread) = count_yellow_arrow_pixels(frame);
    let max_spread = (frame.height as f32 * MAX_Y_SPREAD_RATIO) as u32;
    let spread_ok  = y_spread <= max_spread;

    let detection_summary = if battle_start_found {
        "Phase 1 ✓ バトル開始検出 → BattleInProgress へ移行".to_string()
    } else if !spread_ok {
        format!("Phase 2: NOT_DETECTED — spread={y_spread}px > max={max_spread}px (矢印ではない)")
    } else if yellow_win_px >= MIN_YELLOW_PIXELS {
        format!("Phase 2 ✓ WIN (win_px={yellow_win_px}, spread={y_spread}px, centroid_y={centroid_y:.3})")
    } else if yellow_lose_px >= MIN_YELLOW_PIXELS {
        format!("Phase 2 ✓ LOSE (lose_px={yellow_lose_px}, spread={y_spread}px, centroid_y={centroid_y:.3})")
    } else {
        format!("Phase 2: NOT_DETECTED (win={yellow_win_px} lose={yellow_lose_px} spread={y_spread}px < threshold={MIN_YELLOW_PIXELS})")
    };

    Ok(crate::types::CaptureDebugResult {
        frame_w: frame.width,
        frame_h: frame.height,
        battle_start_text,
        battle_start_found,
        win_roi_text,
        win_text_found,
        yellow_win_px,
        yellow_lose_px,
        centroid_y,
        y_spread,
        detection_summary,
    })
}

// ---------------------------------------------------------------------------
// 内部ヘルパー
// ---------------------------------------------------------------------------

/// ROI を切り出して OCR し、生テキストをそのまま返す
fn ocr_roi_raw(frame: &CapturedFrame, roi: &Roi, lang: &str) -> Result<String> {
    let (x, y, w, h) = roi.to_pixels(frame.width, frame.height);
    let cropped = crop_bgra(&frame.bgra, frame.width, x, y, w, h);
    let result = ocr_from_bgra(&cropped, w, h, Some(lang))?;
    Ok(result.text)
}

/// 黄色プレイヤー矢印 (▶) のピクセルを WIN/LOSE 各エリアでカウントする。
///
/// 黄色判定: R > 200, G > 170, B < 80
/// 戻り値: (win_count, lose_count, centroid_y_ratio, y_spread_px)
fn count_yellow_arrow_pixels(frame: &CapturedFrame) -> (u32, u32, f32, u32) {
    let w = frame.width;
    let h = frame.height;

    let x0    = (ARROW_X_START    * w as f32) as u32;
    let x1    = (ARROW_X_END      * w as f32) as u32;
    let y0    = (ARROW_Y_START    * h as f32) as u32;
    let y1    = (ARROW_Y_END      * h as f32) as u32;
    let y_mid = (PANEL_BOUNDARY_Y * h as f32) as u32;

    let mut win_count  = 0u32;
    let mut lose_count = 0u32;
    let mut sum_y      = 0u64;
    let mut total_px   = 0u32;
    let mut min_y_px   = u32::MAX;
    let mut max_y_px   = 0u32;

    for y in y0..y1.min(h) {
        for x in x0..x1.min(w) {
            let idx = ((y * w + x) * 4) as usize;
            if idx + 2 >= frame.bgra.len() { continue; }
            let b = frame.bgra[idx];
            let g = frame.bgra[idx + 1];
            let r = frame.bgra[idx + 2];

            if r > 200 && g > 170 && b < 80 {
                if y < y_mid { win_count += 1; } else { lose_count += 1; }
                sum_y    += y as u64;
                total_px += 1;
                if y < min_y_px { min_y_px = y; }
                if y > max_y_px { max_y_px = y; }
            }
        }
    }

    let centroid_y = if total_px > 0 {
        (sum_y / total_px as u64) as f32 / h as f32
    } else {
        0.5
    };
    let y_spread = if total_px > 0 { max_y_px - min_y_px } else { 0 };

    (win_count, lose_count, centroid_y, y_spread)
}

// ---------------------------------------------------------------------------
// ユーティリティ: BGRA8 クロップ
// ---------------------------------------------------------------------------

pub fn crop_bgra(bgra: &[u8], full_width: u32, x: u32, y: u32, w: u32, h: u32) -> Vec<u8> {
    let mut out = Vec::with_capacity((w * h * 4) as usize);
    for row in 0..h {
        let src_y = y + row;
        let row_start = ((src_y * full_width + x) * 4) as usize;
        let row_end   = row_start + (w * 4) as usize;
        if row_end <= bgra.len() {
            out.extend_from_slice(&bgra[row_start..row_end]);
        }
    }
    out
}

// ---------------------------------------------------------------------------
// テスト
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_crop_bgra_basic() {
        let mut data = vec![0u8; 4 * 4 * 4];
        let idx = ((2 * 4 + 2) * 4) as usize;
        data[idx + 2] = 255;
        data[idx + 3] = 255;
        let crop = crop_bgra(&data, 4, 2, 2, 2, 2);
        assert_eq!(crop.len(), 16);
        assert_eq!(crop[2], 255);
    }

    #[test]
    fn test_roi_to_pixels_1080p() {
        let (x, y, w, h) = WIN_ROI.to_pixels(1920, 1080);
        assert!(x > 800, "WIN ROI x={x}");
        assert!(y > 200, "WIN ROI y={y}");
        assert!(w > 0 && h > 0);
    }

    #[test]
    fn test_panel_boundary_pixels() {
        let y_mid = (PANEL_BOUNDARY_Y * 816.0) as u32;
        assert!(y_mid > 480, "boundary y={y_mid} too high");
        assert!(y_mid < 560, "boundary y={y_mid} too low");
    }

    #[test]
    fn test_yellow_pixel_threshold() {
        let (r, g, b) = (230u8, 190u8, 50u8);
        assert!(r > 200 && g > 170 && b < 80);
        let (r2, g2, b2) = (255u8, 255u8, 255u8);
        assert!(!(r2 > 200 && g2 > 170 && b2 < 80));
    }

    #[test]
    fn test_detector_initial_phase() {
        let det = ResultDetector::new(30);
        assert_eq!(det.phase, DetectorPhase::WaitingForBattle);
    }

    #[test]
    fn test_count_yellow_arrow_win_area() {
        let w = 100u32;
        let h = 100u32;
        let mut bgra = vec![0u8; (w * h * 4) as usize];
        let px_x = 47u32;
        let px_y = 35u32;
        for dy in 0..5u32 {
            let idx = (((px_y + dy) * w + px_x) * 4) as usize;
            bgra[idx]     = 30;
            bgra[idx + 1] = 190;
            bgra[idx + 2] = 230;
            bgra[idx + 3] = 255;
        }
        let frame = crate::capture::CapturedFrame { bgra, width: w, height: h };
        let (win_px, lose_px, centroid_y, _y_spread) = count_yellow_arrow_pixels(&frame);
        assert!(win_px >= 5, "expected win yellow pixels, got {win_px}");
        assert_eq!(lose_px, 0);
        assert!(centroid_y > 0.3 && centroid_y < 0.5, "centroid_y={centroid_y:.3}");
    }

    #[test]
    fn test_detection_result_helpers() {
        let win  = DetectionResult::Win  { arrow_y_ratio: 0.44 };
        let lose = DetectionResult::Lose { arrow_y_ratio: 0.72 };
        let none = DetectionResult::NotDetected;
        assert_eq!(win.result_str(),  Some("win"));
        assert_eq!(lose.result_str(), Some("lose"));
        assert_eq!(none.result_str(), None);
        assert!((win.arrow_y_ratio().unwrap()  - 0.44).abs() < 1e-5);
        assert!((lose.arrow_y_ratio().unwrap() - 0.72).abs() < 1e-5);
        assert!(none.arrow_y_ratio().is_none());
    }
}
