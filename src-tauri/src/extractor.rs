/// InkGraph — データ抽出モジュール
///
/// 2つの抽出パスを提供する:
///
///   1. YOLO パス (extract_from_yolo_detections):
///      Model 1 の BBox を元にクロップ → PaddleOCR → normalize
///
///   2. ピクセルフォールバックパス (extract_match_data):
///      固定 ROI 座標でクロップ → PaddleOCR → normalize
///      YOLO モデルが未配置の場合に使用する

use crate::{
    capture::CapturedFrame,
    detector::{crop_bgra, Detection, YoloClass, YoloDetector, Roi},
    ocr::ocr_from_bgra,
    ocr_rec,
    preprocess::{self, save_debug_png},
    types::{ExtractedMatchData, OcrDebugField, OcrDebugResult},
};
use anyhow::Result;

// ---------------------------------------------------------------------------
// YOLO パス
// ---------------------------------------------------------------------------

/// YOLO の検出結果を使ってリザルト画面から全データを抽出する。
///
/// `result`: capture_loop が MyPlayerRow の y 座標から判定した "win" / "lose" 文字列。
/// `stats_override`: カスケード推論 (Model 2) で取得したスタッツ。Some の場合は OCR 抽出を上書きする。
/// `header_override`: ヘッダー部 YOLO 推論で取得したモード/ルール/ステージ。
/// 呼び出し元は `tokio::task::block_in_place` でラップすること (OCR がブロッキング)。
pub fn extract_from_yolo_detections(
    frame:           &CapturedFrame,
    detections:      &[Detection],
    result:          &str,
    stats_override:  Option<crate::cascade::PlayerStats>,
    header_override: Option<crate::cascade::HeaderInfo>,
) -> Result<ExtractedMatchData> {
    // ヘッダー: header_override がある場合はそれを優先、なければ OCR で取得
    let (ocr_rule, ocr_stage, ocr_mode) = extract_header_via_ocr(frame, detections);
    let (rule, stage, mode) = if let Some(h) = header_override {
        (
            ocr_rule.or(h.rule),
            ocr_stage.or(h.stage),
            ocr_mode.or(h.mode),
        )
    } else {
        (ocr_rule, ocr_stage, ocr_mode)
    };

    // KDA + 塗りポイント: stats_override があればそちらを優先、なければ固定列 OCR
    let (kill_count, death_count, special_count, paint_count) =
        if let Some(s) = stats_override {
            (s.kill, s.death, s.special, s.paint)
        } else {
            let kda_y = YoloDetector::best_detection(detections, YoloClass::MyArrow)
                .map(|d| (d.bbox.y1 + d.bbox.y2) / 2.0)
                .unwrap_or(0.5);
            let (k, d, sp) = extract_kda(frame, kda_y)?;
            (k, d, sp, None)
        };

    // GoldAward: 検出された BBox の数 = 取得した金表彰の枚数
    let gold_award_count = detections.iter()
        .filter(|d| d.class_id == YoloClass::GoldAward as usize)
        .count() as i64;

    Ok(ExtractedMatchData {
        result: result.to_string(),
        mode,
        kill_count,
        death_count,
        special_count,
        paint_count,
        xp_after: None,
        rule,
        stage,
        gold_award_count: Some(gold_award_count),
    })
}

/// YOLO BBox または固定 ROI から OCR でヘッダー情報を取得する。
///
/// OCR 生テキストは normalize_rule / normalize_stage / normalize_mode に通して
/// 既定リストから最も一致度の高い値を選ぶ。直接 DB に保存することはしない。
fn extract_header_via_ocr(
    frame:      &CapturedFrame,
    detections: &[Detection],
) -> (Option<String>, Option<String>, Option<String>) {
    let rule  = ocr_on_class(frame, detections, YoloClass::RuleText)
        .or_else(|| ocr_on_fixed_roi(frame, &RULE_ROI))
        .and_then(|t| normalize_rule(t.trim()));

    let stage = ocr_on_class(frame, detections, YoloClass::StageText)
        .or_else(|| ocr_on_fixed_roi(frame, &STAGE_ROI))
        .and_then(|t| normalize_stage(t.trim()));

    let mode = ocr_on_class(frame, detections, YoloClass::ModeText)
        .or_else(|| ocr_on_fixed_roi(frame, &MODE_ROI))
        .and_then(|t| normalize_mode(t.trim()));

    (rule, stage, mode)
}

