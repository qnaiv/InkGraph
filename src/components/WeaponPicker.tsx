// InkGraph — ブキ選択 UI

import { useState, useMemo } from 'react';
import { WEAPONS, getRecentWeapons, pushRecentWeapon, type Weapon } from '../assets/weapons';

interface WeaponPickerProps {
  currentWeapon: string | null;
  onSelect: (weaponName: string) => void;
}

export function WeaponPicker({ currentWeapon, onSelect }: WeaponPickerProps) {
  const [isOpen, setIsOpen] = useState(false);
  const [search, setSearch] = useState('');

  // isOpen が変わるたびに最近使ったブキを再取得する意図があるため依存を維持
  // eslint-disable-next-line react-hooks/exhaustive-deps
  const recentWeapons = useMemo(() => getRecentWeapons(5), [isOpen]);

  const filteredWeapons = useMemo(() => {
    if (!search) return WEAPONS;
    return WEAPONS.filter((w) =>
      w.name.includes(search) || w.category.includes(search)
    );
  }, [search]);

  const handleSelect = (weapon: Weapon) => {
    pushRecentWeapon(weapon.id);
    onSelect(weapon.name);
    setIsOpen(false);
    setSearch('');
  };

  return (
    <div className="relative">
      {/* トリガーボタン */}
      <button
        className="w-full flex items-center justify-between bg-slate-700 hover:bg-slate-600 rounded-lg px-3 py-1.5 text-sm transition-colors"
        onClick={() => setIsOpen(!isOpen)}
      >
        <span className={currentWeapon ? 'text-white' : 'text-slate-400'}>
          {currentWeapon ?? 'ブキを選択...'}
        </span>
        <span className="text-slate-400 text-xs">▼</span>
      </button>

      {/* ドロップダウン */}
      {isOpen && (
        <div className="absolute bottom-full left-0 right-0 mb-1 bg-slate-800 border border-slate-600 rounded-xl shadow-2xl z-50">
          {/* 最近使ったブキ */}
          {recentWeapons.length > 0 && (
            <div className="p-2 border-b border-slate-700">
              <p className="text-xs text-slate-500 mb-1.5 px-1">最近使ったブキ</p>
              <div className="flex flex-wrap gap-1">
                {recentWeapons.map((w) => (
                  <button
                    key={w.id}
                    className="px-2 py-1 bg-indigo-600/30 hover:bg-indigo-600/60 text-indigo-300 rounded text-xs transition-colors"
                    onClick={() => handleSelect(w)}
                  >
                    {w.name}
                  </button>
                ))}
              </div>
            </div>
          )}

          {/* 検索 */}
          <div className="p-2 border-b border-slate-700">
            <input
              className="w-full bg-slate-700 text-white placeholder-slate-400 rounded-lg px-3 py-1.5 text-sm outline-none focus:ring-1 focus:ring-indigo-500"
              placeholder="ブキ名を検索..."
              value={search}
              onChange={(e) => setSearch(e.target.value)}
              autoFocus
            />
          </div>

          {/* ブキ一覧 */}
          <div className="max-h-48 overflow-y-auto p-1">
            {filteredWeapons.length === 0 ? (
              <p className="text-slate-500 text-sm text-center py-4">見つかりません</p>
            ) : (
              filteredWeapons.map((w) => (
                <button
                  key={w.id}
                  className="w-full flex items-center gap-2 px-3 py-1.5 rounded-lg hover:bg-slate-700 text-left transition-colors"
                  onClick={() => handleSelect(w)}
                >
                  <span className="text-xs bg-slate-600 text-slate-300 px-1.5 py-0.5 rounded">
                    {w.category}
                  </span>
                  <span className="text-sm text-white">{w.name}</span>
                </button>
              ))
            )}
          </div>
        </div>
      )}
    </div>
  );
}
