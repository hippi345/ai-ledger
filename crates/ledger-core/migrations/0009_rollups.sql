-- Materialized usage rollups (spec §8). One table per grain with the same
-- shape. `bucket` is the bucket's first UTC date (weeks start Monday).
-- Unknown model/project are stored as '' so they can be part of the key.
CREATE TABLE rollups_day (
    bucket            TEXT    NOT NULL,
    app               TEXT    NOT NULL,
    model             TEXT    NOT NULL DEFAULT '',
    project           TEXT    NOT NULL DEFAULT '',
    events            INTEGER NOT NULL,
    requests          INTEGER NOT NULL,
    input_tokens      INTEGER NOT NULL,
    output_tokens     INTEGER NOT NULL,
    cache_read_tokens INTEGER NOT NULL,
    cache_write_tokens INTEGER NOT NULL,
    cost_usd          REAL    NOT NULL,
    estimated_events  INTEGER NOT NULL,
    PRIMARY KEY (bucket, app, model, project)
) WITHOUT ROWID;
CREATE TABLE rollups_week  AS SELECT * FROM rollups_day WHERE 0;
CREATE TABLE rollups_month AS SELECT * FROM rollups_day WHERE 0;
CREATE UNIQUE INDEX rollups_week_key  ON rollups_week  (bucket, app, model, project);
CREATE UNIQUE INDEX rollups_month_key ON rollups_month (bucket, app, model, project);
