/// IkaVision XP — データベースヘルパー
///
/// DB の CRUD は tauri-plugin-sql 経由でフロントエンド (JS) が直接実行します。
/// Rust 側はマイグレーション定義と、キャプチャループで使う Match 生成のみを担います。
use crate::types::Match;
use chrono::Utc;
use uuid::Uuid;

/// DB ファイル名 (lib.rs のマイグレーション設定と共有)
pub const DB_URL: &str = "sqlite:matches.db";

/// WinRT OCR の抽出結果から Match レコードを構築するヘルパー
/// capture_loop.rs から呼ばれ、`match_detected` イベントのペイロードになる。
/// 実際の DB 挿入はフロントエンドが受信後に行う。
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
