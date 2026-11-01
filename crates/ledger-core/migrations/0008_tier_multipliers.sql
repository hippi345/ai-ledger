-- Service-tier multipliers on Standard rates (e.g. OpenAI Batch 0.5, Fast 2).
-- NULL means the model has no such tier; Standard rates apply.
ALTER TABLE prices ADD COLUMN batch_multiplier REAL
    CHECK (batch_multiplier IS NULL OR batch_multiplier >= 0);
ALTER TABLE prices ADD COLUMN flex_multiplier REAL
    CHECK (flex_multiplier IS NULL OR flex_multiplier >= 0);
ALTER TABLE prices ADD COLUMN fast_multiplier REAL
    CHECK (fast_multiplier IS NULL OR fast_multiplier >= 0);
