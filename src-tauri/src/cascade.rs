/// InkGraph — カスケード（2段階）YOLO 推論モジュール
///
/// Step 1: Model 1 の MyArrow BBox を元に画像をクロップ
/// Step 2: Model 2 でクロップ画像内の数字・アイコンを検出
/// Step 3: 検出結果を x_center 昇順にソート
/// Step 4: アイコン座標を基準点として数字を4グループに振り分け、整数値にパース

use crate::{
    capture::CapturedFrame,
    detector::{crop_bgra, Detection, YoloDetector},
};
use anyhow::Result;
use std::{cmp::Ordering, path::PathBuf};

// ---------------------------------------------------------------------------
// 定数
// ---------------------------------------------------------------------------

/// MyArrow y_center の上下に確保するクロップ幅 (px)
const CROP_HALF_H: u32 = 50;

/// スタッツ領域の開始位置 (画面幅に対する比率)
/// 塗りポイント〜SP カラムはすべてこの位置より右にある
const STATS_X_START: f32 = 0.45;

/// スタッツ領域の終了位置 (画面幅に対する比率)
/// SP カウント右端の直後で切る。ここより右は黒背景のみで学習データに含まれていない。
/// 余計な黒余白を含めるとストレッチ後にアイコン位置が学習時とずれて検出精度が落ちる。
const STATS_X_END: f32 = 0.86;

/// 同一位置とみなす x 距離（正規化 [0,1]）。
/// 同位置に複数クラスが検出された場合、最高確信度のみ残す。
const X_DEDUP_TOL: f32 = 0.015;

// ---------------------------------------------------------------------------
// Model 2 クラス名
// ---------------------------------------------------------------------------

/// Model 2 のクラス名一覧。
/// Roboflow の data.yaml アルファベット順と一致させること。
pub const STATS_CLASS_NAMES: &[&str] = &[
    "digit_0",
    "digit_1",
    "digit_2",
    "digit_3",
    "digit_4",
    "digit_5",
    "digit_6",
    "digit_7",
    "digit_8",
    "digit_9",
    "icon_death",
    "icon_kill",
    "icon_special",
];

// ---------------------------------------------------------------------------
// 出力型
// ---------------------------------------------------------------------------

/// カスケード推論で得られた1プレイヤー分のスタッツ
#[derive(Debug, Clone)]
pub struct PlayerStats {
    pub paint:   Option<i64>,
    pub kill:    Option<i64>,
    pub death:   Option<i64>,
    pub special: Option<i64>,
}

// ---------------------------------------------------------------------------
// StatsDetector
// ---------------------------------------------------------------------------

/// Model 2 (yolo_stats.onnx) を保持し、カスケード推論を実行する。
pub struct StatsDetector {
    yolo: YoloDetector,
}

impl StatsDetector {
    pub fn new(model_path: impl Into<PathBuf>) -> Self {
        let class_names: Vec<String> =
            STATS_CLASS_NAMES.iter().map(|s| s.to_string()).collect();
        let mut yolo = YoloDetector::new_with_classes(model_path, class_names);
        // Roboflow はデフォルトで "Stretch" リサイズで学習するため、
        // レターボックスではなくストレッチを使って前処理を一致させる。
        yolo.use_stretch = true;
        Self { yolo }
    }

    pub fn load(&mut self) -> Result<()> {
        self.yolo.load()
    }

    pub fn is_loaded(&self) -> bool {
        self.yolo.is_loaded()
    }

