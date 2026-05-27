# IkaVision XP — 全体設計書

> Version 0.1.0 | 2026-05-27

---

## 目次

1. [アーキテクチャ概要](#1-アーキテクチャ概要)
2. [データフロー](#2-データフロー)
3. [バックエンド設計 (Rust)](#3-バックエンド設計-rust)
4. [データベース設計 (SQLite)](#4-データベース設計-sqlite)
5. [フロントエンド設計 (React)](#5-フロントエンド設計-react)
6. [Tauri IPC 設計](#6-tauri-ipc-設計)
7. [OCR パイプライン詳細](#7-ocr-パイプライン詳細)
8. [開発ロードマップ](#8-開発ロードマップ)
9. [ディレクトリ構成](#9-ディレクトリ構成)

---

## 1. アーキテクチャ概要

```
┌──────────────────────────────────────────────────────────────────┐
│                        Windows Desktop                            │
│                                                                    │
│  ┌──────────────┐  Desktop Duplication  ┌─────────────────────┐  │
│  │  OBS / Game  │ ────────────────────▶ │   Capture Thread    │  │
│  │  Window      │                       │   (DDA, 30fps poll) │  │
│  └──────────────┘                       └────────┬────────────┘  │
│                                                   │ BGRA frame    │
│                                          ┌────────▼────────────┐  │
│                                          │   Detector (OCR)    │  │
│                                          │   WIN/LOSE 検知     │  │
│                                          └────────┬────────────┘  │
│                                                   │ 検知シグナル  │
│                                          ┌────────▼────────────┐  │
│                                          │   Extractor         │  │
│                                          │   クロップ + OCR    │  │
│                                          │   数値パース         │  │
│                                          └────────┬────────────┘  │
│                                                   │ MatchData     │
│                                          ┌────────▼────────────┐  │
│                                          │   DB Layer (SQLite) │  │
│                                          └────────┬────────────┘  │
│                                                   │ tauri::emit   │
│  ┌────────────────────────────────────┐           │               │
│  │   React Frontend (WebView2)        │◀──────────┘               │
│  │   - XP グラフ (Recharts)           │                           │
│  │   - 試合リスト                      │                           │
│  │   - ブキ / タグ / メモ入力          │                           │
│  └────────────────────────────────────┘                           │
└──────────────────────────────────────────────────────────────────┘
```

---

## 2. データフロー

### 2-1. 自動記録フロー

```
[キャプチャスレッド]
  └─ 30fps で対象ウィンドウのフレームを取得 (DDA)
       └─ [検知モジュール] WIN/LOSE ROI を OCR
            └─ テキスト一致? No → 次フレームへ
                          Yes → [抽出モジュール]
                                  ├─ played_at  = システム日時
                                  ├─ result     = "win" | "lose"
                                  ├─ kill_count = ROI1 OCR → 数値パース
                                  ├─ death_count= ROI2 OCR → 数値パース
                                  ├─ xp_after   = ROI3 OCR → f64 パース
                                  ├─ rule       = ROI4 OCR → テキスト正規化
                                  └─ stage      = ROI5 OCR → テキスト正規化
                                  ↓
                               DB INSERT → tauri::emit("match_detected", payload)
```

### 2-2. 手動補完フロー

```
フロントエンド (イベント受信)
  └─ match_detected イベント → 試合リスト先頭に追加 (weapon/tags/note = null)
       └─ ユーザー操作
            ├─ ブキ選択 → invoke("update_weapon", {id, weapon})
            ├─ タグ付与 → invoke("update_tags",   {id, tags})
            └─ メモ入力 → invoke("update_note",   {id, note})
                          ↓
                       DB UPDATE
```

---

## 3. バックエンド設計 (Rust)

### 3-1. モジュール構成

| モジュール | ファイル | 責務 |
|-----------|---------|------|
| `capture` | `capture.rs` | Desktop Duplication API でフレーム取得 |
| `ocr` | `ocr.rs` | WinRT OCR ラッパー。BGRA→SoftwareBitmap 変換、テキスト抽出 |
| `detector` | `detector.rs` | WIN/LOSE ROI 検知、デバウンス (同一試合の重複検知防止) |
| `extractor` | `extractor.rs` | 各 ROI クロップ → OCR → 数値/テキスト パース |
| `db` | `db.rs` | SQLite CRUD 操作、マイグレーション |
| `commands` | `commands.rs` | Tauri コマンド定義 |
| `state` | `state.rs` | アプリグローバル状態 (AppState) |

### 3-2. 主要な型定義

```rust
/// DB に保存する試合データ
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Match {
    pub id: String,           // UUID v4
    pub played_at: String,    // ISO 8601
    pub rule: Option<String>,
    pub stage: Option<String>,
    pub weapon: Option<String>,
    pub result: String,       // "win" | "lose"
    pub kill_count: Option<i64>,
    pub death_count: Option<i64>,
    pub xp_after: Option<f64>,
    pub tags: Option<String>, // JSON 配列文字列
    pub note: Option<String>,
}

/// OCR 検知イベントのペイロード
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchDetectedPayload {
    pub match_data: Match,
    pub ocr_confidence: f32,  // デバッグ用信頼スコア
}
```

### 3-3. キャプチャ & OCR 処理方針

#### Desktop Duplication API
- `windows::Graphics::Capture::*` を使用 (DDA より高レイヤー、Windows 10 1803+)
- `GraphicsCaptureItem` でウィンドウを指定
- `Direct3D11CaptureFramePool` で非同期フレーム取得
- フレームを BGRA8 テクスチャとして CPU にコピー

#### WinRT OCR
- `windows::Media::Ocr::OcrEngine`
- `SoftwareBitmap` (Bgra8 形式) を入力
- 日本語言語パック (`ja-JP`) を優先、フォールバックで `en-US`
- ROI ごとに `SoftwareBitmap::CreateCopyWithAlphaMode` でクロップ

#### 画像前処理 (ルール/ステージ認識向け)
```
1. グレースケール変換
2. 固定閾値 or Otsu 法で2値化
3. モルフォロジー演算 (膨張) でノイズ除去
4. OcrEngine に渡す
```

### 3-4. デバウンス戦略

同一試合のリザルト画面が複数フレームにわたって検知されることを防ぐ：

```rust
struct DetectorState {
    last_detected_at: Option<Instant>,
    cooldown: Duration,  // デフォルト 30 秒
}
```

---

## 4. データベース設計 (SQLite)

### 4-1. テーブル: `matches`

```sql
CREATE TABLE IF NOT EXISTS matches (
    id           TEXT    PRIMARY KEY,
    played_at    DATETIME NOT NULL,
    rule         TEXT,
    stage        TEXT,
    weapon       TEXT,
    result       TEXT    NOT NULL CHECK(result IN ('win', 'lose')),
    kill_count   INTEGER,
    death_count  INTEGER,
    xp_after     REAL,
    tags         TEXT    DEFAULT '[]',   -- JSON 配列
    note         TEXT,
    created_at   DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at   DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- XP グラフ用インデックス
CREATE INDEX IF NOT EXISTS idx_matches_played_at ON matches(played_at);
CREATE INDEX IF NOT EXISTS idx_matches_rule      ON matches(rule);
```

### 4-2. クエリパターン

```sql
-- XP 推移 (ルール別)
SELECT played_at, xp_after, result
FROM matches
WHERE rule = ?
ORDER BY played_at ASC;

-- 直近 N 件
SELECT * FROM matches ORDER BY played_at DESC LIMIT ?;

-- 勝率集計
SELECT
    rule,
    COUNT(*) as total,
    SUM(CASE WHEN result = 'win' THEN 1 ELSE 0 END) as wins
FROM matches
GROUP BY rule;
```

---

## 5. フロントエンド設計 (React)

### 5-1. レイアウト

```
┌─────────────────────────────────────────────────────────────────┐
│ Header: IkaVision XP  [キャプチャ開始▶]  [設定⚙]               │
├──────────────────────────────┬──────────────────────────────────┤
│  Main: XP 推移グラフ          │  Sidebar                         │
│                               │  ┌────────────────────────────┐ │
│  [ガチエリア][ガチヤグラ]      │  │ 直近の試合 (スクロール)    │ │
│  [ガチホコ][ガチアサリ]       │  │                             │ │
│                               │  │ ┌──────────────────────┐   │ │
│  ~~~~~~~~~~~~~~~~~~~~         │  │ │ 2026/05/27 23:14      │   │ │
│   ~~  ~~~~ ~~                 │  │ │ ガチエリア / マテガイ  │   │ │
│       ~~~~    ~~~~            │  │ │ 🟢 WIN  K:5 D:2       │   │ │
│                               │  │ │ XP: 2341.5 → ?        │   │ │
│                               │  │ │ ブキ: [未選択 ▼]       │   │ │
│                               │  │ │ タグ: [初動デス][打開] │   │ │
│                               │  │ │ メモ: ____________     │   │ │
│                               │  │ └──────────────────────┘   │ │
│                               │  └────────────────────────────┘ │
└──────────────────────────────┴──────────────────────────────────┘
```

### 5-2. コンポーネント階層

```
App
├── Header
│   ├── CaptureToggle      # キャプチャ開始/停止
│   └── SettingsButton
├── MainPanel
│   └── XpChart            # Recharts LineChart
│       └── RuleFilter     # タブ式フィルタ
└── Sidebar
    └── MatchList
        └── MatchCard      # 1試合カード
            ├── MatchSummary   # 自動入力データ表示
            ├── WeaponPicker   # ブキ選択
            ├── TagInput       # 反省タグ
            └── NoteInput      # メモ
```

### 5-3. 状態管理

- **ローカル状態:** React `useState` / `useReducer`
- **サーバー状態:** Tauri `invoke` + `listen` でリアクティブ同期
- **グローバル共有:** `useContext` (試合リスト、ウェポンリスト)

```typescript
// 主要な型
interface Match {
  id: string;
  played_at: string;
  rule: string | null;
  stage: string | null;
  weapon: string | null;
  result: 'win' | 'lose';
  kill_count: number | null;
  death_count: number | null;
  xp_after: number | null;
  tags: string[];
  note: string | null;
}

interface AppState {
  matches: Match[];
  isCapturing: boolean;
  recentWeapons: string[];
}
```

### 5-4. Tauri イベント購読

```typescript
// リザルト自動検知
const unlisten = await listen<MatchDetectedPayload>(
  'match_detected',
  (event) => {
    dispatch({ type: 'PREPEND_MATCH', payload: event.payload.match_data });
  }
);
```

---

## 6. Tauri IPC 設計

### 6-1. コマンド一覧

| コマンド名 | 引数 | 戻り値 | 説明 |
|-----------|------|--------|------|
| `start_capture` | `{ window_title: string }` | `()` | キャプチャ開始 |
| `stop_capture` | - | `()` | キャプチャ停止 |
| `list_windows` | - | `Vec<WindowInfo>` | キャプチャ可能なウィンドウ一覧 |
| `get_matches` | `{ limit: u32, rule?: string }` | `Vec<Match>` | 試合一覧取得 |
| `update_weapon` | `{ id: string, weapon: string }` | `()` | ブキ更新 |
| `update_tags` | `{ id: string, tags: string[] }` | `()` | タグ更新 |
| `update_note` | `{ id: string, note: string }` | `()` | メモ更新 |
| `test_ocr` | `{ image_path: string }` | `OcrResult` | OCR テスト |

### 6-2. イベント一覧

| イベント名 | ペイロード | 方向 | 説明 |
|-----------|----------|------|------|
| `match_detected` | `MatchDetectedPayload` | Rust → React | リザルト検知 |
| `capture_status` | `{ active: bool, fps: f32 }` | Rust → React | キャプチャ状態 |
| `ocr_debug` | `{ region: string, text: string }` | Rust → React | デバッグ用OCR結果 |

---

## 7. OCR パイプライン詳細

### 7-1. ROI (Region of Interest) 定義

※ 1920×1080 基準。実際のキャプチャ解像度に応じてスケーリング。

```
┌─────────────────────────────────────┐  1920×1080
│                                     │
│         ┌─────────────┐             │
│         │  WIN / LOSE │  ROI-A      │ y:400-500, x:760-1160
│         └─────────────┘             │
│                                     │
│  ┌──────┐  ┌──────┐  ┌──────┐      │
│  │ Kill │  │ Asst │  │Death │      │
│  │ ROI-B│  │ ROI-C│  │ROI-D │      │ y:600-680, 各x範囲
│  └──────┘  └──────┘  └──────┘      │
│                                     │
│  ┌─────────┐  ┌──────────────────┐  │
│  │ XP ROI-E│  │ Rule/Stage ROI-F │  │ y:900-980
│  └─────────┘  └──────────────────┘  │
└─────────────────────────────────────┘
```

### 7-2. テキスト正規化

```rust
// ルール名の正規化 (OCR 誤認識対策)
fn normalize_rule(raw: &str) -> Option<String> {
    let candidates = [
        ("ガチエリア", &["ガチエリア", "エリア", "AREA"]),
        ("ガチヤグラ", &["ガチヤグラ", "ヤグラ", "TOWER"]),
        ("ガチホコ", &["ガチホコ", "ホコ", "RAINMAKER"]),
        ("ガチアサリ", &["ガチアサリ", "アサリ", "CLAM"]),
    ];
    // 部分一致 or 編集距離で最近傍を返す
}
```

---

## 8. 開発ロードマップ

### Phase 1 — 足場固め (Issue #1 〜 #4)
- [x] Issue #1: プロジェクト初期化 (Tauri + React + SQLite)
- [ ] Issue #2: WinRT OCR 基盤実装
- [ ] Issue #3: Desktop Duplication / WGC キャプチャ実装
- [ ] Issue #4: DB マイグレーション & CRUD コマンド

### Phase 2 — コア自動化 (Issue #5 〜 #7)
- [ ] Issue #5: リザルト検知ロジック (WIN/LOSE OCR)
- [ ] Issue #6: データ抽出 (スコア・XP・ルール・ステージ)
- [ ] Issue #7: 自動検知 → DB 保存 → フロントエンド通知

### Phase 3 — UI 実装 (Issue #8 〜 #11)
- [ ] Issue #8: ダッシュボードレイアウト & Tailwind 設定
- [ ] Issue #9: XP 推移グラフ (Recharts)
- [ ] Issue #10: ブキ選択 UI
- [ ] Issue #11: 反省タグ & メモ UI

### Phase 4 — 仕上げ (Issue #12 〜)
- [ ] Issue #12: キャプチャ対象ウィンドウ選択 UI
- [ ] Issue #13: 設定画面 (ROI キャリブレーション)
- [ ] Issue #14: インストーラー (NSIS) 作成

---

## 9. ディレクトリ構成

```
InkGraph/
├── .github/
│   └── workflows/
│       └── ci.yml           # Windows CI (cargo test + vitest)
├── src/                     # React フロントエンド
│   ├── assets/
│   │   └── weapons.ts       # ブキ一覧データ
│   ├── components/
│   │   ├── Header.tsx
│   │   ├── XpChart.tsx
│   │   ├── MatchList.tsx
│   │   ├── MatchCard.tsx
│   │   ├── WeaponPicker.tsx
│   │   ├── TagInput.tsx
│   │   └── NoteInput.tsx
│   ├── hooks/
│   │   ├── useMatches.ts
│   │   └── useTauriEvents.ts
│   ├── types/
│   │   └── index.ts
│   ├── App.tsx
│   └── main.tsx
├── src-tauri/
│   ├── src/
│   │   ├── main.rs
│   │   ├── lib.rs
│   │   ├── capture.rs       # WGC キャプチャ
│   │   ├── ocr.rs           # WinRT OCR
│   │   ├── detector.rs      # WIN/LOSE 検知
│   │   ├── extractor.rs     # データ抽出
│   │   ├── db.rs            # SQLite CRUD
│   │   ├── commands.rs      # Tauri コマンド
│   │   └── state.rs         # AppState
│   ├── migrations/
│   │   └── 001_initial.sql
│   ├── Cargo.toml
│   └── tauri.conf.json
├── ARCHITECTURE.md
└── README.md
```
