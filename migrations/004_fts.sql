-- Documents table for per-issue documents
CREATE TABLE IF NOT EXISTS documents (
    id TEXT PRIMARY KEY,
    issue_id TEXT NOT NULL,
    doc_type TEXT NOT NULL, -- 'plan', 'prd', 'tasks', 'memories', 'notes'
    title TEXT NOT NULL,
    content TEXT NOT NULL DEFAULT '',
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_documents_issue ON documents(issue_id);
CREATE INDEX IF NOT EXISTS idx_documents_type ON documents(doc_type);

-- FTS5 virtual table for full-text search across all content
CREATE VIRTUAL TABLE IF NOT EXISTS search_index USING fts5(
    source_type,  -- 'issue', 'document', 'transcript', 'summary'
    source_id,
    issue_id,
    title,
    content,
    tokenize='porter unicode61'
);
