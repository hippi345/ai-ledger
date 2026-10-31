-- Long-context pricing: rates that apply when a request's prompt
-- (input + cache read + cache write tokens) exceeds above_prompt_tokens.
CREATE TABLE price_tiers (
    model                TEXT    NOT NULL,
    effective_from       TEXT    NOT NULL,
    source               TEXT    NOT NULL CHECK (source IN ('bundled', 'user')),
    above_prompt_tokens  INTEGER NOT NULL CHECK (above_prompt_tokens > 0),
    input_per_mtok       REAL    NOT NULL CHECK (input_per_mtok >= 0),
    output_per_mtok      REAL    NOT NULL CHECK (output_per_mtok >= 0),
    cache_read_per_mtok  REAL    NOT NULL DEFAULT 0 CHECK (cache_read_per_mtok >= 0),
    cache_write_per_mtok REAL    NOT NULL DEFAULT 0 CHECK (cache_write_per_mtok >= 0),
    PRIMARY KEY (model, effective_from, source, above_prompt_tokens),
    FOREIGN KEY (model, effective_from, source)
        REFERENCES prices (model, effective_from, source) ON DELETE CASCADE
);
