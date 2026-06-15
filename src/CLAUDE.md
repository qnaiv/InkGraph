# フロントエンド (src) ガイド

## コンポーネント構成

| 場所 | 内容 |
|---|---|
| `App.tsx` | ルート。タブ切替 (`graph` / `analysis` / `history`) |
| `hooks/useMatches.ts` | 試合データの状態管理 + Tauri イベント購読 |
| `lib/db.ts` | **DB アクセス層**。`tauri-plugin-sql` 経由で直接 SQL を発行 |
| `components/` | `XpChart` / `MatchList` / `MatchCard` / `AnalysisPanel` / `MatchHistoryPage` / `ManualEntryModal` / `WeaponPicker` / `TagInput` / `OcrDebugPanel` 等 |
| `types/index.ts` | TS 型定義 (Match, Rule 等) |

## 勘所

- **DB アクセスは直接 SQL**: CRUD 用の Tauri コマンドは存在しない。`src/lib/db.ts` から
  `tauri-plugin-sql` 経由で直接 SQL を発行する設計。新しい CRUD 操作も `db.ts` に関数を追加すること。

- **テストフレームワークなし**: 型チェック (`tsc -b --noEmit`) と ESLint (`npm run lint`) で品質担保。
