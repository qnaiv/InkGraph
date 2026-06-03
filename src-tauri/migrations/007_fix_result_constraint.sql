-- 007: result カラムの CHECK 制約を 'in_progress' / 'draw' も許可するよう拡張
--
-- 001_initial.sql では CHECK(result IN ('win', 'lose')) のみ許可していたため、
-- battle_started 時の result='in_progress' INSERT が常に失敗していた。
-- SQLite は既存カラムの CHECK 制約を直接変更できないため、テーブルを再作成する。

PRAGMA foreign_keys=off;

BEGIN TRANSACTION;

CREATE TABLE matches_new (
    id               TEXT     PRIMARY KEY,
    played_at        DATETIME NOT NULL,
    mode             TEXT,
    rule             TEXT,
    stage            TEXT,
    weapon           TEXT,
    result           TEXT     NOT NULL CHECK(result IN ('win', 'lose', 'in_progress', 'draw')),
    kill_count       INTEGER,
    assist_count     INTEGER,
    death_count      INTEGER,
    special_count    INTEGER,
    paint_count      INTEGER,
    xp_after         REAL,
    gold_award_count INTEGER,
    tags             TEXT     DEFAULT '[]',
    note             TEXT,
    created_at       DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at       DATETIME DEFAULT CURRENT_TIMESTAMP
);

INSERT INTO matches_new
    SELECT id, played_at, mode, rule, stage, weapon, result,
           kill_count, assist_count, death_count, special_count, paint_count,
           xp_after, gold_award_count, tags, note, created_at, updated_at
    FROM matches;

DROP TABLE matches;

ALTER TABLE matches_new RENAME TO matches;

CREATE INDEX IF NOT EXISTS idx_matches_played_at ON matches(played_at);
CREATE INDEX IF NOT EXISTS idx_matches_rule      ON matches(rule);
CREATE INDEX IF NOT EXISTS idx_matches_result    ON matches(result);
CREATE INDEX IF NOT EXISTS idx_matches_mode      ON matches(mode);

COMMIT;

PRAGMA foreign_keys=on;
