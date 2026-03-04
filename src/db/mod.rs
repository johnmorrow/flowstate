use chrono::{DateTime, NaiveDate, NaiveDateTime, TimeZone, Utc};
use rusqlite::{params, Connection, Row};

use crate::errors::FlowstateError;
use crate::models::{ScheduleType, Status, Task};

const MIGRATION_001: &str = include_str!("migrations/001_init.sql");

pub struct Database {
    conn: Connection,
}

#[derive(Default)]
pub struct TaskFilters {
    pub status: Option<Status>,
    pub schedule_type: Option<ScheduleType>,
    pub tag: Option<String>,
    pub due_before: Option<DateTime<Utc>>,
    pub parent_id: Option<String>,
}

pub struct TaskUpdates {
    pub title: Option<String>,
    pub status: Option<Status>,
    pub due_at: Option<Option<DateTime<Utc>>>,
    pub tags: Option<Vec<String>>,
    pub recur_rule: Option<Option<String>>,
}

pub fn parse_datetime(s: &str) -> Result<DateTime<Utc>, FlowstateError> {
    // Try RFC 3339 first
    if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
        return Ok(dt.with_timezone(&Utc));
    }
    // Try date-only (YYYY-MM-DD) — treat as end of day UTC
    if let Ok(date) = NaiveDate::parse_from_str(s, "%Y-%m-%d") {
        let naive = NaiveDateTime::new(date, chrono::NaiveTime::from_hms_opt(23, 59, 59).unwrap());
        return Ok(Utc.from_utc_datetime(&naive));
    }
    Err(FlowstateError::Validation(format!(
        "invalid datetime: {s} (expected RFC 3339 or YYYY-MM-DD)"
    )))
}

impl Database {
    pub fn open(path: &str) -> Result<Self, FlowstateError> {
        let conn = Connection::open(path)?;
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")?;
        let db = Database { conn };
        db.run_migrations()?;
        Ok(db)
    }

    fn run_migrations(&self) -> Result<(), FlowstateError> {
        // Ensure schema_migrations exists (it's in migration 1 but we need to check)
        let has_table: bool = self
            .conn
            .query_row(
                "SELECT count(*) > 0 FROM sqlite_master WHERE type='table' AND name='schema_migrations'",
                [],
                |row| row.get(0),
            )?;

        if !has_table {
            self.conn.execute_batch(MIGRATION_001)?;
            self.conn.execute(
                "INSERT INTO schema_migrations (version) VALUES (?1)",
                params![1],
            )?;
            return Ok(());
        }

        let max_version: i64 = self.conn.query_row(
            "SELECT COALESCE(MAX(version), 0) FROM schema_migrations",
            [],
            |row| row.get(0),
        )?;

        if max_version < 1 {
            self.conn.execute_batch(MIGRATION_001)?;
            self.conn.execute(
                "INSERT INTO schema_migrations (version) VALUES (?1)",
                params![1],
            )?;
        }

        Ok(())
    }

