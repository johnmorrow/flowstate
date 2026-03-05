-- Add metadata column to tasks
ALTER TABLE tasks ADD COLUMN metadata TEXT NOT NULL DEFAULT '{}';

-- Attachments table
CREATE TABLE attachments (
    id TEXT PRIMARY KEY,
    task_id TEXT NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    path TEXT NOT NULL,
    mime_type TEXT,
    size_bytes INTEGER,
    created_at TEXT NOT NULL
);

CREATE INDEX idx_attachments_task_id ON attachments(task_id);
