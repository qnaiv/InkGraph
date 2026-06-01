/// InkGraph — リザルト検知モジュール
///
/// 検知方式 (2フェーズ):
///   Phase 1 (WaitingForBattle):
///     「バトルを開始します！」を検知 → DB に "in_progress" レコード作成 → InGame へ
///     検知手段: ①暗い巻物背景のピクセル判定 (高速) ②OCR フォールバック
///
///   Phase 2 (InGame):
///     リザルト画面を構造的に検知 → "in_progress" レコードを win/lose に更新
///     検知手段: ①プレイヤー行のグレー背景判定 (リザルト画面固有の構造)
///               ②黄色矢印の重心位置で WIN/LOSE 判定

use crate::{
    capture::CapturedFrame,
    extractor::{extract_rule_raw, extract_stage_raw, normalize_rule, normalize_stage},
    ocr::{ocr_from_bgra, preprocess_bgra},
};
use anyhow::Result;

// ---------------------------------------------------------------------------
// ROI 定義 (16:9 フレームに対する比率)
// ---------------------------------------------------------------------------

/// 「バトルを開始します！」テキスト領域
const BATTLE_START_ROI: Roi = Roi {
    x_ratio: 0.25,
    y_ratio: 0.36,
    w_ratio: 0.50,
    h_ratio: 0.20,
};

/// WIN! バナー領域 — debug_detect_frame の診断用のみ
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

/// 黄色矢印と判定する最小ピクセル数
const MIN_YELLOW_PIXELS: u32 = 100;

/// グレー行判定の輝度下限 (0–255)
const GREY_LUMINANCE_MIN: u16 = 180;
/// グレー行判定の彩度上限 (max_channel - min_channel)
const GREY_SATURATION_MAX: u8 = 40;
/// リザルト画面と判定するグレー行の最小数 (WIN 側 4行中 / LOSE 側 4行中それぞれ)
/// 両サイドで ≥2 行必要とすることで、バナー画面の誤検知を防ぐ
const RESULT_GREY_ROWS_MIN_PER_HALF: u32 = 2;

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
    /// バトル開始を待機中
    WaitingForBattle,
    /// バトル中 — リザルト画面を待機中
    InGame,
}

pub struct ResultDetector {
    phase: DetectorPhase,
}

/// 検知結果
#[derive(Debug, Clone)]
pub enum DetectionResult {
    /// Phase 1: バトル開始を検知 → "in_progress" レコードを作成すること
    BattleStarted,
    /// Phase 2: WIN を検知 → "in_progress" レコードを "win" に更新すること
    Win  { arrow_y_ratio: f32 },
    /// Phase 2: LOSE を検知 → "in_progress" レコードを "lose" に更新すること
    Lose { arrow_y_ratio: f32 },
    NotDetected,
}

impl DetectionResult {
    pub fn result_str(&self) -> Option<&'static str> {
        match self {
            Self::BattleStarted   => None,
            Self::Win  { .. }     => Some("win"),
            Self::Lose { .. }     => Some("lose"),
            Self::NotDetected     => None,
        }
    }

    pub fn arrow_y_ratio(&self) -> Option<f32> {
        match self {
            Self::Win  { arrow_y_ratio } | Self::Lose { arrow_y_ratio } => Some(*arrow_y_ratio),
            _ => None,
        }
    }
}

impl ResultDetector {
    pub fn new() -> Self {
        Self { phase: DetectorPhase::WaitingForBattle }
    }

    /// バトル開始を待機中か (YOLO パスで利用)
    pub fn is_waiting(&self) -> bool { self.phase == DetectorPhase::WaitingForBattle }

    /// バトル中 (リザルト待ち) か (YOLO パスで利用)
    pub fn is_in_game(&self) -> bool { self.phase == DetectorPhase::InGame }

    /// 強制的に WaitingForBattle へリセット (YOLO がリザルト検知した後に呼ぶ)
    pub fn reset_to_waiting(&mut self) {
        self.phase = DetectorPhase::WaitingForBattle;
        log::debug!("[detector] reset → WaitingForBattle");
    }

