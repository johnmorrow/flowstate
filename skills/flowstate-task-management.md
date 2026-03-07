# Skill: Flowstate Task Management

You have access to `flowstate`, a CLI task manager. Use it to track your work, manage recurring responsibilities, and organize complex tasks into subtasks.

## When to Use

- At the **start of a session**: run `flowstate agenda --json` to see what's due today
- When **beginning work**: create or update tasks to track what you're doing
- When **finishing work**: mark tasks done so recurring tasks regenerate and parent tasks auto-complete
- When **planning**: break large tasks into subtasks using `--parent`
- When **attaching context**: use metadata for structured data, attachments for files

## Core Commands

### Check your agenda

```bash
flowstate agenda --json
```

Returns today's pending tasks, overdue deadlines, daily/weekly recurrences, and in-progress work. **Start every session with this.**

### Create a task

```bash
# Simple one-time task
flowstate task add "Deploy hotfix to production" --json

# Deadline task
flowstate task add "Submit quarterly report" --type deadline --due 2026-03-15 --json

# With tags and metadata
flowstate task add "Review PR #42" \
  --tag "project:api" \
  --tag "agent:openclaw" \
  --metadata '{"pr_url":"https://github.com/org/repo/pull/42","priority":"high"}' \
  --json
```

Always tag your tasks with `agent:openclaw` so they can be filtered later.

### Update a task

```bash
# Mark as in progress
flowstate task update tk_a3f9xz12 --status in_progress --json

# Update metadata
flowstate task update tk_a3f9xz12 --metadata '{"progress":"50%","blocker":"waiting on review"}' --json
```

### Complete or cancel

```bash
flowstate task done tk_a3f9xz12 --json
flowstate task cancel tk_a3f9xz12 --json
```

### Query tasks

```bash
# All your pending tasks
flowstate task list --status pending --tag agent:openclaw --json

# Overdue tasks
flowstate overdue --json

# Tasks by project
flowstate task list --tag project:api --json
```

## Recurring Tasks

Create tasks that regenerate automatically when completed.

```bash
# Daily standup prep
flowstate task add "Prepare standup notes" \
  --type daily \
  --due 2026-03-04T09:00:00Z \
  --tag agent:openclaw \
  --json

# Weekly review
flowstate task add "Weekly code review sweep" \
  --type weekly \
  --recur "weekly:mon" \
  --due 2026-03-03T10:00:00Z \
  --tag agent:openclaw \
  --json

# Every 2 weeks
flowstate task add "Dependency audit" \
  --type recurring \
  --recur "every:2w" \
  --due 2026-03-03T10:00:00Z \
  --tag agent:openclaw \
  --json
```

When you `flowstate task done` a recurring task, the next instance is created automatically with an updated due date. You'll see a confirmation message with the new task ID.

### Recurrence rules

| Rule | Meaning |
|------|---------|
| `daily` | Every day (use `--type daily`) |
| `weekly:mon` | Every week on Monday (use `--type weekly`) |
| `every:3d` | Every 3 days (use `--type recurring`) |
| `every:2w` | Every 2 weeks (use `--type recurring`) |

## Task Breakdowns

Decompose large tasks into subtasks. When all subtasks are done or cancelled, the parent auto-completes.

```bash
# Create parent task
flowstate task add "Migrate database to v2" --tag agent:openclaw --json
# Returns: tk_parent01

# Create subtasks
flowstate task add "Write migration scripts" --parent tk_parent01 --json
flowstate task add "Test on staging" --parent tk_parent01 --json
flowstate task add "Run production migration" --parent tk_parent01 --json
flowstate task add "Verify data integrity" --parent tk_parent01 --json

# View the breakdown
flowstate task breakdown tk_parent01 --json
```

Work through subtasks one by one. Completing the last one auto-completes the parent.

To prevent auto-completion (e.g., if the parent needs manual sign-off):

```bash
flowstate task done tk_child01 --no-auto-complete
```

## Metadata

Store arbitrary structured data on tasks. Useful for tracking context that doesn't fit in tags.