/// YOLO が検出した BBox を crop → PP-OCRv5 rec でテキストを返す。
fn ocr_on_class(
    frame:      &CapturedFrame,
    dets:       &[Detection],
    class:      YoloClass,
) -> Option<String> {
    let det = YoloDetector::best_detection(dets, class)?;
    let x1 = (det.bbox.x1 * frame.width  as f32) as u32;
    let y1 = (det.bbox.y1 * frame.height as f32) as u32;
    let x2 = ((det.bbox.x2 * frame.width  as f32) as u32).min(frame.width.saturating_sub(1));
    let y2 = ((det.bbox.y2 * frame.height as f32) as u32).min(frame.height.saturating_sub(1));
    let w  = x2.saturating_sub(x1).max(1);
    let h  = y2.saturating_sub(y1).max(1);
    let crop = crop_bgra(&frame.bgra, frame.width, x1, y1, w, h);
    ocr_rec::recognize_bgra(&crop, w, h).ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

/// 固定 ROI を crop → PP-OCRv5 rec でテキストを返す。
fn ocr_on_fixed_roi(frame: &CapturedFrame, roi: &Roi) -> Option<String> {
    let (x, y, w, h) = roi.to_pixels(frame.width, frame.height);
    let crop = crop_bgra(&frame.bgra, frame.width, x, y, w, h);
    ocr_rec::recognize_bgra(&crop, w, h).ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

// ---------------------------------------------------------------------------
// ピクセルフォールバックパス (固定 ROI OCR)
// ---------------------------------------------------------------------------

const RULE_ROI: Roi = Roi {
    x_ratio: 0.450, y_ratio: 0.060, w_ratio: 0.120, h_ratio: 0.058,
};
const STAGE_ROI: Roi = Roi {
    x_ratio: 0.545, y_ratio: 0.060, w_ratio: 0.240, h_ratio: 0.058,
};
// ModeText YOLO BBox が未検出の場合の固定 ROI フォールバック。
// 座標は実機で debug_full の ModeText BBox を確認して調整すること。
const MODE_ROI: Roi = Roi {
    x_ratio: 0.350, y_ratio: 0.055, w_ratio: 0.110, h_ratio: 0.060,
};
const XP_ROI: Roi = Roi {
    x_ratio: 0.455, y_ratio: 0.185, w_ratio: 0.180, h_ratio: 0.060,
};

/// 固定 ROI パス: 黄色矢印の y 重心を行基準として KDA などを OCR する。
/// (YOLO モデル未配置時のフォールバック)
pub fn extract_match_data(
    frame:         &CapturedFrame,
    result:        &str,
    arrow_y_ratio: f32,
) -> Result<ExtractedMatchData> {
    let (kill_count, death_count, special_count) = extract_kda(frame, arrow_y_ratio)?;
    let xp_after = extract_xp(frame)?;
    let rule  = extract_rule(frame);
    let stage = extract_stage(frame);

    Ok(ExtractedMatchData {
        result: result.to_string(),
        mode: None,
        kill_count,
        death_count,
        special_count,
        paint_count: None,
        xp_after,
        rule,
        stage,
        gold_award_count: None,
    })
}

// ---------------------------------------------------------------------------
// 共通 KDA 抽出 (YOLO / 固定 ROI 両パスで使用)
// ---------------------------------------------------------------------------

const KILL_COL_X:  f32 = 0.758;
const DEATH_COL_X: f32 = 0.822;
const SPEC_COL_X:  f32 = 0.878; // 0.863 から右にシフト (特殊カウンターの実位置に合わせる)
const KDA_COL_W:   f32 = 0.060; // 0.048 から拡大 (2桁数字をマージン込みで収める)
const KDA_ROW_H:   f32 = 0.052;

/// `y_ratio` はプレイヤー行の y 中心 (0.0–1.0)。
fn extract_kda(
    frame:   &CapturedFrame,
    y_ratio: f32,
) -> Result<(Option<i64>, Option<i64>, Option<i64>)> {
    let y_top = (y_ratio - KDA_ROW_H / 2.0).max(0.0);
    let kill_roi = Roi { x_ratio: KILL_COL_X,  y_ratio: y_top, w_ratio: KDA_COL_W, h_ratio: KDA_ROW_H };
    let deat_roi = Roi { x_ratio: DEATH_COL_X, y_ratio: y_top, w_ratio: KDA_COL_W, h_ratio: KDA_ROW_H };
    let spec_roi = Roi { x_ratio: SPEC_COL_X,  y_ratio: y_top, w_ratio: KDA_COL_W, h_ratio: KDA_ROW_H };
    Ok((
        extract_integer_roi(frame, &kill_roi),
        extract_integer_roi(frame, &deat_roi),
        extract_integer_roi(frame, &spec_roi),
    ))
}

fn extract_xp(frame: &CapturedFrame) -> Result<Option<f64>> {
    let (x, y, w, h) = XP_ROI.to_pixels(frame.width, frame.height);
    let roi = crop_bgra(&frame.bgra, frame.width, x, y, w, h);
    // XP は数値のみで WinRT en-US が安定しているためそのまま使用
    let text = ocr_from_bgra(&roi, w, h, Some("en-US"))?.text;
    Ok(clean_numeric_text(&text).parse::<f64>().ok())
}

fn extract_rule(frame: &CapturedFrame) -> Option<String> {
    normalize_rule(extract_rule_raw(frame).trim())
}

fn extract_stage(frame: &CapturedFrame) -> Option<String> {
    normalize_stage(&extract_stage_raw(frame))
}

/// ルール ROI の生 OCR テキストを返す（デバッグ・通常抽出共用）
pub fn extract_rule_raw(frame: &CapturedFrame) -> String {
    let (x, y, w, h) = RULE_ROI.to_pixels(frame.width, frame.height);
    let crop = crop_bgra(&frame.bgra, frame.width, x, y, w, h);
    ocr_rec::recognize_bgra(&crop, w, h).ok()
        .map(|s| s.trim().to_string())
        .unwrap_or_default()
}

/// ステージ ROI の生 OCR テキストを返す（デバッグ・通常抽出共用）
pub fn extract_stage_raw(frame: &CapturedFrame) -> String {
    let (x, y, w, h) = STAGE_ROI.to_pixels(frame.width, frame.height);
    let crop = crop_bgra(&frame.bgra, frame.width, x, y, w, h);
    ocr_rec::recognize_bgra(&crop, w, h).ok()
        .map(|s| s.trim().to_string())
        .unwrap_or_default()
}

/// crop → PP-OCRv5 rec → 数字パース
///
/// 白テキスト前処理 (extract_white_text) を試みて数字が取れたらそれを採用。
/// 取れなかった場合は前処理なし OCR にフォールバック。
/// (理由: 死カウンターの赤い背景では "5" 等がアンチエイリアスで閾値割れして消えることがある)
fn extract_integer_roi(frame: &CapturedFrame, roi: &Roi) -> Option<i64> {
    let (x, y, w, h) = roi.to_pixels(frame.width, frame.height);
    let cropped = crop_bgra(&frame.bgra, frame.width, x, y, w, h);
    let white = preprocess::extract_white_text(&cropped, w, h);
    let white_result = ocr_rec::recognize_bgra(&white, w, h).ok()
        .and_then(|t| clean_numeric_text(&t).parse::<i64>().ok());
    if white_result.is_some() {
        return white_result;
    }
    ocr_rec::recognize_bgra(&cropped, w, h).ok()
        .and_then(|t| clean_numeric_text(&t).parse::<i64>().ok())
}

fn clean_numeric_text(text: &str) -> String {
    text.chars().filter(|c| c.is_ascii_digit() || *c == '.').collect()
}

// ---------------------------------------------------------------------------
// 正規化
// ---------------------------------------------------------------------------

/// ひらがな→カタカナ、小書きカタカナ→大書きに正規化（OCR 誤読対策）。
fn normalize_kana(s: &str) -> String {
    s.chars().map(|c| {
        let k = if ('\u{3041}'..='\u{3096}').contains(&c) {
            char::from_u32(c as u32 + 0x60).unwrap_or(c)
        } else {
            c
        };
        match k {
            'ァ' => 'ア', 'ィ' => 'イ', 'ゥ' => 'ウ', 'ェ' => 'エ', 'ォ' => 'オ',
            'ャ' => 'ヤ', 'ュ' => 'ユ', 'ョ' => 'ヨ', 'ッ' => 'ツ',
            'ヮ' => 'ワ', 'ヵ' => 'カ', 'ヶ' => 'ケ',
            _ => k,
        }
    }).collect()
}

/// OCR テキスト内に候補文字が何割含まれるかを返す（文字 recall）。
/// 同一文字の重複は消費ベースで正確にカウントする。カナ正規化済み。
fn char_recall(candidate: &str, ocr: &str) -> f32 {
    if candidate.is_empty() { return 0.0; }
    let norm_cand = normalize_kana(candidate);
    let norm_ocr  = normalize_kana(ocr);
    let mut pool: Vec<char> = norm_ocr.chars().collect();
    let matched = norm_cand.chars().filter(|c| {
        if let Some(pos) = pool.iter().position(|x| x == c) {
            pool.remove(pos);
            true
        } else {
            false
        }
    }).count();
    matched as f32 / norm_cand.chars().count() as f32
}

/// 候補リストから OCR テキストへの char_recall が最大で閾値以上の候補を返す。
fn fuzzy_best_match<'a>(raw: &str, candidates: &[&'a str], threshold: f32) -> Option<&'a str> {
    candidates.iter()
        .map(|&c| (c, char_recall(c, raw)))
        .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
        .filter(|(_, s)| *s >= threshold)
        .map(|(c, _)| c)
}

