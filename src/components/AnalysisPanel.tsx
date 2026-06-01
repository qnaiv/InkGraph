// InkGraph — 分析パネル

import { useMemo } from 'react';
import type { Match } from '../types';
import { RULES } from '../types';

interface Props {
  matches: Match[];
}

interface RuleStat {
  rule: string;
  total: number;
  wins: number;
  winRate: number;
}

interface WeaponStat {
  weapon: string;
  total: number;
  wins: number;
  winRate: number;
  avgKill: number | null;
  avgDeath: number | null;
}

function useStats(matches: Match[]) {
  return useMemo(() => {
    const finished = matches.filter((m) => m.result !== 'in_progress');
    const wins = finished.filter((m) => m.result === 'win');

    // 平均K/D
    const killMs = finished.filter((m) => m.kill_count != null);
    const deathMs = finished.filter((m) => m.death_count != null);
    const avgKill =
      killMs.length > 0
        ? killMs.reduce((s, m) => s + m.kill_count!, 0) / killMs.length
        : null;
    const avgDeath =
      deathMs.length > 0
        ? deathMs.reduce((s, m) => s + m.death_count!, 0) / deathMs.length
        : null;

    // 連勝/連敗
    let streak: { type: 'win' | 'lose'; count: number } | null = null;
    if (finished.length > 0) {
      const first = finished[0].result as 'win' | 'lose';
      let count = 0;
      for (const m of finished) {
        if (m.result === first) count++;
        else break;
      }
      streak = { type: first, count };
    }

    // ルール別
    const ruleStats: RuleStat[] = RULES.map((rule) => {
      const rs = finished.filter((m) => m.rule === rule);
      const rw = rs.filter((m) => m.result === 'win').length;
      return {
        rule,
        total: rs.length,
        wins: rw,
        winRate: rs.length > 0 ? (rw / rs.length) * 100 : 0,
      };
    }).filter((s) => s.total > 0);

    // ブキ別 (上位8件)
    const weaponMap = new Map<string, Match[]>();
    for (const m of finished) {
      if (!m.weapon) continue;
      if (!weaponMap.has(m.weapon)) weaponMap.set(m.weapon, []);
      weaponMap.get(m.weapon)!.push(m);
    }
    const weaponStats: WeaponStat[] = Array.from(weaponMap.entries())
      .map(([weapon, ms]) => {
        const ww = ms.filter((m) => m.result === 'win').length;
        const wkm = ms.filter((m) => m.kill_count != null);
        const wdm = ms.filter((m) => m.death_count != null);
        return {
          weapon,
          total: ms.length,
          wins: ww,
          winRate: (ww / ms.length) * 100,
          avgKill:
            wkm.length > 0
              ? wkm.reduce((s, m) => s + m.kill_count!, 0) / wkm.length
              : null,
          avgDeath:
            wdm.length > 0
              ? wdm.reduce((s, m) => s + m.death_count!, 0) / wdm.length
              : null,
        };
      })
      .sort((a, b) => b.total - a.total)
      .slice(0, 8);

    return { finished, wins, avgKill, avgDeath, streak, ruleStats, weaponStats };
  }, [matches]);
}

