// IkaVision XP — メインアプリコンポーネント

import { useState } from 'react';
import { Header } from './components/Header';
import { XpChart } from './components/XpChart';
import { MatchList } from './components/MatchList';
import { OcrDebugPanel } from './components/OcrDebugPanel';
import { useMatches } from './hooks/useMatches';
import type { Rule } from './types';
import './App.css';

export default function App() {
  const [isCapturing, setIsCapturing] = useState(false);
  const [selectedRule, setSelectedRule] = useState<Rule | null>(null);

  const { matches, isLoading, error, updateWeapon, updateTags, updateNote } =
    useMatches(selectedRule);

  return (
    <div className="flex flex-col h-screen bg-slate-900 text-white">
      {/* ヘッダー */}
      <Header isCapturing={isCapturing} onCapturingChange={setIsCapturing} />

      {/* エラー表示 */}
      {error && (
        <div className="bg-red-900/50 border-b border-red-700 px-6 py-2 text-sm text-red-300">
          ⚠️ {error}
        </div>
      )}

      {/* メインコンテンツ */}
      <div className="flex flex-1 overflow-hidden">
        {/* メインエリア: XP グラフ */}
        <main className="flex-1 p-6 flex flex-col overflow-hidden">
          <h2 className="text-sm font-medium text-slate-400 mb-4">XP 推移</h2>
          <div className="flex-1">
            <XpChart
              matches={matches}
              selectedRule={selectedRule}
              onRuleChange={setSelectedRule}
            />
          </div>
        </main>

        {/* サイドバー: 試合リスト */}
        <aside className="w-80 bg-slate-800/40 border-l border-slate-700 flex flex-col">
          <div className="px-4 py-3 border-b border-slate-700">
            <h2 className="text-sm font-medium text-slate-300">
              直近の試合
              {matches.length > 0 && (
                <span className="ml-2 text-xs text-slate-500">({matches.length}件)</span>
              )}
            </h2>
          </div>
          <div className="flex-1 overflow-y-auto px-3 py-3">
            <MatchList
              matches={matches}
              isLoading={isLoading}
              onUpdateWeapon={updateWeapon}
              onUpdateTags={updateTags}
              onUpdateNote={updateNote}
            />
          </div>
        </aside>
      </div>

      {/* 開発モード専用: OCR デバッグパネル (Issue #2 確認用) */}
      {import.meta.env.DEV && <OcrDebugPanel />}
    </div>
  );
}
