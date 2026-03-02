CREATE TABLE IF NOT EXISTS config (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL,
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS accounts (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    provider TEXT NOT NULL DEFAULT 'linear',
    api_key TEXT NOT NULL,
    is_active INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS hotkeys (
    action TEXT PRIMARY KEY,
    key_binding TEXT NOT NULL,
    description TEXT,
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Insert default hotkeys
INSERT OR IGNORE INTO hotkeys (action, key_binding, description) VALUES
    ('quit', 'q', 'Quit application'),
    ('navigate_up', 'k', 'Move cursor up'),
    ('navigate_down', 'j', 'Move cursor down'),
    ('select', 'Enter', 'Select item'),
    ('back', 'Escape', 'Go back'),
    ('search', '/', 'Open search'),
    ('launch_claude', 'c', 'Launch Claude for selected issue'),
    ('new_issue', 'n', 'Create new issue'),
    ('edit_issue', 'e', 'Edit selected issue'),
    ('refresh', 'r', 'Refresh issues'),
    ('command_palette', 'C-p', 'Open command palette'),
    ('settings', 'C-s', 'Open settings'),
    ('next_tab', 'Tab', 'Next tab/section'),
    ('prev_tab', 'S-Tab', 'Previous tab/section');

CREATE TABLE IF NOT EXISTS themes (
    name TEXT PRIMARY KEY,
    data TEXT NOT NULL, -- JSON serialized theme
    is_active INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
