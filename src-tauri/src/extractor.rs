use crate::{
    capture::CapturedFrame,
    detector::crop_bgra,
    ocr::{ocr_from_bgra, preprocess_bgra},
    types::ExtractedMatchData,
};
/// IkaVision XP — データ抽出モジュール
///
/// リザルト画面から各 ROI をクロップして OCR し、
/// キル・デス・XP・ルール・ステージを数値/文字列に変換します。
use anyhow::Result;

// ---------------------------------------------------------------------------
// ROI 定義
// ---------------------------------------------------------------------------
//
// 全座標は 16:9 フレームに対する比率で定義します。
// スクリーンショット実測値 (バンカラマッチ 1456×816, 2026-05-27)。
// Xマッチでも rule/stage/WIN LOSE の配置は同じです。
// KDA・XP の ROI は Xマッチリザルト画面のスクショで要調整。

use crate::detector::Roi;

// ── ルール・ステージ (実測値) ────────────────────────────────────────────
// リザルト画面上部に「ガチヤグラ  リュウグウターミナル」のように並ぶ。

/// ルール名 (ガチエリア / ガチヤグラ / ガチホコ / ガチアサリ)
const RULE_ROI: Roi = Roi {
    x_ratio: 0.410,
    y_ratio: 0.060,
    w_ratio: 0.140,
    h_ratio: 0.065,
};

/// ステージ名
const STAGE_ROI: Roi = Roi {
    x_ratio: 0.500,
    y_ratio: 0.060,
    w_ratio: 0.280,
    h_ratio: 0.065,
};

// ── KDA (TODO: Xマッチスクショで要調整) ────────────────────────────────
// 自チームの自分の行: 左から Kill / Assist / Death のアイコン + 数字。
// 現在値は仮の推定値。

/// 自分のキル数
const KILL_ROI: Roi = Roi {
    x_ratio: 0.750,
    y_ratio: 0.340,
    w_ratio: 0.055,
    h_ratio: 0.060,
};
/// アシスト数
const ASSIST_ROI: Roi = Roi {
    x_ratio: 0.820,
    y_ratio: 0.340,
    w_ratio: 0.045,
    h_ratio: 0.060,
};
/// デス数
const DEATH_ROI: Roi = Roi {
    x_ratio: 0.875,
    y_ratio: 0.340,
    w_ratio: 0.050,
    h_ratio: 0.060,
};

// ── XP (TODO: Xマッチスクショで要調整) ─────────────────────────────────
// Xマッチリザルト画面にのみ表示される Xパワー値。
// バンカラ/ナワバリには存在しないため None を返す。

/// X パワー表示領域 (Xマッチ専用)
const XP_ROI: Roi = Roi {
    x_ratio: 0.455,
    y_ratio: 0.185,
    w_ratio: 0.180,
    h_ratio: 0.060,
};

// ---------------------------------------------------------------------------
// 抽出処理
// ---------------------------------------------------------------------------

/// リザルト画面フレームから全データを抽出する
pub fn extract_match_data(frame: &CapturedFrame, result: &str) -> Result<ExtractedMatchData> {
    let (kill_count, assist_count, death_count) = extract_kda(frame)?;
    let xp_after = extract_xp(frame)?;
    let rule = extract_rule(frame);
    let stage = extract_stage(frame);

    Ok(ExtractedMatchData {
        result: result.to_string(),
        kill_count,
        assist_count,
        death_count,
        xp_after,
        rule,
        stage,
    })
}

/// KDA を抽出する
fn extract_kda(frame: &CapturedFrame) -> Result<(Option<i64>, Option<i64>, Option<i64>)> {
    let kill = extract_integer_roi(frame, &KILL_ROI, "en-US");
    let assist = extract_integer_roi(frame, &ASSIST_ROI, "en-US");
    let death = extract_integer_roi(frame, &DEATH_ROI, "en-US");
    Ok((kill, assist, death))
}

/// XP を抽出する（小数点以下1桁の float）
fn extract_xp(frame: &CapturedFrame) -> Result<Option<f64>> {
    let (x, y, w, h) = XP_ROI.to_pixels(frame.width, frame.height);
    let roi = crop_bgra(&frame.bgra, frame.width, x, y, w, h);

    let ocr_result = ocr_from_bgra(&roi, w, h, Some("en-US"))?;
    let cleaned = clean_numeric_text(&ocr_result.text);

    // XP は "2341.5" や "2341" の形式
    Ok(cleaned.parse::<f64>().ok())
}

/// ルール名を抽出する（前処理あり）
fn extract_rule(frame: &CapturedFrame) -> Option<String> {
    let (x, y, w, h) = RULE_ROI.to_pixels(frame.width, frame.height);
    let roi = crop_bgra(&frame.bgra, frame.width, x, y, w, h);

    // 動く背景対策: 2値化前処理
    let preprocessed = preprocess_bgra(&roi, w, h);

    // ja-JP で日本語テキストを読む
    let text = ocr_from_bgra(&preprocessed, w, h, Some("ja-JP"))
        .ok()
        .map(|r| r.text.trim().to_string())?;

    normalize_rule(&text)
}

