/// InkGraph — データベースヘルパー
///
/// DB の CRUD は tauri-plugin-sql 経由でフロントエンド (JS) が直接実行します。
/// Rust 側はマイグレーション定義と、キャプチャループで使う Match 生成のみを担います。
use crate::types::Match;
use chrono::Utc;
use uuid::Uuid;

/// DB ファイル名 (lib.rs のマイグレーション設定と共有)
pub const DB_URL: &str = "sqlite:matches.db";

/// バトル開始時に作成する "in_progress" レコードを構築する。
/// id は新規 UUID を発行し、capture_loop.rs が pending_match_id として保持する。
pub fn new_in_progress_match() -> Match {
    Match {
        id: Uuid::new_v4().to_string(),
        played_at: Utc::now().to_rfc3339(),
        rule: None,
        stage: None,
        weapon: None,
        result: "in_progress".to_string(),
        kill_count: None,
        assist_count: None,
        death_count: None,
        xp_after: None,
        tags: Some("[]".to_string()),
        note: None,
        created_at: None,
        updated_at: None,
    }
}

/// OCR / YOLO 抽出結果から Match レコードを構築するヘルパー。
///
/// `id` が `Some` の場合は既存の "in_progress" レコードを上書きするため
/// 同じ UUID を使い回す。`None` の場合は新規 UUID を発行する。
pub fn new_match_from_ocr(
    id: Option<String>,
    result: &str,
    kill_count: Option<i64>,
    assist_count: Option<i64>,
    death_count: Option<i64>,
    xp_after: Option<f64>,
    rule: Option<String>,
    stage: Option<String>,
    mode: Option<String>,
) -> Match {
    Match {
        id: id.unwrap_or_else(|| Uuid::new_v4().to_string()),
        played_at: Utc::now().to_rfc3339(),
        mode,
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
    fn test_new_in_progress_match() {
        let m = new_in_progress_match();
        assert!(!m.id.is_empty());
        assert_eq!(m.result, "in_progress");
        assert!(m.kill_count.is_none());
    }

    #[test]
    fn test_new_match_reuses_id() {
        let id = "existing-id".to_string();
        let m = new_match_from_ocr(Some(id.clone()), "win", Some(5), Some(1), Some(2), Some(2341.5), None, None, Some("Xマッチ".to_string()));
        assert_eq!(m.id, id);
        assert_eq!(m.result, "win");
        assert_eq!(m.mode.as_deref(), Some("Xマッチ"));
    }

    #[test]
    fn test_new_match_generates_id_when_none() {
        let m = new_match_from_ocr(None, "lose", None, None, None, None, None, None, None);
        assert!(!m.id.is_empty());
        assert_eq!(m.result, "lose");
        assert!(m.mode.is_none());
    }
}
