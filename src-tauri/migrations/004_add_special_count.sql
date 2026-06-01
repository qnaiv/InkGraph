-- special_count を追加 (未適用の場合のみ)
ALTER TABLE matches ADD COLUMN IF NOT EXISTS special_count INTEGER;
