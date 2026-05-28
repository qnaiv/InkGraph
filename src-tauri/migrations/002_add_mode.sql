-- 002: モード (マッチタイプ) 列追加
-- mode 例: "Xマッチ" / "バンカラマッチ(チャレンジ)" / "バンカラマッチ(オープン)" / "ナワバリバトル"
ALTER TABLE matches ADD COLUMN mode TEXT;

CREATE INDEX IF NOT EXISTS idx_matches_mode ON matches(mode);
