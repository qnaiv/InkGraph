/// IkaVision XP — リザルト検知モジュール
///
/// 検知方式:
/// 1. WIN_ROI への OCR で「WIN!」テキストを確認 → リザルト画面か判定
/// 2. 黄色プレイヤー矢印（▶）をピクセルカラーでスキャン
/// 3. 矢印が WIN パネル側か LOSE パネル側かで勝敗を判定
/// 4. 矢印の y 重心を DetectionResult に含め extractor が行位置を特定できるようにする
///
/// ROI 座標はバンカラマッチ / Xマッチ 1456×816 スクリーンショットで実測 (2026-05-28)。

use crate::{
    capture::CapturedFrame,
    ocr::ocr_from_bgra,
};
use anyhow::Result;
use std::time::{Duration, Instant};

// ---------------------------------------------------------------------------
// デバッグ診断
// ---------------------------------------------------------------------------

/// フレームに対して検知パイプラインを一度だけ実行し、診断情報を返す。
/// クールダウンなし・副作用なし。
pub fn debug_detect_frame(frame: &CapturedFrame) -> Result<crate::types::CaptureDebugResult> {
    let win_roi_text = ocr_roi(frame, &WIN_ROI, "en-US")
        .unwrap_or_else(|e| format!("OCR_ERROR: {e}"));
    let win_text_found = win_roi_text.contains("WIN");
    let (yellow_win_px, yellow_lose_px, centroid_y) = count_yellow_arrow_pixels(frame);

    let detection_summary = if yellow_win_px >= MIN_YELLOW_PIXELS {
        format!("WIN (win_px={yellow_win_px}, centroid_y={centroid_y:.3})")
    } else if yellow_lose_px >= MIN_YELLOW_PIXELS {
        format!("LOSE (lose_px={yellow_lose_px}, centroid_y={centroid_y:.3})")
    } else {
        format!(
            "NOT_DETECTED — 黄色ピクセル不足 (win={yellow_win_px} lose={yellow_lose_px} < threshold={MIN_YELLOW_PIXELS})"
        )
    };

    Ok(crate::types::CaptureDebugResult {
        frame_w: frame.width,
        frame_h: frame.height,
        win_roi_text,
        win_text_found,
        yellow_win_px,
        yellow_lose_px,
        centroid_y,
        detection_summary,
    })
}

// ---------------------------------------------------------------------------
// ROI 定義 (16:9 フレームに対する比率)
// ---------------------------------------------------------------------------
//
// 実測: バンカラマッチ 1456×816 スクショ (2026-05-27)
// Xマッチ / バンカラ / ナワバリで共通レイアウト

/// WIN! バナー領域 — debug_detect_frame の診断用のみ。
/// WinRT OCR はスタイル化フォントを認識できないため detect() では使用しない。
const WIN_ROI: Roi = Roi {
    x_ratio: 0.455,
    y_ratio: 0.267,
    w_ratio: 0.190,
    h_ratio: 0.065,
};

/// プレイヤー矢印スキャン領域 (黄色 ▶ が表示される x 帯)
const ARROW_X_START: f32 = 0.455;
const ARROW_X_END:   f32 = 0.505;
const ARROW_Y_START: f32 = 0.270; // WIN パネル先頭行より上
const ARROW_Y_END:   f32 = 0.940; // LOSE パネル末尾行より下

/// WIN パネルと LOSE パネルの境界 y 比率
/// 実測: LOSE...ヘッダーは y ≈ 510px / 816px ≈ 0.625
/// 旧値 0.570 だと WIN 4番手行(y≈465px)が LOSE と誤判定されるため修正
const PANEL_BOUNDARY_Y: f32 = 0.630;

/// 黄色矢印と判定する最小ピクセル数 (ノイズ除去)
/// リザルト画面では 600+ px が観測されるため 50 で誤検知を防ぎつつ確実に検出できる
const MIN_YELLOW_PIXELS: u32 = 50;

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

pub struct ResultDetector {
    last_detected_at: Option<Instant>,
    cooldown: Duration,
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
    pub fn new(cooldown_secs: u64) -> Self {
        Self {
            last_detected_at: None,
            cooldown: Duration::from_secs(cooldown_secs),
        }
    }

    /// フレームから WIN/LOSE を検知する。
    ///
    /// 黄色プレイヤー矢印ピクセルスキャンのみで判定する。
    /// WIN_ROI OCR はスタイル化フォントを認識できないため使用しない。
    /// 誤検知防止は MIN_YELLOW_PIXELS (50) と cooldown (30s) で担保する。
    pub fn detect(&mut self, frame: &CapturedFrame) -> Result<DetectionResult> {
        // クールダウンチェック
        if let Some(last) = self.last_detected_at {
            if last.elapsed() < self.cooldown {
                return Ok(DetectionResult::NotDetected);
            }
        }

        let (win_px, lose_px, centroid_y) = count_yellow_arrow_pixels(frame);
        log::debug!("[detector] yellow pixels — win={win_px} lose={lose_px} centroid_y={centroid_y:.3}");

        let result = if win_px >= MIN_YELLOW_PIXELS {
            DetectionResult::Win  { arrow_y_ratio: centroid_y }
        } else if lose_px >= MIN_YELLOW_PIXELS {
            DetectionResult::Lose { arrow_y_ratio: centroid_y }
        } else {
            return Ok(DetectionResult::NotDetected);
        };

        self.last_detected_at = Some(Instant::now());
        log::info!(
            "[detector] {:?} detected (win_px={win_px} lose_px={lose_px} arrow_y={centroid_y:.3})",
            result.result_str()
        );
        Ok(result)
    }
}

