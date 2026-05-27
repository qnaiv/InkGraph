// IkaVision XP — 試合カード

import { useState } from 'react';
import { Match } from '../types';
import { WeaponPicker } from './WeaponPicker';
import { TagInput } from './TagInput';

interface MatchCardProps {
  match: Match;
  onUpdateWeapon: (id: string, weapon: string) => void;
  onUpdateTags: (id: string, tags: string[]) => void;
  onUpdateNote: (id: string, note: string) => void;
}

export function MatchCard({ match, onUpdateWeapon, onUpdateTags, onUpdateNote }: MatchCardProps) {
  const [note, setNote] = useState(match.note ?? '');
  const [noteEditing, setNoteEditing] = useState(false);

  const dateStr = new Date(match.played_at).toLocaleString('ja-JP', {
    month: 'numeric',
    day: 'numeric',
    hour: '2-digit',
    minute: '2-digit',
  });

  const handleTagToggle = (tag: string) => {
    const next = match.tags.includes(tag)
      ? match.tags.filter((t) => t !== tag)
      : [...match.tags, tag];
    onUpdateTags(match.id, next);
  };

  const handleNoteBlur = () => {
    setNoteEditing(false);
    if (note !== (match.note ?? '')) {
      onUpdateNote(match.id, note);
    }
  };

  return (
    <div className="bg-slate-800/60 border border-slate-700 rounded-xl p-3 flex flex-col gap-2">
      {/* ヘッダー行 */}
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2">
          <span
            className={`text-xs font-bold px-2 py-0.5 rounded ${
              match.result === 'win'
                ? 'bg-green-500/20 text-green-400 border border-green-500/40'
                : 'bg-red-500/20 text-red-400 border border-red-500/40'
            }`}
          >
            {match.result === 'win' ? 'WIN' : 'LOSE'}
          </span>
          <span className="text-xs text-slate-400">{dateStr}</span>
        </div>
        {match.xp_after != null && (
          <span className="text-sm font-mono text-indigo-300">
            {match.xp_after.toFixed(1)} XP
          </span>
        )}
      </div>

      {/* ルール / ステージ */}
      <div className="flex items-center gap-2 text-xs">
        {match.rule && (
          <span className="bg-slate-700 text-slate-300 px-2 py-0.5 rounded">
            {match.rule}
          </span>
        )}
        {match.stage && (
          <span className="text-slate-400">{match.stage}</span>
        )}
      </div>

      {/* KDA */}
      {(match.kill_count != null || match.death_count != null) && (
        <div className="flex items-center gap-3 text-xs text-slate-300">
          {match.kill_count != null && (
            <span>
              <span className="text-green-400 font-bold">{match.kill_count}</span>
              <span className="text-slate-500"> K</span>
            </span>
          )}
          {match.assist_count != null && (
            <span>
              <span className="text-blue-400 font-bold">{match.assist_count}</span>
              <span className="text-slate-500"> A</span>
            </span>
          )}
          {match.death_count != null && (
            <span>
              <span className="text-red-400 font-bold">{match.death_count}</span>
              <span className="text-slate-500"> D</span>
            </span>
          )}
        </div>
      )}

      {/* ブキ選択 */}
      <WeaponPicker
        currentWeapon={match.weapon}
        onSelect={(weapon) => onUpdateWeapon(match.id, weapon)}
      />

      {/* 反省タグ */}
      <TagInput tags={match.tags} onToggle={handleTagToggle} />

      {/* メモ */}
      {noteEditing ? (
        <textarea
          className="w-full bg-slate-700 text-white placeholder-slate-400 rounded-lg px-2 py-1.5 text-xs resize-none outline-none focus:ring-1 focus:ring-indigo-500"
          rows={2}
          value={note}
          onChange={(e) => setNote(e.target.value)}
          onBlur={handleNoteBlur}
          placeholder="振り返りメモを入力..."
          autoFocus
        />
      ) : (
        <button
          className="text-left text-xs text-slate-500 hover:text-slate-300 transition-colors"
          onClick={() => setNoteEditing(true)}
        >
          {note || '📝 メモを追加...'}
        </button>
      )}
    </div>
  );
}
