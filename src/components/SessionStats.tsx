// InkGraph — 今日のセッション統計

import type { Match } from '../types';

interface SessionStatsProps {
  matches: Match[];
}

// ---------------------------------------------------------------------------
// 計算ヘルパー
// ---------------------------------------------------------------------------

function isTodayMatch(m: Match): boolean {
  const today = new Date();
  today.setHours(0, 0, 0, 0);
  const d = new Date(m.played_at);
  d.setHours(0, 0, 0, 0);
  return d.getTime() === today.getTime();
}

function calcStreak(matches: Match[]): { kind: 'win' | 'lose'; count: number } | null {
  const finished = matches.filter(m => m.result === 'win' || m.result === 'lose');
  if (!finished.length) return null;
  const first = finished[0].result as 'win' | 'lose';
  let count = 0;
  for (const m of finished) {
    if (m.result === first) count++;
    else break;
  }
  return count >= 2 ? { kind: first, count } : null;
}

function calcXpDelta(todayFinished: Match[]): number | null {
  const withXp = todayFinished.filter(m => m.xp_after != null);
  if (withXp.length < 2) return null;
  // matches は played_at DESC 順なので先頭が最新、末尾が最古
  const latest   = withXp[0].xp_after!;
  const earliest = withXp[withXp.length - 1].xp_after!;
  return latest - earliest;
}

// ---------------------------------------------------------------------------
// コンポーネント
// ---------------------------------------------------------------------------

export function SessionStats({ matches: allMatches }: SessionStatsProps) {
  const today       = allMatches.filter(isTodayMatch);
  const inProgress  = today.some(m => m.result === 'in_progress');
  const finished    = today.filter(m => m.result === 'win' || m.result === 'lose');

  if (today.length === 0) return null;

  const wins   = finished.filter(m => m.result === 'win').length;
  const losses = finished.filter(m => m.result === 'lose').length;
  const streak = calcStreak(finished);
  const xpDelta = calcXpDelta(finished);

  return (
    <div className="mx-3 mb-2 bg-slate-700/60 rounded-lg px-3 py-2 text-xs flex flex-wrap items-center gap-x-3 gap-y-1">
      {/* 日付ラベル */}
      <span className="text-slate-400 font-semibold shrink-0">今日</span>

      {/* 勝敗 */}
      <span className="shrink-0">
        <span className="text-green-400 font-bold">{wins}勝</span>
        <span className="text-slate-500 mx-0.5">/</span>
        <span className="text-red-400 font-bold">{losses}敗</span>
      </span>

      {/* 試合中バッジ */}
      {inProgress && (
        <span className="text-amber-400 animate-pulse shrink-0">試合中…</span>
      )}

      {/* 連勝/連敗ストリーク */}
      {streak && (
        <span className={`shrink-0 font-bold ${streak.kind === 'win' ? 'text-green-300' : 'text-red-300'}`}>
          {streak.kind === 'win' ? '🔥' : '💦'}{streak.count}連{streak.kind === 'win' ? '勝' : '敗'}
        </span>
      )}

      {/* XP 増減 */}
      {xpDelta != null && (
        <span className={`shrink-0 font-mono font-bold ${xpDelta >= 0 ? 'text-indigo-300' : 'text-slate-400'}`}>
          {xpDelta >= 0 ? '+' : ''}{xpDelta.toFixed(1)} XP
        </span>
      )}
    </div>
  );
}
