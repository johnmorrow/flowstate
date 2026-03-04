use chrono::{NaiveDate, Utc};

use crate::db::Database;
use crate::errors::FlowstateError;
use crate::output;

pub fn handle_agenda(
    db: &Database,
    date: Option<String>,
    json: bool,
) -> Result<(), FlowstateError> {
    let target_date = match date {
        Some(d) => NaiveDate::parse_from_str(&d, "%Y-%m-%d").map_err(|_| {
            FlowstateError::Validation(format!("invalid date: {d} (expected YYYY-MM-DD)"))
        })?,
        None => Utc::now().date_naive(),
    };

    let tasks = db.get_agenda_tasks(target_date)?;
    output::print_tasks(&tasks, json);
    Ok(())
}

pub fn handle_overdue(db: &Database, json: bool) -> Result<(), FlowstateError> {
    let tasks = db.get_overdue_tasks()?;
    output::print_tasks(&tasks, json);
    Ok(())
}
