# InkGraph 仕様書

## 1. 概要

InkGraph は Splatoon 3 のXマッチ対戦記録を自動収集・可視化する Tauri v2 デスクトップアプリ（Windows専用）。  
画面キャプチャ → YOLO 推論 → データ抽出 → SQLite 保存 → React UI 表示 というパイプラインで動作する。

---

## 2. システム構成

```
[Splatoon 3 / キャプチャカード出力]
        │ BGRA8 フレーム (WGC)
        ▼
[capture_loop.rs] ── 5 fps ポーリング
        │
        ├─ Model 1 推論 (yolo_result.onnx)
        │      BattleStart / Win / Lose / MyArrow 等を検知
        │
        ├─ BattleStart 検知 ──→ battle_started イベント → フロントエンド
        │
        └─ リザルト検知
               │
               ├─ Model 2 推論 (yolo_stats.onnx) ── カスケード推論
               │      スタッツ領域クロップ → digit / icon 検知
               │      → PlayerStats { paint, kill, death, special }
               │
               └─ match_detected イベント → フロントエンド

[フロントエンド (React + TypeScript)]
        │
        ├─ battle_started → insertMatch('in_progress') → SQLite
        └─ match_detected → updateMatchResult('win'/'lose') → SQLite
```

---

## 3. YOLO モデル

### 3.1 Model 1 — `yolo_result.onnx`

リザルト画面・バトル開始画面全体を認識するモデル。

| Class ID | クラス名 | 用途 |
|---|---|---|
| 0 | `BattleStart` | 「バトルを開始します！」オーバーレイ検知 |
| 1 | `Draw` | 引き分けバナー検知 |
| 2 | `GoldAward` | 金表彰アイコン検知（枚数カウント） |
| 3 | `KillLog` | キルログ表示検知（未使用） |
| 4 | `Lose` | LOSE バナー検知 |
| 5 | `ModeText` | モードテキスト領域検知 (OCR 用) |
| 6 | `MyArrow` | 自プレイヤー行の黄色矢印検知（勝敗判定・スタッツ行特定） |
| 7 | `RuleText` | ルールテキスト領域検知 (OCR 用) |
| 8 | `StageText` | ステージテキスト領域検知 (OCR 用) |
| 9 | `Win` | WIN バナー検知 |

**備考:**
- 現在の学習モデルは nc=7（ID 0〜6）の場合がある。Win/RuleText/StageText が未学習の場合は OCR フォールバックが使われる。
- 信頼度閾値: `DEFAULT_CONF_THRESHOLD = 0.50`
- BattleStart 判定には追加で `confidence >= 0.60` を要求

**勝敗判定ロジック:**
- `Win conf >= 0.30` または `Lose conf >= 0.30` または `MyArrow conf >= 0.40` → リザルト画面と判定
- `MyArrow` y 中心 < `PANEL_BOUNDARY_Y (0.630)` → WIN 側、それ以上 → LOSE 側
- YOLO 未検知時はピクセル判定にフォールバック（グレー行カウント + 黄色矢印カウント）

---

### 3.2 Model 2 — `yolo_stats.onnx`

スタッツ列クロップ画像（塗りポイント〜SP 列）に特化したモデル。  
**Roboflow の Stretch リサイズで学習済み**のため、推論時もストレッチ前処理を使用する。

| Class ID (アルファベット順) | クラス名 | 用途 |
|---|---|---|
| 0 | `digit_0` | 数字 0 |
| 1 | `digit_1` | 数字 1 |
| 2 | `digit_2` | 数字 2 |
| 3 | `digit_3` | 数字 3 |
| 4 | `digit_4` | 数字 4 |
| 5 | `digit_5` | 数字 5 |
| 6 | `digit_6` | 数字 6 |
| 7 | `digit_7` | 数字 7 |
| 8 | `digit_8` | 数字 8 |
| 9 | `digit_9` | 数字 9 |
| 10 | `icon_death` | デスアイコン（グループ境界） |
| 11 | `icon_kill` | キルアイコン（グループ境界） |
| 12 | `icon_special` | スペシャルアイコン（グループ境界） |

**備考:**
- クラス ID は Roboflow の `data.yaml` アルファベット順に対応させること
- `icon_weapon` は現在未学習（将来追加予定）
- 塗りポイントグループは `icon_kill` の左端より左にあるすべての数字

---

## 4. カスケード推論パイプライン

