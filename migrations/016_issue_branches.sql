CREATE TABLE IF NOT EXISTS issue_branches (
    issue_id TEXT PRIMARY KEY,
    branch_name TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
