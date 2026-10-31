-- 12-hour prompt-cache writes (OpenAI's extended cache), billed at their
-- own rate, including a separate long-context rate.
ALTER TABLE events ADD COLUMN cache_write_12h_tokens INTEGER NOT NULL DEFAULT 0
    CHECK (cache_write_12h_tokens >= 0);
-- NULL means no separate rate; such writes use the standard cache-write rate.
ALTER TABLE prices ADD COLUMN cache_write_12h_per_mtok REAL
    CHECK (cache_write_12h_per_mtok IS NULL OR cache_write_12h_per_mtok >= 0);
ALTER TABLE price_tiers ADD COLUMN cache_write_12h_per_mtok REAL
    CHECK (cache_write_12h_per_mtok IS NULL OR cache_write_12h_per_mtok >= 0);
