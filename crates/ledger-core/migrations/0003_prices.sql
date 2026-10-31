-- Price table: USD per million tokens, effective from a UTC date.
-- source = 'bundled' (shipped prices.json) or 'user' (Settings override).
CREATE TABLE prices (
    model                TEXT NOT NULL,
    effective_from       TEXT NOT NULL,
    input_per_mtok       REAL NOT NULL CHECK (input_per_mtok >= 0),
    output_per_mtok      REAL NOT NULL CHECK (output_per_mtok >= 0),
    cache_read_per_mtok  REAL NOT NULL DEFAULT 0 CHECK (cache_read_per_mtok >= 0),
    cache_write_per_mtok REAL NOT NULL DEFAULT 0 CHECK (cache_write_per_mtok >= 0),
    source               TEXT NOT NULL CHECK (source IN ('bundled', 'user')),
    PRIMARY KEY (model, effective_from, source)
);

-- Alternate model names that map to a priced model (e.g. Cursor's naming).
CREATE TABLE price_aliases (
    alias TEXT PRIMARY KEY,
    model TEXT NOT NULL
);
