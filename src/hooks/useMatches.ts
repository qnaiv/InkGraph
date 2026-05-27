// IkaVision XP — 試合データ管理フック

import { useState, useEffect, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import type { Match, MatchDetectedPayload, RawMatch, Rule } from '../types';

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

interface UseMatchesReturn {
  matches: Match[];
  isLoading: boolean;
  error: string | null;
  prependMatch: (m: Match) => void;
  updateWeapon: (id: string, weapon: string) => Promise<void>;
  updateTags: (id: string, tags: string[]) => Promise<void>;
  updateNote: (id: string, note: string) => Promise<void>;
}

/** 試合データを管理するカスタムフック */
export function useMatches(ruleFilter?: Rule | null): UseMatchesReturn {
  const [matches, setMatches] = useState<Match[]>([]);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // DB から試合一覧をロード
  const loadMatches = useCallback(async () => {
    setIsLoading(true);
    setError(null);
    try {
      // tauri-plugin-sql は JS 側で直接 SQL を実行する
      // ここではシンプルに invoke でラッパーコマンドを呼ぶ
      // 実装フェーズで tauri-plugin-sql の Database クラスに移行する
      const result = await invoke<RawMatch[]>('get_matches', {
        limit: 100,
        rule: ruleFilter ?? undefined,
      }).catch(() => [] as RawMatch[]);
      setMatches(result.map(parseMatch));
    } catch (e) {
      setError(String(e));
    } finally {
      setIsLoading(false);
    }
  }, [ruleFilter]);

  useEffect(() => {
    loadMatches();
  }, [loadMatches]);

  // Rust からのリアルタイム通知を購読
  useEffect(() => {
    let unlisten: UnlistenFn | null = null;

    listen<MatchDetectedPayload>('match_detected', (event) => {
      const newMatch = parseMatch(event.payload.match_data);
      setMatches((prev) => [newMatch, ...prev]);
    }).then((fn) => {
      unlisten = fn;
    });

    return () => {
      unlisten?.();
    };
  }, []);

  const prependMatch = useCallback((m: Match) => {
    setMatches((prev) => [m, ...prev]);
  }, []);

  const updateWeapon = useCallback(async (id: string, weapon: string) => {
    await invoke('update_weapon', { id, weapon });
    setMatches((prev) =>
      prev.map((m) => (m.id === id ? { ...m, weapon } : m))
    );
  }, []);

  const updateTags = useCallback(async (id: string, tags: string[]) => {
    await invoke('update_tags', { id, tags });
    setMatches((prev) =>
      prev.map((m) => (m.id === id ? { ...m, tags } : m))
    );
  }, []);

  const updateNote = useCallback(async (id: string, note: string) => {
    await invoke('update_note', { id, note });
    setMatches((prev) =>
      prev.map((m) => (m.id === id ? { ...m, note } : m))
    );
  }, []);

  return {
    matches,
    isLoading,
    error,
    prependMatch,
    updateWeapon,
    updateTags,
    updateNote,
  };
}
