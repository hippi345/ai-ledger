-- Schema v1: sources, events, tool_calls, sessions.
-- Timestamps are ISO-8601 UTC strings; token counts are non-negative integers.

CREATE TABLE sources (
    id          INTEGER PRIMARY KEY,
    reader_id   TEXT    NOT NULL,
    tier        INTEGER NOT NULL CHECK (tier IN (1, 2, 3)),
    config_json TEXT    NOT NULL DEFAULT '{}',
    enabled     INTEGER NOT NULL DEFAULT 1 CHECK (enabled IN (0, 1)),
    last_cursor TEXT
);

CREATE TABLE sessions (
    id      TEXT PRIMARY KEY,
    app     TEXT NOT NULL,
    project TEXT,
    started TEXT NOT NULL,
    ended   TEXT
);

CREATE TABLE events (
    id                 INTEGER PRIMARY KEY,
    source_id          INTEGER NOT NULL REFERENCES sources(id) ON DELETE CASCADE,
    ts_utc             TEXT    NOT NULL,
    app                TEXT    NOT NULL,
    model              TEXT,
    project            TEXT,
    session_id         TEXT REFERENCES sessions(id) ON DELETE SET NULL,
    input_tokens       INTEGER NOT NULL DEFAULT 0 CHECK (input_tokens >= 0),
    output_tokens      INTEGER NOT NULL DEFAULT 0 CHECK (output_tokens >= 0),
    cache_read_tokens  INTEGER NOT NULL DEFAULT 0 CHECK (cache_read_tokens >= 0),
    cache_write_tokens INTEGER NOT NULL DEFAULT 0 CHECK (cache_write_tokens >= 0),
    requests           INTEGER NOT NULL DEFAULT 1 CHECK (requests >= 0),
    cost_usd_est       REAL,
    price_version      TEXT,
    estimated          INTEGER NOT NULL DEFAULT 0 CHECK (estimated IN (0, 1)),
    raw_hash           TEXT    NOT NULL UNIQUE
);

CREATE INDEX events_ts      ON events (ts_utc);
CREATE INDEX events_app_ts  ON events (app, ts_utc);
CREATE INDEX events_model   ON events (model);
CREATE INDEX events_session ON events (session_id);

CREATE TABLE tool_calls (
    id          INTEGER PRIMARY KEY,
    event_id    INTEGER NOT NULL REFERENCES events(id) ON DELETE CASCADE,
    ts_utc      TEXT    NOT NULL,
    tool        TEXT    NOT NULL,
    mcp_server  TEXT,
    duration_ms INTEGER CHECK (duration_ms IS NULL OR duration_ms >= 0),
    ok          INTEGER NOT NULL DEFAULT 1 CHECK (ok IN (0, 1))
);

CREATE INDEX tool_calls_event ON tool_calls (event_id);
CREATE INDEX tool_calls_ts    ON tool_calls (ts_utc);
