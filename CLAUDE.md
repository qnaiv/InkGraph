# CLAUDE.md

このファイルは、Claude（および引き継ぐ人間の開発者）がこのリポジトリで効率よく開発するためのガイドです。

## プロジェクト概要

**InkGraph (IkaVision XP)** は Splatoon 3 の X マッチ対戦結果を自動収集・可視化する
Tauri v2 デスクトップアプリ（Windows 専用）。画面キャプチャ → YOLO 推論 (+ WinRT OCR フォールバック)
→ データ抽出 → SQLite 保存 → React UI 表示、というパイプラインで動作する。

**詳細な技術仕様は [`docs/spec.md`](./docs/spec.md) が正（authoritative）**。
YOLO モデルのクラス定義、カスケード推論パイプライン、状態機械、DB スキーマ、IPC イベント/コマンド一覧など
実装の詳細はすべてそこにまとまっている。コードを変更する前に該当セクションを読むこと。

## 開発環境の制約（重要）

- **バックエンドは Windows 専用**: WinRT OCR (`windows-rs`) と Windows Graphics Capture (WGC) に依存している。
  **Linux 環境（このリモート実行環境を含む）では `cargo build` のフルビルドや `npm run tauri:dev` の実行はできない。**
- Linux/CI (Ubuntu) 上で実行できるのは `cargo test --lib`（クロスプラットフォームな単体テスト）と
  フロントエンドの型チェック・lint・ビルドのみ。実機動作確認は Windows 環境でのみ可能。
- 動作要件: Windows 10 (1903+) / Windows 11、Rust 1.78+、Node.js 20+。

## よく使うコマンド

```bash
# フロントエンド
npm run dev              # Vite 開発サーバー
npm run build            # tsc -b && vite build
npm run lint             # ESLint
npx tsc -b --noEmit      # 型チェックのみ

# バックエンド (Rust, src-tauri/ 配下)
cd src-tauri && cargo test --lib -- --test-threads=1   # 単体テスト（非Windowsでも実行可）

# Tauri (Windows 専用 — Linux では動作しない)
npm run tauri:dev
npm run tauri:build
```

CI (`.github/workflows/ci.yml`) では `rust-test`（Ubuntu, cargo test）、
`frontend-check`（Ubuntu, tsc + eslint）、Windows ビルドチェック（PR 時のみ）が走る。

## コードレイアウト早見表

### バックエンド (`src-tauri/src/`)

| モジュール | 責務 |
|---|---|
| `capture.rs` | WGC によるフレーム取得 (BGRA8) |
| `capture_loop.rs` | メインループ (5fps ポーリング、状態機械、`battle_started`/`match_detected` 発火) |
| `detector.rs` | YOLO Model 1 推論 (BattleStart/Win/Lose/MyArrow 等の検知、ピクセル判定フォールバック) |
| `cascade.rs` | YOLO Model 2 カスケード推論 (スタッツ領域 digit/icon 検知、グルーピング) |
| `extractor.rs` | BBox/OCR からのデータ抽出 (数値・テキストパース) |
| `ocr.rs` | WinRT OCR ラッパー |
| `preprocess.rs` | 画像前処理 (letterbox, stretch, 2値化等) |
| `db.rs` | Match 構造体生成・正規化ヘルパー |
| `commands.rs` | Tauri コマンド定義 (キャプチャ制御・デバッグ系のみ) |
| `state.rs` / `screen_state.rs` | グローバル状態 / キャプチャ状態機械 |
| `types.rs` | 共通型定義 |

### フロントエンド (`src/`)

| 場所 | 内容 |
|---|---|
| `App.tsx` | ルート。タブ切替 (`graph` / `analysis` / `history`) |
| `hooks/useMatches.ts` | 試合データの状態管理 + Tauri イベント購読 |
| `lib/db.ts` | **DB アクセス層**。`tauri-plugin-sql` 経由で直接 SQL を発行（CRUD 用 Tauri コマンドは存在しない） |
| `components/` | `XpChart` / `MatchList` / `MatchCard` / `AnalysisPanel` / `MatchHistoryPage` /
  `ManualEntryModal` / `WeaponPicker` / `TagInput` / `OcrDebugPanel` 等 |
| `types/index.ts` | TS 型定義 (Match, Rule 等) |

