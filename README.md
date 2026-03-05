# Flowstate

[![License: Apache 2.0](https://img.shields.io/badge/License-Apache_2.0-blue.svg)](https://opensource.org/licenses/Apache-2.0)
[![Built with Rust](https://img.shields.io/badge/Built_with-Rust-dea584.svg)](https://www.rust-lang.org/)
[![SQLite](https://img.shields.io/badge/Storage-SQLite-003B57.svg)](https://www.sqlite.org/)

> **Agent-first task management for the command line.**

A structured task management CLI designed for AI agents. Supports one-time, daily, weekly, recurring, and deadline-based tasks with hierarchical breakdowns, metadata, and file attachments.

## Installation

```bash
cargo install --path .
```

Or build from source:

```bash
cargo build --release
# Binary at target/release/flowstate
```

## Quick Start

```bash
# Add a task
flowstate task add "Write API docs"

# Add a deadline task with tags
flowstate task add "Ship v2" --type deadline --due 2026-03-10 --tag "project:myapp"

# See what's on your plate today
flowstate agenda

# Mark a task done
flowstate task done tk_a3f9xz12

# Everything as JSON (for programmatic use)
flowstate task list --json
```

## Commands

### `flowstate task add <title>`

Create a new task.

| Flag | Description |
|------|-------------|
| `--type <TYPE>` | Schedule type: `once` (default), `daily`, `weekly`, `recurring`, `deadline` |
| `--due <DATETIME>` | Due date — RFC 3339 (`2026-03-10T17:00:00Z`) or date-only (`2026-03-10`) |
| `--recur <RULE>` | Recurrence rule (see [Recurring Tasks](#recurring-tasks)) |
| `--parent <ID>` | Parent task ID for hierarchical breakdowns |
| `--tag <TAG>` | Tag (repeatable, e.g. `--tag "project:x" --tag "agent:claude"`) |
| `--metadata <JSON>` | Arbitrary JSON object (e.g. `'{"key":"value"}'`) |
| `--json` | Output as JSON |

```bash
flowstate task add "Deploy staging" --type deadline --due 2026-03-15 --tag "team:infra"
flowstate task add "Daily standup" --type daily --due 2026-03-03T09:00:00Z
flowstate task add "Write tests" --parent tk_a3f9xz12
flowstate task add "Investigate bug" --metadata '{"source":"sentry","issue_id":1234}'
```

### `flowstate task get <id>`

Fetch a single task by ID.

```bash
flowstate task get tk_a3f9xz12 --json
```

### `flowstate task list`

List tasks with optional filters.

| Flag | Description |
|------|-------------|
| `--status <STATUS>` | Filter by status: `pending`, `in_progress`, `done`, `cancelled`, `blocked` |
| `--type <TYPE>` | Filter by schedule type |
| `--tag <TAG>` | Filter by tag |
| `--due-before <DATETIME>` | Filter tasks due before a date |
| `--json` | Output as JSON |

```bash
flowstate task list --status pending --json
flowstate task list --tag "agent:claude"
flowstate task list --type deadline --due-before 2026-03-10
```

### `flowstate task update <id>`

Update an existing task. Tags are replaced (not appended) — always specify the full desired set.

| Flag | Description |
|------|-------------|
| `--title <TITLE>` | New title |
| `--status <STATUS>` | New status |
| `--due <DATETIME>` | New due date |
| `--tag <TAG>` | Replace tags (repeatable) |
| `--metadata <JSON>` | Replace metadata with new JSON object |
| `--json` | Output as JSON |

```bash
flowstate task update tk_a3f9xz12 --status in_progress
flowstate task update tk_a3f9xz12 --title "Updated title" --tag "v2" --tag "urgent"
flowstate task update tk_a3f9xz12 --metadata '{"reviewed":true}'
```

### `flowstate task done <id>`

Mark a task as done. Triggers auto-completion of parent tasks and generation of recurring instances.

| Flag | Description |
|------|-------------|
| `--no-auto-complete` | Don't auto-complete the parent task |
| `--json` | Output as JSON |

### `flowstate task cancel <id>`

Cancel a task. Also triggers parent auto-completion checks.

### `flowstate task breakdown <id>`

List all subtasks (children) of a parent task.

```bash
flowstate task breakdown tk_a3f9xz12 --json
```

### `flowstate task attach <id> <path>`

Attach a file to a task. The file name is derived from the path unless `--name` is provided.

| Flag | Description |
|------|-------------|
| `--name <NAME>` | Override attachment display name |
| `--mime-type <TYPE>` | MIME type (e.g. `text/plain`, `application/pdf`) |
| `--json` | Output as JSON |

```bash
flowstate task attach tk_a3f9xz12 ./logs/deploy.log --mime-type text/plain --json
flowstate task attach tk_a3f9xz12 ./screenshot.png --name "error-screenshot.png"
```

### `flowstate task detach <attachment_id>`

Remove an attachment by its ID (e.g. `at_b4g8yz34`).

```bash
flowstate task detach at_b4g8yz34 --json
```

### `flowstate task attachments <id>`

List all attachments for a task.

```bash
flowstate task attachments tk_a3f9xz12 --json
```

### `flowstate agenda`

Show tasks relevant for today: due today, daily tasks, matching weekly tasks, overdue deadlines, and in-progress tasks.

| Flag | Description |
|------|-------------|
| `--date <YYYY-MM-DD>` | Target date (defaults to today) |
| `--json` | Output as JSON |

```bash
flowstate agenda --json
flowstate agenda --date 2026-03-10
```

### `flowstate overdue`

Show all tasks past their due date that haven't been completed or cancelled.

```bash
flowstate overdue --json
```

## IDs

Task IDs are stable 11-character strings: `tk_` prefix + 8 lowercase alphanumeric characters (e.g. `tk_a3f9xz12`).

Attachment IDs use the same format with an `at_` prefix (e.g. `at_b4g8yz34`).

## Recurring Tasks

When a recurring task is marked done, a new pending instance is automatically created with the next due date.

| Type | Behavior |
|------|----------|
| `daily` | Next instance due 1 day later |
| `weekly` | Next instance due 7 days later |
| `recurring` with `--recur` rule | Based on the rule |

Supported recurrence rules for `--type recurring`:

| Rule | Meaning |
|------|---------|
| `daily` | Every day |
| `weekly:mon` | Every week (day hint for agenda matching) |
| `every:Nd` | Every N days (e.g. `every:3d`) |
| `every:Nw` | Every N weeks (e.g. `every:2w`) |

```bash
flowstate task add "Biweekly review" --type recurring --recur "every:2w" --due 2026-03-03T10:00:00Z
```

## Hierarchical Tasks (Breakdowns)

Create parent-child relationships with `--parent`:

```bash
flowstate task add "Launch feature"
# tk_parent01

flowstate task add "Write code" --parent tk_parent01
flowstate task add "Write tests" --parent tk_parent01
flowstate task add "Deploy" --parent tk_parent01

flowstate task breakdown tk_parent01
```

When all children are done or cancelled, the parent is automatically marked done. This can be prevented with:
- `--no-auto-complete` flag on the `done` command
- Tagging the parent with `meta`

## Tags

Tags are arbitrary strings. Convention for agent-created tasks: `agent:<name>` (e.g. `agent:claude`).

The `meta` tag has special behavior: tasks tagged `meta` are never auto-completed when their children resolve.

## JSON Output

Every command supports `--json` for machine-readable output. Without it, output is minimal plaintext.

```bash
# Single task
flowstate task get tk_a3f9xz12 --json
```
```json
{
  "id": "tk_a3f9xz12",
  "title": "Write API docs",
  "status": "pending",
  "schedule_type": "deadline",
  "due_at": "2026-03-10T17:00:00Z",
  "tags": ["project:flowstate"],
  "metadata": {"agent": "claude", "priority": "high"},
  "created_at": "2026-03-03T09:00:00Z",
  "updated_at": "2026-03-03T09:00:00Z"
}
```

The `metadata` field is omitted from output when it is an empty object. Optional fields (`due_at`, `recur_rule`, `parent_id`) are omitted when null.

```bash
# Task list
flowstate task list --json
```
```json
[
  { "id": "tk_a3f9xz12", "title": "Write API docs", "status": "pending", ... },
  { "id": "tk_b7x2km98", "title": "Fix login bug", "status": "in_progress", ... }
]
```

## Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | Not found |
| 2 | Validation error |
| 3 | Conflict (e.g. marking an already-done task as done) |

## Database

Flowstate stores data in a local SQLite file (`.flowstate.db` in the current directory). Override the path with the `FLOWSTATE_DB` environment variable:

```bash
FLOWSTATE_DB=/tmp/test.db flowstate task list
```

## Development

```bash
cargo fmt                        # Format
cargo clippy -- -D warnings      # Lint
cargo test                       # Run tests (30 integration tests)
```

## License

Apache 2.0 — see [LICENSE](LICENSE).
