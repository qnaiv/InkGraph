// IkaVision XP — OCR デバッグパネル
// Issue #2 の Windows 実機確認用。開発ビルド時のみ表示する。

import { useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { OcrTestResult } from '../types';

export function OcrDebugPanel() {
  const [imagePath, setImagePath] = useState('');
  const [result, setResult] = useState<OcrTestResult | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);

  const runOcr = async () => {
    if (!imagePath.trim()) return;
    setLoading(true);
    setError(null);
    setResult(null);
    try {
      const res = await invoke<OcrTestResult>('test_ocr', {
        imagePath: imagePath.trim(),
      });
      setResult(res);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="fixed bottom-4 right-4 w-96 bg-slate-900 border border-amber-500/50 rounded-xl shadow-2xl p-4 z-50 text-sm">
      <div className="flex items-center justify-between mb-3">
        <span className="text-amber-400 font-bold text-xs tracking-widest">
          🔬 OCR DEBUG (Issue #2)
        </span>
        <span className="text-slate-500 text-xs">開発モード専用</span>
      </div>

      {/* パス入力 */}
      <div className="flex gap-2 mb-3">
        <input
          className="flex-1 bg-slate-800 text-white placeholder-slate-500 rounded-lg px-3 py-1.5 text-xs outline-none focus:ring-1 focus:ring-amber-500"
          placeholder="C:\Users\...\result.png"
          value={imagePath}
          onChange={(e) => setImagePath(e.target.value)}
          onKeyDown={(e) => e.key === 'Enter' && runOcr()}
        />
        <button
          className="px-3 py-1.5 bg-amber-600 hover:bg-amber-500 text-white text-xs rounded-lg disabled:opacity-50 transition-colors"
          onClick={runOcr}
          disabled={loading || !imagePath.trim()}
        >
          {loading ? '…' : 'OCR 実行'}
        </button>
      </div>

      {/* エラー */}
      {error && (
        <div className="bg-red-900/50 border border-red-700 rounded-lg p-2 mb-2 text-red-300 text-xs break-all">
          ❌ {error}
        </div>
      )}

      {/* 結果 */}
      {result && (
        <div className="space-y-2">
          <div className="bg-slate-800 rounded-lg p-2">
            <p className="text-slate-400 text-xs mb-1">認識テキスト (raw)</p>
            <pre className="text-green-300 text-xs whitespace-pre-wrap break-all max-h-32 overflow-y-auto">
              {result.raw_text || '(空)'}
            </pre>
          </div>
          <div className="bg-slate-800 rounded-lg p-2">
            <p className="text-slate-400 text-xs mb-1">
              行分割 ({result.lines.length} 行)
            </p>
            <ul className="text-white text-xs space-y-0.5 max-h-20 overflow-y-auto">
              {result.lines.map((l, i) => (
                <li key={i} className="flex gap-2">
                  <span className="text-slate-500 w-4 text-right">{i + 1}</span>
                  <span>{l}</span>
                </li>
              ))}
            </ul>
          </div>
          {/* WIN/LOSE 検出チェック */}
          <div className="flex gap-2">
            {['WIN', 'LOSE', 'ガチエリア', 'ガチヤグラ', 'ガチホコ', 'ガチアサリ'].map(
              (kw) => {
                const found = result.raw_text
                  .toUpperCase()
                  .includes(kw.toUpperCase());
                return (
                  <span
                    key={kw}
                    className={`text-xs px-1.5 py-0.5 rounded ${
                      found
                        ? 'bg-green-500/20 text-green-400 border border-green-500/40'
                        : 'bg-slate-700 text-slate-500'
                    }`}
                  >
                    {found ? '✓' : '✗'} {kw}
                  </span>
                );
              }
            )}
          </div>
        </div>
      )}

      {/* ヒント */}
      <p className="text-slate-600 text-xs mt-3 leading-relaxed">
        PNG/BMP を絶対パスで入力 → Enter。<br />
        ja-JP 言語パックが必要: 設定 → 時刻と言語 → 言語 → 日本語
      </p>
    </div>
  );
}