    /// Steps 1〜4 を実行して PlayerStats を返す。
    ///
    /// `arrow`: Model 1 が検出した MyArrow Detection。
    pub fn run_cascade(
        &mut self,
        frame: &CapturedFrame,
        arrow: &Detection,
    ) -> Result<PlayerStats> {
        // Step 1: MyArrow y_center ±CROP_HALF_H px、STATS_X_START ≤ x ≤ STATS_X_END でクロップ
        // 右側の余分な黒背景を除くことで、ストレッチ後のアイコン位置を学習データと一致させる
        let y_px =
            ((arrow.bbox.y1 + arrow.bbox.y2) / 2.0 * frame.height as f32) as u32;
        let crop_y = y_px.saturating_sub(CROP_HALF_H);
        let crop_h = (CROP_HALF_H * 2).min(frame.height.saturating_sub(crop_y));
        let crop_x = (frame.width as f32 * STATS_X_START) as u32;
        let crop_right = ((frame.width as f32 * STATS_X_END) as u32).min(frame.width);
        let crop_w = crop_right.saturating_sub(crop_x);

        let cropped = crop_bgra(&frame.bgra, frame.width, crop_x, crop_y, crop_w, crop_h);

        // Step 2: Model 2 推論
        let mut dets = self.yolo.detect_bgra(&cropped, crop_w, crop_h)?;

        log::debug!(
            "[cascade] crop=({crop_x},{crop_y},{crop_w}x{crop_h}), detections={}",
            dets.len()
        );

        // Step 3: x_center 昇順ソート
        dets.sort_by(|a, b| {
            let cx = |d: &Detection| (d.bbox.x1 + d.bbox.x2) / 2.0;
            cx(a).partial_cmp(&cx(b)).unwrap_or(Ordering::Equal)
        });

        // Steps 3–4: アンカー特定 → クラスタリング → パース
        // dets は detect_bgra 時点で 0.50 フィルタ済みなので digit_min_conf=0.0 でよい
        Ok(cluster_and_parse(&dets, 0.0))
    }
}

// ---------------------------------------------------------------------------
// クラスタリング・パース（純粋関数 — 単体テスト可）
// ---------------------------------------------------------------------------

/// ソート済み検出結果からアンカーを特定し、4グループにパースする。
///
/// `digit_min_conf`: デジット収集に使う最低確信度。
///   0.0  → 全候補を使用（本番は detect_bgra 時点で既にフィルタ済み）
///   0.50 → デバッグ時、低確信度ノイズを除外して本番相当の集計にする
///
/// アイコン境界は `dets` 全体から探す（低確信度アイコンも境界として利用可）。
pub fn cluster_and_parse(dets: &[Detection], digit_min_conf: f32) -> PlayerStats {
    let kill_lo    = icon_left_x(dets, "icon_kill");
    let death_lo   = icon_left_x(dets, "icon_death");
    let special_lo = icon_left_x(dets, "icon_special");

    log::debug!(
        "[cascade] boundaries (icon x1): kill={:?} death={:?} special={:?}",
        kill_lo, death_lo, special_lo
    );

    let paint_digits = if kill_lo.is_some() {
        collect_best_digits(dets, None, kill_lo, digit_min_conf)
    } else { vec![] };

    let kill_digits = if kill_lo.is_some() && death_lo.is_some() {
        collect_best_digits(dets, kill_lo, death_lo, digit_min_conf)
    } else { vec![] };

    let death_digits = if death_lo.is_some() && special_lo.is_some() {
        collect_best_digits(dets, death_lo, special_lo, digit_min_conf)
    } else { vec![] };

    let special_digits = if special_lo.is_some() {
        collect_best_digits(dets, special_lo, None, digit_min_conf)
    } else { vec![] };

    log::debug!(
        "[cascade] paint={:?} kill={:?} death={:?} sp={:?}",
        paint_digits, kill_digits, death_digits, special_digits
    );

    PlayerStats {
        paint:   digits_to_int(&paint_digits),
        kill:    digits_to_int(&kill_digits),
        death:   digits_to_int(&death_digits),
        special: digits_to_int(&special_digits),
    }
}

/// 指定クラス名のアイコン x_center を返す（表示用）。
fn icon_x(dets: &[Detection], name: &str) -> Option<f32> {
    dets.iter()
        .filter(|d| d.class_name == name)
        .max_by(|a, b| a.confidence.partial_cmp(&b.confidence).unwrap_or(Ordering::Equal))
        .map(|d| (d.bbox.x1 + d.bbox.x2) / 2.0)
}

/// 指定クラス名のアイコン左端 (x1) を返す（グループ境界計算用）。
fn icon_left_x(dets: &[Detection], name: &str) -> Option<f32> {
    dets.iter()
        .filter(|d| d.class_name == name)
        .max_by(|a, b| a.confidence.partial_cmp(&b.confidence).unwrap_or(Ordering::Equal))
        .map(|d| d.bbox.x1)
}