```bash
# Add context when creating
flowstate task add "Fix authentication bug" \
  --metadata '{"error_code":"AUTH_TIMEOUT","affected_users":142,"sentry_id":"PROJ-1234"}' \
  --json

# Update context as you work
flowstate task update tk_a3f9xz12 \
  --metadata '{"error_code":"AUTH_TIMEOUT","root_cause":"connection pool exhaustion","fix":"increase pool size"}' \
  --json
```

Metadata must be a valid JSON object. It replaces the existing metadata on update (not merged).

## Attachments

Link files and documents to tasks.

```bash
# Attach a log file
flowstate task attach tk_a3f9xz12 ./logs/error.log --mime-type text/plain --json

# Attach with a custom name
flowstate task attach tk_a3f9xz12 ./output.png --name "screenshot-before-fix.png" --json

# List attachments
flowstate task attachments tk_a3f9xz12 --json

# Remove an attachment
flowstate task detach at_b4g8yz34 --json
```

## State Serialization (Version Control)

Export and import task state as per-task JSON files, suitable for checking into git.

### Export tasks

```bash
# Export all tasks (metadata excluded by default for security)
flowstate state export --json

# Export with metadata included
flowstate state export --include-metadata --json

# Export only pending tasks to a custom directory
flowstate state export --dir ./tasks --status pending --json
```

Metadata is excluded by default to prevent accidental secret leakage (API keys, tokens stored in metadata). Use `--include-metadata` to opt in.

### Import tasks

```bash
# Import from default directory (skip existing tasks)
flowstate state import --json

# Import and overwrite existing tasks with file versions
flowstate state import --strategy overwrite --json

# Import, only updating tasks where the file is newer
flowstate state import --strategy update-newer --json
```

| Strategy | Behavior |
|----------|----------|
| `skip` | Only create new tasks, leave existing ones alone (default) |
| `overwrite` | Always replace existing tasks with file version |
| `update-newer` | Replace only if the file's `updated_at` is more recent |

### Version control workflow

```bash
# Export tasks alongside your code
flowstate state export
git add .flowstate/tasks/
git commit -m "Update task state"

# On another machine, import
git pull
flowstate state import --strategy update-newer
```

## Recommended Workflow

1. **Start of session:**
   ```bash
   flowstate agenda --json
   ```
   Review what's due. Pick a task to work on.

2. **Begin work:**
   ```bash
   flowstate task update tk_xxx --status in_progress --json
   ```

3. **Track progress** (optional):
   ```bash
   flowstate task update tk_xxx --metadata '{"notes":"halfway done, waiting on API response"}' --json
   ```

4. **Finish work:**
   ```bash
   flowstate task done tk_xxx --json
   ```

5. **End of session:**
   ```bash
   flowstate overdue --json
   flowstate task list --status in_progress --tag agent:openclaw --json
   ```
   Check for anything left hanging.

## Output Format

All commands support `--json` for structured output. **Always use `--json`** so you can parse responses reliably.

Task JSON shape:

```json
{
  "id": "tk_a3f9xz12",
  "title": "Write API docs",
  "status": "pending",
  "schedule_type": "deadline",
  "due_at": "2026-03-10T17:00:00Z",
  "tags": ["agent:openclaw", "project:api"],
  "metadata": {"priority": "high"},
  "created_at": "2026-03-03T09:00:00Z",
  "updated_at": "2026-03-03T09:00:00Z"
}
```

Attachment JSON shape:

```json
{
  "id": "at_b4g8yz34",
  "task_id": "tk_a3f9xz12",
  "name": "error.log",
  "path": "./logs/error.log",
  "mime_type": "text/plain",
  "size_bytes": 4096,
  "created_at": "2026-03-03T09:15:00Z"
}
```

## Exit Codes

| Code | Meaning |
|------|---------|
| `0` | Success |
| `1` | Not found |
| `2` | Validation error |
| `3` | Conflict (e.g., task already done) |

## Statuses

| Status | Use when |
|--------|----------|
| `pending` | Task created, not yet started |
| `in_progress` | Actively working on it |
| `done` | Completed |
| `cancelled` | No longer needed |
| `blocked` | Waiting on something external |

## Tagging Conventions

- `agent:openclaw` — tag all tasks you create
- `project:<name>` — associate with a project
- `meta` — housekeeping tasks (prevents auto-completion of parent)
