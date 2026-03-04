CREATE TABLE IF NOT EXISTS schema_migrations (
    version INTEGER PRIMARY KEY,
    applied_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS tasks (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending',
    schedule_type TEXT NOT NULL DEFAULT 'once',
    due_at TEXT,
    recur_rule TEXT,
    parent_id TEXT REFERENCES tasks(id),
    tags TEXT NOT NULL DEFAULT '[]',
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_tasks_status ON tasks(status);
CREATE INDEX IF NOT EXISTS idx_tasks_parent_id ON tasks(parent_id);
CREATE INDEX IF NOT EXISTS idx_tasks_due_at ON tasks(due_at);