/// `lo <= x_center < hi` の範囲のデジット検出を収集し、
/// 近傍位置（X_DEDUP_TOL 以内）の重複は最高確信度のみ残す。
/// dets は x_center 昇順ソート済みであること。
fn collect_best_digits(dets: &[Detection], lo: Option<f32>, hi: Option<f32>, min_conf: f32) -> Vec<u8> {
    let in_range: Vec<&Detection> = dets.iter()
        .filter(|d| {
            if digit_value(&d.class_name).is_none() { return false; }
            if d.confidence < min_conf { return false; }
            let cx = (d.bbox.x1 + d.bbox.x2) / 2.0;
            lo.map_or(true, |l| cx >= l) && hi.map_or(true, |h| cx < h)
        })
        .collect();

    let mut result: Vec<u8> = Vec::new();
    let mut i = 0;
    while i < in_range.len() {
        let cxi = (in_range[i].bbox.x1 + in_range[i].bbox.x2) / 2.0;
        let mut best = in_range[i];
        let mut j = i + 1;
        while j < in_range.len() {
            let cxj = (in_range[j].bbox.x1 + in_range[j].bbox.x2) / 2.0;
            if (cxj - cxi).abs() <= X_DEDUP_TOL {
                if in_range[j].confidence > best.confidence {
                    best = in_range[j];
                }
                j += 1;
            } else {
                break;
            }
        }
        if let Some(v) = digit_value(&best.class_name) {
            result.push(v);
        }
        i = j;
    }
    result
}

/// `"digit_N"` クラス名から数字値を取り出す。
fn digit_value(class_name: &str) -> Option<u8> {
    class_name.strip_prefix("digit_")?.parse().ok()
}

/// 数字スライスを左から結合して i64 にパースする。空なら None。
fn digits_to_int(digits: &[u8]) -> Option<i64> {
    if digits.is_empty() {
        return None;
    }
    digits.iter().fold(Some(0i64), |acc, &d| {
        acc?.checked_mul(10)?.checked_add(d as i64)
    })
}

// ---------------------------------------------------------------------------
// デバッグ
// ---------------------------------------------------------------------------

impl StatsDetector {
    /// カスケード推論の中間状態を含む診断結果を返す。
    /// `arrow` が None の場合は MyArrow 未検出として early-return する。
    pub fn run_cascade_debug(
        &mut self,
        frame: &CapturedFrame,
        arrow: Option<&Detection>,
    ) -> crate::types::CascadeDebugResult {
        use crate::types::{CascadeDebugDetection, CascadeDebugResult};
        let (frame_w, frame_h) = (frame.width, frame.height);
        let stats_model_loaded = self.is_loaded();

        let arrow = match arrow {
            Some(a) => a,
            None => return CascadeDebugResult {
                frame_w, frame_h, stats_model_loaded,
                arrow_found: false,
                crop_x: 0, crop_y: 0, crop_w: 0, crop_h: 0,
                crop_image_base64: None,
                detections: vec![],
                kill_anchor_x: None, death_anchor_x: None, special_anchor_x: None,
                paint: None, kill: None, death: None, special: None,
                error: None,
            },
        };

        // Step 1: クロップ（右側の余分な黒背景を除いて学習データと一致させる）
        let y_px = ((arrow.bbox.y1 + arrow.bbox.y2) / 2.0 * frame_h as f32) as u32;
        let crop_y = y_px.saturating_sub(CROP_HALF_H);
        let crop_h = (CROP_HALF_H * 2).min(frame_h.saturating_sub(crop_y));
        let crop_x = (frame_w as f32 * STATS_X_START) as u32;
        let crop_right = ((frame_w as f32 * STATS_X_END) as u32).min(frame_w);
        let crop_w = crop_right.saturating_sub(crop_x);
        let cropped = crop_bgra(&frame.bgra, frame_w, crop_x, crop_y, crop_w, crop_h);

        // クロップ画像を base64 PNG にエンコード (フロントエンド表示用)
        let crop_image_base64 = encode_crop_base64(&cropped, crop_w, crop_h);

        // Step 2: Model 2 推論 (デバッグ時は閾値 0.10 で全候補を取得)
        let mut dets = match self.yolo.detect_debug_bgra(&cropped, crop_w, crop_h) {
            Ok(mut d) => {
                d.sort_by(|a, b| {
                    let cx = |d: &Detection| (d.bbox.x1 + d.bbox.x2) / 2.0;
                    cx(a).partial_cmp(&cx(b)).unwrap_or(std::cmp::Ordering::Equal)
                });
                d
            }
            Err(e) => return CascadeDebugResult {
                frame_w, frame_h, stats_model_loaded,
                arrow_found: true,
                crop_x, crop_y, crop_w, crop_h,
                crop_image_base64,
                detections: vec![],
                kill_anchor_x: None, death_anchor_x: None, special_anchor_x: None,
                paint: None, kill: None, death: None, special: None,
                error: Some(format!("Model 2 推論エラー: {e}")),
            },
        };

        // アンカー中心座標（表示用）
        let kill_anchor_x    = icon_x(&dets, "icon_kill");
        let death_anchor_x   = icon_x(&dets, "icon_death");
        let special_anchor_x = icon_x(&dets, "icon_special");

        // アンカー左端座標（グループ境界・ラベル付与用）
        let kill_lo    = icon_left_x(&dets, "icon_kill");
        let death_lo   = icon_left_x(&dets, "icon_death");
        let special_lo = icon_left_x(&dets, "icon_special");

        // スタッツ集計: digit_min_conf=0.50 でデジットノイズを除外し本番相当にする。
        // アイコン境界は dets 全体から取るため、低確信度アイコンも境界として活用できる。
        let stats = cluster_and_parse(&dets, 0.50);

        // 全候補（閾値 0.10）にグループラベルを付与（デバッグ表示用）
        let debug_dets: Vec<CascadeDebugDetection> = dets.iter().map(|d| {
            let cx = (d.bbox.x1 + d.bbox.x2) / 2.0;
            CascadeDebugDetection {
                class_name: d.class_name.clone(),
                confidence: d.confidence,
                x_center:   cx,
                group: assign_group(cx, &d.class_name, kill_lo, death_lo, special_lo),
            }
        }).collect();

        CascadeDebugResult {
            frame_w, frame_h, stats_model_loaded,
            arrow_found: true,
            crop_x, crop_y, crop_w, crop_h,
            crop_image_base64,
            detections: debug_dets,
            kill_anchor_x, death_anchor_x, special_anchor_x,
            paint:   stats.paint,
            kill:    stats.kill,
            death:   stats.death,
            special: stats.special,
            error:   None,
        }
    }
}

