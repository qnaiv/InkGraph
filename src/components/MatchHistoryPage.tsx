// InkGraph — 全試合一覧ページ

import { useState, useEffect, useCallback } from 'react';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { selectAllMatches, dbDeleteMatch } from '../lib/db';
import { getLastWeaponName } from '../assets/weapons';
import { RULES } from '../types';
import type { Match, MatchDetectedPayload, RawMatch } from '../types';

function parseMatch(raw: RawMatch): Match {
  let tags: string[] = [];
  if (raw.tags) {
    try { tags = JSON.parse(raw.tags); } catch { tags = []; }
  }
  return { ...raw, tags, auto_recorded: Boolean(raw.auto_recorded) };
}

type ResultFilter = 'all' | 'win' | 'lose';

interface Props {
  onEdit: (match: Match) => void;
  /** 「+ 手動入力」ボタン押下時に呼ばれる */
  onAddNew: () => void;
  /** 更新時にインクリメントされると一覧を再取得する */
  refreshKey: number;
}

export function MatchHistoryPage({ onEdit, onAddNew, refreshKey }: Props) {
  const [allMatches, setAllMatches] = useState<Match[]>([]);
  const [isLoading, setIsLoading] = useState(false);
  const [resultFilter, setResultFilter] = useState<ResultFilter>('all');
  const [ruleFilter, setRuleFilter] = useState<string>('all');

  const load = useCallback(async () => {
    setIsLoading(true);
    try {
      const rows = await selectAllMatches();
      setAllMatches(rows.map(parseMatch));
    } finally {
      setIsLoading(false);
    }
  }, []);

  // eslint-disable-next-line react-hooks/set-state-in-effect
  useEffect(() => { load(); }, [load, refreshKey]);

  // ── Rust からのリアルタイム通知を購読 (DB への書き込みは useMatches 側で行う) ──
  useEffect(() => {
    let cancelled = false;
    const cleanupFns: UnlistenFn[] = [];

    async function setup() {
      // バトル開始 → "in_progress" 行を一覧の先頭に追加
      const fn1 = await listen<MatchDetectedPayload>('battle_started', (event) => {
        // ブキはキャプチャでは認識できないため、未入力なら前回使用したブキを補完する (useMatches 側と表示を揃える)
        const raw: RawMatch = { ...event.payload.match_data, weapon: event.payload.match_data.weapon ?? getLastWeaponName() };
        setAllMatches((prev) => [parseMatch(raw), ...prev]);
      });
      if (cancelled) { fn1(); return; }
      cleanupFns.push(fn1);

      // リザルト確定 → 既存の "in_progress" 行を更新 (無ければ先頭に追加)
      const fn2 = await listen<MatchDetectedPayload>('match_detected', (event) => {
        const raw = event.payload.match_data;
        setAllMatches((prev) => {
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

  const handleDelete = useCallback(async (id: string) => {
    await dbDeleteMatch(id);
    setAllMatches((prev) => prev.filter((m) => m.id !== id));
  }, []);

  const filtered = allMatches.filter((m) => {
    if (resultFilter !== 'all' && m.result !== resultFilter) return false;
    if (ruleFilter !== 'all' && m.rule !== ruleFilter) return false;
    return true;
  });

  const wins = filtered.filter((m) => m.result === 'win').length;
  const losses = filtered.filter((m) => m.result === 'lose').length;
  const winRate = wins + losses > 0 ? Math.round((wins / (wins + losses)) * 100) : null;

  return (
    <div className="flex flex-col h-full overflow-hidden">
      {/* フィルターバー */}
      <div className="flex flex-wrap items-center gap-3 pb-3 border-b border-slate-700 shrink-0">
        {/* 勝敗フィルター */}
        <div className="flex gap-1">
          {(['all', 'win', 'lose'] as const).map((v) => (
            <button
              key={v}
              onClick={() => setResultFilter(v)}
              className={`px-3 py-1 rounded-full text-xs font-medium transition-colors ${
                resultFilter === v
                  ? v === 'win' ? 'bg-green-600 text-white'
                  : v === 'lose' ? 'bg-red-600 text-white'
                  : 'bg-indigo-600 text-white'
                  : 'bg-slate-700 text-slate-400 hover:bg-slate-600'
              }`}
            >
              {v === 'all' ? 'すべて' : v === 'win' ? 'WIN' : 'LOSE'}
            </button>
          ))}
        </div>

        {/* ルールフィルター */}
        <div className="flex gap-1 flex-wrap">
          <button
            onClick={() => setRuleFilter('all')}
            className={`px-3 py-1 rounded-full text-xs font-medium transition-colors ${
              ruleFilter === 'all'
                ? 'bg-indigo-600 text-white'
                : 'bg-slate-700 text-slate-400 hover:bg-slate-600'
            }`}
          >
            全ルール
          </button>
          {RULES.map((r) => (
            <button
              key={r}
              onClick={() => setRuleFilter(r)}
              className={`px-3 py-1 rounded-full text-xs font-medium transition-colors ${
                ruleFilter === r
                  ? 'bg-indigo-600 text-white'
                  : 'bg-slate-700 text-slate-400 hover:bg-slate-600'
              }`}
            >
              {r}
            </button>
          ))}
        </div>

        {/* 集計 */}
        <div className="ml-auto text-xs text-slate-500 shrink-0">
          {filtered.length}件
          {winRate != null && (
            <span className="ml-2">
              WIN {wins} / LOSE {losses}
              <span className={`ml-1.5 font-semibold ${winRate >= 50 ? 'text-green-400' : 'text-red-400'}`}>
                ({winRate}%)
              </span>
            </span>
          )}
        </div>

        {/* 手動入力 */}
        <button
          className="px-3 py-1 bg-indigo-600 hover:bg-indigo-500 text-white text-xs font-medium rounded-lg transition-colors shrink-0"
          onClick={onAddNew}
        >
          + 手動入力
        </button>
      </div>

      {/* テーブルヘッダー */}
      <div className="grid grid-cols-[76px_88px_104px_76px_1fr_1fr_72px_56px_64px_56px_32px_32px] gap-x-2 px-3 py-1.5 text-[10px] text-slate-500 font-medium uppercase tracking-wide border-b border-slate-700 shrink-0">
        <span>勝敗</span>
        <span>日時</span>
        <span>モード</span>
        <span>ルール</span>
        <span>ステージ</span>
        <span>ブキ</span>
        <span className="text-center">K/D/S</span>
        <span className="text-right">ぬり</span>
        <span className="text-right">XP</span>
        <span className="text-right">金イクラ</span>
        <span></span>
        <span></span>
      </div>

      {/* 一覧 */}
      {isLoading ? (
        <div className="flex-1 flex items-center justify-center text-slate-500 text-sm">
          読み込み中...
        </div>
      ) : filtered.length === 0 ? (
        <div className="flex-1 flex flex-col items-center justify-center text-slate-500 text-sm gap-2">
          <span className="text-3xl">🎮</span>
          <span>該当する試合がありません</span>
        </div>
      ) : (
        <div className="flex-1 overflow-y-auto">
          {filtered.map((match) => (
            <MatchHistoryRow key={match.id} match={match} onEdit={onEdit} onDelete={handleDelete} />
          ))}
        </div>
      )}
    </div>
  );
}

function MatchHistoryRow({ match, onEdit, onDelete }: { match: Match; onEdit: (m: Match) => void; onDelete: (id: string) => void }) {
  const [confirmDelete, setConfirmDelete] = useState(false);
  const dateStr = new Date(match.played_at).toLocaleString('ja-JP', {
    month: 'numeric',
    day: 'numeric',
    hour: '2-digit',
    minute: '2-digit',
  });

  const kda = [
    match.kill_count    != null ? match.kill_count    : null,
    match.death_count   != null ? match.death_count   : null,
    match.special_count != null ? match.special_count : null,
  ].map((v) => (v != null ? String(v) : '-')).join('/');

  // キャプチャによる自動認識のまま、まだ編集ダイアログで確定保存されていないレコード
  const isUnconfirmed = match.auto_recorded && match.result !== 'in_progress';
  const hasExtra = match.tags.length > 0 || !!match.note;

  return (
    <div
      className={`border-b border-slate-700/40 hover:bg-slate-700/20 transition-colors ${
        match.result === 'in_progress' ? 'opacity-60' : isUnconfirmed ? 'opacity-70' : ''
      }`}
    >
    <div className="grid grid-cols-[76px_88px_104px_76px_1fr_1fr_72px_56px_64px_56px_32px_32px] gap-x-2 px-3 py-2 text-xs items-center">

      {/* 勝敗 */}
      {match.result === 'in_progress' ? (
        <span className="inline-flex justify-center">
          <span className="text-[10px] font-bold px-2 py-0.5 rounded bg-amber-500/20 text-amber-400 border border-amber-500/40 animate-pulse">
            試合中
          </span>
        </span>
      ) : (
        <span className="inline-flex flex-col items-center gap-0.5">
          <span className={`text-[10px] font-bold px-2 py-0.5 rounded ${
            match.result === 'win'
              ? 'bg-green-500/20 text-green-400 border border-green-500/40'
              : 'bg-red-500/20 text-red-400 border border-red-500/40'
          }`}>
            {match.result === 'win' ? 'WIN' : 'LOSE'}
          </span>
          {isUnconfirmed && (
            <span
              className="text-[8px] font-semibold px-1 leading-tight rounded bg-amber-500/20 text-amber-400 border border-amber-500/40"
              title="キャプチャによる自動認識のため、編集して保存するまでは未確定です"
            >
              未確定
            </span>
          )}
        </span>
      )}

      {/* 日時 */}
      <span className="text-slate-500 text-[10px]">{dateStr}</span>

      {/* モード */}
      <span className="text-slate-400 truncate">{match.mode ?? '—'}</span>

      {/* ルール */}
      <span className="text-slate-300 truncate">{match.rule ?? '—'}</span>

      {/* ステージ */}
      <span className="text-slate-400 truncate">{match.stage ?? '—'}</span>

      {/* ブキ */}
      <span className="text-slate-400 truncate">{match.weapon ?? '—'}</span>

      {/* K/D/S */}
      <span className="text-slate-400 font-mono text-center text-[10px]">{kda}</span>

      {/* ぬり */}
      <span className="font-mono text-right text-[11px] text-slate-400">
        {match.paint_count != null ? match.paint_count : '—'}
      </span>

      {/* XP */}
      <span className={`font-mono text-right text-[11px] ${match.xp_after != null ? 'text-indigo-300' : 'text-slate-600'}`}>
        {match.xp_after != null ? match.xp_after.toFixed(1) : '—'}
      </span>

      {/* 金イクラ */}
      <span className="font-mono text-right text-[11px] text-amber-300/80">
        {match.gold_award_count != null ? match.gold_award_count : '—'}
      </span>

      {/* 編集 / 削除ボタン (削除確認中は確認UIに置き換え) */}
      {confirmDelete ? (
        <div className="col-span-2 flex gap-1 items-center justify-end">
          <button
            className="px-1.5 py-0.5 text-[10px] text-white bg-red-600 hover:bg-red-500 rounded"
            onClick={() => { setConfirmDelete(false); onDelete(match.id); }}
          >
            削除
          </button>
          <button
            className="px-1.5 py-0.5 text-[10px] text-slate-400 hover:text-white rounded"
            onClick={() => setConfirmDelete(false)}
          >
            ✕
          </button>
        </div>
      ) : (
        <>
          <button
            onClick={() => onEdit(match)}
            className="text-slate-600 hover:text-indigo-400 transition-colors text-center"
            title="編集"
            disabled={match.result === 'in_progress'}
          >
            ✏️
          </button>
          <button
            onClick={() => setConfirmDelete(true)}
            className="text-slate-700 hover:text-red-400 transition-colors text-center"
            title="削除"
          >
            🗑
          </button>
        </>
      )}
    </div>

    {/* タグ・メモ (存在する場合のみ2行目に表示) */}
    {hasExtra && (
      <div className="flex flex-wrap items-center gap-1.5 px-3 pb-2 -mt-1 text-[11px]">
        {match.tags.map((tag) => (
          <span
            key={tag}
            className="px-1.5 py-0.5 rounded-full bg-amber-500/10 border border-amber-500/30 text-amber-400 text-[10px]"
          >
            {tag}
          </span>
        ))}
        {match.note && (
          <span className="text-slate-500 truncate">{match.note}</span>
        )}
      </div>
    )}
    </div>
  );
}
