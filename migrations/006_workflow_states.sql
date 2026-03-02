CREATE TABLE IF NOT EXISTS workflow_states (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    team_id TEXT NOT NULL,
    color TEXT NOT NULL DEFAULT '',
    position REAL NOT NULL DEFAULT 0,
    cached_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_workflow_states_team ON workflow_states(team_id);
