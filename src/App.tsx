// InkGraph — メインアプリコンポーネント

import { useState } from 'react';
import { Header } from './components/Header';
import { XpChart } from './components/XpChart';
import { ManualEntryModal } from './components/ManualEntryModal';
import { AnalysisPanel } from './components/AnalysisPanel';
import { MatchHistoryPage } from './components/MatchHistoryPage';
import { OcrDebugPanel } from './components/OcrDebugPanel';
import { useMatches } from './hooks/useMatches';
import type { Match, RawMatch, Rule } from './types';
import './App.css';

type MainTab = 'graph' | 'analysis' | 'history';

export default function App() {
  const [isCapturing, setIsCapturing] = useState(false);
  const [selectedRule, setSelectedRule] = useState<Rule | null>(null);
  const [activeTab, setActiveTab] = useState<MainTab>('graph');
  const [showManualEntry, setShowManualEntry] = useState(false);
  const [editingMatch, setEditingMatch] = useState<Match | null>(null);
  const [historyRefreshKey, setHistoryRefreshKey] = useState(0);

  const { matches, error, addMatch, updateMatch } = useMatches(selectedRule);

  const handleEditSubmit = async (raw: RawMatch) => {
    await updateMatch(raw);
    setHistoryRefreshKey((k) => k + 1);
  };

  const handleOpenEdit = (match: Match) => {
    setEditingMatch(match);
  };

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
        {/* メインエリア */}
        <main className="flex-1 p-6 flex flex-col overflow-hidden">
          {/* タブ切替 */}
          <div className="flex gap-1 mb-4">
            {(
              [
                ['graph', 'XP推移'],
                ['analysis', '分析'],
                ['history', '全試合'],
              ] as const
            ).map(([tab, label]) => (
              <button
                key={tab}
                className={`px-4 py-1.5 rounded-lg text-sm font-medium transition-colors ${
                  activeTab === tab
                    ? 'bg-indigo-600 text-white'
                    : 'bg-slate-700 text-slate-300 hover:bg-slate-600'
                }`}
                onClick={() => setActiveTab(tab)}
              >
                {label}
              </button>
            ))}
          </div>

          {/* コンテンツ切替 */}
          <div className="flex-1 overflow-hidden">
            {activeTab === 'graph' ? (
              <XpChart
                matches={matches}
                selectedRule={selectedRule}
                onRuleChange={setSelectedRule}
              />
            ) : activeTab === 'analysis' ? (
              <AnalysisPanel matches={matches} />
            ) : (
              <MatchHistoryPage
                onEdit={handleOpenEdit}
                onAddNew={() => setShowManualEntry(true)}
                refreshKey={historyRefreshKey}
              />
            )}
          </div>
        </main>
      </div>

      {/* 手動入力モーダル */}
      {showManualEntry && (
        <ManualEntryModal
          onClose={() => setShowManualEntry(false)}
          onSubmit={async (raw) => {
            await addMatch(raw);
            setHistoryRefreshKey((k) => k + 1);
          }}
        />
      )}

      {/* 編集モーダル */}
      {editingMatch && (
        <ManualEntryModal
          initialMatch={editingMatch}
          onClose={() => setEditingMatch(null)}
          onSubmit={handleEditSubmit}
        />
      )}

      {/* 開発モード専用: OCR デバッグパネル */}
      {import.meta.env.DEV && <OcrDebugPanel />}
    </div>
  );
}