// ---------------------------------------------------------------------------
// 内部ヘルパー
// ---------------------------------------------------------------------------

/// ROI を切り出して OCR し、大文字テキストを返す
fn ocr_roi(frame: &CapturedFrame, roi: &Roi, lang: &str) -> Result<String> {
    let (x, y, w, h) = roi.to_pixels(frame.width, frame.height);
    let cropped = crop_bgra(&frame.bgra, frame.width, x, y, w, h);
    let result = ocr_from_bgra(&cropped, w, h, Some(lang))?;
    Ok(result.text.to_uppercase())
}

/// 黄色プレイヤー矢印（▶）のピクセルを WIN/LOSE 各エリアでカウントし、
/// 全黄色ピクセルの y 重心比率も返す。
///
/// 黄色判定: R > 200, G > 170, B < 80
/// 戻り値: (win_count, lose_count, centroid_y_ratio)
fn count_yellow_arrow_pixels(frame: &CapturedFrame) -> (u32, u32, f32) {
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

    for y in y0..y1.min(h) {
        for x in x0..x1.min(w) {
            let idx = ((y * w + x) * 4) as usize;
            if idx + 2 >= frame.bgra.len() {
                continue;
            }
            let b = frame.bgra[idx];
            let g = frame.bgra[idx + 1];
            let r = frame.bgra[idx + 2];

            if r > 200 && g > 170 && b < 80 {
                if y < y_mid {
                    win_count += 1;
                } else {
                    lose_count += 1;
                }
                sum_y    += y as u64;
                total_px += 1;
            }
        }
    }

    let centroid_y = if total_px > 0 {
        (sum_y / total_px as u64) as f32 / h as f32
    } else {
        0.5
    };

    (win_count, lose_count, centroid_y)
}

// ---------------------------------------------------------------------------
// ユーティリティ: BGRA8 クロップ
// ---------------------------------------------------------------------------

pub fn crop_bgra(bgra: &[u8], full_width: u32, x: u32, y: u32, w: u32, h: u32) -> Vec<u8> {
    let mut out = Vec::with_capacity((w * h * 4) as usize);
    for row in 0..h {
        let src_y = y + row;
        let row_start = ((src_y * full_width + x) * 4) as usize;
        let row_end = row_start + (w * 4) as usize;
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
        data[idx + 2] = 255; // R
        data[idx + 3] = 255; // A
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
        // 1456×816 で LOSE...ヘッダーは y≈510px → boundary は 480〜560 の間
        let y_mid = (PANEL_BOUNDARY_Y * 816.0) as u32;
        assert!(y_mid > 480, "boundary y={y_mid} too high (WIN row4 ≈ 465px)");
        assert!(y_mid < 560, "boundary y={y_mid} too low (LOSE row1 ≈ 560px)");
    }

    #[test]
    fn test_yellow_pixel_threshold() {
        // 黄色判定: R>200 G>170 B<80
        let r: u8 = 230;
        let g: u8 = 190;
        let b: u8 = 50;
        assert!(r > 200 && g > 170 && b < 80, "yellow threshold check");

        // 非黄色 (白)
        let (r2, g2, b2) = (255u8, 255u8, 255u8);
        assert!(!(r2 > 200 && g2 > 170 && b2 < 80), "white should not be yellow");

        // 非黄色 (緑 = LOSE パネル背景)
        let (r3, g3, b3) = (100u8, 200u8, 80u8);
        assert!(!(r3 > 200 && g3 > 170 && b3 < 80), "green should not be yellow");
    }

    #[test]
    fn test_detector_initial_state() {
        let det = ResultDetector::new(30);
        assert!(det.last_detected_at.is_none());
    }

    #[test]
    fn test_count_yellow_arrow_win_area() {
        // WIN エリア (y < PANEL_BOUNDARY_Y=0.630) に黄色ピクセルを配置
        let w = 100u32;
        let h = 100u32;
        let mut bgra = vec![0u8; (w * h * 4) as usize];

        // ARROW_X_START*100=45..51, y=35 (< boundary*100=63)
        let px_x = 47u32;
        let px_y = 35u32;
        for dy in 0..5u32 {
            let idx = (((px_y + dy) * w + px_x) * 4) as usize;
            bgra[idx]     = 30;  // B
            bgra[idx + 1] = 190; // G
            bgra[idx + 2] = 230; // R
            bgra[idx + 3] = 255; // A
        }

        let frame = crate::capture::CapturedFrame { bgra, width: w, height: h };
        let (win_px, lose_px, centroid_y) = count_yellow_arrow_pixels(&frame);
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