```
[フレーム全体]
    │
    └─ Model 1 → MyArrow BBox (y1, y2, y_center)
                      │
    ┌─────────────────┘
    │ Step 1: クロップ
    │   crop_x = frame_w × 0.45
    │   crop_right = frame_w × 0.86
    │   crop_half_h = (arrow_h_px × CROP_HALF_H_RATIO=0.6).max(15px)
    │   crop_y = y_center - crop_half_h
    │
    ▼
[クロップ画像 (スタッツ列のみ)]
    │
    └─ Model 2 → digit_* + icon_* BBoxes
                      │
    ┌─────────────────┘
    │ Step 2: x_center 昇順ソート
    │
    │ Step 3: アイコン左端で境界特定
    │   kill_lo   = icon_kill.x1
    │   death_lo  = icon_death.x1
    │   special_lo = icon_special.x1
    │
    │ Step 4: グループ分け・重複除去・パース
    │   paint_digits  : cx < kill_lo
    │   kill_digits   : kill_lo <= cx < death_lo
    │   death_digits  : death_lo <= cx < special_lo
    │   special_digits: cx >= special_lo
    │
    │   重複除去: X_DEDUP_TOL = 0.008 以内を同一位置とみなし最高確信度を採用
    │
    └─ PlayerStats { paint, kill, death, special }
```

**座標系:**  
モデル出力座標は `[0, 1]`（クロップ画像幅・高さに対する正規化値）。  
`x_norm = pixel_x_in_crop / crop_w` のため、フレーム解像度が変わっても  
スケール比が一定であれば正規化座標は不変（サイズ非依存）。

---

## 5. キャプチャループ状態機械

```
[起動]
  └─ pending_match_id = None
  └─ battle_started_at = None

  ┌──────────────────────────────────────┐
  │ loop (5 fps)                         │
  │                                      │
  │  [固着チェック]                       │
  │  pending && elapsed >= 30分          │
  │    → pending = None (強制リセット)    │
  │                                      │
  │  if pending == None                  │
  │    BattleStart >= 0.60 ?             │
  │      → pending = new UUID            │
  │      → emit battle_started           │
  │                                      │
  │  else (pending != None)              │
  │    BattleStart再検知? → warn ログ    │
  │    elapsed >= 15s ?                  │
  │      → Win/Lose/Draw/Arrow 判定      │
  │      → 確定したら emit match_detected │
  │      → pending = None               │
  └──────────────────────────────────────┘
```

---

## 6. データベーススキーマ

```sql
CREATE TABLE matches (
    id               TEXT     PRIMARY KEY,           -- UUID
    played_at        DATETIME NOT NULL,              -- RFC3339
    mode             TEXT,                           -- "Xマッチ" / "バンカラマッチ(チャレンジ)" 等
    rule             TEXT,                           -- "ガチエリア" / "ガチヤグラ" 等
    stage            TEXT,                           -- "マテガイ放水路" 等
    weapon           TEXT,                           -- ユーザー手動入力
    result           TEXT NOT NULL                   -- 'win' | 'lose' | 'draw' | 'in_progress'
                     CHECK(result IN ('win','lose','draw','in_progress')),
    kill_count       INTEGER,
    assist_count     INTEGER,
    death_count      INTEGER,
    special_count    INTEGER,
    paint_count      INTEGER,                        -- 塗りポイント (Model 2 で取得)
    xp_after         REAL,                           -- X パワー (将来実装)
    gold_award_count INTEGER,                        -- 金表彰枚数
    tags             TEXT DEFAULT '[]',              -- JSON 配列文字列
    note             TEXT,
    created_at       DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at       DATETIME DEFAULT CURRENT_TIMESTAMP
);
```

### マイグレーション履歴

| Version | ファイル | 内容 |
|---|---|---|
| 1 | `001_initial.sql` | 初期スキーマ (result CHECK は win/lose のみ) |
| 2 | `002_add_mode.sql` | `mode` カラム追加 |
| 3 | `003_add_special_count.sql` | `special_count` カラム追加 |
| 4 | `004_add_special_count.sql` | (重複マイグレーション) |
| 5 | `005_add_gold_award.sql` | `gold_award_count` カラム追加 |
| 6 | `006_add_paint_count.sql` | `paint_count` カラム追加 |
| 7 | `007_fix_result_constraint.sql` | テーブル再作成で CHECK を `in_progress`/`draw` も許可に拡張 |

