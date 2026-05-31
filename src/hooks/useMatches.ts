// InkGraph — 試合データ管理フック

import { useState, useEffect, useCallback } from 'react';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import type { Match, MatchDetectedPayload, RawMatch, Rule } from '../types';
import {
  selectMatches,
  insertMatch,
  updateMatchResult,
  dbUpdateWeapon,
  dbUpdateTags,
  dbUpdateNote,
} from '../lib/db';

// ---------------------------------------------------------------------------
// 内部ユーティリティ
// ---------------------------------------------------------------------------

function parseMatch(raw: RawMatch): Match {
  let tags: string[] = [];
  if (raw.tags) {
    try { tags = JSON.parse(raw.tags); } catch { tags = []; }
  }
  return { ...raw, tags };
}

// ---------------------------------------------------------------------------
// フック本体
// ---------------------------------------------------------------------------

interface UseMatchesReturn {
  matches: Match[];
  isLoading: boolean;
  error: string | null;
  addMatch:    (raw: RawMatch) => Promise<void>;
  updateWeapon: (id: string, weapon: string) => Promise<void>;
  updateTags:   (id: string, tags: string[]) => Promise<void>;
  updateNote:   (id: string, note: string)   => Promise<void>;
}

export function useMatches(ruleFilter?: Rule | null): UseMatchesReturn {
  const [matches, setMatches] = useState<Match[]>([]);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // ── DB から試合一覧をロード ───────────────────────────────────
  useEffect(() => {
    let cancelled = false;
    async function load() {
      setIsLoading(true);
      setError(null);
      try {
        const rows = await selectMatches(100, ruleFilter);
        if (!cancelled) setMatches(rows.map(parseMatch));
      } catch (e) {
        if (!cancelled) setError(String(e));
      } finally {
        if (!cancelled) setIsLoading(false);
      }
    }
    load();
    return () => { cancelled = true; };
  }, [ruleFilter]);

  // ── Rust からのリアルタイム通知を購読 ───────────────────────
  useEffect(() => {
    const unlisteners: UnlistenFn[] = [];

    // Phase 1: バトル開始 → "in_progress" レコードを追加
    listen<MatchDetectedPayload>('battle_started', async (event) => {
      const raw = event.payload.match_data;
      try {
        await insertMatch(raw);
      } catch (e) {
        console.error('[useMatches] insertMatch(in_progress) failed:', e);
      }
      setMatches((prev) => [parseMatch(raw), ...prev]);
    }).then((fn) => unlisteners.push(fn));

    // Phase 2: リザルト確定 → 既存の "in_progress" レコードを win/lose に更新
    listen<MatchDetectedPayload>('match_detected', async (event) => {
      const raw = event.payload.match_data;
      try {
        await updateMatchResult(raw);
      } catch (e) {
        console.error('[useMatches] updateMatchResult failed:', e);
      }
      setMatches((prev) => {
        const exists = prev.some((m) => m.id === raw.id);
        if (exists) {
          // in_progress → win/lose にインプレース更新
          return prev.map((m) => (m.id === raw.id ? parseMatch(raw) : m));
        }
        // キャプチャ途中開始などで in_progress がない場合は先頭に追加
        return [parseMatch(raw), ...prev];
      });
    }).then((fn) => unlisteners.push(fn));

    return () => { unlisteners.forEach((fn) => fn()); };
  }, []);

  // ── 手動追加 ─────────────────────────────────────────────────
  const addMatch = useCallback(async (raw: RawMatch) => {
    await insertMatch(raw);
    setMatches((prev) => [parseMatch(raw), ...prev]);
  }, []);

  // ── 更新操作 ─────────────────────────────────────────────────
  const updateWeapon = useCallback(async (id: string, weapon: string) => {
    await dbUpdateWeapon(id, weapon);
    setMatches((prev) => prev.map((m) => (m.id === id ? { ...m, weapon } : m)));
  }, []);

  const updateTags = useCallback(async (id: string, tags: string[]) => {
    await dbUpdateTags(id, tags);
    setMatches((prev) => prev.map((m) => (m.id === id ? { ...m, tags } : m)));
  }, []);

  const updateNote = useCallback(async (id: string, note: string) => {
    await dbUpdateNote(id, note);
    setMatches((prev) => prev.map((m) => (m.id === id ? { ...m, note } : m)));
  }, []);

  return { matches, isLoading, error, addMatch, updateWeapon, updateTags, updateNote };
}
