// IkaVision XP — 直近の試合リスト

import { Match } from '../types';
import { MatchCard } from './MatchCard';

interface MatchListProps {
  matches: Match[];
  isLoading: boolean;
  onUpdateWeapon: (id: string, weapon: string) => void;
  onUpdateTags: (id: string, tags: string[]) => void;
  onUpdateNote: (id: string, note: string) => void;
}

export function MatchList({
  matches,
  isLoading,
  onUpdateWeapon,
  onUpdateTags,
  onUpdateNote,
}: MatchListProps) {
  if (isLoading) {
    return (
      <div className="flex items-center justify-center h-32 text-slate-500 text-sm">
        読み込み中...
      </div>
    );
  }

  if (matches.length === 0) {
    return (
      <div className="flex flex-col items-center justify-center h-32 text-slate-500 text-sm gap-2">
        <span className="text-2xl">🎮</span>
        <span>対戦記録がありません</span>
      </div>
    );
  }

  return (
    <div className="flex flex-col gap-2">
      {matches.map((match) => (
        <MatchCard
          key={match.id}
          match={match}
          onUpdateWeapon={onUpdateWeapon}
          onUpdateTags={onUpdateTags}
          onUpdateNote={onUpdateNote}
        />
      ))}
    </div>
  );
}
