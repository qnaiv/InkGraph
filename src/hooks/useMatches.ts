// InkGraph — 試合データ管理フック

import { useState, useEffect, useCallback } from 'react';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { getLastWeaponName } from '../assets/weapons';
import type { Match, MatchDetectedPayload, RawMatch, Rule } from '../types';
import {
  selectMatches,
  insertMatch,
  updateMatchResult,
  dbUpdateFullMatch,
  dbDeleteMatch,
} from '../lib/db';

// ---------------------------------------------------------------------------
// 内部ユーティリティ
// ---------------------------------------------------------------------------

function parseMatch(raw: RawMatch): Match {
  let tags: string[] = [];
  if (raw.tags) {
    try { tags = JSON.parse(raw.tags); } catch { tags = []; }
  }
  return { ...raw, tags, auto_recorded: Boolean(raw.auto_recorded) };
}

// ---------------------------------------------------------------------------
// フック本体
// ---------------------------------------------------------------------------

interface UseMatchesReturn {
  matches: Match[];
  isLoading: boolean;
  error: string | null;
  addMatch:     (raw: RawMatch) => Promise<void>;
  updateMatch:  (raw: RawMatch) => Promise<void>;
  deleteMatch:  (id: string) => Promise<void>;
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
    let cancelled = false;
    const cleanupFns: UnlistenFn[] = [];

    async function setup() {
      // Phase 1: バトル開始 → "in_progress" レコードを追加
      const fn1 = await listen<MatchDetectedPayload>('battle_started', async (event) => {
        // ブキはキャプチャでは認識できないため、未入力なら前回使用したブキを補完する
        const raw: RawMatch = { ...event.payload.match_data, weapon: event.payload.match_data.weapon ?? getLastWeaponName() };
        try {
          await insertMatch(raw, true);
        } catch (e) {
          console.error('[useMatches] insertMatch(in_progress) failed:', e);
        }
        setMatches((prev) => [parseMatch(raw), ...prev]);
      });
      // StrictMode で cleanup が先に走った場合は即解除
      if (cancelled) { fn1(); return; }
      cleanupFns.push(fn1);

      // Phase 2: リザルト確定 → 既存の "in_progress" レコードを win/lose に更新
      const fn2 = await listen<MatchDetectedPayload>('match_detected', async (event) => {
        const raw = event.payload.match_data;
        try {
          await updateMatchResult(raw);
        } catch (e) {
          console.error('[useMatches] updateMatchResult failed:', e);
        }
        setMatches((prev) => {
          const exists = prev.some((m) => m.id === raw.id);
          if (exists) {
            return prev.map((m) => (m.id === raw.id ? parseMatch(raw) : m));
          }
          return [parseMatch(raw), ...prev];
        });
      });
      if (cancelled) { fn2(); return; }
      cleanupFns.push(fn2);
    }

    setup();

    return () => {
      cancelled = true;
      cleanupFns.forEach((fn) => fn());
    };
  }, []);

  // ── 手動追加 ─────────────────────────────────────────────────
  const addMatch = useCallback(async (raw: RawMatch) => {
    await insertMatch(raw, false);
    setMatches((prev) => [parseMatch(raw), ...prev]);
  }, []);

  // ── 全フィールド更新 ──────────────────────────────────────────
  const updateMatch = useCallback(async (raw: RawMatch) => {
    await dbUpdateFullMatch(raw);
    setMatches((prev) => prev.map((m) => (m.id === raw.id ? parseMatch(raw) : m)));
  }, []);

  // ── 削除操作 ─────────────────────────────────────────────────
  const deleteMatch = useCallback(async (id: string) => {
    await dbDeleteMatch(id);
    setMatches((prev) => prev.filter((m) => m.id !== id));
  }, []);

  return { matches, isLoading, error, addMatch, updateMatch, deleteMatch };
}