    pub fn insert_task(&self, task: &Task) -> Result<(), FlowstateError> {
        // Validate parent exists if specified
        if let Some(ref pid) = task.parent_id {
            let exists: bool = self.conn.query_row(
                "SELECT count(*) > 0 FROM tasks WHERE id = ?1",
                params![pid],
                |row| row.get(0),
            )?;
            if !exists {
                return Err(FlowstateError::NotFound(format!(
                    "parent task {pid} not found"
                )));
            }
        }

        let tags_json = serde_json::to_string(&task.tags)
            .map_err(|e| FlowstateError::Validation(e.to_string()))?;

        self.conn.execute(
            "INSERT INTO tasks (id, title, status, schedule_type, due_at, recur_rule, parent_id, tags, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                task.id,
                task.title,
                task.status.to_string(),
                task.schedule_type.to_string(),
                task.due_at.map(|d| d.to_rfc3339()),
                task.recur_rule,
                task.parent_id,
                tags_json,
                task.created_at.to_rfc3339(),
                task.updated_at.to_rfc3339(),
            ],
        )?;
        Ok(())
    }

    pub fn get_task(&self, id: &str) -> Result<Task, FlowstateError> {
        let task = self
            .conn
            .query_row("SELECT * FROM tasks WHERE id = ?1", params![id], |row| {
                row_to_task(row)
            })
            .map_err(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => {
                    FlowstateError::NotFound(format!("task {id} not found"))
                }
                other => FlowstateError::Database(other),
            })?;
        Ok(task)
    }

    pub fn list_tasks(&self, filters: &TaskFilters) -> Result<Vec<Task>, FlowstateError> {
        let mut conditions = Vec::new();
        let mut param_values: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

        if let Some(ref status) = filters.status {
            conditions.push(format!("status = ?{}", param_values.len() + 1));
            param_values.push(Box::new(status.to_string()));
        }
        if let Some(ref stype) = filters.schedule_type {
            conditions.push(format!("schedule_type = ?{}", param_values.len() + 1));
            param_values.push(Box::new(stype.to_string()));
        }
        if let Some(ref tag) = filters.tag {
            conditions.push(format!(
                "EXISTS (SELECT 1 FROM json_each(tags) WHERE json_each.value = ?{})",
                param_values.len() + 1
            ));
            param_values.push(Box::new(tag.clone()));
        }
        if let Some(ref due_before) = filters.due_before {
            conditions.push(format!(
                "due_at IS NOT NULL AND due_at <= ?{}",
                param_values.len() + 1
            ));
            param_values.push(Box::new(due_before.to_rfc3339()));
        }
        if let Some(ref parent_id) = filters.parent_id {
            conditions.push(format!("parent_id = ?{}", param_values.len() + 1));
            param_values.push(Box::new(parent_id.clone()));
        }

        let where_clause = if conditions.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", conditions.join(" AND "))
        };

        let sql = format!("SELECT * FROM tasks {where_clause} ORDER BY created_at ASC");
        let params_refs: Vec<&dyn rusqlite::types::ToSql> =
            param_values.iter().map(|p| p.as_ref()).collect();

        let mut stmt = self.conn.prepare(&sql)?;
        let tasks = stmt
            .query_map(params_refs.as_slice(), row_to_task)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(tasks)
    }

    pub fn update_task(&self, id: &str, updates: &TaskUpdates) -> Result<Task, FlowstateError> {
        // Verify task exists
        let _existing = self.get_task(id)?;

        let mut sets = Vec::new();
        let mut param_values: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

        if let Some(ref title) = updates.title {
            sets.push(format!("title = ?{}", param_values.len() + 1));
            param_values.push(Box::new(title.clone()));
        }
        if let Some(ref status) = updates.status {
            sets.push(format!("status = ?{}", param_values.len() + 1));
            param_values.push(Box::new(status.to_string()));
        }
        if let Some(ref due_at) = updates.due_at {
            sets.push(format!("due_at = ?{}", param_values.len() + 1));
            param_values.push(Box::new(due_at.map(|d| d.to_rfc3339())));
        }
        if let Some(ref tags) = updates.tags {
            let tags_json = serde_json::to_string(tags)
                .map_err(|e| FlowstateError::Validation(e.to_string()))?;
            sets.push(format!("tags = ?{}", param_values.len() + 1));
            param_values.push(Box::new(tags_json));
        }
        if let Some(ref recur_rule) = updates.recur_rule {
            sets.push(format!("recur_rule = ?{}", param_values.len() + 1));
            param_values.push(Box::new(recur_rule.clone()));
        }

        if sets.is_empty() {
            return self.get_task(id);
        }

        // Always update updated_at
        sets.push(format!("updated_at = ?{}", param_values.len() + 1));
        param_values.push(Box::new(Utc::now().to_rfc3339()));

        let id_param = param_values.len() + 1;
        param_values.push(Box::new(id.to_string()));

        let sql = format!(
            "UPDATE tasks SET {} WHERE id = ?{}",
            sets.join(", "),
            id_param
        );
        let params_refs: Vec<&dyn rusqlite::types::ToSql> =
            param_values.iter().map(|p| p.as_ref()).collect();

        self.conn.execute(&sql, params_refs.as_slice())?;
        self.get_task(id)
    }

    pub fn get_children(&self, parent_id: &str) -> Result<Vec<Task>, FlowstateError> {
        self.list_tasks(&TaskFilters {
            parent_id: Some(parent_id.to_string()),
            ..Default::default()
        })
    }

    /// Check if all children of a parent are done/cancelled.
    /// If so, auto-complete the parent (unless it has the `meta` tag).
    /// Returns the parent task if it was auto-completed.
    pub fn check_auto_complete(&self, parent_id: &str) -> Result<Option<Task>, FlowstateError> {
        let parent = self.get_task(parent_id)?;

        // Don't auto-complete meta-tagged tasks
        if parent.has_tag("meta") {
            return Ok(None);
        }

        // Already done/cancelled
        if parent.status == Status::Done || parent.status == Status::Cancelled {
            return Ok(None);
        }

        let children = self.get_children(parent_id)?;
        if children.is_empty() {
            return Ok(None);
        }

        let all_resolved = children
            .iter()
            .all(|c| c.status == Status::Done || c.status == Status::Cancelled);

        if all_resolved {
            let updated = self.update_task(
                parent_id,
                &TaskUpdates {
                    status: Some(Status::Done),
                    title: None,
                    due_at: None,
                    tags: None,
                    recur_rule: None,
                },
            )?;
            Ok(Some(updated))
        } else {
            Ok(None)
        }
    }

    pub fn get_agenda_tasks(&self, date: NaiveDate) -> Result<Vec<Task>, FlowstateError> {
        let day_start = Utc
            .from_utc_datetime(&NaiveDateTime::new(
                date,
                chrono::NaiveTime::from_hms_opt(0, 0, 0).unwrap(),
            ))
            .to_rfc3339();
        let day_end = Utc
            .from_utc_datetime(&NaiveDateTime::new(
                date,
                chrono::NaiveTime::from_hms_opt(23, 59, 59).unwrap(),
            ))
            .to_rfc3339();

        // Tasks that are:
        // 1. Due today (due_at between day_start and day_end)
        // 2. Daily schedule type with pending/in_progress status
        // 3. Weekly schedule type matching today's day name
        // 4. Overdue deadlines (due_at < day_start and not done/cancelled)
        // 5. Currently in_progress
        let weekday = date.format("%a").to_string().to_lowercase(); // mon, tue, etc.
        let sql = format!(
            "SELECT * FROM tasks WHERE
                (due_at >= ?1 AND due_at <= ?2 AND status NOT IN ('done', 'cancelled'))
                OR (schedule_type = 'daily' AND status IN ('pending', 'in_progress'))
                OR (schedule_type = 'weekly' AND recur_rule LIKE '%{weekday}%' AND status IN ('pending', 'in_progress'))
                OR (due_at < ?1 AND status NOT IN ('done', 'cancelled'))
                OR (status = 'in_progress')
             ORDER BY due_at ASC NULLS LAST, created_at ASC"
        );
        let mut stmt = self.conn.prepare(&sql)?;
        let tasks = stmt
            .query_map(params![day_start, day_end], row_to_task)?
            .collect::<Result<Vec<_>, _>>()?;

        // Deduplicate (a task may match multiple conditions)
        let mut seen = std::collections::HashSet::new();
        let unique: Vec<Task> = tasks
            .into_iter()
            .filter(|t| seen.insert(t.id.clone()))
            .collect();
        Ok(unique)
    }

    pub fn get_overdue_tasks(&self) -> Result<Vec<Task>, FlowstateError> {
        let now = Utc::now().to_rfc3339();
        let mut stmt = self.conn.prepare(
            "SELECT * FROM tasks WHERE due_at IS NOT NULL AND due_at < ?1 AND status NOT IN ('done', 'cancelled')
             ORDER BY due_at ASC",
        )?;
        let tasks = stmt
            .query_map(params![now], row_to_task)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(tasks)
    }
}

