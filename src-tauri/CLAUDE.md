# バックエンド (src-tauri) ガイド

## モジュール構成

| モジュール | 責務 |
|---|---|
| `capture.rs` | WGC によるフレーム取得 (BGRA8) |
| `capture_loop.rs` | メインループ (5fps、ScreenStateMachine、battle_started / match_detected 発火) |
| `detector.rs` | YOLO Model 1 推論 (BattleStart/Win/Lose/MyArrow 検知、ピクセル判定フォールバック) |
| `cascade.rs` | YOLO Model 2 カスケード推論 (スタッツ領域 digit/icon 検知、グルーピング) |
| `extractor.rs` | BBox/OCR からのデータ抽出 (数値・テキストパース) |
| `ocr.rs` | WinRT OCR ラッパー |
| `preprocess.rs` | 画像前処理 (letterbox, stretch, 2値化等) |
| `db.rs` | Match 構造体生成・正規化ヘルパー |
| `commands.rs` | Tauri コマンド定義 (キャプチャ制御・デバッグ系のみ) |
| `state.rs` / `screen_state.rs` | グローバル状態 / キャプチャ状態機械 |
| `types.rs` | 共通型定義 |

## 勘所

- **YOLO クラス ID**: Model 1 (`yolo_result.onnx`) と Model 2 (`yolo_stats.onnx`) のクラス ID は
  Roboflow の `data.yaml` アルファベット順に対応する。変更時は `docs/spec.md` §3 参照。

- **チューニング定数の場所**:
  - `cascade.rs`: `CROP_HALF_H_RATIO`, `STATS_X_START/END`, `X_DEDUP_TOL`, `HEADER_CROP_*`
  - `detector.rs`: `ARROW_X/Y_START/END`, `PANEL_BOUNDARY_Y`, `DEFAULT_CONF_THRESHOLD`, `MIN_YELLOW_PIXELS`
  - `extractor.rs`: `KILL_COL_X`, `DEATH_COL_X`, `SPEC_COL_X`, `KDA_COL_W`, `KDA_ROW_H`
  - 正規化座標 `[0, 1]` ベースのため解像度非依存。調整は実機の `debug_full` 結果を見ながら行う。

- **デバッグコマンド**: `debug_full`（Model1+Model2 統合、crop画像 base64 付き）と `debug_capture`
  （1フレーム診断）が `OcrDebugPanel.tsx` から呼び出せる。チューニング必須ツール。

- **DB マイグレーション**: スキーマ変更は必ず新規連番ファイル (`src-tauri/migrations/00N_xxx.sql`) を追加。
  **既存ファイルは変更しない**。

## テスト

```bash
cargo test --lib -- --test-threads=1
```

Windows 固有 API を使うテストは `#[cfg(target_os = "windows")]` で除外済み。非 Windows でも実行可能。
