# IkaVision XP 🦑

> スプラトゥーン3 Xマッチ対戦記録トラッカー — WinRT OCR で限界まで自動化

![License](https://img.shields.io/badge/license-MIT-blue.svg)
![Platform](https://img.shields.io/badge/platform-Windows%2010%2F11-0078D4.svg)
![Tauri](https://img.shields.io/badge/Tauri-v2-FFC131.svg)
![Rust](https://img.shields.io/badge/Rust-1.78+-orange.svg)

---

## 概要

IkaVision XP は、スプラトゥーン3のXマッチ対戦結果を **ほぼ操作ゼロ** で記録・可視化する Windows 専用デスクトップアプリです。  
OBS プレビューウィンドウ等を監視し、リザルト画面が出た瞬間に WinRT OCR でスコア・XP・ルール・ステージを自動抽出します。  
プレイヤーが手動で入力するのは「使ったブキ」「反省タグ」「メモ」のみです。

```
キャプチャ → OCR検知(WIN/LOSE) → データ抽出 → DB保存 → グラフ更新
                                           ↑
                              ユーザー: ブキ選択 / タグ / メモだけ
```

---

## 主な機能

| 機能 | 自動/手動 | 説明 |
|------|-----------|------|
| リザルト自動検知 | 🤖 自動 | WIN/LOSE テキストを OCR で検知 |
| スコア抽出 | 🤖 自動 | キル・アシスト・デス・XP を数値化 |
| ルール / ステージ認識 | 🤖 自動 | 2値化前処理付きで動く背景にも対応 |
| ブキ選択 | 👤 手動 (1クリック) | 前回ブキを自動表示、最近使用リスト付き |
| 反省タグ | 👤 手動 (1クリック) | 定型タグをワンタップで付与 |
| 振り返りメモ | 👤 手動 | フリーテキスト |
| XP 推移グラフ | 📊 自動 | ルール別フィルタ付き Recharts グラフ |

---

## 技術スタック

```
┌─────────────────────────────────────────┐
│  Frontend  React 18 + TypeScript + Vite  │
│            Tailwind CSS / Recharts        │
├─────────────────────────────────────────┤
│  Bridge    Tauri v2 IPC (commands/events)│
├─────────────────────────────────────────┤
│  Backend   Rust 2021 edition              │
│  OCR       windows-rs (WinRT OCR)         │
│  Capture   Desktop Duplication API        │
│  DB        SQLite (tauri-plugin-sql)      │
└─────────────────────────────────────────┘
```

---

## 動作要件

- **OS:** Windows 10 (1903+) / Windows 11
- **必須機能:** Windows.Media.Ocr (標準搭載)
- **推奨:** OBS Studio (キャプチャソースとして使用)
- Rust 1.78+ / Node.js 20+

---

## セットアップ

```bash
# リポジトリをクローン
git clone https://github.com/qnaiv/InkGraph.git
cd InkGraph

# 依存関係インストール
npm install

# 開発サーバー起動 (Windows のみ)
npm run tauri dev

# リリースビルド
npm run tauri build
```

> **注意:** Windows 以外の OS ではビルドできません（WinRT API 依存のため）。

---

## プロジェクト構成

```
InkGraph/
├── src/                    # React フロントエンド
│   ├── components/
│   │   ├── XpChart.tsx     # XP 推移グラフ
│   │   ├── MatchList.tsx   # 直近試合リスト
│   │   ├── WeaponPicker.tsx# ブキ選択 UI
│   │   └── TagInput.tsx    # 反省タグ UI
│   ├── hooks/
│   │   └── useMatches.ts   # 試合データ管理
│   └── App.tsx
├── src-tauri/
│   ├── src/
│   │   ├── main.rs         # Tauri エントリポイント
│   │   ├── lib.rs
│   │   ├── capture.rs      # 画面キャプチャ (DDA)
│   │   ├── ocr.rs          # WinRT OCR エンジン
│   │   ├── detector.rs     # リザルト検知ロジック
│   │   ├── extractor.rs    # データ抽出 (クロップ + OCR)
│   │   └── db.rs           # SQLite 操作
│   └── Cargo.toml
├── docs/spec.md            # 詳細仕様書
└── README.md
```

---

## 開発フロー

このプロジェクトは **GitHub Flow** で開発します。

- `main` ブランチは常にリリース可能な状態を維持
- 機能追加・バグ修正は必ず Issue を作成してから着手
- Pull Request にはレビューを必須とする
- CI が通らない PR はマージ不可

詳細は [docs/spec.md](./docs/spec.md) を参照してください。

---

## ライセンス

MIT © 2026 qnaiv
