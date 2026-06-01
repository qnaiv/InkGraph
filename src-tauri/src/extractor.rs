/// InkGraph — データ抽出モジュール
///
/// 2つの抽出パスを提供する:
///
///   1. YOLO パス (extract_from_yolo_detections):
///      YOLO が返した BBox を元にクロップ → 白文字抽出 → WinRT OCR
///      モード (Xマッチ/ナワバリ等) も抽出できる
///
///   2. ピクセルフォールバックパス (extract_match_data):
///      固定 ROI 座標でクロップ → WinRT OCR
///      YOLO モデルが未配置の場合に使用する

use crate::{
    capture::CapturedFrame,
    detector::{crop_bgra, Detection, YoloClass, YoloDetector, Roi},
    ocr::{ocr_from_bgra, preprocess_bgra},
    preprocess::extract_white_text,
    types::{ExtractedMatchData, OcrDebugField, OcrDebugResult},
};
use anyhow::Result;

// ---------------------------------------------------------------------------
// YOLO パス
// ---------------------------------------------------------------------------

/// YOLO の検出結果を使ってリザルト画面から全データを抽出する。
///
/// `result`: capture_loop が MyPlayerRow の y 座標から判定した "win" / "lose" 文字列。
/// BBox が検出されなかった項目は `None` を返す (OCR 失敗扱い)。
/// 呼び出し元は `tokio::task::block_in_place` でラップすること (OCR がブロッキング)。
pub fn extract_from_yolo_detections(
    frame:      &CapturedFrame,
    detections: &[Detection],
    result:     &str,           // "win" | "lose"  (capture_loop 側で判定済み)
) -> Result<ExtractedMatchData> {
    // ルール・ステージ・モード: BBox クロップ → 白文字抽出 → OCR → 正規化
    let rule  = extract_ocr_from_class(frame, detections, YoloClass::RuleText,  "ja-JP")
        .and_then(|t| normalize_rule(&t));
    let stage = extract_ocr_from_class(frame, detections, YoloClass::StageText, "ja-JP")
        .and_then(|t| normalize_stage(&t));
    let mode  = extract_ocr_from_class(frame, detections, YoloClass::ModeText,  "ja-JP")
        .and_then(|t| normalize_mode(&t));

    // KDA: MyArrow BBox の y 中心 → 固定列位置でクロップ → OCR
    let kda_y = YoloDetector::best_detection(detections, YoloClass::MyArrow)
        .map(|d| (d.bbox.y1 + d.bbox.y2) / 2.0)
        .unwrap_or(0.5);
    let (kill_count, death_count, special_count) = extract_kda(frame, kda_y)?;

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
        xp_after: None, // フェーズ2 (Xパワー画面) で実装
        rule,
        stage,
        gold_award_count: Some(gold_award_count),
    })
}

/// 指定クラスの最高確信度 BBox をクロップして白文字 OCR し生テキストを返す。
fn extract_ocr_from_class(
    frame:      &CapturedFrame,
    detections: &[Detection],
    class:      YoloClass,
    lang:       &str,
) -> Option<String> {
    let det = YoloDetector::best_detection(detections, class)?;

    let x1 = (det.bbox.x1 * frame.width  as f32) as u32;
    let y1 = (det.bbox.y1 * frame.height as f32) as u32;
    let x2 = ((det.bbox.x2 * frame.width  as f32) as u32).min(frame.width.saturating_sub(1));
    let y2 = ((det.bbox.y2 * frame.height as f32) as u32).min(frame.height.saturating_sub(1));
    let w  = x2.saturating_sub(x1).max(1);
    let h  = y2.saturating_sub(y1).max(1);

    let cropped      = crop_bgra(&frame.bgra, frame.width, x1, y1, w, h);
    let preprocessed = extract_white_text(&cropped, w, h);
    let text = ocr_from_bgra(&preprocessed, w, h, Some(lang)).ok()?.text;
    let trimmed = text.trim().to_string();
    if trimmed.is_empty() { None } else { Some(trimmed) }
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
        mode: None, // 固定 ROI パスではモード取得なし
        kill_count,
        death_count,
        special_count,
        xp_after,
        rule,
        stage,
        gold_award_count: None, // ピクセルパスでは金表彰取得なし
    })
}

// ---------------------------------------------------------------------------
// 共通 KDA 抽出 (YOLO / 固定 ROI 両パスで使用)
// ---------------------------------------------------------------------------

const KILL_COL_X:  f32 = 0.758;
const DEATH_COL_X: f32 = 0.822;
const SPEC_COL_X:  f32 = 0.863;
const KDA_COL_W:   f32 = 0.048;
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
        extract_integer_roi(frame, &kill_roi,  "en-US"),
        extract_integer_roi(frame, &deat_roi,  "en-US"),
        extract_integer_roi(frame, &spec_roi,  "en-US"),
    ))
}

