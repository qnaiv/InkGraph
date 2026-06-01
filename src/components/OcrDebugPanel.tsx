// InkGraph — OCR デバッグパネル
// Issue #2 の Windows 実機確認用。開発ビルド時のみ表示する。

import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import type { OcrTestResult, CaptureDebugResult, WindowInfo, CaptureStatusPayload, YoloDebugResult } from '../types';

export function OcrDebugPanel() {
  const [minimized, setMinimized] = useState(true);
  const [captureStatus, setCaptureStatus] = useState<CaptureStatusPayload | null>(null);

  useEffect(() => {
    const unlisten = listen<CaptureStatusPayload>('capture_status', (e) => {
      setCaptureStatus(e.payload);
    });
    return () => { unlisten.then((fn) => fn()); };
  }, []);

  // ── ファイル OCR ──────────────────────────────────────────────────────────
  const [imagePath, setImagePath] = useState('');
  const [ocrResult, setOcrResult] = useState<OcrTestResult | null>(null);
  const [ocrError, setOcrError]   = useState<string | null>(null);
  const [ocrLoading, setOcrLoading] = useState(false);

  const runOcr = async () => {
    if (!imagePath.trim()) return;
    setOcrLoading(true);
    setOcrError(null);
    setOcrResult(null);
    try {
      const res = await invoke<OcrTestResult>('test_ocr', { imagePath: imagePath.trim() });
      setOcrResult(res);
    } catch (e) {
      setOcrError(String(e));
    } finally {
      setOcrLoading(false);
    }
  };

  // ── ライブキャプチャ診断 ──────────────────────────────────────────────────
  const [windows, setWindows]       = useState<WindowInfo[]>([]);
  const [diagHwnd, setDiagHwnd]     = useState<number | null>(null);
  const [diagResult, setDiagResult] = useState<CaptureDebugResult | null>(null);
  const [diagError, setDiagError]   = useState<string | null>(null);
  const [diagLoading, setDiagLoading] = useState(false);

  // ── YOLO 診断 ──────────────────────────────────────────────────────────────
  const [yoloResult, setYoloResult]   = useState<YoloDebugResult | null>(null);
  const [yoloError, setYoloError]     = useState<string | null>(null);
  const [yoloLoading, setYoloLoading] = useState(false);

  const runYoloDiag = async () => {
    if (diagHwnd == null) return;
    setYoloLoading(true);
    setYoloError(null);
    setYoloResult(null);
    try {
      const res = await invoke<YoloDebugResult>('debug_yolo', { hwnd: diagHwnd });
      setYoloResult(res);
    } catch (e) {
      setYoloError(String(e));
    } finally {
      setYoloLoading(false);
    }
  };

  const loadWindows = async () => {
    try {
      const list = await invoke<WindowInfo[]>('list_windows');
      setWindows(list);
    } catch { /* ignore */ }
  };

  const runDiag = async () => {
    if (diagHwnd == null) return;
    setDiagLoading(true);
    setDiagError(null);
    setDiagResult(null);
    try {
      const res = await invoke<CaptureDebugResult>('debug_capture', { hwnd: diagHwnd });
      setDiagResult(res);
    } catch (e) {
      setDiagError(String(e));
    } finally {
      setDiagLoading(false);
    }
  };

  if (minimized) {
    return (
      <button
        className="fixed bottom-4 left-4 bg-slate-900 border border-amber-500/50 rounded-lg px-3 py-1.5 text-amber-400 text-xs font-bold shadow-xl z-50 hover:bg-slate-800 transition-colors flex items-center gap-2"
        onClick={() => setMinimized(false)}
      >
        🔬 DEBUG
        {captureStatus?.active && (
          <span className={`px-1.5 py-0.5 rounded text-xs font-bold ${
            captureStatus.yolo_loaded
              ? 'bg-violet-600/40 text-violet-300 border border-violet-500/50'
              : 'bg-slate-700 text-slate-400 border border-slate-600'
          }`}>
            {captureStatus.yolo_loaded ? 'YOLO' : 'Pixel'}
          </span>
        )}
      </button>
    );
  }

  return (
    <div className="fixed bottom-4 left-4 w-[420px] bg-slate-900 border border-amber-500/50 rounded-xl shadow-2xl p-4 z-50 text-sm space-y-5 max-h-[90vh] overflow-y-auto">
      <div className="flex items-center justify-between">
        <span className="text-amber-400 font-bold text-xs tracking-widest">🔬 DEBUG PANEL</span>
        <button
          className="text-slate-400 hover:text-white text-xs px-2 py-0.5 rounded hover:bg-slate-700 transition-colors"
          onClick={() => setMinimized(true)}
        >
          最小化
        </button>
      </div>

      {/* ── 検知エンジン状態 ─────────────────────────────────────────────── */}
      <div className={`rounded-lg p-3 flex items-center justify-between text-xs border ${
        !captureStatus?.active
          ? 'bg-slate-800 border-slate-700'
          : captureStatus.yolo_loaded
            ? 'bg-violet-900/30 border-violet-500/50'
            : 'bg-slate-800 border-slate-600'
      }`}>
        <div className="flex items-center gap-2">
          <span className={`w-2 h-2 rounded-full ${captureStatus?.active ? 'bg-green-400 animate-pulse' : 'bg-slate-600'}`} />
          <span className="text-slate-300 font-semibold">検知エンジン</span>
        </div>
        <div className="flex items-center gap-2">
          {captureStatus?.active ? (
            captureStatus.yolo_loaded ? (
              <span className="px-2 py-0.5 rounded font-bold text-violet-200 bg-violet-600/40 border border-violet-400/60 tracking-wide">
                🤖 YOLO (ONNX)
              </span>
            ) : (
              <span className="px-2 py-0.5 rounded font-bold text-slate-300 bg-slate-700 border border-slate-500 tracking-wide">
                📐 Pixel Fallback
              </span>
            )
          ) : (
            <span className="text-slate-500">キャプチャ停止中</span>
          )}
        </div>
      </div>

      {/* ── ① ライブキャプチャ診断 ─────────────────────────────────────── */}
      <section>
        <p className="text-amber-300 text-xs font-semibold mb-2">① ライブキャプチャ診断</p>
        <p className="text-slate-500 text-xs mb-2 leading-relaxed">
          リザルト画面を表示した状態でウィンドウを選び「診断」を押す。<br />
          各ステップの通過状況が確認できる。
        </p>

        <div className="flex gap-2 mb-2">
          <select
            className="flex-1 bg-slate-800 text-white text-xs rounded-lg px-2 py-1.5 outline-none cursor-pointer"
            value={diagHwnd ?? ''}
            onFocus={loadWindows}
            onChange={(e) => setDiagHwnd(e.target.value ? Number(e.target.value) : null)}
          >
            <option value="">ウィンドウを選択...</option>
            {windows.map((w) => (
              <option key={w.hwnd} value={w.hwnd}>{w.title.slice(0, 45)}</option>
            ))}
          </select>
          <button
            className="px-3 py-1.5 bg-amber-600 hover:bg-amber-500 text-white text-xs rounded-lg disabled:opacity-50 transition-colors whitespace-nowrap"
            onClick={runDiag}
            disabled={diagLoading || diagHwnd == null}
          >
            {diagLoading ? '…' : '診断'}
          </button>
          <button
            className="px-3 py-1.5 bg-violet-700 hover:bg-violet-600 text-white text-xs rounded-lg disabled:opacity-50 transition-colors whitespace-nowrap"
            onClick={runYoloDiag}
            disabled={yoloLoading || diagHwnd == null}
          >
            {yoloLoading ? '…' : 'YOLO'}
          </button>
        </div>

        {/* YOLO 診断結果 */}
        {yoloError && (
          <div className="bg-red-900/50 border border-red-700 rounded-lg p-2 text-red-300 text-xs break-all mb-2">
            ❌ {yoloError}
          </div>
        )}
        {yoloResult && (
          <div className="bg-slate-800 rounded-lg p-2 mb-2 text-xs space-y-1">
            <div className="flex justify-between items-center">
              <span className="text-slate-400">YOLO診断</span>
              <span className="text-slate-300 font-mono">{yoloResult.frame_w}×{yoloResult.frame_h}</span>
            </div>
            {yoloResult.error && (
              <p className="text-red-400 break-all">{yoloResult.error}</p>
            )}
            {yoloResult.detections.length === 0 ? (
              <p className="text-yellow-400">検出なし (信頼度0.10以上の候補がゼロ → モデルかフレームに問題あり)</p>
            ) : (
              <div className="space-y-0.5 max-h-40 overflow-y-auto">
                {yoloResult.detections
                  .sort((a, b) => b.confidence - a.confidence)
                  .map((d, i) => (
                    <div key={i} className="flex items-center gap-2 bg-slate-900 rounded px-1.5 py-0.5">
                      <span className={`font-bold w-4 text-right ${d.confidence >= 0.70 ? 'text-green-400' : d.confidence >= 0.40 ? 'text-yellow-400' : 'text-slate-500'}`}>
                        {d.confidence >= 0.70 ? '✓' : d.confidence >= 0.40 ? '△' : '✗'}
                      </span>
                      <span className="text-white font-mono w-24 truncate">{d.class_name}</span>
                      <div className="flex-1 bg-slate-700 rounded-full h-1.5">
                        <div
                          className={`h-1.5 rounded-full ${d.confidence >= 0.70 ? 'bg-green-500' : d.confidence >= 0.40 ? 'bg-yellow-500' : 'bg-slate-500'}`}
                          style={{ width: `${Math.round(d.confidence * 100)}%` }}
                        />
                      </div>
                      <span className="text-slate-300 font-mono w-10 text-right">{(d.confidence * 100).toFixed(0)}%</span>
                    </div>
                  ))}
              </div>
            )}
            <p className="text-slate-500 text-xs">✓=0.70↑(有効) △=0.40-0.70(閾値未満) ✗=0.10-0.40(弱い)</p>
          </div>
        )}

        {diagError && (
          <div className="bg-red-900/50 border border-red-700 rounded-lg p-2 text-red-300 text-xs break-all">
            ❌ {diagError}
          </div>
        )}

        {diagResult && (
          <div className="space-y-1.5">
            {/* フレーム情報 */}
            <div className="bg-slate-800 rounded-lg p-2 flex justify-between text-xs">
              <span className="text-slate-400">フレームサイズ</span>
              <span className="text-white font-mono">{diagResult.frame_w} × {diagResult.frame_h}</span>
            </div>

            {/* Phase 1: バトル開始 */}
            <div className="bg-slate-800 rounded-lg p-2 text-xs">
              <div className="flex justify-between mb-1">
                <span className="text-slate-400">Phase 1: バトル開始</span>
                <div className="flex gap-2">
                  <span className={diagResult.dark_scroll_found ? 'text-green-400' : 'text-slate-600'}>
                    {diagResult.dark_scroll_found ? '✓ 暗巻物' : '✗ 暗巻物'}
                  </span>
                  <span className={diagResult.battle_start_found ? 'text-green-400' : 'text-slate-500'}>
                    {diagResult.battle_start_found ? '✓ 検出' : '✗ 未検出'}
                  </span>
                </div>
              </div>
              <pre className="text-slate-300 text-xs whitespace-pre-wrap break-all max-h-16 overflow-y-auto bg-slate-900 rounded p-1">
                {diagResult.battle_start_text || '(空)'}
              </pre>
            </div>

            {/* Phase 2: リザルト画面判定 */}
            <div className="bg-slate-800 rounded-lg p-2 text-xs space-y-1">
              <div className="flex justify-between">
                <span className="text-slate-400">Phase 2: リザルト画面判定</span>
                <span className={diagResult.win_grey_rows >= 2 && diagResult.lose_grey_rows >= 2 ? 'text-green-400' : 'text-slate-500'}>
                  WIN側 {diagResult.win_grey_rows}/4 {diagResult.win_grey_rows >= 2 ? '✓' : '✗'}
                  {' | '}
                  LOSE側 {diagResult.lose_grey_rows}/4 {diagResult.lose_grey_rows >= 2 ? '✓' : '✗'}
                </span>
              </div>
              <div className="grid grid-cols-4 gap-1 mt-1">
                <div className="bg-slate-900 rounded p-1 text-center">
                  <p className="text-slate-500 text-xs">WIN 側</p>
                  <p className={`font-mono font-bold ${diagResult.yellow_win_px >= 100 ? 'text-green-400' : 'text-slate-300'}`}>
                    {diagResult.yellow_win_px}
                  </p>
                </div>
                <div className="bg-slate-900 rounded p-1 text-center">
                  <p className="text-slate-500 text-xs">LOSE 側</p>
                  <p className={`font-mono font-bold ${diagResult.yellow_lose_px >= 100 ? 'text-yellow-400' : 'text-slate-300'}`}>
                    {diagResult.yellow_lose_px}
                  </p>
                </div>
                <div className="bg-slate-900 rounded p-1 text-center">
                  <p className="text-slate-500 text-xs">重心 y</p>
                  <p className="text-white font-mono font-bold">{diagResult.centroid_y.toFixed(3)}</p>
                </div>
                <div className="bg-slate-900 rounded p-1 text-center">
                  <p className="text-slate-500 text-xs">spread</p>
                  <p className="text-slate-300 font-mono font-bold">
                    {diagResult.y_spread}px
                  </p>
                </div>
              </div>
            </div>

            {/* ルール・ステージ OCR */}
            <div className="bg-slate-800 rounded-lg p-2 text-xs space-y-1.5">
              <p className="text-slate-400 font-semibold">ルール / ステージ OCR</p>
              <div className="flex gap-2">
                <div className="flex-1 bg-slate-900 rounded p-1.5">
                  <p className="text-slate-500 text-xs mb-0.5">ルール (raw)</p>
                  <p className="text-white font-mono break-all">{diagResult.rule_ocr_text || '(空)'}</p>
                  <p className={`text-xs mt-0.5 ${diagResult.rule_normalized ? 'text-green-400' : 'text-slate-500'}`}>
                    → {diagResult.rule_normalized ?? '未マッチ'}
                  </p>
                </div>
                <div className="flex-1 bg-slate-900 rounded p-1.5">
                  <p className="text-slate-500 text-xs mb-0.5">ステージ (raw)</p>
                  <p className="text-white font-mono break-all">{diagResult.stage_ocr_text || '(空)'}</p>
                  <p className={`text-xs mt-0.5 ${diagResult.stage_normalized ? 'text-green-400' : 'text-slate-500'}`}>
                    → {diagResult.stage_normalized ?? '未マッチ'}
                  </p>
                </div>
              </div>
            </div>

            {/* 判定サマリー */}
            <div className={`rounded-lg p-2 text-xs break-all ${
              diagResult.detection_summary.includes('✓ WIN') ? 'bg-green-900/40 border border-green-700 text-green-300' :
              diagResult.detection_summary.includes('✓ LOSE') ? 'bg-yellow-900/40 border border-yellow-700 text-yellow-300' :
              diagResult.detection_summary.includes('Phase 1 ✓') ? 'bg-blue-900/40 border border-blue-700 text-blue-300' :
              'bg-slate-800 border border-slate-600 text-slate-400'
            }`}>
              {diagResult.detection_summary}
            </div>
          </div>
        )}
      </section>

      <hr className="border-slate-700" />

      {/* ── ② ファイル OCR テスト ────────────────────────────────────────── */}
      <section>
        <p className="text-amber-300 text-xs font-semibold mb-2">② ファイル OCR テスト</p>

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
            disabled={ocrLoading || !imagePath.trim()}
          >
            {ocrLoading ? '…' : 'OCR 実行'}
          </button>
        </div>

        {ocrError && (
          <div className="bg-red-900/50 border border-red-700 rounded-lg p-2 mb-2 text-red-300 text-xs break-all">
            ❌ {ocrError}
          </div>
        )}

        {ocrResult && (
          <div className="space-y-2">
            <div className="bg-slate-800 rounded-lg p-2">
              <p className="text-slate-400 text-xs mb-1">認識テキスト (raw)</p>
              <pre className="text-green-300 text-xs whitespace-pre-wrap break-all max-h-32 overflow-y-auto">
                {ocrResult.raw_text || '(空)'}
              </pre>
            </div>
            <div className="bg-slate-800 rounded-lg p-2">
              <p className="text-slate-400 text-xs mb-1">行分割 ({ocrResult.lines.length} 行)</p>
              <ul className="text-white text-xs space-y-0.5 max-h-20 overflow-y-auto">
                {ocrResult.lines.map((l, i) => (
                  <li key={i} className="flex gap-2">
                    <span className="text-slate-500 w-4 text-right">{i + 1}</span>
                    <span>{l}</span>
                  </li>
                ))}
              </ul>
            </div>
            <div className="flex flex-wrap gap-1">
              {['WIN', 'LOSE', 'ガチエリア', 'ガチヤグラ', 'ガチホコ', 'ガチアサリ'].map((kw) => {
                const found = ocrResult.raw_text.toUpperCase().includes(kw.toUpperCase());
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
              })}
            </div>
          </div>
        )}

        <p className="text-slate-600 text-xs mt-3 leading-relaxed">
          PNG/BMP を絶対パスで入力 → Enter。<br />
          ja-JP 言語パックが必要: 設定 → 時刻と言語 → 言語 → 日本語
        </p>
      </section>
    </div>
  );
}