## 開発フロー / Git 規約

- **GitHub Flow** で開発する（README 参照）: `main` は常にリリース可能な状態を維持し、
  機能追加・修正は Issue → ブランチ → PR → レビュー → CI 通過 → マージ、の流れを基本とする。
- コミットメッセージは `<type>(<scope>): <日本語の説明>` 形式が主流（例: `fix(cascade): X_DEDUP_TOL を 0.015→0.008 に縮小して隣接桁の誤統合を修正`、
  `feat: ...`、`docs: ...`、`fix(frontend): ...`）。スコープには変更したモジュール名を入れる。
- 指定がない限り、自分の判断で `git push --force` や `git reset --hard` 等の破壊的操作は行わない。

## このプロジェクト特有の勘所

- **YOLO クラス ID の対応**: Model 1 (`yolo_result.onnx`) と Model 2 (`yolo_stats.onnx`) のクラス ID は
  Roboflow の `data.yaml` のアルファベット順に対応する。クラス定義を変更する際は `docs/spec.md` §3 を参照し、
  実装 (`detector.rs` / `cascade.rs` の `YoloClass` 等) とズレないよう注意する。
- **チューニング定数が散在している**: 検出領域・閾値はハードコードされた定数として各モジュールに分散している。
  代表例:
  - `cascade.rs`: `CROP_HALF_H_RATIO`, `CROP_HALF_H_MIN`, `STATS_X_START/END`, `X_DEDUP_TOL`,
    `HEADER_CROP_X_START/END`, `HEADER_CROP_Y_START/END`
  - `detector.rs`: `ARROW_X_START/END`, `ARROW_Y_START/END`, `PANEL_BOUNDARY_Y`,
    `DEFAULT_CONF_THRESHOLD`, `DEFAULT_IOU_THRESHOLD`, `MIN_YELLOW_PIXELS` など
  - `extractor.rs`: `KILL_COL_X`, `DEATH_COL_X`, `SPEC_COL_X`, `KDA_COL_W`, `KDA_ROW_H`
  - これらは正規化座標 `[0, 1]` ベースなのでフレーム解像度に依存しない設計だが、
    クロップ範囲やアイコン境界の調整は実機の検出結果を見ながら試行錯誤するのが基本。
- **デバッグコマンドを活用する**: `debug_full`（Model1+Model2 統合デバッグ、crop画像 base64 付き）と
  `debug_capture`（1フレーム診断）は、検出精度のチューニング作業で必須。`OcrDebugPanel.tsx` から呼び出せる。
- **DB マイグレーション**: スキーマ変更時は必ず新しい連番マイグレーションファイル
  (`src-tauri/migrations/00N_xxx.sql`) を追加すること。既存ファイルは変更しない
  （`004_add_special_count.sql` が `003` と重複している例があるが、これも「既存マイグレーションを変更しない」
  運用の結果。重複を解消したい場合も新しいマイグレーションで対応する）。
- **DB アクセスは直接 SQL**: 試合データの CRUD はフロントエンドの `src/lib/db.ts` から
  `tauri-plugin-sql` 経由で直接 SQL を発行する設計。新しい CRUD 操作を追加する際も、
  Tauri コマンドではなく `db.ts` に関数を追加するのが既存の流儀に沿う。

## テスト方針

- **Rust**: `#[cfg(test)]` によるインラインテスト中心。`cascade.rs` / `detector.rs` / `extractor.rs` /
  `preprocess.rs` / `ocr.rs` / `screen_state.rs` / `db.rs` に存在。
  `cargo test --lib -- --test-threads=1` で実行（Windows 固有 API を使うテストは `#[cfg(target_os = "windows")]` で除外されているため非 Windows でも実行可能）。
- **フロントエンド**: 専用のテストフレームワークは無く、型チェック (`tsc -b --noEmit`) と ESLint (`npm run lint`) で品質を担保している。

## ドキュメントの鮮度を保つ

YOLO モデル仕様・カスケードパイプライン・DB スキーマ・IPC イベント/コマンドなど
**挙動に関わる変更を行った場合は `docs/spec.md` も合わせて更新すること**。
更新漏れがないか確認したい場合は `spec-sync` サブエージェント（`.claude/agents/spec-sync.md`）を使うとよい。
