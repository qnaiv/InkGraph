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

// ── ルール・ステージ (実測値: 1456×816 Xマッチスクショ 2026-05-28) ──────
// リザルト画面上部に「ガチエリア  タラポートショッピングパーク」のように並ぶ。

/// ルール名: 実測 x≈668/1456=0.459, w≈135px
const RULE_ROI: Roi = Roi {
    x_ratio: 0.450,
    y_ratio: 0.060,
    w_ratio: 0.120,
    h_ratio: 0.058,
};

/// ステージ名: 実測 x≈820/1456=0.563, w≈330px
const STAGE_ROI: Roi = Roi {
    x_ratio: 0.545,
    y_ratio: 0.060,
    w_ratio: 0.240,
    h_ratio: 0.058,
};

// ── KDA 列 x 座標 (実測値: 1456×816 Xマッチスクショ 2026-05-28) ────────
// Splatoon3 リザルトパネルの列構成: [キル] [デス] [スペシャル]
// y 座標は矢印の y 重心から動的に決める (プレイヤー行は 1〜4 で変わるため)。
// ExtractedMatchData の kill / assist / death フィールドに格納する。

const KILL_COL_X:  f32 = 0.758; // x≈1104/1456
const DEATH_COL_X: f32 = 0.822; // x≈1197/1456
const SPEC_COL_X:  f32 = 0.863; // x≈1257/1456
const KDA_COL_W:   f32 = 0.048; // 幅≈70px
const KDA_ROW_H:   f32 = 0.052; // 高さ≈42px

// ── XP (Xマッチ専用・別画面) ─────────────────────────────────────────
// Xパワーは試合リザルトパネルではなく個人サマリ画面に表示される。
// TODO: 実際の表示画面スクショで座標を確定する

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

/// リザルト画面フレームから全データを抽出する。
/// `arrow_y_ratio`: 黄色矢印の y 重心比率 (detector から受け取る)。
/// プレイヤーが何行目にいるかが毎試合変わるため、この値で KDA 行を動的に特定する。
pub fn extract_match_data(frame: &CapturedFrame, result: &str, arrow_y_ratio: f32) -> Result<ExtractedMatchData> {
    let (kill_count, assist_count, death_count) = extract_kda(frame, arrow_y_ratio)?;
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

/// KDA を抽出する。
/// `arrow_y_ratio` から y_top を計算し、プレイヤー行の KDA 列を読む。
/// 列の意味: kill=キル / assist=デス / death=スペシャル (DB スキーマ名に合わせて格納)
fn extract_kda(frame: &CapturedFrame, arrow_y_ratio: f32) -> Result<(Option<i64>, Option<i64>, Option<i64>)> {
    // 矢印重心を行の中央とみなし、上下 KDA_ROW_H/2 の範囲を切り出す
    let y_ratio = (arrow_y_ratio - KDA_ROW_H / 2.0).max(0.0);

    let kill_roi = Roi { x_ratio: KILL_COL_X,  y_ratio, w_ratio: KDA_COL_W, h_ratio: KDA_ROW_H };
    let deat_roi = Roi { x_ratio: DEATH_COL_X, y_ratio, w_ratio: KDA_COL_W, h_ratio: KDA_ROW_H };
    let spec_roi = Roi { x_ratio: SPEC_COL_X,  y_ratio, w_ratio: KDA_COL_W, h_ratio: KDA_ROW_H };

    let kill  = extract_integer_roi(frame, &kill_roi, "en-US");
    let death = extract_integer_roi(frame, &deat_roi, "en-US");
    let spec  = extract_integer_roi(frame, &spec_roi, "en-US");

    Ok((kill, death, spec))
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
