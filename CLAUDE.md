# CLAUDE.md

このファイルはこのリポジトリで開発するための必須ガイドです。

## プロジェクト概要

**InkGraph** は Splatoon 3 の X マッチ対戦結果を自動収集・可視化する Tauri v2 デスクトップアプリ（Windows 専用）。  
画面キャプチャ → YOLO 推論 → データ抽出 → SQLite 保存 → React UI、というパイプラインで動作する。

**詳細な技術仕様は `docs/spec.md` が正（authoritative）**。実装を変更する前に該当セクションを必ず読むこと。

## 環境制約（重要）

- バックエンドは Windows 専用（WinRT OCR / WGC 依存）。**Linux では `cargo build` のフルビルドや `npm run tauri:dev` は不可**。
- Linux/CI で実行可能なのは `cargo test --lib` とフロントエンドの型チェック・lint・ビルドのみ。

## よく使うコマンド

```bash
npm run dev              # Vite 開発サーバー
npm run build            # tsc -b && vite build
npm run lint             # ESLint
npx tsc -b --noEmit      # 型チェックのみ
cd src-tauri && cargo test --lib -- --test-threads=1  # Rust 単体テスト
npm run tauri:dev         # Windows 専用
```

## 開発フロー / Git 規約

- GitHub Flow: `main` は常にリリース可能。Issue → ブランチ → PR → CI → マージ。
- コミットメッセージ形式: `<type>(<scope>): <日本語の説明>` （例: `fix(cascade): X_DEDUP_TOL を縮小`）
- `git push --force` / `git reset --hard` 等の破壊的操作は指示なしに行わない。

## ドキュメントの鮮度

挙動に関わる変更をした場合は **`docs/spec.md` も合わせて更新**すること。  
確認したい場合は `spec-sync` サブエージェントを使う。
