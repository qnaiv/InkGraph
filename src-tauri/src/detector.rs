/// IkaVision XP — リザルト検知モジュール
///
/// キャプチャフレームの ROI(WIN/LOSE 領域)に OCR をかけ、
/// リザルト画面を検知するロジックです。
/// 同一試合の重複検知を防ぐデバウンスも実装します。

use crate::{
    capture::CapturedFrame,
    ocr::ocr_from_bgra,
};
use anyhow::Result;
use std::time::{Duration, Instant};

// ---------------------------------------------------------------------------
// ROI 定義
// ---------------------------------------------------------------------------
//
// 全座標は 16:9 フレームに対する比率で定義します。
// スプラトゥーン3 リザルト画面のスクリーンショット実測値 (2026-05-27)。
// 解像度が変わっても to_pixels() が自動スケールします。
//
// 測定画像: バンカラマッチ(オープン) 1456×816 スクショ
// Xマッチでも WIN!/LOSE... バナーは同位置に表示されます。

/// WIN! バナー (ピンク帯) の領域
const WIN_ROI: Roi = Roi {
    x_ratio: 0.455,
    y_ratio: 0.267,
    w_ratio: 0.190,
    h_ratio: 0.065,
};

/// LOSE... バナー (緑帯) の領域
const LOSE_ROI: Roi = Roi {
    x_ratio: 0.455,
    y_ratio: 0.620,
    w_ratio: 0.190,
    h_ratio: 0.065,
};

/// ROI を相対比率で定義する構造体
#[derive(Debug, Clone, Copy)]
pub struct Roi {
    /// 左端 X (フレーム幅に対する比率)
    pub x_ratio: f32,
    /// 上端 Y (フレーム高さに対する比率)
    pub y_ratio: f32,
    /// 幅 (フレーム幅に対する比率)
    pub w_ratio: f32,
    /// 高さ (フレーム高さに対する比率)
    pub h_ratio: f32,
}

impl Roi {
    /// ピクセル座標に変換 (左上 x, y, 幅, 高さ)
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

/// リザルト検知の状態を保持する
pub struct ResultDetector {
    last_detected_at: Option<Instant>,
    /// 同一試合を重複検知しないクールダウン (秒)
    cooldown: Duration,
}

/// 検知結果
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
    /// WIN_ROI と LOSE_ROI をそれぞれ OCR して判定します。
    /// クールダウン中は NotDetected を返します。
    pub fn detect(&mut self, frame: &CapturedFrame) -> Result<DetectionResult> {
        // クールダウンチェック
        if let Some(last) = self.last_detected_at {
            if last.elapsed() < self.cooldown {
                return Ok(DetectionResult::NotDetected);
            }
        }

        // WIN バナーを確認
        let win_text = ocr_roi(frame, &WIN_ROI, "en-US")?;
        if win_text.contains("WIN") {
            self.record_detection();
            log::info!("[detector] WIN detected (text: {:?})", win_text);
            return Ok(DetectionResult::Win);
        }

        // LOSE バナーを確認
        let lose_text = ocr_roi(frame, &LOSE_ROI, "en-US")?;
        if lose_text.contains("LOSE") || lose_text.contains("LOSS") {
            self.record_detection();
            log::info!("[detector] LOSE detected (text: {:?})", lose_text);
            return Ok(DetectionResult::Lose);
        }

        Ok(DetectionResult::NotDetected)
    }

    fn record_detection(&mut self) {
        self.last_detected_at = Some(Instant::now());
    }
}

/// ROI を切り出して OCR し、大文字テキストを返す
fn ocr_roi(frame: &CapturedFrame, roi: &Roi, lang: &str) -> Result<String> {
    let (x, y, w, h) = roi.to_pixels(frame.width, frame.height);
    let cropped = crop_bgra(&frame.bgra, frame.width, x, y, w, h);
    let result = ocr_from_bgra(&cropped, w, h, Some(lang))?;
    Ok(result.text.to_uppercase())
}

// ---------------------------------------------------------------------------
// ユーティリティ: BGRA8 クロップ
// ---------------------------------------------------------------------------

/// BGRA8 バイト列の指定領域を切り出す
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
        data[idx] = 0;
        data[idx + 1] = 0;
        data[idx + 2] = 255; // R
        data[idx + 3] = 255; // A
        let crop = crop_bgra(&data, 4, 2, 2, 2, 2);
        assert_eq!(crop.len(), 16);
        assert_eq!(crop[2], 255);
    }

    #[test]
    fn test_roi_to_pixels_1080p() {
        let (x, y, w, h) = WIN_ROI.to_pixels(1920, 1080);
        // WIN バナーは画面中央右寄りのはず
        assert!(x > 800, "WIN ROI x={x} should be right of center");
        assert!(y > 200, "WIN ROI y={y} should be below 1/4 height");
        assert!(w > 0 && h > 0);
    }

    #[test]
    fn test_roi_to_pixels_816p() {
        let (x, y, w, h) = WIN_ROI.to_pixels(1456, 816);
        assert!(x > 600);
        assert!(y > 180);
        assert!(w > 0 && h > 0);
    }

    #[test]
    fn test_detector_initial_state() {
        let det = ResultDetector::new(30);
        assert!(det.last_detected_at.is_none());
    }
}
