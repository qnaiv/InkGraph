-- 008: 自動認識フラグ追加
--
-- キャプチャ機能 (YOLO + OCR) による自動認識で挿入されたレコードかどうかを記録する。
-- auto_recorded = 1 の場合、ユーザーが編集ダイアログを開いて保存するまでは
-- "未確定" として UI 上で区別して表示する。

ALTER TABLE matches ADD COLUMN auto_recorded INTEGER NOT NULL DEFAULT 0;