pub fn normalize_rule(raw: &str) -> Option<String> {
    let candidates: &[(&str, &[&str])] = &[
        ("ガチエリア",  &["ガチエリア", "エリア", "AREA", "SPLAT ZONES"]),
        ("ガチヤグラ",  &["ガチヤグラ", "ヤグラ", "TOWER", "TOWER CONTROL"]),
        ("ガチホコ",    &["ガチホコ", "ホコ", "RAINMAKER"]),
        ("ガチアサリ",  &["ガチアサリ", "アサリ", "CLAM", "CLAM BLITZ"]),
    ];
    let upper = raw.to_uppercase();
    for (canonical, aliases) in candidates {
        for alias in *aliases {
            if upper.contains(&alias.to_uppercase()) || raw.contains(alias) {
                return Some(canonical.to_string());
            }
        }
    }
    let rule_names = ["ガチエリア", "ガチヤグラ", "ガチホコ", "ガチアサリ"];
    if let Some(m) = fuzzy_best_match(raw.trim(), &rule_names, 0.30) {
        return Some(m.to_string());
    }
    None
}

pub fn normalize_stage(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() { return None; }
    const STAGES: &[&str] = &[
        "ユノハナ大渓谷", "ゴンズイ地区", "ヤガラ市場", "マテガイ放水路",
        "ナメロウ金属", "ナンプラー遺跡", "クサヤ温泉", "ヒラメが丘団地",
        "マサバ海峡大橋", "スメーシーワールド", "キンメダイ美術館",
        "タラポートショッピングパーク", "バイガイ亭", "海女美術大学",
        "チョウザメ造船", "ザトウマーケット", "リュウグウターミナル",
        "オヒョウ海運", "カジキ空港",
        "ネギトロ炭鉱", "ショッツル鉱山", "デカライン高架下",
        "コンブトラック", "マヒマヒリゾート&スパ", "マンタマリア号", "タカアシ経済特区",
    ];
    for s in STAGES {
        if trimmed.contains(s) { return Some(s.to_string()); }
        if trimmed.chars().count() >= 4 && s.contains(trimmed) {
            return Some(s.to_string());
        }
    }
    fuzzy_best_match(trimmed, STAGES, 0.35).map(|s| s.to_string())
}