fn row_to_task(row: &Row) -> Result<Task, rusqlite::Error> {
    let status_str: String = row.get("status")?;
    let schedule_str: String = row.get("schedule_type")?;
    let due_at_str: Option<String> = row.get("due_at")?;
    let tags_str: String = row.get("tags")?;
    let created_str: String = row.get("created_at")?;
    let updated_str: String = row.get("updated_at")?;

    let status: Status = status_str.parse().map_err(|e: String| {
        rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::from(e))
    })?;
    let schedule_type: ScheduleType = schedule_str.parse().map_err(|e: String| {
        rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::from(e))
    })?;
    let due_at = due_at_str.and_then(|s| {
        DateTime::parse_from_rfc3339(&s)
            .ok()
            .map(|d| d.with_timezone(&Utc))
    });
    let tags: Vec<String> = serde_json::from_str(&tags_str).unwrap_or_default();
    let created_at = DateTime::parse_from_rfc3339(&created_str)
        .map(|d| d.with_timezone(&Utc))
        .unwrap_or_else(|_| Utc::now());
    let updated_at = DateTime::parse_from_rfc3339(&updated_str)
        .map(|d| d.with_timezone(&Utc))
        .unwrap_or_else(|_| Utc::now());

    Ok(Task {
        id: row.get("id")?,
        title: row.get("title")?,
        status,
        schedule_type,
        due_at,
        recur_rule: row.get("recur_rule")?,
        parent_id: row.get("parent_id")?,
        tags,
        created_at,
        updated_at,
    })
}
