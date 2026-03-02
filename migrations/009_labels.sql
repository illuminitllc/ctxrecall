CREATE TABLE IF NOT EXISTS labels (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    color TEXT NOT NULL DEFAULT '',
    team_id TEXT,
    cached_at TEXT NOT NULL DEFAULT (datetime('now'))
);