pub fn normalize_mode(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() { return None; }

    let candidates: &[(&str, &[&str])] = &[
        ("Xマッチ",                   &["Xマッチ", "X BATTLE", "Xバトル", "X MATCH"]),
        ("バンカラマッチ(チャレンジ)", &["チャレンジ", "CHALLENGE", "ANARCHY OPEN"]),
        ("バンカラマッチ(オープン)",   &["オープン", "OPEN", "ANARCHY BATTLE", "バンカラ", "オーフン", "オーブン"]),
        ("ナワバリバトル",             &["ナワバリ", "TURF WAR"]),
        ("サーモンラン",               &["サーモン", "SALMON"]),
    ];
    let upper = raw.to_uppercase();
    for (canonical, aliases) in candidates {
        for alias in *aliases {
            if upper.contains(&alias.to_uppercase()) || raw.contains(alias) {
                return Some(canonical.to_string());
            }
        }
    }

    let rule_names_exact = ["ガチエリア", "ガチヤグラ", "ガチホコ", "ガチアサリ",
                             "AREA", "TOWER", "RAINMAKER", "CLAM"];
    let upper_raw = raw.to_uppercase();
    for r in &rule_names_exact {
        if upper_raw.contains(&r.to_uppercase()) { return None; }
    }
    let rule_names_fuzzy = ["ガチエリア", "ガチヤグラ", "ガチホコ", "ガチアサリ"];
    if fuzzy_best_match(trimmed, &rule_names_fuzzy, 0.35).is_some() { return None; }

    const MODE_NAMES: &[&str] = &[
        "Xマッチ", "バンカラマッチ(チャレンジ)",
        "バンカラマッチ(オープン)", "ナワバリバトル",
    ];
    fuzzy_best_match(trimmed, MODE_NAMES, 0.40).map(|s| s.to_string())
}