fn extract_xp(frame: &CapturedFrame) -> Result<Option<f64>> {
    let (x, y, w, h) = XP_ROI.to_pixels(frame.width, frame.height);
    let roi = crop_bgra(&frame.bgra, frame.width, x, y, w, h);
    let text = ocr_from_bgra(&roi, w, h, Some("en-US"))?.text;
    Ok(clean_numeric_text(&text).parse::<f64>().ok())
}

fn extract_rule(frame: &CapturedFrame) -> Option<String> {
    let (x, y, w, h) = RULE_ROI.to_pixels(frame.width, frame.height);
    let roi = crop_bgra(&frame.bgra, frame.width, x, y, w, h);
    let preprocessed = preprocess_bgra(&roi, w, h);
    let text = ocr_from_bgra(&preprocessed, w, h, Some("ja-JP")).ok()?.text;
    normalize_rule(text.trim())
}

fn extract_stage(frame: &CapturedFrame) -> Option<String> {
    normalize_stage(&extract_stage_raw(frame))
}

/// ルール ROI の生 OCR テキストを返す（デバッグ・通常抽出共用）
pub fn extract_rule_raw(frame: &CapturedFrame) -> String {
    let (x, y, w, h) = RULE_ROI.to_pixels(frame.width, frame.height);
    let roi = crop_bgra(&frame.bgra, frame.width, x, y, w, h);
    let preprocessed = preprocess_bgra(&roi, w, h);
    ocr_from_bgra(&preprocessed, w, h, Some("ja-JP"))
        .ok()
        .map(|r| r.text.trim().to_string())
        .unwrap_or_default()
}

/// ステージ ROI の生 OCR テキストを返す（デバッグ・通常抽出共用）
pub fn extract_stage_raw(frame: &CapturedFrame) -> String {
    let (x, y, w, h) = STAGE_ROI.to_pixels(frame.width, frame.height);
    let roi = crop_bgra(&frame.bgra, frame.width, x, y, w, h);
    let preprocessed = preprocess_bgra(&roi, w, h);
    ocr_from_bgra(&preprocessed, w, h, Some("ja-JP"))
        .ok()
        .map(|r| r.text.trim().to_string())
        .unwrap_or_default()
}

fn extract_integer_roi(frame: &CapturedFrame, roi: &Roi, lang: &str) -> Option<i64> {
    let (x, y, w, h) = roi.to_pixels(frame.width, frame.height);
    let cropped = crop_bgra(&frame.bgra, frame.width, x, y, w, h);
    let text = ocr_from_bgra(&cropped, w, h, Some(lang)).ok()?.text;
    clean_numeric_text(&text).parse::<i64>().ok()
}

fn clean_numeric_text(text: &str) -> String {
    text.chars().filter(|c| c.is_ascii_digit() || *c == '.').collect()
}

// ---------------------------------------------------------------------------
// 正規化
// ---------------------------------------------------------------------------

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
    if raw.trim().is_empty() { None } else { Some(raw.trim().to_string()) }
}

pub fn normalize_stage(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() { return None; }
    let stages = [
        "ユノハナ大渓谷", "ゴンズイ地区", "ヤガラ市場", "マテガイ放水路",
        "ナメロウ金属", "ナンプラー遺跡", "クサヤ温泉", "ヒラメが丘団地",
        "マサバ海峡大橋", "スメーシーワールド", "キンメダイ美術館",
        "タラポートショッピングパーク", "バイガイ亭", "海女美術大学",
        "チョウザメ造船", "ザトウマーケット", "リュウグウターミナル",
        "オヒョウ海運", "カジキ空港", "バンカラ街", "冷凍倉庫",
        "ネギトロ炭鉱", "ショッツル鉱山",
    ];
    for stage in &stages {
        if trimmed.contains(stage) { return Some(stage.to_string()); }
        if trimmed.chars().count() >= 4 && stage.contains(trimmed) {
            return Some(stage.to_string());
        }
    }
    None
}

