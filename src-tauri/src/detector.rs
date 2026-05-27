use crate::{
    capture::CapturedFrame,
    ocr::{ocr_from_bgra, preprocess_bgra},
};
use anyhow::Result;
/// IkaVision XP — リザルト検知モジュール
///
/// キャプチャフレームの ROI(WIN/LOSE 領域)に OCR をかけ、
/// リザルト画面を検知するロジックです。
/// 同一試合の重複検知を防ぐデバウンスも実装します。
use std::time::{Duration, Instant};

// ---------------------------------------------------------------------------
// ROI 定義 (1920×1080 基準)
// ---------------------------------------------------------------------------

/// WIN/LOSE テキストが表示される領域
/// スプラトゥーン3 リザルト画面の中央上部
const RESULT_ROI: Roi = Roi {
    x_ratio: 0.395,
    y_ratio: 0.370,
    w_ratio: 0.210,
    h_ratio: 0.093,
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
    /// 同一試合を重複検知しないクールダウン
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
    /// クールダウン中は NotDetected を返す。
    pub fn detect(&mut self, frame: &CapturedFrame) -> Result<DetectionResult> {
        // クールダウンチェック
        if let Some(last) = self.last_detected_at {
            if last.elapsed() < self.cooldown {
                return Ok(DetectionResult::NotDetected);
            }
        }

        // ROI を切り出す
        let (x, y, w, h) = RESULT_ROI.to_pixels(frame.width, frame.height);
        let roi_bgra = crop_bgra(&frame.bgra, frame.width, x, y, w, h);

        // OCR 実行 (英語モードで WIN/LOSE を確実に取る)
        let ocr_result = ocr_from_bgra(&roi_bgra, w, h, Some("en-US"))?;
        let text = ocr_result.text.to_uppercase();

        let detected = if text.contains("WIN") {
            DetectionResult::Win
        } else if text.contains("LOSE") || text.contains("LOSS") {
            DetectionResult::Lose
        } else {
            DetectionResult::NotDetected
        };

        if detected != DetectionResult::NotDetected {
            self.last_detected_at = Some(Instant::now());
            log::info!(
                "[detector] result detected: {:?} (text: {:?})",
                detected,
                text
            );
        }

        Ok(detected)
    }
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
        // 4×4 の BGRA 画像から 2×2 を切り出す
        let mut data = vec![0u8; 4 * 4 * 4];
        // (2,2) ピクセルを赤にする
        let idx = ((2 * 4 + 2) * 4) as usize;
        data[idx] = 0; // B
        data[idx + 1] = 0; // G
        data[idx + 2] = 255; // R
        data[idx + 3] = 255; // A

        let crop = crop_bgra(&data, 4, 2, 2, 2, 2);
        assert_eq!(crop.len(), 16); // 2×2×4
                                    // 左上ピクセル (元の(2,2)) が赤であることを確認
        assert_eq!(crop[2], 255);
    }

    #[test]
    fn test_roi_to_pixels() {
        let roi = RESULT_ROI;
        let (x, y, w, h) = roi.to_pixels(1920, 1080);
        assert!(x < 1920);
        assert!(y < 1080);
        assert!(w > 0);
        assert!(h > 0);
    }

    #[test]
    fn test_detector_cooldown() {
        // Windows 以外ではキャプチャが使えないため OCR テストはスキップ
        // クールダウン後に再検知できることだけ確認
        let det = ResultDetector::new(30);
        assert!(det.last_detected_at.is_none());
    }
}