// ---------------------------------------------------------------------------
// デバッグ OCR (全フィールド生テキスト + 正規化)
// ---------------------------------------------------------------------------

/// YOLO 検出結果を使って全フィールドの OCR 生テキストと正規化値を返す。
/// debug_yolo コマンドから呼ばれる。ブロッキング呼び出しのため block_in_place 必須。
/// `cascade_kda`: Model 2 カスケードで取得した (kill, death, special)。
/// Some の場合は固定 ROI OCR の normalized を上書きする。
///
/// `kda_anchors`: cascade が検出した (kill_x, death_x, special_x) アイコン中心の正規化 X 座標。
/// Some の場合はハードコード定数の代わりにアンカーを中心としたクロップを行う。
pub fn extract_debug_ocr(
    frame:       &CapturedFrame,
    detections:  &[Detection],
    cascade_kda: Option<(Option<i64>, Option<i64>, Option<i64>)>,
    kda_anchors: Option<(Option<f32>, Option<f32>, Option<f32>)>,
) -> OcrDebugResult {
    let (rule_raw,  rule_crop)  = bbox_to_raw_text_debug(frame, detections, YoloClass::RuleText,  "rule");
    let (stage_raw, stage_crop) = bbox_to_raw_text_debug(frame, detections, YoloClass::StageText, "stage");
    let (mode_raw,  mode_crop)  = bbox_to_raw_text_debug(frame, detections, YoloClass::ModeText,  "mode");

    let arrow_y = YoloDetector::best_detection(detections, YoloClass::MyArrow)
        .map(|d| (d.bbox.y1 + d.bbox.y2) / 2.0);

    // アンカーが取れている列はアンカー中心を使い、取れていない列はハードコード定数で補完する
    let anchor_x = |default: f32, anchor: Option<Option<f32>>| -> f32 {
        anchor.and_then(|a| a)
            .map(|a| (a - KDA_COL_W / 2.0).max(0.0))
            .unwrap_or(default)
    };
    let (kill_x, death_x, spec_x) = match kda_anchors {
        Some((k, d, s)) => (
            anchor_x(KILL_COL_X,  Some(k)),
            anchor_x(DEATH_COL_X, Some(d)),
            anchor_x(SPEC_COL_X,  Some(s)),
        ),
        None => (KILL_COL_X, DEATH_COL_X, SPEC_COL_X),
    };

    let ((kill_raw, kill_crop), (death_raw, death_crop), (special_raw, special_crop))
        = arrow_y.map(|y| {
        let y_top = (y - KDA_ROW_H / 2.0).max(0.0);
        (
            roi_to_raw_debug(frame, &Roi { x_ratio: kill_x,  y_ratio: y_top, w_ratio: KDA_COL_W, h_ratio: KDA_ROW_H }, "kill"),
            roi_to_raw_debug(frame, &Roi { x_ratio: death_x, y_ratio: y_top, w_ratio: KDA_COL_W, h_ratio: KDA_ROW_H }, "death"),
            roi_to_raw_debug(frame, &Roi { x_ratio: spec_x,  y_ratio: y_top, w_ratio: KDA_COL_W, h_ratio: KDA_ROW_H }, "special"),
        )
    }).unwrap_or_default();

    // cascade_kda が Some の場合はその値を normalized に使う。
    // None の場合は固定 ROI OCR 結果にフォールバック。
    let (cas_kill, cas_death, cas_special) = cascade_kda
        .unwrap_or((None, None, None));
    let kill_norm    = cas_kill.map(|v| v.to_string())
        .or_else(|| clean_numeric_text(&kill_raw).parse::<i64>().ok().map(|v| v.to_string()));
    let death_norm   = cas_death.map(|v| v.to_string())
        .or_else(|| clean_numeric_text(&death_raw).parse::<i64>().ok().map(|v| v.to_string()));
    let special_norm = cas_special.map(|v| v.to_string())
        .or_else(|| clean_numeric_text(&special_raw).parse::<i64>().ok().map(|v| v.to_string()));

    OcrDebugResult {
        rule:    OcrDebugField { normalized: normalize_rule(&rule_raw),   raw: rule_raw,   crop_image_base64: rule_crop   },
        stage:   OcrDebugField { normalized: normalize_stage(&stage_raw), raw: stage_raw,  crop_image_base64: stage_crop  },
        mode:    OcrDebugField { normalized: normalize_mode(&mode_raw),   raw: mode_raw,   crop_image_base64: mode_crop   },
        kill:    OcrDebugField { normalized: kill_norm,    raw: kill_raw,    crop_image_base64: kill_crop    },
        death:   OcrDebugField { normalized: death_norm,   raw: death_raw,   crop_image_base64: death_crop   },
        special: OcrDebugField { normalized: special_norm, raw: special_raw, crop_image_base64: special_crop },
        arrow_y,
    }
}

