// IkaVision XP — 反省タグ入力 UI

import { PRESET_TAGS } from '../types';

interface TagInputProps {
  tags: string[];
  onToggle: (tag: string) => void;
}

export function TagInput({ tags, onToggle }: TagInputProps) {
  return (
    <div className="flex flex-wrap gap-1">
      {PRESET_TAGS.map((tag) => {
        const active = tags.includes(tag);
        return (
          <button
            key={tag}
            onClick={() => onToggle(tag)}
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
  );
}
