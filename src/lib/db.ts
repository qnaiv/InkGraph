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
    kill_count, death_count, special_count, paint_count, xp_after, gold_award_count,
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
        kill_count, death_count, special_count, paint_count, xp_after, gold_award_count, tags, note)
     VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15)`,
    [
      m.id,
      m.played_at,
      m.mode              ?? null,
      m.rule              ?? null,
      m.stage             ?? null,
      m.weapon            ?? null,
      m.result,
      m.kill_count        ?? null,
      m.death_count       ?? null,
      m.special_count     ?? null,
      m.paint_count       ?? null,
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
         kill_count = $5, death_count = $6, special_count = $7, paint_count = $8,
         xp_after = $9, gold_award_count = $10, updated_at = CURRENT_TIMESTAMP
     WHERE id = $11`,
    [
      m.result,
      m.mode             ?? null,
      m.rule             ?? null,
      m.stage            ?? null,
      m.kill_count       ?? null,
      m.death_count      ?? null,
      m.special_count    ?? null,
      m.paint_count      ?? null,
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

/** 既存レコードの全フィールドを更新する (手動編集用) */
export async function dbUpdateFullMatch(m: RawMatch): Promise<void> {
  const db = await getDb();
  await db.execute(
    `UPDATE matches
     SET played_at = $1, mode = $2, rule = $3, stage = $4, weapon = $5,
         result = $6, kill_count = $7, death_count = $8, special_count = $9,
         paint_count = $10, xp_after = $11, gold_award_count = $12,
         tags = $13, note = $14, updated_at = CURRENT_TIMESTAMP
     WHERE id = $15`,
    [
      m.played_at,
      m.mode             ?? null,
      m.rule             ?? null,
      m.stage            ?? null,
      m.weapon           ?? null,
      m.result,
      m.kill_count       ?? null,
      m.death_count      ?? null,
      m.special_count    ?? null,
      m.paint_count      ?? null,
      m.xp_after         ?? null,
      m.gold_award_count ?? null,
      m.tags             ?? '[]',
      m.note             ?? null,
      m.id,
    ],
  );
}

/** 試合レコードを削除する */
export async function dbDeleteMatch(id: string): Promise<void> {
  const db = await getDb();
  await db.execute('DELETE FROM matches WHERE id = $1', [id]);
}

/** 全試合を取得する (件数上限なし) */
export async function selectAllMatches(rule?: string | null): Promise<RawMatch[]> {
  const db = await getDb();
  const cols = `
    id, played_at, mode, rule, stage, weapon, result,
    kill_count, death_count, special_count, paint_count, xp_after, gold_award_count,
    tags, note, created_at, updated_at
  `;
  if (rule) {
    return db.select<RawMatch[]>(
      `SELECT ${cols} FROM matches WHERE rule = $1 ORDER BY played_at DESC`,
      [rule],
    );
  }
  return db.select<RawMatch[]>(
    `SELECT ${cols} FROM matches ORDER BY played_at DESC`,
    [],
  );
}