fn encode_crop_base64(bgra: &[u8], width: u32, height: u32) -> Option<String> {
    let rgba: Vec<u8> = bgra.chunks_exact(4)
        .flat_map(|c| [c[2], c[1], c[0], c[3]])
        .collect();
    let img = image::RgbaImage::from_raw(width, height, rgba)?;
    let mut buf = std::io::Cursor::new(Vec::<u8>::new());
    img.write_to(&mut buf, image::ImageFormat::Png).ok()?;
    use base64::Engine as _;
    Some(base64::engine::general_purpose::STANDARD.encode(buf.into_inner()))
}

/// 各検出の x_center とクラス名からグループ名を返す（デバッグ表示用）。
/// アイコン左端を境界として使用する。
fn assign_group(
    cx:         f32,
    class_name: &str,
    kill_lo:    Option<f32>,
    death_lo:   Option<f32>,
    special_lo: Option<f32>,
) -> String {
    match class_name {
        "icon_kill"    => return "anchor_kill".to_string(),
        "icon_death"   => return "anchor_death".to_string(),
        "icon_special" => return "anchor_special".to_string(),
        _ => {}
    }
    if digit_value(class_name).is_none() {
        return "ignored".to_string();
    }
    // アイコン左端を境界とした単純な区間判定
    if kill_lo.map_or(true, |kx| cx < kx) { "paint".to_string() }
    else if death_lo.map_or(true, |dx| cx < dx) { "kill".to_string() }
    else if special_lo.map_or(true, |sx| cx < sx) { "death".to_string() }
    else { "special".to_string() }
}

