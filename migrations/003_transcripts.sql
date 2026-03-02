CREATE TABLE IF NOT EXISTS claude_sessions (
    id TEXT PRIMARY KEY,
    issue_id TEXT NOT NULL,
    session_id TEXT NOT NULL, -- Claude CLI session ID
    started_at TEXT NOT NULL DEFAULT (datetime('now')),
    ended_at TEXT,
    FOREIGN KEY (issue_id) REFERENCES issues(id)
);

CREATE INDEX IF NOT EXISTS idx_claude_sessions_issue ON claude_sessions(issue_id);

CREATE TABLE IF NOT EXISTS transcripts (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,
    issue_id TEXT NOT NULL,
    content TEXT NOT NULL,
    captured_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (session_id) REFERENCES claude_sessions(id),
    FOREIGN KEY (issue_id) REFERENCES issues(id)
);

CREATE TABLE IF NOT EXISTS summaries (
    id TEXT PRIMARY KEY,
    transcript_id TEXT NOT NULL,
    issue_id TEXT NOT NULL,
    content TEXT NOT NULL,
    provider TEXT NOT NULL, -- 'claude', 'openai', 'ollama'
    model TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (transcript_id) REFERENCES transcripts(id),
    FOREIGN KEY (issue_id) REFERENCES issues(id)
);