pub fn normalize_mode(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() { return None; }
    let candidates: &[(&str, &[&str])] = &[
        ("Xマッチ",                   &["Xマッチ", "X BATTLE", "Xバトル", "X MATCH"]),
        ("バンカラマッチ(チャレンジ)", &["チャレンジ", "CHALLENGE", "ANARCHY OPEN"]),
        ("バンカラマッチ(オープン)",   &["オープン", "OPEN", "ANARCHY BATTLE", "バンカラ"]),
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
    Some(trimmed.to_string())
}

// ---------------------------------------------------------------------------
// デバッグ OCR (全フィールド生テキスト + 正規化)
// ---------------------------------------------------------------------------

/// YOLO 検出結果を使って全フィールドの OCR 生テキストと正規化値を返す。
/// debug_yolo コマンドから呼ばれる。ブロッキング呼び出しのため block_in_place 必須。
pub fn extract_debug_ocr(frame: &CapturedFrame, detections: &[Detection]) -> OcrDebugResult {
    let rule_raw  = bbox_to_raw_text(frame, detections, YoloClass::RuleText,  "ja-JP");
    let stage_raw = bbox_to_raw_text(frame, detections, YoloClass::StageText, "ja-JP");
    let mode_raw  = bbox_to_raw_text(frame, detections, YoloClass::ModeText,  "ja-JP");

    let arrow_y = YoloDetector::best_detection(detections, YoloClass::MyArrow)
        .map(|d| (d.bbox.y1 + d.bbox.y2) / 2.0);

    let (kill_raw, death_raw, special_raw) = arrow_y.map(|y| {
        let y_top = (y - KDA_ROW_H / 2.0).max(0.0);
        (
            roi_to_raw(frame, &Roi { x_ratio: KILL_COL_X,  y_ratio: y_top, w_ratio: KDA_COL_W, h_ratio: KDA_ROW_H }, "en-US"),
            roi_to_raw(frame, &Roi { x_ratio: DEATH_COL_X, y_ratio: y_top, w_ratio: KDA_COL_W, h_ratio: KDA_ROW_H }, "en-US"),
            roi_to_raw(frame, &Roi { x_ratio: SPEC_COL_X,  y_ratio: y_top, w_ratio: KDA_COL_W, h_ratio: KDA_ROW_H }, "en-US"),
        )
    }).unwrap_or_default();

    OcrDebugResult {
        rule:    OcrDebugField { normalized: normalize_rule(&rule_raw),   raw: rule_raw  },
        stage:   OcrDebugField { normalized: normalize_stage(&stage_raw), raw: stage_raw },
        mode:    OcrDebugField { normalized: normalize_mode(&mode_raw),   raw: mode_raw  },
        kill:    OcrDebugField { normalized: clean_numeric_text(&kill_raw).parse::<i64>().ok().map(|v| v.to_string()),    raw: kill_raw    },
        death:   OcrDebugField { normalized: clean_numeric_text(&death_raw).parse::<i64>().ok().map(|v| v.to_string()),  raw: death_raw   },
        special: OcrDebugField { normalized: clean_numeric_text(&special_raw).parse::<i64>().ok().map(|v| v.to_string()), raw: special_raw },
        arrow_y,
    }
}

/// BBox クロップ → 白文字抽出 → OCR 生テキスト。検出なければ空文字。
fn bbox_to_raw_text(frame: &CapturedFrame, detections: &[Detection], class: YoloClass, lang: &str) -> String {
    let Some(det) = YoloDetector::best_detection(detections, class) else { return String::new(); };
    let x1 = (det.bbox.x1 * frame.width  as f32) as u32;
    let y1 = (det.bbox.y1 * frame.height as f32) as u32;
    let x2 = ((det.bbox.x2 * frame.width  as f32) as u32).min(frame.width.saturating_sub(1));
    let y2 = ((det.bbox.y2 * frame.height as f32) as u32).min(frame.height.saturating_sub(1));
    let w  = x2.saturating_sub(x1).max(1);
    let h  = y2.saturating_sub(y1).max(1);
    let cropped      = crop_bgra(&frame.bgra, frame.width, x1, y1, w, h);
    let preprocessed = extract_white_text(&cropped, w, h);
    ocr_from_bgra(&preprocessed, w, h, Some(lang)).ok()
        .map(|r| r.text.trim().to_string())
        .unwrap_or_default()
}

/// ROI → OCR 生テキスト (前処理なし、数値用)。
fn roi_to_raw(frame: &CapturedFrame, roi: &Roi, lang: &str) -> String {
    let (x, y, w, h) = roi.to_pixels(frame.width, frame.height);
    let cropped = crop_bgra(&frame.bgra, frame.width, x, y, w, h);
    ocr_from_bgra(&cropped, w, h, Some(lang)).ok()
        .map(|r| r.text.trim().to_string())
        .unwrap_or_default()
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
    }

    #[test]
    fn test_normalize_stage() {
        assert_eq!(normalize_stage("マテガイ放水路"), Some("マテガイ放水路".to_string()));
        assert_eq!(normalize_stage("ナメロウ金属"),   Some("ナメロウ金属".to_string()));
        assert_eq!(normalize_stage(""),               None);
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
    }

    #[test]
    fn test_clean_numeric_text() {
        assert_eq!(clean_numeric_text("2341.5 XP"), "2341.5");
        assert_eq!(clean_numeric_text("Kill: 5"),   "5");
        assert_eq!(clean_numeric_text("abc"),        "");
    }
}
