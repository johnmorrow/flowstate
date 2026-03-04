# Flowstate

A task management CLI for AI agents. Supports daily, weekly, deadline-based, recurring, and hierarchical task breakdowns.

## Project Overview

Flowstate is a Rust CLI tool designed to be used by AI agents (like Claude) as a structured task management layer. Agents can create, query, update, and complete tasks during long-running workflows. The data model is designed to be agent-legible — minimal ambiguity, predictable output formats, machine-friendly responses.

## Tech Stack

- **Language:** Rust (stable toolchain)
- **CLI framework:** `clap` (derive API)
- **Storage:** Local SQLite via `rusqlite` (single `.flowstate.db` file, located in project root or `~/.flowstate/`)
- **Serialization:** `serde` + `serde_json` for structured output
- **Date/time:** `chrono`
- **Error handling:** `anyhow` for application errors, `thiserror` for library errors

## CLI Design Principles

Flowstate is **agent-first**. Every design decision should optimize for programmatic use over human aesthetics.

- **Always support `--json` output** on every command so agents can parse responses reliably
- **Exit codes are meaningful:** `0` = success, `1` = not found, `2` = validation error, `3` = conflict
- **Output to stdout only** for data; use stderr for logs/warnings
- **No interactive prompts** — all inputs must be passable as flags
- **IDs are short and stable** — use nanoid-style 8-char alphanumeric IDs (e.g. `tk_a3f9xz12`)

## Core Concepts

### Task
The fundamental unit. Every task has:
- `id` — stable short ID (e.g. `tk_a3f9xz12`)
- `title` — plain text description
- `status` — `pending | in_progress | done | cancelled | blocked`
- `schedule_type` — `once | daily | weekly | recurring | deadline`
- `due_at` — optional ISO 8601 datetime
- `recur_rule` — optional cron-style or simple string (`"daily"`, `"weekly:mon"`, etc.)
- `parent_id` — optional, links to a parent task (for breakdowns)
- `tags` — array of strings (e.g. `["agent:claude", "project:flowstate"]`)
- `created_at`, `updated_at` — ISO 8601

### Breakdown
A parent task with child subtasks. Agents decompose large tasks into breakdowns. Completing all children auto-transitions the parent to `done` unless `--no-auto-complete` is set.

### Schedule Types
- `once` — a one-time task, optionally with a `due_at`
- `daily` — recurs every day, generates a new instance when marked done
- `weekly` — recurs on specific day(s) of the week
- `recurring` — driven by a `recur_rule`
- `deadline` — has a hard `due_at`, surfaces urgency in queries

## Commands

```
flowstate task add <title> [--type <schedule_type>] [--due <datetime>] [--recur <rule>] [--parent <id>] [--tag <tag>]
flowstate task get <id> [--json]
flowstate task list [--status <status>] [--type <schedule_type>] [--tag <tag>] [--due-before <datetime>] [--json]
flowstate task update <id> [--title <title>] [--status <status>] [--due <datetime>] [--tag <tag>]
flowstate task done <id>
flowstate task cancel <id>
flowstate task breakdown <id>          # List subtasks of a parent task
flowstate agenda [--date <date>] [--json]  # Today's pending/due tasks
flowstate overdue [--json]             # Tasks past their due_at
```

## Output Format

Default (human) output is minimal plaintext. With `--json`, always return a valid JSON object or array — never mixed text. Example:

```json
{
  "id": "tk_a3f9xz12",
  "title": "Write API docs",
  "status": "pending",
  "schedule_type": "deadline",
  "due_at": "2025-03-10T17:00:00Z",
  "tags": ["project:flowstate"],
  "parent_id": null,
  "created_at": "2025-03-03T09:00:00Z",
  "updated_at": "2025-03-03T09:00:00Z"
}
```

## Project Structure

```
flowstate/
├── src/
│   ├── main.rs          # Entry point, CLI setup via clap
│   ├── cli/             # Command definitions (one file per command group)
│   │   ├── task.rs
│   │   └── agenda.rs
│   ├── db/              # SQLite layer (migrations, queries)
│   │   ├── mod.rs
│   │   └── migrations/
│   ├── models/          # Task, ScheduleType, Status structs + serde impls
│   ├── recur.rs         # Recurring task logic
│   └── output.rs        # Formatting helpers (json / plaintext)
├── tests/               # Integration tests using assert_cmd
├── Cargo.toml
├── CLAUDE.md            # This file
└── .flowstate.db        # Runtime DB (gitignored)
```

## Development Conventions

- Run `cargo clippy -- -D warnings` before considering any task done
- Run `cargo test` — all tests must pass
- Use `cargo fmt` for formatting (default settings)
- Avoid `unwrap()` in non-test code — propagate errors with `?`
- Prefer `snake_case` for DB column names and Rust fields
- DB migrations live in `src/db/migrations/` as numbered `.sql` files (`001_init.sql`, etc.)

## Agent Usage Notes

When Claude Code is working on this project:

- Prefer modifying existing commands over adding new ones unless explicitly asked
- When adding a new field to `Task`, update: the struct, the DB schema (new migration), the SQL queries, and the JSON output — all in one go
- `flowstate agenda` is the primary entrypoint for an agent starting a session — think of it as the agent's "what should I do now" command
- Tag convention for agent-created tasks: `agent:<agent-name>` (e.g. `agent:claude`)
- Tasks tagged `meta` are housekeeping tasks about Flowstate itself — don't auto-complete or modify these unless explicitly instructed
