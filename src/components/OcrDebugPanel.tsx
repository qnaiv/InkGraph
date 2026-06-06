// InkGraph — デバッグパネル

import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import type {
  OcrTestResult, CaptureDebugResult, WindowInfo, CaptureStatusPayload,
  FullDebugResult, OcrDebugResult, CascadeDebugDetection, HeaderDebugDetection,
} from '../types';

export function OcrDebugPanel() {
  const [minimized, setMinimized] = useState(true);
  const [captureStatus, setCaptureStatus] = useState<CaptureStatusPayload | null>(null);

  useEffect(() => {
    const unlisten = listen<CaptureStatusPayload>('capture_status', (e) => {
      setCaptureStatus(e.payload);
    });
    return () => { unlisten.then((fn) => fn()); };
  }, []);

  // ── ウィンドウ選択 ────────────────────────────────────────────────────────
  const [windows, setWindows]   = useState<WindowInfo[]>([]);
  const [diagHwnd, setDiagHwnd] = useState<number | null>(null);

  const loadWindows = async () => {
    try {
      const list = await invoke<WindowInfo[]>('list_windows');
      setWindows(list);
    } catch { /* ignore */ }
  };

  // ── キャプチャ診断 (pixel ベース) ─────────────────────────────────────────
  const [diagResult, setDiagResult]   = useState<CaptureDebugResult | null>(null);
  const [diagError, setDiagError]     = useState<string | null>(null);
  const [diagLoading, setDiagLoading] = useState(false);

  const runDiag = async () => {
    if (diagHwnd == null) return;
    setDiagLoading(true); setDiagError(null); setDiagResult(null);
    try {
      setDiagResult(await invoke<CaptureDebugResult>('debug_capture', { hwnd: diagHwnd }));
    } catch (e) { setDiagError(String(e)); }
    finally { setDiagLoading(false); }
  };

  // ── モデル診断 (YOLO + カスケード 統合) ──────────────────────────────────
  const [fullResult, setFullResult]   = useState<FullDebugResult | null>(null);
  const [fullError, setFullError]     = useState<string | null>(null);
  const [fullLoading, setFullLoading] = useState(false);

  const runFullDiag = async () => {
    if (diagHwnd == null) return;
    setFullLoading(true); setFullError(null); setFullResult(null);
    try {
      setFullResult(await invoke<FullDebugResult>('debug_full', { hwnd: diagHwnd }));
    } catch (e) { setFullError(String(e)); }
    finally { setFullLoading(false); }
  };

  // ── ファイル OCR ──────────────────────────────────────────────────────────
  const [imagePath, setImagePath]     = useState('');
  const [ocrResult, setOcrResult]     = useState<OcrTestResult | null>(null);
  const [ocrError, setOcrError]       = useState<string | null>(null);
  const [ocrLoading, setOcrLoading]   = useState(false);

  const runOcr = async () => {
    if (!imagePath.trim()) return;
    setOcrLoading(true); setOcrError(null); setOcrResult(null);
    try {
      setOcrResult(await invoke<OcrTestResult>('test_ocr', { imagePath: imagePath.trim() }));
    } catch (e) { setOcrError(String(e)); }
    finally { setOcrLoading(false); }
  };

  // ─────────────────────────────────────────────────────────────────────────

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
    <div className="fixed inset-0 z-50 bg-black/70 flex items-stretch justify-stretch">
      <div className="flex-1 bg-slate-900 border border-amber-500/30 text-sm flex flex-col overflow-hidden">

        {/* ヘッダー */}
        <div className="flex items-center justify-between px-4 py-2 border-b border-slate-700 shrink-0">
          <div className="flex items-center gap-3">
            <span className="text-amber-400 font-bold text-xs tracking-widest">🔬 DEBUG PANEL</span>
            {/* 検知エンジン状態 */}
            {captureStatus?.active ? (
              captureStatus.yolo_loaded ? (
                <span className="px-2 py-0.5 rounded text-xs font-bold text-violet-200 bg-violet-600/40 border border-violet-400/60">
                  🤖 YOLO (ONNX)
                </span>
              ) : (
                <span className="px-2 py-0.5 rounded text-xs font-bold text-slate-300 bg-slate-700 border border-slate-500">
                  📐 Pixel Fallback
                </span>
              )
            ) : (
              <span className="text-slate-500 text-xs">キャプチャ停止中</span>
            )}
          </div>
          <button
            className="text-slate-400 hover:text-white text-xs px-2 py-0.5 rounded hover:bg-slate-700 transition-colors"
            onClick={() => setMinimized(true)}
          >
            閉じる
          </button>
        </div>

        {/* スクロール可能なコンテンツ */}
        <div className="flex-1 overflow-y-auto p-4 space-y-5">

          {/* ── ウィンドウ選択 + ボタン ───────────────────────────────── */}
          <div className="flex gap-2">
            <select
              className="flex-1 bg-slate-800 text-white text-xs rounded-lg px-2 py-1.5 outline-none cursor-pointer"
              value={diagHwnd ?? ''}
              onFocus={loadWindows}
              onChange={(e) => setDiagHwnd(e.target.value ? Number(e.target.value) : null)}
            >
              <option value="">ウィンドウを選択...</option>
              {windows.map((w) => (
                <option key={w.hwnd} value={w.hwnd}>{w.title.slice(0, 60)}</option>
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
              onClick={runFullDiag}
              disabled={fullLoading || diagHwnd == null}
            >
              {fullLoading ? '…' : 'モデル診断'}
            </button>
          </div>

          {/* ── モデル診断結果 ──────────────────────────────────────────── */}
          {fullError && (
            <div className="bg-red-900/50 border border-red-700 rounded-lg p-2 text-red-300 text-xs break-all">
              ❌ {fullError}
            </div>
          )}
          {fullResult && <FullDebugPanel result={fullResult} />}

          <hr className="border-slate-700" />

          {/* ── キャプチャ診断結果 ─────────────────────────────────────── */}
          <section>
            <p className="text-amber-300 text-xs font-semibold mb-2">キャプチャ診断 (pixel ベース)</p>
            {diagError && (
              <div className="bg-red-900/50 border border-red-700 rounded-lg p-2 text-red-300 text-xs break-all mb-2">
                ❌ {diagError}
              </div>
            )}
            {diagResult && <CaptureDiagSection result={diagResult} />}
          </section>

          <hr className="border-slate-700" />

          {/* ── ファイル OCR テスト ────────────────────────────────────── */}
          <section>
            <p className="text-amber-300 text-xs font-semibold mb-2">ファイル OCR テスト</p>
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
              </div>
            )}
            <p className="text-slate-600 text-xs mt-3 leading-relaxed">
              PNG/BMP を絶対パスで入力 → Enter。<br />
              ja-JP 言語パックが必要: 設定 → 時刻と言語 → 言語 → 日本語
            </p>
          </section>

        </div>
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// モデル診断パネル (YOLO + カスケード 統合)
// ---------------------------------------------------------------------------

function FullDebugPanel({ result }: { result: FullDebugResult }) {
  const PANEL_BOUNDARY_Y = 0.630;

  // ── Model 1 の勝敗判定ロジック ──
  const get = (name: string) =>
    result.detections.filter((d) => d.class_name === name)
      .sort((a, b) => b.confidence - a.confidence)[0];
  const winDet   = get('Win');
  const loseDet  = get('Lose');
  const drawDet  = get('Draw');
  const arrowDet = get('MyArrow');
  const isResultScreen = (winDet?.confidence ?? 0) >= 0.30 || (loseDet?.confidence ?? 0) >= 0.30;
  const arrowY = arrowDet ? (arrowDet.y1 + arrowDet.y2) / 2 : null;
  const arrowDecision = arrowY === null ? null : arrowY < PANEL_BOUNDARY_Y ? 'WIN' : 'LOSE';
  const drawDetected = (drawDet?.confidence ?? 0) >= 0.55;
  const verdict = drawDetected ? 'DRAW' : isResultScreen ? (arrowDecision ?? 'MyArrow未検知') : 'リザルト画面未検知';
  const verdictColor = verdict === 'WIN' ? 'text-green-400' : verdict === 'LOSE' ? 'text-red-400' : verdict === 'DRAW' ? 'text-yellow-400' : 'text-slate-500';

  return (
    <div className="space-y-3 text-xs">
      {/* フレーム + モデル状態 */}
      <div className="flex items-center gap-3 flex-wrap">
        <span className="text-slate-300 font-mono">{result.frame_w}×{result.frame_h}</span>
        <span className={`px-1.5 py-0.5 rounded font-bold border ${result.model1_loaded ? 'bg-green-900/40 text-green-300 border-green-700/60' : 'bg-red-900/40 text-red-400 border-red-700/60'}`}>
          {result.model1_loaded ? '✓ Model 1' : '✗ Model 1'}
        </span>
        <span className={`px-1.5 py-0.5 rounded font-bold border ${result.model2_loaded ? 'bg-teal-900/40 text-teal-300 border-teal-700/60' : 'bg-slate-700/40 text-slate-500 border-slate-600/40'}`}>
          {result.model2_loaded ? '✓ Model 2' : '✗ Model 2'}
        </span>
        {result.error && <span className="text-red-400 break-all">{result.error}</span>}
      </div>

      {/* ── Model 1: 全クラス検出一覧 ── */}
      <div className="bg-slate-800 rounded-lg p-2 space-y-1">
        <p className="text-slate-400 font-semibold mb-1">Model 1 — 全クラス検出 (閾値 0.10)</p>
        {result.detections.length === 0 ? (
          <p className="text-yellow-400">検出なし (信頼度0.10以上の候補がゼロ)</p>
        ) : (
          <div className="space-y-0.5 max-h-40 overflow-y-auto">
            {result.detections
              .sort((a, b) => b.confidence - a.confidence)
              .map((d, i) => (
                <div key={i} className="flex items-center gap-2 bg-slate-900 rounded px-1.5 py-0.5">
                  <span className={`font-bold w-4 text-right ${d.confidence >= 0.60 ? 'text-green-400' : d.confidence >= 0.40 ? 'text-yellow-400' : 'text-slate-500'}`}>
                    {d.confidence >= 0.60 ? '✓' : d.confidence >= 0.40 ? '△' : '✗'}
                  </span>
                  <span className="text-white font-mono w-24 truncate">{d.class_name}</span>
                  <div className="flex-1 bg-slate-700 rounded-full h-1.5">
                    <div
                      className={`h-1.5 rounded-full ${d.confidence >= 0.60 ? 'bg-green-500' : d.confidence >= 0.40 ? 'bg-yellow-500' : 'bg-slate-500'}`}
                      style={{ width: `${Math.round(d.confidence * 100)}%` }}
                    />
                  </div>
                  <span className="text-slate-300 font-mono w-10 text-right">{(d.confidence * 100).toFixed(0)}%</span>
                </div>
              ))}
          </div>
        )}
        <p className="text-slate-600">✓=0.60↑(有効) △=0.40-0.60(閾値未満) ✗=0.10-0.40(弱い)</p>

        {/* 勝敗判定ロジック */}
        <div className="border-t border-slate-700 pt-1 space-y-0.5 mt-1">
          <p className="text-slate-400 mb-0.5">勝敗判定ログ</p>
          {[
            { label: 'Win',     det: winDet,   threshold: 0.30, note: '≥0.30でリザルト画面検知' },
            { label: 'Lose',    det: loseDet,  threshold: 0.30, note: '≥0.30でリザルト画面検知' },
            { label: 'Draw',    det: drawDet,  threshold: 0.55, note: '≥0.55で引き分け確定' },
            { label: 'MyArrow', det: arrowDet, threshold: 0.60, note: arrowY !== null ? `Y=${arrowY.toFixed(3)} → ${arrowDecision}` : '未検知 → ピクセル判定' },
          ].map(({ label, det, threshold, note }) => (
            <div key={label} className="flex items-center gap-1.5 bg-slate-900 rounded px-1.5 py-0.5">
              <span className={`font-bold w-4 text-right ${det && det.confidence >= threshold ? 'text-green-400' : 'text-slate-600'}`}>
                {det && det.confidence >= threshold ? '✓' : '✗'}
              </span>
              <span className="text-slate-300 font-mono w-20">{label}</span>
              <span className="text-slate-400 font-mono w-8 text-right">{det ? `${(det.confidence * 100).toFixed(0)}%` : '—'}</span>
              <span className="text-slate-500 truncate flex-1">{note}</span>
            </div>
          ))}
          <div className="flex items-center gap-1.5 bg-slate-900/80 rounded px-1.5 py-1">
            <span className="text-slate-400">→ 判定:</span>
            <span className={`font-bold ${verdictColor}`}>{verdict}</span>
          </div>
        </div>
      </div>

      {/* YOLO OCR 結果 */}
      {result.ocr && <OcrDebugSection ocr={result.ocr} />}

      {/* ── Model 2: カスケード ── */}
      <div className="bg-slate-800 rounded-lg p-2 space-y-2">
        <div className="flex items-center justify-between">
          <p className="text-teal-400 font-semibold">Model 2 — カスケード (stats)</p>
          <div className="flex gap-2">
            <span className={`px-1.5 py-0.5 rounded text-xs font-bold border ${result.arrow_found ? 'bg-green-900/40 text-green-300 border-green-700/60' : 'bg-yellow-900/40 text-yellow-400 border-yellow-700/60'}`}>
              {result.arrow_found ? '✓ MyArrow 検出' : '✗ MyArrow 未検出'}
            </span>
          </div>
        </div>

        {result.arrow_found && (
          <>
            {/* クロップ情報 */}
            <div className="bg-slate-900 rounded p-1.5 space-y-1">
              <p className="text-slate-400 text-xs">
                クロップ: x={result.crop_x} y={result.crop_y} &nbsp;/&nbsp; {result.crop_w}×{result.crop_h}px
              </p>
              <div className="flex gap-3 flex-wrap">
                {[
                  { label: 'icon_kill',    x: result.kill_anchor_x,    color: 'text-green-400' },
                  { label: 'icon_death',   x: result.death_anchor_x,   color: 'text-red-400' },
                  { label: 'icon_special', x: result.special_anchor_x, color: 'text-purple-400' },
                ].map(({ label, x, color }) => (
                  <span key={label} className="text-slate-500">
                    {label}: <span className={`font-mono ${x !== null ? color : 'text-slate-600'}`}>{x !== null ? x.toFixed(3) : '—'}</span>
                  </span>
                ))}
              </div>
            </div>

            {/* クロップ画像 */}
            {result.crop_image_base64 && (
              <div className="bg-black rounded overflow-hidden">
                <img
                  src={`data:image/png;base64,${result.crop_image_base64}`}
                  className="w-full"
                  style={{ imageRendering: 'pixelated', minHeight: '40px', maxHeight: '120px', objectFit: 'contain' }}
                  alt="cascade crop"
                />
              </div>
            )}

            {/* 検出一覧 */}
            {result.cascade_detections.length === 0 ? (
              <p className="text-yellow-400">検出なし (信頼度0.10以上の候補がゼロ — クロップ画像を確認してください)</p>
            ) : (
              <div>
                <p className="text-slate-400 mb-1">検出 ({result.cascade_detections.length}件, x_center 昇順)</p>
                <div className="space-y-0.5 max-h-48 overflow-y-auto">
                  {result.cascade_detections.map((d, i) => (
                    <CascadeDetRow key={i} index={i} det={d} />
                  ))}
                </div>
              </div>
            )}

            {/* パース結果 */}
            <div className="grid grid-cols-4 gap-1">
              {[
                { label: '塗り', value: result.paint,   color: 'text-cyan-300' },
                { label: 'K',    value: result.kill,    color: 'text-green-300' },
                { label: 'D',    value: result.death,   color: 'text-red-300' },
                { label: 'SP',   value: result.special, color: 'text-purple-300' },
              ].map(({ label, value, color }) => (
                <div key={label} className="bg-slate-900 rounded p-1.5 text-center">
                  <p className="text-slate-500 text-xs">{label}</p>
                  <p className={`font-mono font-bold text-sm ${value !== null ? color : 'text-slate-600'}`}>
                    {value !== null ? value : '—'}
                  </p>
                </div>
              ))}
            </div>
          </>
        )}
      </div>

      {/* ── Model 2: ヘッダーカスケード (モード/ルール/ステージ) ── */}
      <HeaderCascadePanel header={result.header} />
    </div>
  );
}

// ---------------------------------------------------------------------------
// ヘッダーカスケード (モード/ルール/ステージ)
// ---------------------------------------------------------------------------

function HeaderCascadePanel({ header }: { header: FullDebugResult['header'] }) {
  return (
    <div className="bg-slate-800 rounded-lg p-2 space-y-2">
      <p className="text-teal-400 font-semibold">Model 2 — ヘッダーカスケード (モード/ルール/ステージ)</p>

      {/* クロップ情報 */}
      <div className="bg-slate-900 rounded p-1.5 space-y-1">
        <p className="text-slate-400 text-xs">
          クロップ: x={header.crop_x} y={header.crop_y} &nbsp;/&nbsp; {header.crop_w}×{header.crop_h}px
        </p>
        {header.error && <p className="text-red-400 break-all">{header.error}</p>}
      </div>

      {/* クロップ画像 */}
      {header.crop_image_base64 && (
        <div className="bg-black rounded overflow-hidden">
          <img
            src={`data:image/png;base64,${header.crop_image_base64}`}
            className="w-full"
            style={{ imageRendering: 'pixelated', minHeight: '40px', maxHeight: '120px', objectFit: 'contain' }}
            alt="header crop"
          />
        </div>
      )}

      {/* 検出一覧 */}
      {header.detections.length === 0 ? (
        <p className="text-yellow-400">検出なし (信頼度0.10以上の候補がゼロ — クロップ画像を確認してください)</p>
      ) : (
        <div>
          <p className="text-slate-400 mb-1">検出 ({header.detections.length}件, 確信度降順)</p>
          <div className="space-y-0.5 max-h-48 overflow-y-auto">
            {header.detections.map((d, i) => (
              <HeaderDetRow key={i} index={i} det={d} />
            ))}
          </div>
        </div>
      )}

      {/* パース結果 */}
      <div className="grid grid-cols-3 gap-1">
        {[
          { label: 'モード',     value: header.mode,  color: 'text-orange-300' },
          { label: 'ルール',     value: header.rule,  color: 'text-pink-300' },
          { label: 'ステージ',   value: header.stage, color: 'text-cyan-300' },
        ].map(({ label, value, color }) => (
          <div key={label} className="bg-slate-900 rounded p-1.5 text-center">
            <p className="text-slate-500 text-xs">{label}</p>
            <p className={`font-mono font-bold text-sm truncate ${value !== null ? color : 'text-slate-600'}`}>
              {value !== null ? value : '—'}
            </p>
          </div>
        ))}
      </div>
    </div>
  );
}

const HEADER_GROUP_STYLES: Record<string, string> = {
  mode:  'bg-orange-900/60 text-orange-300 border-orange-700/60',
  rule:  'bg-pink-900/60 text-pink-300 border-pink-700/60',
  stage: 'bg-cyan-900/60 text-cyan-300 border-cyan-700/60',
  other: 'bg-slate-700/40 text-slate-500 border-slate-600/40',
};

function headerGroupOf(className: string): string {
  if (className.startsWith('mode_'))  return 'mode';
  if (className.startsWith('rule_'))  return 'rule';
  if (className.startsWith('stage_')) return 'stage';
  return 'other';
}

function HeaderDetRow({ index, det }: { index: number; det: HeaderDebugDetection }) {
  const group = headerGroupOf(det.class_name);
  return (
    <div className="flex items-center gap-1.5 bg-slate-900 rounded px-1.5 py-0.5 text-xs">
      <span className="text-slate-500 font-mono w-5 text-right">{index + 1}</span>
      <span className="text-white font-mono w-28 truncate">{det.class_name}</span>
      <div className="flex-1 bg-slate-700 rounded-full h-1.5">
        <div
          className={`h-1.5 rounded-full ${det.confidence >= 0.60 ? 'bg-teal-500' : det.confidence >= 0.40 ? 'bg-yellow-500' : 'bg-slate-500'}`}
          style={{ width: `${Math.round(det.confidence * 100)}%` }}
        />
      </div>
      <span className="text-slate-300 font-mono w-9 text-right">{(det.confidence * 100).toFixed(0)}%</span>
      <span className="font-mono text-slate-500 w-12 text-right">{det.x_center.toFixed(3)}</span>
      <span className={`px-1 py-px rounded border font-bold w-16 text-center truncate ${HEADER_GROUP_STYLES[group]}`}>
        {group}
      </span>
    </div>
  );
}

// ---------------------------------------------------------------------------
// カスケード検出行
// ---------------------------------------------------------------------------

const GROUP_STYLES: Record<string, string> = {
  paint:          'bg-cyan-900/60 text-cyan-300 border-cyan-700/60',
  kill:           'bg-green-900/60 text-green-300 border-green-700/60',
  death:          'bg-red-900/60 text-red-300 border-red-700/60',
  special:        'bg-purple-900/60 text-purple-300 border-purple-700/60',
  anchor_kill:    'bg-green-700/40 text-green-200 border-green-600/60',
  anchor_death:   'bg-red-700/40 text-red-200 border-red-600/60',
  anchor_special: 'bg-purple-700/40 text-purple-200 border-purple-600/60',
  ignored:        'bg-slate-700/40 text-slate-500 border-slate-600/40',
};

function CascadeDetRow({ index, det }: { index: number; det: CascadeDebugDetection }) {
  return (
    <div className="flex items-center gap-1.5 bg-slate-900 rounded px-1.5 py-0.5 text-xs">
      <span className="text-slate-500 font-mono w-5 text-right">{index + 1}</span>
      <span className="text-white font-mono w-20 truncate">{det.class_name}</span>
      <div className="flex-1 bg-slate-700 rounded-full h-1.5">
        <div
          className={`h-1.5 rounded-full ${det.confidence >= 0.60 ? 'bg-teal-500' : det.confidence >= 0.40 ? 'bg-yellow-500' : 'bg-slate-500'}`}
          style={{ width: `${Math.round(det.confidence * 100)}%` }}
        />
      </div>
      <span className="text-slate-300 font-mono w-9 text-right">{(det.confidence * 100).toFixed(0)}%</span>
      <span className="font-mono text-slate-500 w-12 text-right">{det.x_center.toFixed(3)}</span>
      <span className={`px-1 py-px rounded border font-bold w-20 text-center truncate ${GROUP_STYLES[det.group] ?? 'text-slate-400'}`}>
        {det.group}
      </span>
    </div>
  );
}

// ---------------------------------------------------------------------------
// OCR デバッグセクション
// ---------------------------------------------------------------------------

function OcrDebugSection({ ocr }: { ocr: OcrDebugResult }) {
  const fields: { label: string; field: import('../types').OcrDebugField }[] = [
    { label: 'ルール',     field: ocr.rule    },
    { label: 'ステージ',   field: ocr.stage   },
    { label: 'モード',     field: ocr.mode    },
    { label: 'キル',       field: ocr.kill    },
    { label: 'デス',       field: ocr.death   },
    { label: 'スペシャル', field: ocr.special },
  ];

  return (
    <div className="bg-slate-800 rounded-lg p-2 text-xs space-y-1">
      <div className="flex justify-between items-center mb-1">
        <span className="text-slate-400 font-semibold">YOLO OCR 結果</span>
        {ocr.arrow_y !== null && (
          <span className="text-slate-500 font-mono">MyArrow Y={ocr.arrow_y.toFixed(3)}</span>
        )}
      </div>
      {fields.map(({ label, field }) => (
        <div key={label} className="bg-slate-900 rounded px-1.5 py-0.5">
          <div className="flex items-start gap-2">
            <span className="text-slate-500 w-16 shrink-0">{label}</span>
            <div className="flex-1 min-w-0">
              <p className="text-slate-300 font-mono truncate">
                {field.raw || <span className="text-slate-600">(空)</span>}
              </p>
              {field.normalized ? (
                <p className="text-green-400 font-mono">→ {field.normalized}</p>
              ) : field.raw ? (
                <p className="text-red-400">→ 未マッチ</p>
              ) : (
                <p className="text-slate-600">→ 検出なし</p>
              )}
            </div>
          </div>
        </div>
      ))}
    </div>
  );
}

// ---------------------------------------------------------------------------
// キャプチャ診断セクション
// ---------------------------------------------------------------------------

function CaptureDiagSection({ result }: { result: CaptureDebugResult }) {
  return (
    <div className="space-y-1.5 text-xs">
      <div className="bg-slate-800 rounded-lg p-2 flex justify-between">
        <span className="text-slate-400">フレームサイズ</span>
        <span className="text-white font-mono">{result.frame_w} × {result.frame_h}</span>
      </div>
      <div className="bg-slate-800 rounded-lg p-2 space-y-1">
        <div className="flex justify-between">
          <span className="text-slate-400">Phase 1: バトル開始</span>
          <div className="flex gap-2">
            <span className={result.dark_scroll_found ? 'text-green-400' : 'text-slate-600'}>
              {result.dark_scroll_found ? '✓ 暗巻物' : '✗ 暗巻物'}
            </span>
            <span className={result.battle_start_found ? 'text-green-400' : 'text-slate-500'}>
              {result.battle_start_found ? '✓ 検出' : '✗ 未検出'}
            </span>
          </div>
        </div>
        <pre className="text-slate-300 whitespace-pre-wrap break-all max-h-16 overflow-y-auto bg-slate-900 rounded p-1">
          {result.battle_start_text || '(空)'}
        </pre>
      </div>
      <div className="bg-slate-800 rounded-lg p-2 space-y-1">
        <div className="flex justify-between">
          <span className="text-slate-400">Phase 2: リザルト画面</span>
          <span className={result.win_grey_rows >= 2 && result.lose_grey_rows >= 2 ? 'text-green-400' : 'text-slate-500'}>
            WIN {result.win_grey_rows}/4 | LOSE {result.lose_grey_rows}/4
          </span>
        </div>
        <div className="grid grid-cols-4 gap-1">
          {[
            { label: 'WIN黄', value: result.yellow_win_px,  ok: result.yellow_win_px >= 100 },
            { label: 'LOSE黄', value: result.yellow_lose_px, ok: result.yellow_lose_px >= 100 },
            { label: '重心y', value: result.centroid_y.toFixed(3), ok: true },
            { label: 'spread', value: `${result.y_spread}px`, ok: true },
          ].map(({ label, value, ok }) => (
            <div key={label} className="bg-slate-900 rounded p-1 text-center">
              <p className="text-slate-500">{label}</p>
              <p className={`font-mono font-bold ${ok ? 'text-slate-300' : 'text-slate-600'}`}>{value}</p>
            </div>
          ))}
        </div>
      </div>
      <div className="bg-slate-800 rounded-lg p-2 space-y-1">
        <p className="text-slate-400 font-semibold">ルール / ステージ OCR</p>
        <div className="flex gap-2">
          {[
            { label: 'ルール', raw: result.rule_ocr_text, norm: result.rule_normalized },
            { label: 'ステージ', raw: result.stage_ocr_text, norm: result.stage_normalized },
          ].map(({ label, raw, norm }) => (
            <div key={label} className="flex-1 bg-slate-900 rounded p-1.5">
              <p className="text-slate-500 mb-0.5">{label} (raw)</p>
              <p className="text-white font-mono break-all">{raw || '(空)'}</p>
              <p className={`mt-0.5 ${norm ? 'text-green-400' : 'text-slate-500'}`}>→ {norm ?? '未マッチ'}</p>
            </div>
          ))}
        </div>
      </div>
      <div className={`rounded-lg p-2 break-all ${
        result.detection_summary.includes('✓ WIN') ? 'bg-green-900/40 border border-green-700 text-green-300' :
        result.detection_summary.includes('✓ LOSE') ? 'bg-yellow-900/40 border border-yellow-700 text-yellow-300' :
        result.detection_summary.includes('Phase 1 ✓') ? 'bg-blue-900/40 border border-blue-700 text-blue-300' :
        'bg-slate-800 border border-slate-600 text-slate-400'
      }`}>
        {result.detection_summary}
      </div>
    </div>
  );
}