/// ステージ名を抽出する（前処理あり）
fn extract_stage(frame: &CapturedFrame) -> Option<String> {
    let (x, y, w, h) = STAGE_ROI.to_pixels(frame.width, frame.height);
    let roi = crop_bgra(&frame.bgra, frame.width, x, y, w, h);

    let preprocessed = preprocess_bgra(&roi, w, h);
    let text = ocr_from_bgra(&preprocessed, w, h, Some("ja-JP"))
        .ok()
        .map(|r| r.text.trim().to_string())?;

    normalize_stage(&text)
}

// ---------------------------------------------------------------------------
// ユーティリティ
// ---------------------------------------------------------------------------

/// ROI を切り出して整数値を OCR で取得する
fn extract_integer_roi(frame: &CapturedFrame, roi: &Roi, lang: &str) -> Option<i64> {
    let (x, y, w, h) = roi.to_pixels(frame.width, frame.height);
    let cropped = crop_bgra(&frame.bgra, frame.width, x, y, w, h);
    let text = ocr_from_bgra(&cropped, w, h, Some(lang)).ok()?.text;
    clean_numeric_text(&text).parse::<i64>().ok()
}

/// OCR テキストから数字以外を除去する
fn clean_numeric_text(text: &str) -> String {
    text.chars()
        .filter(|c| c.is_ascii_digit() || *c == '.')
        .collect()
}

/// ルール名の正規化 (OCR 誤認識対策)
pub fn normalize_rule(raw: &str) -> Option<String> {
    let candidates: &[(&str, &[&str])] = &[
        (
            "ガチエリア",
            &["ガチエリア", "エリア", "AREA", "SPLAT ZONES"],
        ),
        (
            "ガチヤグラ",
            &["ガチヤグラ", "ヤグラ", "TOWER", "TOWER CONTROL"],
        ),
        ("ガチホコ", &["ガチホコ", "ホコ", "RAINMAKER"]),
        (
            "ガチアサリ",
            &["ガチアサリ", "アサリ", "CLAM", "CLAM BLITZ"],
        ),
    ];

    let upper = raw.to_uppercase();
    for (canonical, aliases) in candidates {
        for alias in *aliases {
            if upper.contains(&alias.to_uppercase()) || raw.contains(alias) {
                return Some(canonical.to_string());
            }
        }
    }
    if raw.trim().is_empty() {
        None
    } else {
        Some(raw.trim().to_string())
    }
}

/// ステージ名の正規化
pub fn normalize_stage(raw: &str) -> Option<String> {
    // スプラトゥーン3 全ステージ (2024-2026 時点)
    let stages = [
        "ユノハナ大渓谷",
        "ゴンズイ地区",
        "ヤガラ市場",
        "マテガイ放水路",
        "ナメロウ金属",
        "ナンプラー遺跡",
        "クサヤ温泉",
        "ヒラメが丘団地",
        "マサバ海峡大橋",
        "スメーシーワールド",
        "キンメダイ美術館",
        "タラポートショッピングパーク",
        "バイガイ亭",
        "海女美術大学",
        "チョウザメ造船",
        "ザトウマーケット",
        "リュウグウターミナル",
        "オヒョウ海運",
        "カジキ空港",
        "バンカラ街",
        "冷凍倉庫",
        "ネギトロ炭鉱",
        "ショッツル鉱山",
    ];

    // OCR 結果と部分一致するステージを探す
    for stage in &stages {
        if raw.contains(stage) || stage.contains(raw.trim()) {
            return Some(stage.to_string());
        }
    }

    if raw.trim().is_empty() {
        None
    } else {
        Some(raw.trim().to_string())
    }
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
        assert_eq!(normalize_rule("エリア"), Some("ガチエリア".to_string()));
        assert_eq!(normalize_rule("AREA"), Some("ガチエリア".to_string()));
        assert_eq!(normalize_rule("ガチヤグラ"), Some("ガチヤグラ".to_string()));
        assert_eq!(normalize_rule("RAINMAKER"), Some("ガチホコ".to_string()));
    }

    #[test]
    fn test_normalize_stage() {
        assert_eq!(
            normalize_stage("マテガイ放水路"),
            Some("マテガイ放水路".to_string())
        );
        assert_eq!(
            normalize_stage("ナメロウ金属"),
            Some("ナメロウ金属".to_string())
        );
    }

    #[test]
    fn test_clean_numeric_text() {
        assert_eq!(clean_numeric_text("2341.5 XP"), "2341.5");
        assert_eq!(clean_numeric_text("Kill: 5"), "5");
        assert_eq!(clean_numeric_text("abc"), "");
    }
}
