use chrono::{Duration, Utc};

use crate::db::Database;
use crate::errors::FlowstateError;
use crate::models::{generate_task_id, ScheduleType, Status, Task};

/// Generate the next instance of a recurring task after it's been completed.
/// Returns the newly created task, or None if the task isn't recurring.
pub fn generate_next_instance(
    completed_task: &Task,
    db: &Database,
) -> Result<Option<Task>, FlowstateError> {
    let next_due = match completed_task.schedule_type {
        ScheduleType::Daily => {
            let base = completed_task.due_at.unwrap_or_else(Utc::now);
            Some(base + Duration::days(1))
        }
        ScheduleType::Weekly => {
            let base = completed_task.due_at.unwrap_or_else(Utc::now);
            Some(base + Duration::weeks(1))
        }
        ScheduleType::Recurring => {
            if let Some(ref rule) = completed_task.recur_rule {
                let base = completed_task.due_at.unwrap_or_else(Utc::now);
                parse_recur_rule(rule).map(|d| base + d)
            } else {
                None
            }
        }
        _ => None,
    };

    let next_due = match next_due {
        Some(d) => d,
        None => return Ok(None),
    };

    let now = Utc::now();
    let new_task = Task {
        id: generate_task_id(),
        title: completed_task.title.clone(),
        status: Status::Pending,
        schedule_type: completed_task.schedule_type,
        due_at: Some(next_due),
        recur_rule: completed_task.recur_rule.clone(),
        parent_id: completed_task.parent_id.clone(),
        tags: completed_task.tags.clone(),
        created_at: now,
        updated_at: now,
    };

    db.insert_task(&new_task)?;
    Ok(Some(new_task))
}

/// Parse a recur_rule string into a Duration.
/// Supported formats:
///   "daily"       -> 1 day
///   "weekly:mon"  -> 7 days (day hint ignored for duration calc)
///   "every:Nd"    -> N days
///   "every:Nw"    -> N weeks
fn parse_recur_rule(rule: &str) -> Option<Duration> {
    if rule == "daily" {
        return Some(Duration::days(1));
    }
    if rule.starts_with("weekly") {
        return Some(Duration::weeks(1));
    }
    if let Some(spec) = rule.strip_prefix("every:") {
        if let Some(days_str) = spec.strip_suffix('d') {
            if let Ok(n) = days_str.parse::<i64>() {
                return Some(Duration::days(n));
            }
        }
        if let Some(weeks_str) = spec.strip_suffix('w') {
            if let Ok(n) = weeks_str.parse::<i64>() {
                return Some(Duration::weeks(n));
            }
        }
    }
    None
}