export function AnalysisPanel({ matches }: Props) {
  const { finished, wins, avgKill, avgDeath, streak, ruleStats, weaponStats } =
    useStats(matches);

  if (finished.length === 0) {
    return (
      <div className="flex items-center justify-center h-full text-slate-500">
        <div className="text-center">
          <div className="text-4xl mb-2">📊</div>
          <p>まだ試合データがありません</p>
        </div>
      </div>
    );
  }

  const winRate = (wins.length / finished.length) * 100;

  return (
    <div className="overflow-y-auto h-full space-y-5 pr-1">
      {/* 総合サマリー */}
      <section>
        <h3 className="text-xs text-slate-400 font-medium mb-2 uppercase tracking-wide">
          総合
        </h3>
        <div className="grid grid-cols-2 gap-2 sm:grid-cols-4">
          <StatCard
            label="試合数"
            value={`${finished.length}`}
            sub="試合"
          />
          <StatCard
            label="勝率"
            value={`${winRate.toFixed(1)}%`}
            sub={`${wins.length}勝 ${finished.length - wins.length}敗`}
            highlight={winRate >= 50 ? 'green' : 'red'}
          />
          {avgKill != null && (
            <StatCard label="平均キル" value={avgKill.toFixed(1)} sub="kills" />
          )}
          {avgDeath != null && (
            <StatCard label="平均デス" value={avgDeath.toFixed(1)} sub="deaths" />
          )}
          {streak && streak.count >= 2 && (
            <div
              className={`bg-slate-700/50 rounded-lg px-3 py-2 text-center ${
                streak.type === 'win'
                  ? 'ring-1 ring-green-600/50'
                  : 'ring-1 ring-red-600/50'
              }`}
            >
              <p className="text-xs text-slate-400">現在の連続</p>
              <p
                className={`text-xl font-bold ${
                  streak.type === 'win' ? 'text-green-400' : 'text-red-400'
                }`}
              >
                {streak.count}
              </p>
              <p className="text-xs text-slate-500">
                {streak.type === 'win' ? '連勝' : '連敗'}
              </p>
            </div>
          )}
        </div>
      </section>

      {/* ルール別勝率 */}
      {ruleStats.length > 0 && (
        <section>
          <h3 className="text-xs text-slate-400 font-medium mb-2 uppercase tracking-wide">
            ルール別勝率
          </h3>
          <div className="space-y-2">
            {ruleStats.map((s) => (
              <div key={s.rule} className="bg-slate-700/30 rounded-lg p-3">
                <div className="flex items-center justify-between mb-1.5">
                  <span className="text-sm text-white font-medium">{s.rule}</span>
                  <div className="flex items-center gap-3 text-xs text-slate-400">
                    <span>{s.total}試合</span>
                    <span
                      className={`font-bold ${
                        s.winRate >= 50 ? 'text-green-400' : 'text-red-400'
                      }`}
                    >
                      {s.winRate.toFixed(1)}%
                    </span>
                  </div>
                </div>
                <div className="w-full bg-slate-700 rounded-full h-1.5">
                  <div
                    className={`h-1.5 rounded-full ${
                      s.winRate >= 50 ? 'bg-green-500' : 'bg-red-500'
                    }`}
                    style={{ width: `${s.winRate}%` }}
                  />
                </div>
                <div className="flex justify-between text-xs text-slate-600 mt-0.5">
                  <span>{s.wins}勝</span>
                  <span>{s.total - s.wins}敗</span>
                </div>
              </div>
            ))}
          </div>
        </section>
      )}

      {/* ブキ別成績 */}
      {weaponStats.length > 0 && (
        <section>
          <h3 className="text-xs text-slate-400 font-medium mb-2 uppercase tracking-wide">
            ブキ別成績
          </h3>
          <div className="space-y-1.5">
            {weaponStats.map((s) => (
              <div
                key={s.weapon}
                className="bg-slate-700/30 rounded-lg px-3 py-2 flex items-center gap-3"
              >
                <div className="flex-1 min-w-0">
                  <span className="text-sm text-white truncate block">{s.weapon}</span>
                  <span className="text-xs text-slate-500">{s.total}試合</span>
                </div>
                {/* 勝率バー */}
                <div className="w-16">
                  <div className="w-full bg-slate-700 rounded-full h-1 mb-0.5">
                    <div
                      className={`h-1 rounded-full ${
                        s.winRate >= 50 ? 'bg-green-500' : 'bg-red-500'
                      }`}
                      style={{ width: `${s.winRate}%` }}
                    />
                  </div>
                </div>
                <div className="text-right w-20 shrink-0">
                  <p
                    className={`text-sm font-bold ${
                      s.winRate >= 50 ? 'text-green-400' : 'text-red-400'
                    }`}
                  >
                    {s.winRate.toFixed(0)}%
                  </p>
                  {s.avgKill != null && s.avgDeath != null && (
                    <p className="text-xs text-slate-500">
                      {s.avgKill.toFixed(1)}k/{s.avgDeath.toFixed(1)}d
                    </p>
                  )}
                </div>
              </div>
            ))}
          </div>
        </section>
      )}
    </div>
  );
}

function StatCard({
  label,
  value,
  sub,
  highlight,
}: {
  label: string;
  value: string;
  sub: string;
  highlight?: 'green' | 'red';
}) {
  return (
    <div className="bg-slate-700/50 rounded-lg px-3 py-2 text-center">
      <p className="text-xs text-slate-400">{label}</p>
      <p
        className={`text-xl font-bold ${
          highlight === 'green'
            ? 'text-green-400'
            : highlight === 'red'
              ? 'text-red-400'
              : 'text-white'
        }`}
      >
        {value}
      </p>
      <p className="text-xs text-slate-500">{sub}</p>
    </div>
  );
}
