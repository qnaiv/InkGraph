// InkGraph — 手動試合入力モーダル

import { useState } from 'react';
import { RULES, PRESET_TAGS } from '../types';
import { WeaponPicker } from './WeaponPicker';
import type { RawMatch } from '../types';

const MODES = [
  'Xマッチ',
  'バンカラマッチ(チャレンジ)',
  'バンカラマッチ(オープン)',
  'ナワバリバトル',
];

const STAGES = [
  'ユノハナ大渓谷',
  'ゴンズイ地区',
  'ヤガラ市場',
  'マテガイ放水路',
  'ナメロウ金属',
  'クサヤ温泉',
  'ヒラメが丘団地',
  'マサバ海峡大橋',
  'チョウザメ造船',
  'ザトウマーケット',
  'スメーシーワールド',
  'キンメダイ美術館',
  'タラポートショッピングパーク',
  'マヒマヒリゾート&スパ',
  '海女美術大学',
  'オヒョウ海運',
  'バイガイ亭',
  '万国博覧会',
];

interface Props {
  onClose: () => void;
  onSubmit: (match: RawMatch) => Promise<void>;
}

export function ManualEntryModal({ onClose, onSubmit }: Props) {
  const nowStr = new Date().toISOString().slice(0, 16);

  const [playedAt, setPlayedAt] = useState(nowStr);
  const [result, setResult] = useState<'win' | 'lose'>('win');
  const [mode, setMode] = useState('Xマッチ');
  const [rule, setRule] = useState<string>(RULES[0]);
  const [stage, setStage] = useState('');
  const [weapon, setWeapon] = useState<string | null>(null);
  const [kill, setKill] = useState('');
  const [death, setDeath] = useState('');
  const [special, setSpecial] = useState('');
  const [xp, setXp] = useState('');
  const [tags, setTags] = useState<string[]>([]);
  const [note, setNote] = useState('');
  const [isSubmitting, setIsSubmitting] = useState(false);

  const toggleTag = (tag: string) =>
    setTags((prev) =>
      prev.includes(tag) ? prev.filter((t) => t !== tag) : [...prev, tag],
    );

  const handleSubmit = async () => {
    setIsSubmitting(true);
    const match: RawMatch = {
      id: crypto.randomUUID(),
      played_at: new Date(playedAt).toISOString(),
      result,
      mode: mode || null,
      rule: rule || null,
      stage: stage || null,
      weapon: weapon || null,
      kill_count: kill !== '' ? parseInt(kill, 10) : null,
      death_count: death !== '' ? parseInt(death, 10) : null,
      special_count: special !== '' ? parseInt(special, 10) : null,
      xp_after: xp !== '' ? parseFloat(xp) : null,
      tags: JSON.stringify(tags),
      note: note || null,
    };
    try {
      await onSubmit(match);
      onClose();
    } finally {
      setIsSubmitting(false);
    }
  };

  return (
    <div className="fixed inset-0 bg-black/60 z-50 flex items-center justify-center p-4" onClick={onClose}>
      <div
        className="bg-slate-800 rounded-xl border border-slate-600 w-full max-w-md max-h-[90vh] overflow-y-auto shadow-2xl"
        onClick={(e) => e.stopPropagation()}
      >
        {/* ヘッダー */}
        <div className="flex items-center justify-between px-5 py-4 border-b border-slate-700 sticky top-0 bg-slate-800 z-10">
          <h2 className="text-white font-semibold text-base">手動入力</h2>
          <button
            onClick={onClose}
            className="text-slate-400 hover:text-white text-xl leading-none w-7 h-7 flex items-center justify-center rounded hover:bg-slate-700 transition-colors"
          >
            ✕
          </button>
        </div>

        <div className="p-5 space-y-4">
          {/* 日時 */}
          <div>
            <label className="text-xs text-slate-400 block mb-1">日時</label>
            <input
              type="datetime-local"
              className="w-full bg-slate-700 text-white rounded-lg px-3 py-1.5 text-sm outline-none focus:ring-1 focus:ring-indigo-500"
              value={playedAt}
              onChange={(e) => setPlayedAt(e.target.value)}
            />
          </div>

          {/* 勝敗 */}
          <div>
            <label className="text-xs text-slate-400 block mb-1">勝敗</label>
            <div className="flex gap-2">
              <button
                className={`flex-1 py-2 rounded-lg text-sm font-bold transition-colors ${
                  result === 'win'
                    ? 'bg-green-600 text-white'
                    : 'bg-slate-700 text-slate-400 hover:bg-slate-600'
                }`}
                onClick={() => setResult('win')}
              >
                WIN
              </button>
              <button
                className={`flex-1 py-2 rounded-lg text-sm font-bold transition-colors ${
                  result === 'lose'
                    ? 'bg-red-600 text-white'
                    : 'bg-slate-700 text-slate-400 hover:bg-slate-600'
                }`}
                onClick={() => setResult('lose')}
              >
                LOSE
              </button>
            </div>
          </div>

          {/* モード */}
          <div>
            <label className="text-xs text-slate-400 block mb-1">モード</label>
            <select
              className="w-full bg-slate-700 text-white rounded-lg px-3 py-1.5 text-sm outline-none cursor-pointer"
              value={mode}
              onChange={(e) => setMode(e.target.value)}
            >
              {MODES.map((m) => (
                <option key={m} value={m}>{m}</option>
              ))}
            </select>
          </div>

          {/* ルール */}
          <div>
            <label className="text-xs text-slate-400 block mb-1">ルール</label>
            <div className="flex gap-1.5 flex-wrap">
              {RULES.map((r) => (
                <button
                  key={r}
                  className={`px-3 py-1 rounded-full text-xs font-medium transition-colors ${
                    rule === r
                      ? 'bg-indigo-600 text-white'
                      : 'bg-slate-700 text-slate-300 hover:bg-slate-600'
                  }`}
                  onClick={() => setRule(r)}
                >
                  {r}
                </button>
              ))}
            </div>
          </div>

          {/* ステージ */}
          <div>
            <label className="text-xs text-slate-400 block mb-1">ステージ</label>
            <select
              className="w-full bg-slate-700 text-white rounded-lg px-3 py-1.5 text-sm outline-none cursor-pointer"
              value={stage}
              onChange={(e) => setStage(e.target.value)}
            >
              <option value="">未選択</option>
              {STAGES.map((s) => (
                <option key={s} value={s}>{s}</option>
              ))}
            </select>
          </div>

          {/* ブキ */}
          <div>
            <label className="text-xs text-slate-400 block mb-1">ブキ</label>
            <WeaponPicker currentWeapon={weapon} onSelect={setWeapon} />
          </div>

          {/* K/D/S */}
          <div>
            <label className="text-xs text-slate-400 block mb-1">キル / デス / スペシャル</label>
            <div className="flex gap-2">
              {(
                [
                  ['キル', kill, setKill],
                  ['デス', death, setDeath],
                  ['スペシャル', special, setSpecial],
                ] as const
              ).map(([label, value, setter]) => (
                <div key={label} className="flex-1">
                  <p className="text-xs text-slate-500 text-center mb-1">{label}</p>
                  <input
                    type="number"
                    min="0"
                    className="w-full bg-slate-700 text-white text-center rounded-lg px-2 py-1.5 text-sm outline-none focus:ring-1 focus:ring-indigo-500"
                    value={value}
                    onChange={(e) => setter(e.target.value)}
                    placeholder="0"
                  />
                </div>
              ))}
            </div>
          </div>

          {/* XP */}
          <div>
            <label className="text-xs text-slate-400 block mb-1">XP（試合後）</label>
            <input
              type="number"
              className="w-full bg-slate-700 text-white rounded-lg px-3 py-1.5 text-sm outline-none focus:ring-1 focus:ring-indigo-500"
              value={xp}
              onChange={(e) => setXp(e.target.value)}
              placeholder="例: 2150.5"
              step="0.1"
            />
          </div>

          {/* 反省タグ */}
          <div>
            <label className="text-xs text-slate-400 block mb-1">反省タグ</label>
            <div className="flex flex-wrap gap-1">
              {PRESET_TAGS.map((tag) => {
                const active = tags.includes(tag);
                return (
                  <button
                    key={tag}
                    onClick={() => toggleTag(tag)}
                    className={`text-xs px-2 py-0.5 rounded-full border transition-all ${
                      active
                        ? 'bg-amber-500/20 border-amber-500 text-amber-300'
                        : 'bg-slate-700/50 border-slate-600 text-slate-400 hover:border-slate-500 hover:text-slate-300'
                    }`}
                  >
                    {active && <span className="mr-1">✓</span>}
                    {tag}
                  </button>
                );
              })}
            </div>
          </div>

          {/* メモ */}
          <div>
            <label className="text-xs text-slate-400 block mb-1">メモ</label>
            <textarea
              className="w-full bg-slate-700 text-white rounded-lg px-3 py-1.5 text-sm outline-none focus:ring-1 focus:ring-indigo-500 resize-none"
              value={note}
              onChange={(e) => setNote(e.target.value)}
              placeholder="気づいたことなど..."
              rows={2}
            />
          </div>
        </div>

        {/* フッター */}
        <div className="px-5 py-4 border-t border-slate-700 flex gap-3 sticky bottom-0 bg-slate-800">
          <button
            className="flex-1 py-2 bg-slate-700 hover:bg-slate-600 text-white rounded-lg text-sm transition-colors"
            onClick={onClose}
          >
            キャンセル
          </button>
          <button
            className="flex-1 py-2 bg-indigo-600 hover:bg-indigo-500 text-white rounded-lg text-sm font-medium transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
            onClick={handleSubmit}
            disabled={isSubmitting}
          >
            {isSubmitting ? '保存中...' : '保存'}
          </button>
        </div>
      </div>
    </div>
  );
}
