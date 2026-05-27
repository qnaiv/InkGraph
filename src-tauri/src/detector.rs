/// IkaVision XP — リザルト検知モジュール
///
/// 検知方式:
/// 1. WIN_ROI への OCR で「WIN!」テキストを確認 → リザルト画面か判定
/// 2. 黄色プレイヤー矢印（▶）をピクセルカラーでスキャン
/// 3. 矢印が WIN パネル側か LOSE パネル側かで勝敗を判定
///
/// OCR のみの方式だと WIN! LOSE... が常に両方表示されるため判定不可。
/// ピクセルスキャンは OCR より高速かつ誤認識がない。

use crate::{
    capture::CapturedFrame,
    ocr::ocr_from_bgra,
};
use anyhow::Result;
use std::time::{Duration, Instant};

// ---------------------------------------------------------------------------
// ROI 定義 (16:9 フレームに対する比率)
// ---------------------------------------------------------------------------
//
// 実測: バンカラマッチ 1456×816 スクショ (2026-05-27)
// Xマッチ / バンカラ / ナワバリで共通レイアウト

/// リザルト画面確認用: WIN! バナー (ピンク帯)
/// OCR で「WIN」テキストが取れればリザルト画面と判定する
const WIN_ROI: Roi = Roi {
    x_ratio: 0.455,
    y_ratio: 0.267,
    w_ratio: 0.190,
    h_ratio: 0.065,
};

/// プレイヤー矢印スキャン領域 (黄色 ▶ が表示される x 帯)
/// y 方向は WIN/LOSE 両パネルをカバー
const ARROW_X_START: f32 = 0.455;
const ARROW_X_END:   f32 = 0.500;
const ARROW_Y_START: f32 = 0.280; // WIN パネル内プレイヤー行の上端
const ARROW_Y_END:   f32 = 0.920; // LOSE パネル内プレイヤー行の下端

/// WIN パネルと LOSE パネルの境界 y 比率
/// この値より上 → WIN チーム / 以下 → LOSE チーム
const PANEL_BOUNDARY_Y: f32 = 0.570;

/// 黄色矢印と判定する最小ピクセル数 (ノイズ除去)
const MIN_YELLOW_PIXELS: u32 = 15;

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DetectionResult {
    Win,
    Lose,
    NotDetected,
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
    /// Step 1: WIN_ROI OCR で「WIN」テキスト確認 → リザルト画面ガード
    /// Step 2: 黄色矢印ピクセルスキャンでプレイヤーの勝敗を判定
    pub fn detect(&mut self, frame: &CapturedFrame) -> Result<DetectionResult> {
        // クールダウンチェック
        if let Some(last) = self.last_detected_at {
            if last.elapsed() < self.cooldown {
                return Ok(DetectionResult::NotDetected);
            }
        }

        // Step 1: リザルト画面確認 (WIN! テキストの存在チェック)
        let win_text = ocr_roi(frame, &WIN_ROI, "en-US")?;
        if !win_text.contains("WIN") {
            return Ok(DetectionResult::NotDetected);
        }

        // Step 2: 黄色矢印でプレイヤー結果を判定
        let (win_px, lose_px) = count_yellow_arrow_pixels(frame);
        log::debug!("[detector] yellow pixels — win_area={win_px} lose_area={lose_px}");

        let result = if win_px >= MIN_YELLOW_PIXELS {
            DetectionResult::Win
        } else if lose_px >= MIN_YELLOW_PIXELS {
            DetectionResult::Lose
        } else {
            // 矢印が見つからない: まだ画面が安定していない可能性
            return Ok(DetectionResult::NotDetected);
        };

        self.last_detected_at = Some(Instant::now());
        log::info!(
            "[detector] {:?} detected (win_px={win_px} lose_px={lose_px})",
            result
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

/// 黄色プレイヤー矢印（▶）のピクセルを WIN/LOSE 各エリアでカウントする
///
/// 黄色判定: R > 200, G > 170, B < 80
/// 矢印は ARROW_X_START〜X_END の細い帯に現れる
fn count_yellow_arrow_pixels(frame: &CapturedFrame) -> (u32, u32) {
    let w = frame.width;
    let h = frame.height;

    let x0    = (ARROW_X_START      * w as f32) as u32;
    let x1    = (ARROW_X_END        * w as f32) as u32;
    let y0    = (ARROW_Y_START      * h as f32) as u32;
    let y1    = (ARROW_Y_END        * h as f32) as u32;
    let y_mid = (PANEL_BOUNDARY_Y   * h as f32) as u32;

    let mut win_count  = 0u32;
    let mut lose_count = 0u32;

    for y in y0..y1.min(h) {
        for x in x0..x1.min(w) {
            let idx = ((y * w + x) * 4) as usize;
            if idx + 2 >= frame.bgra.len() {
                continue;
            }
            let b = frame.bgra[idx];
            let g = frame.bgra[idx + 1];
            let r = frame.bgra[idx + 2];

            // 黄色矢印の色 (スプラトゥーン3 の UI カラーに合わせた閾値)
            if r > 200 && g > 170 && b < 80 {
                if y < y_mid {
                    win_count += 1;
                } else {
                    lose_count += 1;
                }
            }
        }
    }

    (win_count, lose_count)
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
        // 1456×816 でパネル境界が正しい位置にあることを確認
        let y_mid = (PANEL_BOUNDARY_Y * 816.0) as u32;
        assert!(y_mid > 400, "boundary y={y_mid} too high");
        assert!(y_mid < 550, "boundary y={y_mid} too low");
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
        // WIN エリア (y < PANEL_BOUNDARY_Y) に黄色ピクセルを配置して Win を期待
        let w = 100u32;
        let h = 100u32;
        let mut bgra = vec![0u8; (w * h * 4) as usize];

        // x=48, y=30 (= ARROW_X_START*100=45..50, ARROW_Y_START*100=28..BOUNDARY*100=57)
        let px_x = 47u32;
        let px_y = 35u32; // y < boundary(57)
        for dy in 0..5u32 {
            let idx = (((px_y + dy) * w + px_x) * 4) as usize;
            bgra[idx]     = 30;  // B
            bgra[idx + 1] = 190; // G
            bgra[idx + 2] = 230; // R
            bgra[idx + 3] = 255; // A
        }

        let frame = crate::capture::CapturedFrame { bgra, width: w, height: h };
        let (win_px, lose_px) = count_yellow_arrow_pixels(&frame);
        assert!(win_px >= 5, "expected win yellow pixels, got {win_px}");
        assert_eq!(lose_px, 0);
    }
}
