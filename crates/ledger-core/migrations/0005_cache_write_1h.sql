-- Extended-lifetime prompt-cache writes (e.g. Anthropic's 1-hour cache),
-- billed at a different rate from standard (5-minute) cache writes.
ALTER TABLE events ADD COLUMN cache_write_1h_tokens INTEGER NOT NULL DEFAULT 0
    CHECK (cache_write_1h_tokens >= 0);
-- NULL means the model has no separate rate; such writes use cache_write_per_mtok.
ALTER TABLE prices ADD COLUMN cache_write_1h_per_mtok REAL
    CHECK (cache_write_1h_per_mtok IS NULL OR cache_write_1h_per_mtok >= 0);
