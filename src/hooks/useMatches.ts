// IkaVision XP — 試合データ管理フック

import { useState, useEffect, useCallback } from 'react';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import type { Match, MatchDetectedPayload, RawMatch, Rule } from '../types';
import {
  selectMatches,
  insertMatch,
  dbUpdateWeapon,
  dbUpdateTags,
  dbUpdateNote,
} from '../lib/db';

// ---------------------------------------------------------------------------
// 内部ユーティリティ
// ---------------------------------------------------------------------------

/** DB から取得した生データをフロントエンド型に変換 */
function parseMatch(raw: RawMatch): Match {
  let tags: string[] = [];
  if (raw.tags) {
    try {
      tags = JSON.parse(raw.tags);
    } catch {
      tags = [];
    }
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
  updateWeapon: (id: string, weapon: string) => Promise<void>;
  updateTags:   (id: string, tags: string[]) => Promise<void>;
  updateNote:   (id: string, note: string)   => Promise<void>;
}

/** 試合データを管理するカスタムフック */
export function useMatches(ruleFilter?: Rule | null): UseMatchesReturn {
  const [matches, setMatches] = useState<Match[]>([]);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // ── DB から試合一覧をロード ───────────────────────────────────
  const loadMatches = useCallback(async () => {
    setIsLoading(true);
    setError(null);
    try {
      const rows = await selectMatches(100, ruleFilter);
      setMatches(rows.map(parseMatch));
    } catch (e) {
      setError(String(e));
    } finally {
      setIsLoading(false);
    }
  }, [ruleFilter]);

  useEffect(() => {
    loadMatches();
  }, [loadMatches]);

  // ── Rust からのリアルタイム通知を購読 ───────────────────────
  useEffect(() => {
    let unlisten: UnlistenFn | null = null;

    listen<MatchDetectedPayload>('match_detected', async (event) => {
      const raw = event.payload.match_data;
      // DB に永続化してから UI に追加
      try {
        await insertMatch(raw);
      } catch (e) {
        console.error('[useMatches] insertMatch failed:', e);
      }
      setMatches((prev) => [parseMatch(raw), ...prev]);
    }).then((fn) => { unlisten = fn; });

    return () => { unlisten?.(); };
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

  return { matches, isLoading, error, updateWeapon, updateTags, updateNote };
}