/// BGRA バイト列を PNG にエンコードして base64 文字列で返す (フロントエンド表示用)。
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

/// デバッグ版: YOLO BBox を crop → OCR → (テキスト, base64クロップ画像) を返す。
fn bbox_to_raw_text_debug(
    frame:      &CapturedFrame,
    detections: &[Detection],
    class:      YoloClass,
    label:      &str,
) -> (String, Option<String>) {
    let Some(det) = YoloDetector::best_detection(detections, class) else { return (String::new(), None); };
    let x1 = (det.bbox.x1 * frame.width  as f32) as u32;
    let y1 = (det.bbox.y1 * frame.height as f32) as u32;
    let x2 = ((det.bbox.x2 * frame.width  as f32) as u32).min(frame.width.saturating_sub(1));
    let y2 = ((det.bbox.y2 * frame.height as f32) as u32).min(frame.height.saturating_sub(1));
    let w  = x2.saturating_sub(x1).max(1);
    let h  = y2.saturating_sub(y1).max(1);
    let cropped = crop_bgra(&frame.bgra, frame.width, x1, y1, w, h);
    save_debug_png(&format!("{label}_1crop"), &cropped, w, h);
    let text = ocr_rec::recognize_bgra(&cropped, w, h).unwrap_or_default();
    let crop = encode_crop_base64(&cropped, w, h);
    (text, crop)
}

/// デバッグ版: ROI を crop → OCR → (テキスト, base64クロップ画像) を返す (KDA 用)。
fn roi_to_raw_debug(frame: &CapturedFrame, roi: &Roi, label: &str) -> (String, Option<String>) {
    let (x, y, w, h) = roi.to_pixels(frame.width, frame.height);
    let cropped = crop_bgra(&frame.bgra, frame.width, x, y, w, h);
    save_debug_png(&format!("{label}_1crop"), &cropped, w, h);
    let white = preprocess::extract_white_text(&cropped, w, h);
    save_debug_png(&format!("{label}_2white"), &white, w, h);
    // extract_integer_roi と同じ 2 段パス (白テキスト → raw)
    let white_text = ocr_rec::recognize_bgra(&white, w, h).unwrap_or_default();
    let text = if clean_numeric_text(&white_text).parse::<i64>().is_ok() {
        white_text
    } else {
        ocr_rec::recognize_bgra(&cropped, w, h).unwrap_or_default()
    };
    let crop = encode_crop_base64(&cropped, w, h);
    (text, crop)
}

