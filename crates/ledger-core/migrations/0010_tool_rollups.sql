-- Tool / MCP call rollups (spec §8), same buckets as the usage rollups.
-- Calls with no MCP server are stored with mcp_server = ''.
CREATE TABLE rollups_tools_day (
    bucket      TEXT    NOT NULL,
    app         TEXT    NOT NULL,
    tool        TEXT    NOT NULL,
    mcp_server  TEXT    NOT NULL DEFAULT '',
    calls       INTEGER NOT NULL,
    failures    INTEGER NOT NULL,
    duration_ms INTEGER NOT NULL,
    PRIMARY KEY (bucket, app, tool, mcp_server)
) WITHOUT ROWID;
CREATE TABLE rollups_tools_week  AS SELECT * FROM rollups_tools_day WHERE 0;
CREATE TABLE rollups_tools_month AS SELECT * FROM rollups_tools_day WHERE 0;
CREATE UNIQUE INDEX rollups_tools_week_key  ON rollups_tools_week  (bucket, app, tool, mcp_server);
CREATE UNIQUE INDEX rollups_tools_month_key ON rollups_tools_month (bucket, app, tool, mcp_server);