    /// 2フェーズ検知。detect() は Phase 1 で blocking OCR を呼ぶため、
    /// 呼び出し元は tokio::task::block_in_place でラップすること。
    pub fn detect(&mut self, frame: &CapturedFrame) -> Result<DetectionResult> {
        match self.phase {
            // ------------------------------------------------------------------
            // Phase 1: バトル開始検知
            // ------------------------------------------------------------------
            DetectorPhase::WaitingForBattle => {
                // OCR で「バトルを開始します！」テキストを検出
                // WinRT OCR は文字間スペースを挿入するため除去して照合
                // "開" が落ちる場合もあるので "始します" も補完キーワードとして使う
                let text = ocr_roi_raw(frame, &BATTLE_START_ROI, "ja-JP")
                    .unwrap_or_default()
                    .replace(char::is_whitespace, "");
                let battle_start = text.contains("バトル")
                    || text.contains("開始")
                    || text.contains("始します");

                if battle_start {
                    self.phase = DetectorPhase::InGame;
                    log::info!("[detector] battle start (OCR) → InGame");
                    return Ok(DetectionResult::BattleStarted);
                }
                Ok(DetectionResult::NotDetected)
            }

            // ------------------------------------------------------------------
            // Phase 2: リザルト画面検知
            // ------------------------------------------------------------------
            DetectorPhase::InGame => {
                // リザルト画面の構造的確認: WIN 側・LOSE 側それぞれで
                // プレイヤー行がグレー背景で並んでいるか確認する。
                // 両サイドで ≥2 行を要求することで、バトル開始直後の
                // バナー画面 (WIN 側が暗い) での誤検知を完全に排除できる。
                let (win_grey, lose_grey) = count_grey_rows(frame);
                if win_grey < RESULT_GREY_ROWS_MIN_PER_HALF || lose_grey < RESULT_GREY_ROWS_MIN_PER_HALF {
                    return Ok(DetectionResult::NotDetected);
                }

                // 黄色矢印の重心で WIN/LOSE を判定
                let (win_px, lose_px, centroid_y, _) = count_yellow_arrow_pixels(frame);
                if win_px < MIN_YELLOW_PIXELS && lose_px < MIN_YELLOW_PIXELS {
                    return Ok(DetectionResult::NotDetected);
                }

                self.phase = DetectorPhase::WaitingForBattle;
                let result = if win_px >= lose_px {
                    DetectionResult::Win  { arrow_y_ratio: centroid_y }
                } else {
                    DetectionResult::Lose { arrow_y_ratio: centroid_y }
                };
                log::info!(
                    "[detector] {:?} (win_grey={win_grey} lose_grey={lose_grey} win_px={win_px} lose_px={lose_px} y={centroid_y:.3})",
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
    // Phase 1
    let dark_scroll_found = detect_dark_scroll(frame);
    let battle_start_text = ocr_roi_raw(frame, &BATTLE_START_ROI, "ja-JP")
        .unwrap_or_else(|e| format!("OCR_ERROR: {e}"));
    let battle_start_clean = battle_start_text.replace(char::is_whitespace, "");
    let ocr_found = battle_start_clean.contains("バトル")
        || battle_start_clean.contains("開始")
        || battle_start_clean.contains("始します");
    let battle_start_found = dark_scroll_found || ocr_found;

    // Phase 2
    let win_roi_text = ocr_roi_raw(frame, &WIN_ROI, "en-US")
        .map(|t| t.to_uppercase())
        .unwrap_or_else(|e| format!("OCR_ERROR: {e}"));
    let win_text_found    = win_roi_text.contains("WIN");
    let (win_grey_rows, lose_grey_rows) = count_grey_rows(frame);
    let result_screen_ok  = win_grey_rows >= RESULT_GREY_ROWS_MIN_PER_HALF
                         && lose_grey_rows >= RESULT_GREY_ROWS_MIN_PER_HALF;
    let (yellow_win_px, yellow_lose_px, centroid_y, y_spread) = count_yellow_arrow_pixels(frame);

    let detection_summary = if battle_start_found {
        let m = if dark_scroll_found { "暗巻物" } else { "OCR" };
        format!("Phase 1 ✓ バトル開始 ({m}) → InGame")
    } else if !result_screen_ok {
        format!("Phase 2: NOT_RESULT_SCREEN (WIN側グレー={win_grey_rows}/4 LOSE側={lose_grey_rows}/4 最小{RESULT_GREY_ROWS_MIN_PER_HALF})")
    } else if yellow_win_px >= MIN_YELLOW_PIXELS {
        format!("Phase 2 ✓ WIN (WIN_grey={win_grey_rows} LOSE_grey={lose_grey_rows} win_px={yellow_win_px} y={centroid_y:.3})")
    } else if yellow_lose_px >= MIN_YELLOW_PIXELS {
        format!("Phase 2 ✓ LOSE (WIN_grey={win_grey_rows} LOSE_grey={lose_grey_rows} lose_px={yellow_lose_px} y={centroid_y:.3})")
    } else {
        format!("Phase 2: NOT_DETECTED (WIN_grey={win_grey_rows} LOSE_grey={lose_grey_rows} win_px={yellow_win_px} lose_px={yellow_lose_px})")
    };

    // Phase 2: ルール・ステージ OCR (リザルト画面上部から読む)
    let rule_ocr_text  = extract_rule_raw(frame);
    let stage_ocr_text = extract_stage_raw(frame);
    let rule_normalized  = normalize_rule(&rule_ocr_text);
    let stage_normalized = normalize_stage(&stage_ocr_text);

    Ok(crate::types::CaptureDebugResult {
        frame_w: frame.width,
        frame_h: frame.height,
        battle_start_text,
        battle_start_found,
        dark_scroll_found,
        win_roi_text,
        win_text_found,
        win_grey_rows,
        lose_grey_rows,
        rule_ocr_text,
        rule_normalized,
        stage_ocr_text,
        stage_normalized,
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

/// ROI を切り出して Otsu 二値化し OCR する
fn ocr_roi_raw(frame: &CapturedFrame, roi: &Roi, lang: &str) -> Result<String> {
    let (x, y, w, h) = roi.to_pixels(frame.width, frame.height);
    let cropped   = crop_bgra(&frame.bgra, frame.width, x, y, w, h);
    let processed = preprocess_bgra(&cropped, w, h);
    Ok(ocr_from_bgra(&processed, w, h, Some(lang))?.text)
}

/// 暗い巻物背景をピクセル輝度で検出する（診断用・detect() では使用しない）。
/// BATTLE_START_ROI 内で輝度 < 30 のピクセルが 40% 以上あれば true。
fn detect_dark_scroll(frame: &CapturedFrame) -> bool {
    let (x0, y0, w, h) = BATTLE_START_ROI.to_pixels(frame.width, frame.height);
    let mut dark = 0u32;
    let mut total = 0u32;
    for row in 0..h {
        for col in 0..w {
            let idx = (((y0 + row) * frame.width + x0 + col) * 4) as usize;
            if idx + 2 >= frame.bgra.len() { continue; }
            let b = frame.bgra[idx]     as u32;
            let g = frame.bgra[idx + 1] as u32;
            let r = frame.bgra[idx + 2] as u32;
            total += 1;
            if (299 * r + 587 * g + 114 * b) / 1000 < 30 { dark += 1; }
        }
    }
    total > 0 && dark * 100 / total >= 40
}

/// リザルト画面のプレイヤー行をグレー背景で検出する。
///
/// WIN 側 4行・LOSE 側 4行を独立してカウントし `(win_grey, lose_grey)` で返す。
/// 両サイドで ≥2 を要求することで、バトル開始直後バナー画面
/// (WIN 側中央が暗い墨色 → win_grey=0) での誤検知を排除する。
///
/// 各行: x = [0.55, 0.65, 0.75] の 3点をサンプリングし、
/// 輝度 ≥ 180 かつ 彩度 ≤ 40 のサンプルが 2点以上なら「グレー行」と判定。
fn count_grey_rows(frame: &CapturedFrame) -> (u32, u32) {
    // WIN 側 4行 (リザルト画面上半分)
    const WIN_ROW_Y:  [f32; 4] = [0.324, 0.411, 0.499, 0.586];
    // LOSE 側 4行 (リザルト画面下半分)
    const LOSE_ROW_Y: [f32; 4] = [0.669, 0.746, 0.824, 0.901];
    // プレイヤー統計列 (キル/デス/XP が白背景で並ぶ領域)
    const SAMPLE_X: [f32; 3] = [0.55, 0.65, 0.75];

    let sample_grey = |y_r: f32| -> bool {
        let y = (y_r * frame.height as f32) as u32;
        let mut grey_samples = 0u32;
        for &x_r in &SAMPLE_X {
            let x   = (x_r * frame.width as f32) as u32;
            let idx = ((y * frame.width + x) * 4) as usize;
            if idx + 2 >= frame.bgra.len() { continue; }
            let b = frame.bgra[idx];
            let g = frame.bgra[idx + 1];
            let r = frame.bgra[idx + 2];
            let lum = (r as u16 + g as u16 + b as u16) / 3;
            let sat = r.max(g).max(b) - r.min(g).min(b);
            if lum >= GREY_LUMINANCE_MIN && sat <= GREY_SATURATION_MAX {
                grey_samples += 1;
            }
        }
        grey_samples >= 2
    };

    let win_grey  = WIN_ROW_Y.iter().filter(|&&y| sample_grey(y)).count() as u32;
    let lose_grey = LOSE_ROW_Y.iter().filter(|&&y| sample_grey(y)).count() as u32;
    (win_grey, lose_grey)
}

/// 黄色プレイヤー矢印 (▶) のピクセルを WIN/LOSE 各エリアでカウントする。
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
    } else { 0.5 };
    let y_spread = if total_px > 0 { max_y_px - min_y_px } else { 0 };
    (win_count, lose_count, centroid_y, y_spread)
}

// ---------------------------------------------------------------------------
// ユーティリティ: BGRA8 クロップ
// ---------------------------------------------------------------------------

pub fn crop_bgra(bgra: &[u8], full_width: u32, x: u32, y: u32, w: u32, h: u32) -> Vec<u8> {
    let mut out = Vec::with_capacity((w * h * 4) as usize);
    for row in 0..h {
        let src_y     = y + row;
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
        let det = ResultDetector::new();
        assert_eq!(det.phase, DetectorPhase::WaitingForBattle);
    }

    #[test]
    fn test_grey_row_detection_all_white() {
        // 全白フレームでは WIN 側 4行・LOSE 側 4行ともすべてグレー行
        let w = 1920u32;
        let h = 1080u32;
        let bgra = vec![255u8; (w * h * 4) as usize];
        let frame = crate::capture::CapturedFrame { bgra, width: w, height: h };
        let (win_grey, lose_grey) = count_grey_rows(&frame);
        assert_eq!(win_grey,  4, "all-white: expected 4 win grey rows, got {win_grey}");
        assert_eq!(lose_grey, 4, "all-white: expected 4 lose grey rows, got {lose_grey}");
    }

    #[test]
    fn test_grey_row_detection_all_colored() {
        // 鮮やかな黄色フレームではどちらの側もグレー行なし
        let w = 1920u32;
        let h = 1080u32;
        let mut bgra = vec![0u8; (w * h * 4) as usize];
        for i in 0..(w * h) as usize {
            bgra[i * 4]     = 30;  // B
            bgra[i * 4 + 1] = 200; // G
            bgra[i * 4 + 2] = 240; // R  → 高彩度
            bgra[i * 4 + 3] = 255;
        }
        let frame = crate::capture::CapturedFrame { bgra, width: w, height: h };
        let (win_grey, lose_grey) = count_grey_rows(&frame);
        assert_eq!(win_grey,  0, "saturated yellow: expected 0 win grey rows");
        assert_eq!(lose_grey, 0, "saturated yellow: expected 0 lose grey rows");
    }

    #[test]
    fn test_grey_row_detection_banner_screen_simulation() {
        // バナー画面シミュレーション: WIN 側 (y<0.63) が暗い墨色、LOSE 側は明るい
        // → win_grey=0 → 閾値未達でリザルト画面と判定されない
        let w = 1920u32;
        let h = 1080u32;
        let mut bgra = vec![0u8; (w * h * 4) as usize];
        for py in 0..h {
            for px in 0..w {
                let idx = ((py * w + px) * 4) as usize;
                let y_r = py as f32 / h as f32;
                if y_r < 0.63 {
                    // WIN 側: 暗い墨色 (輝度 < 60) → グレー行条件を満たさない
                    bgra[idx]     = 20; // B
                    bgra[idx + 1] = 25; // G
                    bgra[idx + 2] = 15; // R
                } else {
                    // LOSE 側: 明るいグレー
                    bgra[idx]     = 200; // B
                    bgra[idx + 1] = 205; // G
                    bgra[idx + 2] = 200; // R
                }
                bgra[idx + 3] = 255;
            }
        }
        let frame = crate::capture::CapturedFrame { bgra, width: w, height: h };
        let (win_grey, lose_grey) = count_grey_rows(&frame);
        assert_eq!(win_grey, 0, "banner screen WIN side should have 0 grey rows");
        assert!(lose_grey >= 2, "banner screen LOSE side may have grey rows: {lose_grey}");
        // 両サイド ≥2 の条件を満たさない → 誤検知しない
        assert!(
            win_grey < 2 || lose_grey < 2,
            "banner screen should NOT pass both-half threshold (win={win_grey} lose={lose_grey})"
        );
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
        let (win_px, lose_px, centroid_y, _) = count_yellow_arrow_pixels(&frame);
        assert!(win_px >= 5, "expected win yellow pixels, got {win_px}");
        assert_eq!(lose_px, 0);
        assert!(centroid_y > 0.3 && centroid_y < 0.5, "centroid_y={centroid_y:.3}");
    }

    #[test]
    fn test_detection_result_helpers() {
        let started = DetectionResult::BattleStarted;
        let win     = DetectionResult::Win  { arrow_y_ratio: 0.44 };
        let lose    = DetectionResult::Lose { arrow_y_ratio: 0.72 };
        let none    = DetectionResult::NotDetected;
        assert_eq!(started.result_str(), None);
        assert_eq!(win.result_str(),     Some("win"));
        assert_eq!(lose.result_str(),    Some("lose"));
        assert_eq!(none.result_str(),    None);
        assert!((win.arrow_y_ratio().unwrap()  - 0.44).abs() < 1e-5);
        assert!((lose.arrow_y_ratio().unwrap() - 0.72).abs() < 1e-5);
        assert!(started.arrow_y_ratio().is_none());
    }
}

// ===========================================================================
// YOLO/ONNX 推論エンジン
// ===========================================================================
//
// 使用モデル: YOLOv8 Nano (ユーザーが学習・配置)
// モデルパス: <app_dir>/assets/models/yolo_result.onnx
//
// 前提:
//   - ort 2.x (load-dynamic): onnxruntime.dll が実行時に PATH 上にあること
//   - モデルはリザルト画面用クラスで学習済みであること
//   - 推論は ResultDetector が ResultScreen 状態に遷移したときのみ呼び出す
//
// 座標系:
//   Detection.bbox は元フレームに対する正規化座標 [0, 1]。
//   capture_loop で frame.width / frame.height を掛けてピクセル座標へ変換する。

use crate::preprocess::letterbox_bgra;
use ndarray::Array4;
use ort::{session::{Session, builder::GraphOptimizationLevel}, value::Tensor};
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// クラス定義
// ---------------------------------------------------------------------------

/// YOLO モデルが検出するクラス (yolo_result.onnx の学習クラス順と一致させること)
///
/// 【設計方針】
/// - BattleStart: 「試合を開始します」テキスト → バトル開始トリガー
/// - Win / Lose / Draw: WIN!/LOSE!/DRAW! バナー → 勝敗判定 (y 座標不要)
/// - MyArrow: 自分の黄色 ▶ マーカー → KDA 行の y 基準座標を取得
/// - GoldAward: 金表彰アイコン (検出数 = 取得した金表彰の枚数)
/// - KillLog: 試合中のキルログ通知 (将来的なリアルタイムキル追跡用)
/// - RuleText / StageText / ModeText: テキスト BBox → OCR 用
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[repr(usize)]
pub enum YoloClass {
    BattleStart = 0,
    GoldAward   = 1,
    KillLog     = 2,
    Win         = 3,
    Lose        = 4,
    Draw        = 5,
    ModeText    = 6,
    MyArrow     = 7,
    RuleText    = 8,
    StageText   = 9,
}

impl YoloClass {
    pub fn from_id(id: usize) -> Option<Self> {
        match id {
            0 => Some(Self::BattleStart),
            1 => Some(Self::GoldAward),
            2 => Some(Self::KillLog),
            3 => Some(Self::Win),
            4 => Some(Self::Lose),
            5 => Some(Self::Draw),
            6 => Some(Self::ModeText),
            7 => Some(Self::MyArrow),
            8 => Some(Self::RuleText),
            9 => Some(Self::StageText),
            _ => None,
        }
    }

    pub fn num_classes() -> usize { 10 }
}

// ---------------------------------------------------------------------------
// 検出結果構造体
// ---------------------------------------------------------------------------

/// YOLOv8 の 1検出エントリ
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BBox {
    /// 左端 (正規化 [0,1])
    pub x1: f32,
    /// 上端 (正規化 [0,1])
    pub y1: f32,
    /// 右端 (正規化 [0,1])
    pub x2: f32,
    /// 下端 (正規化 [0,1])
    pub y2: f32,
}

impl BBox {
    pub fn width(&self)  -> f32 { self.x2 - self.x1 }
    pub fn height(&self) -> f32 { self.y2 - self.y1 }

    /// IoU (Intersection over Union)
    pub fn iou(&self, other: &BBox) -> f32 {
        let ix1 = self.x1.max(other.x1);
        let iy1 = self.y1.max(other.y1);
        let ix2 = self.x2.min(other.x2);
        let iy2 = self.y2.min(other.y2);
        let inter = (ix2 - ix1).max(0.0) * (iy2 - iy1).max(0.0);
        let union = self.width() * self.height() + other.width() * other.height() - inter;
        if union <= 0.0 { 0.0 } else { inter / union }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Detection {
    pub bbox:       BBox,
    pub class_id:   usize,
    pub class_name: String,
    pub confidence: f32,
}

// ---------------------------------------------------------------------------
// YoloDetector
// ---------------------------------------------------------------------------

/// YOLO セッションの ONNX 入力サイズ (正方形)
const YOLO_INPUT_SIZE: u32 = 640;

/// 確信度の下限。これ未満の検出はノイズとして破棄する
const DEFAULT_CONF_THRESHOLD: f32 = 0.70;

/// NMS の IoU 閾値
const DEFAULT_IOU_THRESHOLD: f32 = 0.45;

pub struct YoloDetector {
    session:        Option<Session>,
    model_path:     PathBuf,
    conf_threshold: f32,
    iou_threshold:  f32,
}

impl YoloDetector {
    /// `model_path`: `assets/models/yolo_result.onnx` への絶対パスを渡すこと。
    pub fn new(model_path: impl Into<PathBuf>) -> Self {
        Self {
            session:        None,
            model_path:     model_path.into(),
            conf_threshold: DEFAULT_CONF_THRESHOLD,
            iou_threshold:  DEFAULT_IOU_THRESHOLD,
        }
    }

    pub fn is_loaded(&self) -> bool { self.session.is_some() }

    /// モデルをロードし ort セッションを初期化する。
    /// `load-dynamic` feature では、onnxruntime.dll が
    /// PATH または実行ファイルと同ディレクトリにある必要がある。
    pub fn load(&mut self) -> Result<()> {
        if !self.model_path.exists() {
            anyhow::bail!(
                "YOLO model not found: {}  (place yolo_result.onnx here)",
                self.model_path.display()
            );
        }

        let session = Session::builder()?
            .with_optimization_level(GraphOptimizationLevel::Level3)?
            .with_intra_threads(2)?
            .commit_from_file(&self.model_path)?;

        log::info!("[yolo] model loaded: {}", self.model_path.display());
        self.session = Some(session);
        Ok(())
    }

    /// フレームに対して推論を実行し、確信度閾値を超えた `Detection` 一覧を返す。
    ///
    /// モデル未ロード時は即座に `Ok(vec![])` を返す (ロード前の呼び出しを安全に無視)。
    pub fn detect(&self, frame: &CapturedFrame) -> Result<Vec<Detection>> {
        let session = match &self.session {
            Some(s) => s,
            None    => return Ok(vec![]),
        };

        // 1. 前処理: BGRA → レターボックスリサイズ → CHW f32 テンソル
        let (chw, params) = letterbox_bgra(
            &frame.bgra, frame.width, frame.height, YOLO_INPUT_SIZE,
        )?;

        // ndarray: [1, 3, 640, 640]
        let input = Array4::from_shape_vec(
            (1, 3, YOLO_INPUT_SIZE as usize, YOLO_INPUT_SIZE as usize),
            chw,
        )?;

        // 2. 推論実行
        let input_tensor = Tensor::<f32>::from_array(input)?;
        let outputs = session.run(ort::inputs!["images" => input_tensor])?;

        // 3. 出力パース
        // YOLOv8 output shape: [1, 4+num_classes, 8400]
        let output_tensor = outputs["output0"].extract_tensor::<f32>()?;
        let view = output_tensor.view();

        let num_classes  = YoloClass::num_classes();
        let num_anchors  = view.shape()[2]; // 8400

        let mut raw_detections: Vec<Detection> = Vec::new();

        for anchor_idx in 0..num_anchors {
            // cx, cy, w, h (in YOLO input pixel space)
            let cx = view[[0, 0, anchor_idx]];
            let cy = view[[0, 1, anchor_idx]];
            let bw = view[[0, 2, anchor_idx]];
            let bh = view[[0, 3, anchor_idx]];

            // クラス確信度の最大値とそのクラス id を探す
            let mut max_conf   = 0f32;
            let mut max_class  = 0usize;
            for c in 0..num_classes {
                let score = view[[0, 4 + c, anchor_idx]];
                if score > max_conf {
                    max_conf  = score;
                    max_class = c;
                }
            }

            if max_conf < self.conf_threshold { continue; }

            // 座標を元フレームの正規化座標へ変換
            let (x1, y1, x2, y2) = params.to_normalized(cx, cy, bw, bh);

            raw_detections.push(Detection {
                bbox: BBox { x1, y1, x2, y2 },
                class_id:   max_class,
                class_name: format!("{:?}", YoloClass::from_id(max_class)
                    .unwrap_or(YoloClass::BattleStart)),
                confidence: max_conf,
            });
        }

        // 4. クラスごとに NMS を適用
        let detections = nms_per_class(raw_detections, self.iou_threshold);

        log::debug!(
            "[yolo] detect: {} detections (conf>{:.2})",
            detections.len(), self.conf_threshold
        );
        Ok(detections)
    }

    /// 特定クラスの最高確信度検出を 1件返す便利メソッド。
    pub fn best_detection<'a>(
        detections: &'a [Detection],
        class: YoloClass,
    ) -> Option<&'a Detection> {
        detections
            .iter()
            .filter(|d| d.class_id == class as usize)
            .max_by(|a, b| a.confidence.partial_cmp(&b.confidence).unwrap())
    }
}

// ---------------------------------------------------------------------------
// NMS (Non-Maximum Suppression) — クラスごと独立適用
// ---------------------------------------------------------------------------

fn nms_per_class(detections: Vec<Detection>, iou_threshold: f32) -> Vec<Detection> {
    let max_class = detections.iter().map(|d| d.class_id).max().unwrap_or(0);
    let mut result: Vec<Detection> = Vec::new();

    for class_id in 0..=max_class {
        let mut class_dets: Vec<&Detection> = detections
            .iter()
            .filter(|d| d.class_id == class_id)
            .collect();

        // 確信度の降順にソート
        class_dets.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap());

        let mut kept: Vec<BBox> = Vec::new();
        for det in class_dets {
            // kept の各 BBox と IoU を比較; 重複していれば破棄
            let overlaps = kept.iter().any(|k| det.bbox.iou(k) > iou_threshold);
            if !overlaps {
                kept.push(BBox {
                    x1: det.bbox.x1,
                    y1: det.bbox.y1,
                    x2: det.bbox.x2,
                    y2: det.bbox.y2,
                });
                result.push(Detection {
                    bbox:       BBox { x1: det.bbox.x1, y1: det.bbox.y1, x2: det.bbox.x2, y2: det.bbox.y2 },
                    class_id:   det.class_id,
                    class_name: det.class_name.clone(),
                    confidence: det.confidence,
                });
            }
        }
    }
    result
}

// ---------------------------------------------------------------------------
// YoloDetector テスト
// ---------------------------------------------------------------------------

#[cfg(test)]
mod yolo_tests {
    use super::*;

    #[test]
    fn test_bbox_iou_identical() {
        let b = BBox { x1: 0.1, y1: 0.1, x2: 0.5, y2: 0.5 };
        assert!((b.iou(&b) - 1.0).abs() < 1e-5, "IoU of identical bbox should be 1.0");
    }

    #[test]
    fn test_bbox_iou_no_overlap() {
        let a = BBox { x1: 0.0, y1: 0.0, x2: 0.2, y2: 0.2 };
        let b = BBox { x1: 0.8, y1: 0.8, x2: 1.0, y2: 1.0 };
        assert_eq!(a.iou(&b), 0.0, "non-overlapping boxes should have IoU=0");
    }

    #[test]
    fn test_yolo_class_num() {
        assert_eq!(YoloClass::num_classes(), 10);
    }

    #[test]
    fn test_yolo_class_from_id() {
        assert_eq!(YoloClass::from_id(0), Some(YoloClass::BattleStart));
        assert_eq!(YoloClass::from_id(3), Some(YoloClass::Win));
        assert_eq!(YoloClass::from_id(7), Some(YoloClass::MyArrow));
        assert_eq!(YoloClass::from_id(9), Some(YoloClass::StageText));
        assert!(YoloClass::from_id(99).is_none());
    }

    #[test]
    fn test_yolo_detector_not_loaded_returns_empty() {
        let detector = YoloDetector::new("/nonexistent/path/model.onnx");
        assert!(!detector.is_loaded());
        let frame = crate::capture::CapturedFrame {
            bgra: vec![0u8; 4 * 4 * 4],
            width: 4,
            height: 4,
        };
        let result = detector.detect(&frame).unwrap();
        assert!(result.is_empty(), "unloaded detector should return empty vec");
    }

    #[test]
    fn test_nms_removes_duplicate() {
        let dets = vec![
            Detection {
                bbox: BBox { x1: 0.1, y1: 0.1, x2: 0.5, y2: 0.5 },
                class_id: 0, class_name: "win".to_string(), confidence: 0.95,
            },
            Detection {
                // 上と完全重複、確信度が低い方
                bbox: BBox { x1: 0.1, y1: 0.1, x2: 0.5, y2: 0.5 },
                class_id: 0, class_name: "win".to_string(), confidence: 0.80,
            },
        ];
        let kept = nms_per_class(dets, 0.45);
        assert_eq!(kept.len(), 1, "NMS should keep only 1 of 2 identical boxes");
        assert!((kept[0].confidence - 0.95).abs() < 1e-5, "should keep higher confidence");
    }

    #[test]
    fn test_nms_keeps_different_classes() {
        let dets = vec![
            Detection {
                bbox: BBox { x1: 0.1, y1: 0.1, x2: 0.5, y2: 0.5 },
                class_id: 0, class_name: "win".to_string(), confidence: 0.90,
            },
            Detection {
                bbox: BBox { x1: 0.1, y1: 0.1, x2: 0.5, y2: 0.5 },
                class_id: 2, class_name: "rule".to_string(), confidence: 0.85,
            },
        ];
        let kept = nms_per_class(dets, 0.45);
        assert_eq!(kept.len(), 2, "NMS is per-class; different classes should both be kept");
    }
}
