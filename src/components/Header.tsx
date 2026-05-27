// IkaVision XP — ヘッダーコンポーネント

import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { CaptureStatusPayload, WindowInfo } from '../types';

interface HeaderProps {
  isCapturing: boolean;
  onCapturingChange: (capturing: boolean) => void;
}

export function Header({ isCapturing, onCapturingChange }: HeaderProps) {
  const [windows, setWindows] = useState<WindowInfo[]>([]);
  const [selectedHwnd, setSelectedHwnd] = useState<number | null>(null);
  const [fps, setFps] = useState(0);
  const [showWindowPicker, setShowWindowPicker] = useState(false);

  // キャプチャ状態イベントを購読
  useEffect(() => {
    const unlisten = listen<CaptureStatusPayload>('capture_status', (e) => {
      onCapturingChange(e.payload.active);
      setFps(e.payload.fps);
    });
    return () => { unlisten.then((fn) => fn()); };
  }, [onCapturingChange]);

  const loadWindows = async () => {
    try {
      const list = await invoke<WindowInfo[]>('list_windows');
      setWindows(list);
    } catch (e) {
      console.error('list_windows failed:', e);
    }
  };

  const handleStartCapture = async () => {
    if (selectedHwnd == null) {
      setShowWindowPicker(true);
      await loadWindows();
      return;
    }
    const win = windows.find((w) => w.hwnd === selectedHwnd);
    if (!win) return;
    try {
      await invoke('start_capture', { windowTitle: win.title });
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
          <h1 className="text-lg font-bold text-white leading-none">IkaVision XP</h1>
          <p className="text-xs text-slate-400 leading-none">Splatoon 3 X-Match Tracker</p>
        </div>
      </div>

      {/* キャプチャ制御 */}
      <div className="flex items-center gap-3">
        {isCapturing && (
          <div className="flex items-center gap-1.5 text-xs text-green-400">
            <span className="w-2 h-2 bg-green-400 rounded-full animate-pulse" />
            キャプチャ中
            {fps > 0 && <span className="text-slate-400">({fps.toFixed(0)} fps)</span>}
          </div>
        )}

        {showWindowPicker && !isCapturing && (
          <select
            className="bg-slate-700 text-white text-sm rounded-lg px-2 py-1 outline-none"
            value={selectedHwnd ?? ''}
            onChange={(e) => setSelectedHwnd(Number(e.target.value))}
          >
            <option value="">ウィンドウを選択</option>
            {windows.map((w) => (
              <option key={w.hwnd} value={w.hwnd}>
                {w.title.slice(0, 40)}
              </option>
            ))}
          </select>
        )}

        {isCapturing ? (
          <button
            className="px-4 py-1.5 bg-red-600 hover:bg-red-500 text-white text-sm font-medium rounded-lg transition-colors"
            onClick={handleStopCapture}
          >
            ⏹ 停止
          </button>
        ) : (
          <button
            className="px-4 py-1.5 bg-indigo-600 hover:bg-indigo-500 text-white text-sm font-medium rounded-lg transition-colors"
            onClick={handleStartCapture}
          >
            ▶ キャプチャ開始
          </button>
        )}
      </div>
    </header>
  );
}