// ---------------------------------------------------------------------------
// テスト
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::detector::BBox;

    fn make_det(class_name: &str, cx: f32, conf: f32) -> Detection {
        Detection {
            bbox: BBox {
                x1: cx - 0.01,
                y1: 0.3,
                x2: cx + 0.01,
                y2: 0.7,
            },
            class_id: 0,
            class_name: class_name.to_string(),
            confidence: conf,
        }
    }

    // ---------- digits_to_int ----------

    #[test]
    fn test_digits_empty() {
        assert_eq!(digits_to_int(&[]), None);
    }

    #[test]
    fn test_digits_single() {
        assert_eq!(digits_to_int(&[7]), Some(7));
    }

    #[test]
    fn test_digits_multi() {
        assert_eq!(digits_to_int(&[1, 3, 4, 1]), Some(1341));
    }

    #[test]
    fn test_digits_zero_padded() {
        assert_eq!(digits_to_int(&[0, 5]), Some(5));
    }

    #[test]
    fn test_digits_overflow() {
        // i64 の桁数を超える場合は None
        let many_nines: Vec<u8> = vec![9; 20];
        assert_eq!(digits_to_int(&many_nines), None);
    }

    // ---------- digit_value ----------

    #[test]
    fn test_digit_value_all() {
        for i in 0u8..=9 {
            assert_eq!(digit_value(&format!("digit_{i}")), Some(i));
        }
    }

    #[test]
    fn test_digit_value_icon() {
        assert_eq!(digit_value("icon_kill"),  None);
        assert_eq!(digit_value("icon_death"),   None);
        assert_eq!(digit_value("icon_special"), None);
    }

    // ---------- cluster_and_parse ----------

    #[test]
    fn test_cluster_no_detections() {
        let stats = cluster_and_parse(&[], 0.0);
        assert!(stats.paint.is_none());
        assert!(stats.kill.is_none());
        assert!(stats.death.is_none());
        assert!(stats.special.is_none());
    }

    #[test]
    fn test_cluster_no_anchors() {
        // アンカーなし → 境界が定まらないため全グループ None
        let dets = vec![make_det("digit_3", 0.5, 0.9)];
        let stats = cluster_and_parse(&dets, 0.0);
        assert!(stats.paint.is_none(), "paint should be None without kill anchor");
        assert!(stats.kill.is_none());
        assert!(stats.death.is_none());
        assert!(stats.special.is_none());
    }

    /// icon_kill=0.30, icon_death=0.50, icon_special=0.70
    /// paint: 0.10,0.15 → [1,5] = 15
    /// kill:  0.35,0.40 → [3,0] = 30
    /// death: 0.55      → [2]   = 2
    /// sp:    0.80,0.85 → [1,2] = 12
    #[test]
    fn test_cluster_clean() {
        let mut dets = vec![
            make_det("digit_1",    0.10, 0.9),
            make_det("digit_5",    0.15, 0.9),
            make_det("icon_kill",0.30, 0.9),
            make_det("digit_3",    0.35, 0.9),
            make_det("digit_0",    0.40, 0.9),
            make_det("icon_death", 0.50, 0.9),
            make_det("digit_2",    0.55, 0.9),
            make_det("icon_special",0.70, 0.9),
            make_det("digit_1",    0.80, 0.9),
            make_det("digit_2",    0.85, 0.9),
        ];
        dets.sort_by(|a, b| {
            let cx = |d: &Detection| (d.bbox.x1 + d.bbox.x2) / 2.0;
            cx(a).partial_cmp(&cx(b)).unwrap_or(Ordering::Equal)
        });

        let stats = cluster_and_parse(&dets, 0.0);
        assert_eq!(stats.paint,   Some(15));
        assert_eq!(stats.kill,    Some(30));
        assert_eq!(stats.death,   Some(2));
        assert_eq!(stats.special, Some(12));
    }

    /// アイコン左端境界: アイコン直前の数字は paint グループへ
    #[test]
    fn test_cluster_boundary_at_icon_left_edge() {
        // icon_kill の make_det は x1 = cx - 0.01 = 0.29
        // digit_1 at cx=0.281 < 0.29 → paint グループ
        // digit_1 at cx=0.35  ≥ 0.29 かつ < death_x1=0.49 → kill グループ
        let mut dets = vec![
            make_det("digit_9",      0.10,  0.9),
            make_det("icon_kill",    0.30,  0.9),
            make_det("digit_1",      0.281, 0.9), // icon x1=0.29 より左 → paint
            make_det("digit_2",      0.35,  0.9), // icon x1=0.29 より右 → kill
            make_det("icon_death",   0.50,  0.9),
            make_det("icon_special", 0.70,  0.9),
        ];
        dets.sort_by(|a, b| {
            let cx = |d: &Detection| (d.bbox.x1 + d.bbox.x2) / 2.0;
            cx(a).partial_cmp(&cx(b)).unwrap_or(Ordering::Equal)
        });

        let stats = cluster_and_parse(&dets, 0.0);
        assert_eq!(stats.paint, Some(91)); // digit_9 と digit_1(0.281)
        assert_eq!(stats.kill,  Some(2));  // digit_2(0.35) のみ
        assert_eq!(stats.death, None);     // 数字なし
        assert_eq!(stats.special, None);   // 数字なし
    }
}
