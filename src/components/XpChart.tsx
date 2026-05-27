// IkaVision XP — XP 推移グラフ

import {
  LineChart,
  Line,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  ResponsiveContainer,
  ReferenceLine,
} from 'recharts';
import { Match, RULES, Rule } from '../types';

interface XpChartProps {
  matches: Match[];
  selectedRule: Rule | null;
  onRuleChange: (rule: Rule | null) => void;
}

interface ChartDataPoint {
  label: string;
  xp: number;
  result: 'win' | 'lose';
  date: string;
}

function buildChartData(matches: Match[]): ChartDataPoint[] {
  return matches
    .filter((m) => m.xp_after != null)
    .sort((a, b) => a.played_at.localeCompare(b.played_at))
    .map((m, i) => ({
      label: `#${i + 1}`,
      xp: m.xp_after!,
      result: m.result,
      date: new Date(m.played_at).toLocaleString('ja-JP', {
        month: 'numeric',
        day: 'numeric',
        hour: '2-digit',
        minute: '2-digit',
      }),
    }));
}

const RESULT_COLORS = {
  win: '#4ade80',   // green-400
  lose: '#f87171',  // red-400
};

function CustomDot(props: any) {
  const { cx, cy, payload } = props;
  const color = RESULT_COLORS[payload.result as 'win' | 'lose'];
  return (
    <circle
      cx={cx}
      cy={cy}
      r={5}
      fill={color}
      stroke="#1e293b"
      strokeWidth={1.5}
    />
  );
}

function CustomTooltip({ active, payload, label }: any) {
  if (!active || !payload?.length) return null;
  const d = payload[0].payload as ChartDataPoint;
  return (
    <div className="bg-slate-800 border border-slate-600 rounded-lg px-3 py-2 text-sm shadow-xl">
      <p className="text-slate-300">{d.date}</p>
      <p className="font-bold text-white">XP: {d.xp.toFixed(1)}</p>
      <p className={d.result === 'win' ? 'text-green-400' : 'text-red-400'}>
        {d.result === 'win' ? '✅ WIN' : '❌ LOSE'}
      </p>
    </div>
  );
}

export function XpChart({ matches, selectedRule, onRuleChange }: XpChartProps) {
  const data = buildChartData(matches);

  return (
    <div className="flex flex-col h-full gap-4">
      {/* ルールフィルタ */}
      <div className="flex gap-2 flex-wrap">
        <button
          className={`px-3 py-1 rounded-full text-sm font-medium transition-colors ${
            selectedRule === null
              ? 'bg-indigo-600 text-white'
              : 'bg-slate-700 text-slate-300 hover:bg-slate-600'
          }`}
          onClick={() => onRuleChange(null)}
        >
          全ルール
        </button>
        {RULES.map((rule) => (
          <button
            key={rule}
            className={`px-3 py-1 rounded-full text-sm font-medium transition-colors ${
              selectedRule === rule
                ? 'bg-indigo-600 text-white'
                : 'bg-slate-700 text-slate-300 hover:bg-slate-600'
            }`}
            onClick={() => onRuleChange(rule)}
          >
            {rule}
          </button>
        ))}
      </div>

      {/* グラフ */}
      {data.length === 0 ? (
        <div className="flex-1 flex items-center justify-center text-slate-500">
          <div className="text-center">
            <div className="text-4xl mb-2">🦑</div>
            <p>まだ試合データがありません</p>
            <p className="text-sm mt-1">キャプチャを開始して対戦を記録しましょう</p>
          </div>
        </div>
      ) : (
        <div className="flex-1">
          <ResponsiveContainer width="100%" height="100%">
            <LineChart data={data} margin={{ top: 8, right: 16, bottom: 8, left: 0 }}>
              <CartesianGrid strokeDasharray="3 3" stroke="#334155" />
              <XAxis
                dataKey="label"
                tick={{ fill: '#94a3b8', fontSize: 11 }}
                axisLine={{ stroke: '#475569' }}
                tickLine={false}
              />
              <YAxis
                tick={{ fill: '#94a3b8', fontSize: 11 }}
                axisLine={{ stroke: '#475569' }}
                tickLine={false}
                domain={['auto', 'auto']}
                tickFormatter={(v) => v.toFixed(0)}
              />
              <Tooltip content={<CustomTooltip />} />
              <Line
                type="monotone"
                dataKey="xp"
                stroke="#818cf8"
                strokeWidth={2}
                dot={<CustomDot />}
                activeDot={{ r: 7, fill: '#818cf8' }}
              />
            </LineChart>
          </ResponsiveContainer>
        </div>
      )}

      {/* 統計サマリー */}
      {data.length > 0 && (
        <div className="grid grid-cols-3 gap-3 text-center">
          <StatCard
            label="試合数"
            value={`${data.length}`}
            sub="試合"
          />
          <StatCard
            label="勝率"
            value={`${((matches.filter(m => m.result === 'win').length / matches.length) * 100).toFixed(1)}%`}
            sub={`${matches.filter(m => m.result === 'win').length}勝 ${matches.filter(m => m.result === 'lose').length}敗`}
          />
          <StatCard
            label="最高XP"
            value={`${Math.max(...matches.filter(m => m.xp_after != null).map(m => m.xp_after!)).toFixed(1)}`}
            sub="XP"
          />
        </div>
      )}
    </div>
  );
}

function StatCard({ label, value, sub }: { label: string; value: string; sub: string }) {
  return (
    <div className="bg-slate-700/50 rounded-lg px-3 py-2">
      <p className="text-xs text-slate-400">{label}</p>
      <p className="text-xl font-bold text-white">{value}</p>
      <p className="text-xs text-slate-500">{sub}</p>
    </div>
  );
}
