-- マイグレーション 003: special_count カラムを追加
-- assist_count は互換性のため残すが今後は使用しない
ALTER TABLE matches ADD COLUMN special_count INTEGER;
