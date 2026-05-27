/// IkaVision XP — データベース操作
///
/// tauri-plugin-sql を使って SQLite に対する CRUD 操作を提供します。
/// マイグレーションは `migrations/` ディレクトリで管理します。
use crate::types::Match;
use anyhow::Result;
use chrono::Utc;
use serde_json::Value;
use uuid::Uuid;

/// DB ファイル名
pub const DB_URL: &str = "sqlite:matches.db";

// ---------------------------------------------------------------------------
// CRUD 操作
// (tauri-plugin-sql は Tauri コマンド内で直接使うため、
//  ここでは生の SQL 文字列とパラメータ構造を定義する)
// ---------------------------------------------------------------------------

/// 試合レコードを INSERT するための SQL
pub fn insert_match_sql() -> &'static str {
    r#"
    INSERT INTO matches
        (id, played_at, rule, stage, weapon, result,
         kill_count, assist_count, death_count, xp_after, tags, note)
    VALUES
        ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
    "#
}

/// ブキを更新する SQL
pub fn update_weapon_sql() -> &'static str {
    "UPDATE matches SET weapon = $1, updated_at = CURRENT_TIMESTAMP WHERE id = $2"
}

/// タグを更新する SQL
pub fn update_tags_sql() -> &'static str {
    "UPDATE matches SET tags = $1, updated_at = CURRENT_TIMESTAMP WHERE id = $2"
}

/// メモを更新する SQL
pub fn update_note_sql() -> &'static str {
    "UPDATE matches SET note = $1, updated_at = CURRENT_TIMESTAMP WHERE id = $2"
}

/// 直近 N 件を取得する SQL
pub fn select_recent_sql(rule_filter: bool) -> String {
    if rule_filter {
        r#"
        SELECT id, played_at, rule, stage, weapon, result,
               kill_count, assist_count, death_count, xp_after, tags, note,
               created_at, updated_at
        FROM matches
        WHERE rule = $1
        ORDER BY played_at DESC
        LIMIT $2
        "#
        .to_string()
    } else {
        r#"
        SELECT id, played_at, rule, stage, weapon, result,
               kill_count, assist_count, death_count, xp_after, tags, note,
               created_at, updated_at
        FROM matches
        ORDER BY played_at DESC
        LIMIT $1
        "#
        .to_string()
    }
}

/// XP 推移グラフ用データを取得する SQL
pub fn select_xp_history_sql(rule_filter: bool) -> String {
    if rule_filter {
        r#"
        SELECT played_at, xp_after, result
        FROM matches
        WHERE rule = $1 AND xp_after IS NOT NULL
        ORDER BY played_at ASC
        "#
        .to_string()
    } else {
        r#"
        SELECT played_at, xp_after, result
        FROM matches
        WHERE xp_after IS NOT NULL
        ORDER BY played_at ASC
        "#
        .to_string()
    }
}

/// 新しい Match を構築するヘルパー
pub fn new_match_from_ocr(
    result: &str,
    kill_count: Option<i64>,
    assist_count: Option<i64>,
    death_count: Option<i64>,
    xp_after: Option<f64>,
    rule: Option<String>,
    stage: Option<String>,
) -> Match {
    Match {
        id: Uuid::new_v4().to_string(),
        played_at: Utc::now().to_rfc3339(),
        rule,
        stage,
        weapon: None,
        result: result.to_string(),
        kill_count,
        assist_count,
        death_count,
        xp_after,
        tags: Some("[]".to_string()),
        note: None,
        created_at: None,
        updated_at: None,
    }
}

/// JSON Value の列から Match を構築する
pub fn match_from_row(row: &Value) -> Option<Match> {
    Some(Match {
        id: row.get("id")?.as_str()?.to_string(),
        played_at: row.get("played_at")?.as_str()?.to_string(),
        rule: row.get("rule").and_then(|v| v.as_str()).map(String::from),
        stage: row.get("stage").and_then(|v| v.as_str()).map(String::from),
        weapon: row.get("weapon").and_then(|v| v.as_str()).map(String::from),
        result: row.get("result")?.as_str()?.to_string(),
        kill_count: row.get("kill_count").and_then(|v| v.as_i64()),
        assist_count: row.get("assist_count").and_then(|v| v.as_i64()),
        death_count: row.get("death_count").and_then(|v| v.as_i64()),
        xp_after: row.get("xp_after").and_then(|v| v.as_f64()),
        tags: row.get("tags").and_then(|v| v.as_str()).map(String::from),
        note: row.get("note").and_then(|v| v.as_str()).map(String::from),
        created_at: row
            .get("created_at")
            .and_then(|v| v.as_str())
            .map(String::from),
        updated_at: row
            .get("updated_at")
            .and_then(|v| v.as_str())
            .map(String::from),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_match_has_valid_uuid() {
        let m = new_match_from_ocr("win", Some(5), Some(1), Some(2), Some(2341.5), None, None);
        assert!(!m.id.is_empty());
        assert_eq!(m.result, "win");
        assert_eq!(m.kill_count, Some(5));
        assert_eq!(m.tags, Some("[]".to_string()));
    }
}
