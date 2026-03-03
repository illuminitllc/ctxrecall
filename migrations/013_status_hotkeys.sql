-- Add Ctrl-key status shortcut hotkeys
INSERT OR IGNORE INTO hotkeys (action, key_binding, description) VALUES
    ('set_status_done', 'C-d', 'Set status to Done'),
    ('set_status_backlog', 'C-b', 'Set status to Backlog'),
    ('set_status_todo', 'C-t', 'Set status to Todo'),
    ('set_status_in_progress', 'C-i', 'Set status to In Progress');
