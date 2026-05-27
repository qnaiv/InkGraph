-- IkaVision XP: 初期マイグレーション
-- matches テーブル: Xマッチ対戦記録

CREATE TABLE IF NOT EXISTS matches (
    id           TEXT     PRIMARY KEY,
    played_at    DATETIME NOT NULL,
    rule         TEXT,
    stage        TEXT,
    weapon       TEXT,
    result       TEXT     NOT NULL CHECK(result IN ('win', 'lose')),
    kill_count   INTEGER,
    assist_count INTEGER,
    death_count  INTEGER,
    xp_after     REAL,
    tags         TEXT     DEFAULT '[]',  -- JSON 配列文字列
    note         TEXT,
    created_at   DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at   DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- XP 推移グラフ用インデックス
CREATE INDEX IF NOT EXISTS idx_matches_played_at ON matches(played_at);
CREATE INDEX IF NOT EXISTS idx_matches_rule      ON matches(rule);
CREATE INDEX IF NOT EXISTS idx_matches_result    ON matches(result);
