// InkGraph — ヘッダーコンポーネント

import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import type { CaptureStatusPayload, WindowInfo } from '../types';

interface HeaderProps {
  isCapturing: boolean;
  onCapturingChange: (capturing: boolean) => void;
}

export function Header({ isCapturing, onCapturingChange }: HeaderProps) {
  const [windows, setWindows]           = useState<WindowInfo[]>([]);
  const [selectedHwnd, setSelectedHwnd] = useState<number | null>(null);
  const [fps, setFps]                   = useState(0);
  const [yoloLoaded, setYoloLoaded]     = useState<boolean | null>(null);
  const [loadError, setLoadError]       = useState<string | null>(null);

  // キャプチャ状態イベントを購読
  useEffect(() => {
    const unlisten = listen<CaptureStatusPayload>('capture_status', (e) => {
      onCapturingChange(e.payload.active);
      setFps(e.payload.fps);
      if (e.payload.active) {
        setYoloLoaded(e.payload.yolo_loaded);
      } else {
        setYoloLoaded(null);
      }
    });
    return () => { unlisten.then((fn) => fn()); };
  }, [onCapturingChange]);

  // ドロップダウンにフォーカスが当たったときウィンドウ一覧を更新
  const loadWindows = async () => {
    setLoadError(null);
    try {
      const list = await invoke<WindowInfo[]>('list_windows');
      setWindows(list);
    } catch (e) {
      setLoadError(String(e));
    }
  };

  const handleStartCapture = async () => {
    if (selectedHwnd == null) return;
    try {
      await invoke('start_capture', { hwnd: selectedHwnd });
      onCapturingChange(true);
    } catch (e) {
      console.error('start_capture failed:', e);
    }
  };

  const handleStopCapture = async () => {
    try {
      await invoke('stop_capture');
      onCapturingChange(false);
    } catch (e) {
      console.error('stop_capture failed:', e);
    }
  };

  return (
    <header className="flex items-center justify-between px-6 py-3 bg-slate-900 border-b border-slate-700">
      {/* ロゴ */}
      <div className="flex items-center gap-3">
        <span className="text-2xl">🦑</span>
        <div>
          <h1 className="text-lg font-bold text-white leading-none">InkGraph</h1>
          <p className="text-xs text-slate-400 leading-none">Splatoon 3 X-Match Tracker</p>
        </div>
      </div>

      {/* キャプチャ制御 */}
      <div className="flex items-center gap-3">
        {isCapturing && (
          <div className="flex items-center gap-2 text-xs text-green-400">
            <span className="w-2 h-2 bg-green-400 rounded-full animate-pulse" />
            キャプチャ中
            {fps > 0 && <span className="text-slate-400">({fps.toFixed(0)} fps)</span>}
            {yoloLoaded !== null && (
              <span className={`px-1.5 py-0.5 rounded font-bold tracking-wide ${
                yoloLoaded
                  ? 'bg-violet-600/30 text-violet-300 border border-violet-500/50'
                  : 'bg-slate-700/60 text-slate-400 border border-slate-600/50'
              }`}>
                {yoloLoaded ? 'YOLO' : 'Pixel'}
              </span>
            )}
          </div>
        )}

        {/* ウィンドウ選択 + キャプチャ開始 (キャプチャ中は非表示) */}
        {!isCapturing && (
          <div className="flex items-center gap-2">
            <div className="flex flex-col">
              <select
                className="bg-slate-700 text-white text-sm rounded-lg px-2 py-1.5 outline-none cursor-pointer"
                value={selectedHwnd ?? ''}
                onFocus={loadWindows}
                onChange={(e) => setSelectedHwnd(e.target.value ? Number(e.target.value) : null)}
              >
                <option value="">ウィンドウを選択...</option>
                {windows.map((w) => (
                  <option key={w.hwnd} value={w.hwnd}>
                    {w.title.slice(0, 50)}
                  </option>
                ))}
              </select>
              {loadError && (
                <p className="text-xs text-red-400 mt-0.5">{loadError}</p>
              )}
            </div>

            <button
              className={`px-4 py-1.5 text-white text-sm font-medium rounded-lg transition-colors
                ${selectedHwnd == null
                  ? 'bg-indigo-600/40 cursor-not-allowed'
                  : 'bg-indigo-600 hover:bg-indigo-500 cursor-pointer'}`}
              disabled={selectedHwnd == null}
              onClick={handleStartCapture}
            >
              ▶ キャプチャ開始
            </button>
          </div>
        )}

        {isCapturing && (
          <button
            className="px-4 py-1.5 bg-red-600 hover:bg-red-500 text-white text-sm font-medium rounded-lg transition-colors"
            onClick={handleStopCapture}
          >
            ⏹ 停止
          </button>
        )}
      </div>
    </header>
  );
}
