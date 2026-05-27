import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
import tailwindcss from '@tailwindcss/vite';

// https://vite.dev/config/
export default defineConfig({
  plugins: [react(), tailwindcss()],
  // Tauri が使用するポート
  server: {
    port: 1420,
    strictPort: true,
  },
  // Tauri の開発環境向け設定
  envPrefix: ['VITE_', 'TAURI_'],
  build: {
    // Tauri は ES Modules を使用
    target: ['es2021', 'chrome100', 'safari13'],
    minify: !process.env.TAURI_DEBUG ? 'esbuild' : false,
    sourcemap: !!process.env.TAURI_DEBUG,
  },
});