---

## 7. イベントフロー (Rust → Frontend)

### `battle_started`

- **発火タイミング**: BattleStart クラスを `confidence >= 0.60` で検知
- **Payload**: `MatchDetectedPayload { match_data: Match (result="in_progress"), ocr_confidence: 1.0 }`
- **フロントエンド処理**: `insertMatch` → SQLite 挿入、`setMatches` で UI 追加（amber「試合中」バッジ）

### `match_detected`

- **発火タイミング**: リザルト画面確定（Win/Lose/Draw + MyArrow y 座標）
- **Payload**: `MatchDetectedPayload { match_data: Match (result="win"/"lose"/"draw"), ocr_confidence: 1.0 }`
- **フロントエンド処理**: `updateMatchResult` → SQLite UPDATE（同 ID レコードを上書き）。レコードなければ `insertMatch` フォールバック

### `capture_status`

- **発火タイミング**: キャプチャ開始・停止時
- **Payload**: `CaptureStatusPayload { active, fps, window_title, yolo_loaded }`

---

## 8. フロントエンドアーキテクチャ

```
App.tsx
  └─ useMatches(selectedRule)          // src/hooks/useMatches.ts
       ├─ selectMatches(100, rule)      // 起動時・フィルター変更時 DB ロード
       ├─ listen('battle_started')      // リアルタイム試合開始受信
       └─ listen('match_detected')      // リアルタイムリザルト受信

  └─ MatchList
       └─ MatchCard                     // 結果バッジ / KDA / 武器 / タグ / ノート

src/lib/db.ts
  ├─ insertMatch(m)                     // INSERT
  ├─ updateMatchResult(m)              // UPDATE by id (rowsAffected=0 なら insertMatch)
  ├─ dbUpdateWeapon / Tags / Note      // フィールド個別更新
  └─ selectMatches / selectAllMatches  // SELECT
```

---

## 9. ファイル構成

```
InkGraph/
├─ src/                          # フロントエンド (React + TypeScript)
│   ├─ App.tsx
│   ├─ hooks/
│   │   └─ useMatches.ts
│   ├─ lib/
│   │   └─ db.ts
│   ├─ components/
│   │   ├─ MatchList.tsx
│   │   └─ MatchCard.tsx
│   └─ types/
│       └─ index.ts
│
├─ src-tauri/
│   ├─ src/
│   │   ├─ lib.rs               # エントリポイント・マイグレーション登録
│   │   ├─ capture.rs           # WGC フレームキャプチャ
│   │   ├─ capture_loop.rs      # メインキャプチャループ
│   │   ├─ cascade.rs           # Model 2 カスケード推論
│   │   ├─ commands.rs          # Tauri コマンド定義
│   │   ├─ db.rs                # Rust 側 Match 生成ヘルパー
│   │   ├─ detector.rs          # YoloDetector + ResultDetector
│   │   ├─ extractor.rs         # データ抽出 (OCR / YOLO BBox)
│   │   ├─ ocr.rs               # WinRT OCR ラッパー
│   │   ├─ preprocess.rs        # letterbox / stretch / 白文字抽出
│   │   ├─ screen_state.rs      # 画面状態管理
│   │   ├─ state.rs             # AppState (Tauri グローバル状態)
│   │   └─ types.rs             # 共通型定義
│   └─ migrations/
│       ├─ 001_initial.sql
│       ├─ 002_add_mode.sql
│       ├─ 003_add_special_count.sql
│       ├─ 004_add_special_count.sql
│       ├─ 005_add_gold_award.sql
│       ├─ 006_add_paint_count.sql
│       └─ 007_fix_result_constraint.sql
│
└─ docs/
    └─ spec.md                   # 本ドキュメント
```

---

## 10. 開発・デバッグコマンド

| Tauri コマンド | 説明 |
|---|---|
| `debug_capture` | 1フレームスナップショットで Phase 1/2 診断（OCR・ピクセル判定結果） |
| `debug_full` | Model 1 + Model 2 統合デバッグ（全検出 conf 0.10 以上 + クロップ画像 base64） |
| `list_windows` | キャプチャ可能ウィンドウ一覧取得 |
| `start_capture` | キャプチャループ開始 (hwnd 指定) |
| `stop_capture` | キャプチャループ停止 |
| `test_ocr` | 画像ファイルパス指定で OCR テスト |