// ---------------------------------------------------------------------------
// テスト
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_rule() {
        assert_eq!(normalize_rule("ガチエリア"), Some("ガチエリア".to_string()));
        assert_eq!(normalize_rule("エリア"),     Some("ガチエリア".to_string()));
        assert_eq!(normalize_rule("AREA"),       Some("ガチエリア".to_string()));
        assert_eq!(normalize_rule("ガチヤグラ"), Some("ガチヤグラ".to_string()));
        assert_eq!(normalize_rule("RAINMAKER"),  Some("ガチホコ".to_string()));
        assert_eq!(normalize_rule("ヤグ"),       Some("ガチヤグラ".to_string()));
    }

    #[test]
    fn test_normalize_stage() {
        assert_eq!(normalize_stage("マテガイ放水路"), Some("マテガイ放水路".to_string()));
        assert_eq!(normalize_stage("ナメロウ金属"),   Some("ナメロウ金属".to_string()));
        assert_eq!(normalize_stage(""),               None);
        assert_eq!(normalize_stage("リュウグウラ - ミカ↓"), Some("リュウグウターミナル".to_string()));
        assert_eq!(normalize_stage("デカライン高架下"),      Some("デカライン高架下".to_string()));
    }

    #[test]
    fn test_normalize_mode() {
        assert_eq!(normalize_mode("Xマッチ"),       Some("Xマッチ".to_string()));
        assert_eq!(normalize_mode("X BATTLE"),       Some("Xマッチ".to_string()));
        assert_eq!(normalize_mode("チャレンジ"),     Some("バンカラマッチ(チャレンジ)".to_string()));
        assert_eq!(normalize_mode("ANARCHY BATTLE"), Some("バンカラマッチ(オープン)".to_string()));
        assert_eq!(normalize_mode("ナワバリ"),       Some("ナワバリバトル".to_string()));
        assert_eq!(normalize_mode("TURF WAR"),       Some("ナワバリバトル".to_string()));
        assert_eq!(normalize_mode(""),               None);
        assert_eq!(normalize_mode("多ャグラ"),       None);
        assert_eq!(normalize_mode("ガチヤグラ"),     None);
    }

    #[test]
    fn test_char_recall() {
        let score = char_recall("リュウグウターミナル", "リュウグウラ - ミカ↓");
        assert!(score >= 0.35, "recall={score}");
        let score2 = char_recall("ガチヤグラ", "ヤグ");
        assert!(score2 >= 0.30, "recall={score2}");
        let score3 = char_recall("ガチヤグラ", "多ャグう");
        assert!(score3 >= 0.35, "recall={score3}");
    }

    #[test]
    fn test_normalize_mode_garbled() {
        assert_eq!(normalize_mode("多ャグう"), None);
        assert_eq!(normalize_mode("zzzz"), None);
    }

    #[test]
    fn test_normalize_mode_ocr_errors() {
        // "プ"→"フ" 誤読 ("オープン"→"オーフン")
        assert_eq!(normalize_mode("オーフン"), Some("バンカラマッチ(オープン)".to_string()));
        // 実際の PP-OCRv5 誤読例: "バンカラマッチ(オープン)" → "バンッチ(オーフン)"
        assert_eq!(normalize_mode("バンッチ(オーフン)"), Some("バンカラマッチ(オープン)".to_string()));
        // ルール名は依然として None を返す
        assert_eq!(normalize_mode("ガチヤグラ"), None);
    }

    #[test]
    fn test_normalize_rule_garbled() {
        assert_eq!(normalize_rule("多ャワう"), None);
        assert_eq!(normalize_rule("多ャグう"), Some("ガチヤグラ".to_string()));
    }

    #[test]
    fn test_clean_numeric_text() {
        assert_eq!(clean_numeric_text("2341.5 XP"), "2341.5");
        assert_eq!(clean_numeric_text("Kill: 5"),   "5");
        assert_eq!(clean_numeric_text("abc"),        "");
    }
}
