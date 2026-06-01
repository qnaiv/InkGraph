// InkGraph — SQLite アクセス層
//
// tauri-plugin-sql の Database クラスを薄くラップします。
// DB 接続はシングルトンとして保持し、初回アクセス時に自動接続します。
// マイグレーションは Rust 側 (lib.rs) が起動時に自動実行します。

import Database from '@tauri-apps/plugin-sql';
import type { RawMatch } from '../types';

// ---------------------------------------------------------------------------
// 接続管理
// ---------------------------------------------------------------------------

let _db: Database | null = null;

export async function getDb(): Promise<Database> {
  if (!_db) {
    _db = await Database.load('sqlite:matches.db');
  }
  return _db;
}

// ---------------------------------------------------------------------------
// SELECT
// ---------------------------------------------------------------------------

/** 直近の試合一覧を取得する */
export async function selectMatches(
  limit: number,
  rule?: string | null,
): Promise<RawMatch[]> {
  const db = await getDb();
  const cols = `
    id, played_at, mode, rule, stage, weapon, result,
    kill_count, assist_count, death_count, xp_after, gold_award_count,
    tags, note, created_at, updated_at
  `;
  if (rule) {
    return db.select<RawMatch[]>(
      `SELECT ${cols} FROM matches WHERE rule = $1 ORDER BY played_at DESC LIMIT $2`,
      [rule, limit],
    );
  }
  return db.select<RawMatch[]>(
    `SELECT ${cols} FROM matches ORDER BY played_at DESC LIMIT $1`,
    [limit],
  );
}

// ---------------------------------------------------------------------------
// INSERT
// ---------------------------------------------------------------------------

/** 試合レコードを挿入する (match_detected イベント受信時に呼ぶ) */
export async function insertMatch(m: RawMatch): Promise<void> {
  const db = await getDb();
  await db.execute(
    `INSERT INTO matches
       (id, played_at, mode, rule, stage, weapon, result,
        kill_count, assist_count, death_count, xp_after, gold_award_count, tags, note)
     VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14)`,
    [
      m.id,
      m.played_at,
      m.mode   ?? null,
      m.rule   ?? null,
      m.stage  ?? null,
      m.weapon ?? null,
      m.result,
      m.kill_count        ?? null,
      m.assist_count      ?? null,
      m.death_count       ?? null,
      m.xp_after          ?? null,
      m.gold_award_count  ?? null,
      m.tags ?? '[]',
      m.note ?? null,
    ],
  );
}

// ---------------------------------------------------------------------------
// UPDATE
// ---------------------------------------------------------------------------

/**
 * "in_progress" レコードを win/lose に更新する。
 * 同一 ID のレコードが存在しない場合は新規 INSERT にフォールバックする。
 */
export async function updateMatchResult(m: RawMatch): Promise<void> {
  const db = await getDb();
  const res = await db.execute(
    `UPDATE matches
     SET result = $1, mode = $2, rule = $3, stage = $4,
         kill_count = $5, assist_count = $6, death_count = $7, xp_after = $8,
         gold_award_count = $9, updated_at = CURRENT_TIMESTAMP
     WHERE id = $10`,
    [
      m.result,
      m.mode             ?? null,
      m.rule             ?? null,
      m.stage            ?? null,
      m.kill_count       ?? null,
      m.assist_count     ?? null,
      m.death_count      ?? null,
      m.xp_after         ?? null,
      m.gold_award_count ?? null,
      m.id,
    ],
  );
  if (res.rowsAffected === 0) {
    // in_progress レコードが存在しない場合 (capture 途中から開始した等) は挿入
    await insertMatch(m);
  }
}

export async function dbUpdateWeapon(id: string, weapon: string): Promise<void> {
  const db = await getDb();
  await db.execute(
    'UPDATE matches SET weapon = $1, updated_at = CURRENT_TIMESTAMP WHERE id = $2',
    [weapon, id],
  );
}

export async function dbUpdateTags(id: string, tags: string[]): Promise<void> {
  const db = await getDb();
  await db.execute(
    'UPDATE matches SET tags = $1, updated_at = CURRENT_TIMESTAMP WHERE id = $2',
    [JSON.stringify(tags), id],
  );
}

export async function dbUpdateNote(id: string, note: string): Promise<void> {
  const db = await getDb();
  await db.execute(
    'UPDATE matches SET note = $1, updated_at = CURRENT_TIMESTAMP WHERE id = $2',
    [note, id],
  );
}
