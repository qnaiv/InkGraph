// InkGraph — 試合カード (コンパクト行 / 展開で編集)

import { useState } from 'react';
import type { Match } from '../types';
import { WeaponPicker } from './WeaponPicker';
import { TagInput } from './TagInput';

interface MatchCardProps {
  match: Match;
  onUpdateWeapon: (id: string, weapon: string) => void;
  onUpdateTags: (id: string, tags: string[]) => void;
  onUpdateNote: (id: string, note: string) => void;
  onEdit?: (match: Match) => void;
  onDelete?: (id: string) => void;
}

export function MatchCard({ match, onUpdateWeapon, onUpdateTags, onUpdateNote, onEdit, onDelete }: MatchCardProps) {
  const [expanded, setExpanded] = useState(false);
  const [note, setNote] = useState(match.note ?? '');
  const [noteEditing, setNoteEditing] = useState(false);
  const [confirmDelete, setConfirmDelete] = useState(false);

  const dateStr = new Date(match.played_at).toLocaleString('ja-JP', {
    month: 'numeric',
    day: 'numeric',
    hour: '2-digit',
    minute: '2-digit',
  });

  const kda = [
    match.kill_count    != null ? `${match.kill_count}K`    : null,
    match.death_count   != null ? `${match.death_count}D`   : null,
    match.special_count != null ? `${match.special_count}S` : null,
  ].filter(Boolean).join('/');

  const handleNoteBlur = () => {
    setNoteEditing(false);
    if (note !== (match.note ?? '')) onUpdateNote(match.id, note);
  };

  const handleTagToggle = (tag: string) => {
    const next = match.tags.includes(tag)
      ? match.tags.filter((t) => t !== tag)
      : [...match.tags, tag];
    onUpdateTags(match.id, next);
  };

  return (
    <div className="border-b border-slate-700/50 last:border-0">
      {/* コンパクト行 */}
      <button
        className="w-full flex items-center gap-1.5 px-1 py-1.5 text-left hover:bg-slate-700/30 transition-colors rounded disabled:cursor-default"
        onClick={() => match.result !== 'in_progress' && setExpanded((v) => !v)}
        disabled={match.result === 'in_progress'}
      >
        {/* WIN / LOSE / 試合中 */}
        {match.result === 'in_progress' ? (
          <span className="shrink-0 text-[10px] font-bold w-9 text-center py-0.5 rounded bg-amber-500/20 text-amber-400 border border-amber-500/40 animate-pulse">
            試合中
          </span>
        ) : (
          <span className={`shrink-0 text-[10px] font-bold w-9 text-center py-0.5 rounded ${
            match.result === 'win'
              ? 'bg-green-500/20 text-green-400 border border-green-500/40'
              : 'bg-red-500/20 text-red-400 border border-red-500/40'
          }`}>
            {match.result === 'win' ? 'WIN' : 'LOSE'}
          </span>
        )}

        {/* 日時 */}
        <span className="shrink-0 text-[10px] text-slate-500 w-[68px]">{dateStr}</span>

        {/* ルール + ステージ */}
        <span className="flex-1 text-xs text-slate-300 truncate">
          {[match.rule, match.stage].filter(Boolean).join(' ')}
        </span>

        {/* KDA */}
        {kda && (
          <span className="shrink-0 text-[10px] text-slate-400 font-mono">{kda}</span>
        )}

        {/* XP */}
        {match.xp_after != null && (
          <span className="shrink-0 text-[10px] font-mono text-indigo-300 w-14 text-right">
            {match.xp_after.toFixed(1)}
          </span>
        )}

        {/* 展開アイコン */}
        <span className="shrink-0 text-slate-600 text-[10px] w-3 text-center">
          {expanded ? '▲' : '▼'}
        </span>
      </button>

      {/* 展開セクション: ブキ / タグ / メモ */}
      {expanded && (
        <div className="px-2 pb-3 pt-1 space-y-2 border-t border-slate-700/40">
          <div className="flex gap-1.5">
            {onEdit && (
              <button
                className="flex-1 py-1 text-xs text-indigo-400 hover:text-indigo-300 hover:bg-indigo-500/10 rounded transition-colors border border-indigo-500/30"
                onClick={() => onEdit(match)}
              >
                ✏️ 全項目を編集
              </button>
            )}
            {onDelete && !confirmDelete && (
              <button
                className="py-1 px-2 text-xs text-slate-500 hover:text-red-400 hover:bg-red-500/10 rounded transition-colors border border-slate-600/50 hover:border-red-500/40"
                onClick={() => setConfirmDelete(true)}
                title="削除"
              >
                🗑
              </button>
            )}
            {onDelete && confirmDelete && (
              <div className="flex gap-1 items-center">
                <span className="text-xs text-red-400">削除しますか?</span>
                <button
                  className="py-0.5 px-2 text-xs text-white bg-red-600 hover:bg-red-500 rounded transition-colors"
                  onClick={() => { setConfirmDelete(false); onDelete(match.id); }}
                >
                  削除
                </button>
                <button
                  className="py-0.5 px-2 text-xs text-slate-400 hover:text-white rounded transition-colors"
                  onClick={() => setConfirmDelete(false)}
                >
                  キャンセル
                </button>
              </div>
            )}
          </div>
          <WeaponPicker
            currentWeapon={match.weapon}
            onSelect={(weapon) => onUpdateWeapon(match.id, weapon)}
          />
          <TagInput tags={match.tags} onToggle={handleTagToggle} />
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
      )}
    </div>
  );
}
