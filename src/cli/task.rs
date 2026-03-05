use clap::Subcommand;

use crate::db::{parse_datetime, Database, TaskFilters, TaskUpdates};
use crate::errors::FlowstateError;
use crate::models::{generate_task_id, ScheduleType, Status, Task};
use crate::output;
use crate::recur;
use chrono::Utc;
use std::path::Path;

struct UpdateParams {
    title: Option<String>,
    status: Option<String>,
    due: Option<String>,
    tags: Vec<String>,
    metadata: Option<String>,
    json: bool,
}

struct AddParams {
    title: String,
    schedule_type: Option<String>,
    due: Option<String>,
    recur: Option<String>,
    parent: Option<String>,
    tags: Vec<String>,
    metadata: Option<String>,
    json: bool,
}

#[derive(Subcommand)]
pub enum TaskAction {
    /// Add a new task
    Add {
        /// Task title
        title: String,
        /// Schedule type
        #[arg(long = "type", value_name = "TYPE")]
        schedule_type: Option<String>,
        /// Due date (RFC 3339 or YYYY-MM-DD)
        #[arg(long)]
        due: Option<String>,
        /// Recurrence rule
        #[arg(long)]
        recur: Option<String>,
        /// Parent task ID
        #[arg(long)]
        parent: Option<String>,
        /// Tags (repeatable)
        #[arg(long, action = clap::ArgAction::Append)]
        tag: Vec<String>,
        /// Metadata as JSON object
        #[arg(long)]
        metadata: Option<String>,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Get a task by ID
    Get {
        /// Task ID
        id: String,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// List tasks with optional filters
    List {
        /// Filter by status
        #[arg(long)]
        status: Option<String>,
        /// Filter by schedule type
        #[arg(long = "type", value_name = "TYPE")]
        schedule_type: Option<String>,
        /// Filter by tag
        #[arg(long)]
        tag: Option<String>,
        /// Filter by due before date
        #[arg(long)]
        due_before: Option<String>,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Update a task
    Update {
        /// Task ID
        id: String,
        /// New title
        #[arg(long)]
        title: Option<String>,
        /// New status
        #[arg(long)]
        status: Option<String>,
        /// New due date
        #[arg(long)]
        due: Option<String>,
        /// Set tags (replaces existing)
        #[arg(long, action = clap::ArgAction::Append)]
        tag: Vec<String>,
        /// Metadata as JSON object
        #[arg(long)]
        metadata: Option<String>,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Mark a task as done
    Done {
        /// Task ID
        id: String,
        /// Don't auto-complete parent
        #[arg(long)]
        no_auto_complete: bool,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Cancel a task
    Cancel {
        /// Task ID
        id: String,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// List subtasks of a parent task
    Breakdown {
        /// Parent task ID
        id: String,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Attach a file to a task
    Attach {
        /// Task ID
        task_id: String,
        /// File path
        path: String,
        /// Attachment name (defaults to filename)
        #[arg(long)]
        name: Option<String>,
        /// MIME type
        #[arg(long)]
        mime_type: Option<String>,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Remove an attachment
    Detach {
        /// Attachment ID
        attachment_id: String,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// List attachments for a task
    Attachments {
        /// Task ID
        task_id: String,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
}

pub fn handle(action: TaskAction, db: &Database) -> Result<(), FlowstateError> {
    match action {
        TaskAction::Add {
            title,
            schedule_type,
            due,
            recur,
            parent,
            tag,
            metadata,
            json,
        } => cmd_add(
            db,
            AddParams {
                title,
                schedule_type,
                due,
                recur,
                parent,
                tags: tag,
                metadata,
                json,
            },
        ),
        TaskAction::Get { id, json } => cmd_get(db, &id, json),
        TaskAction::List {
            status,
            schedule_type,
            tag,
            due_before,
            json,
        } => cmd_list(db, status, schedule_type, tag, due_before, json),
        TaskAction::Update {
            id,
            title,
            status,
            due,
            tag,
            metadata,
            json,
        } => cmd_update(
            db,
            &id,
            UpdateParams {
                title,
                status,
                due,
                tags: tag,
                metadata,
                json,
            },
        ),
        TaskAction::Done {
            id,
            no_auto_complete,
            json,
        } => cmd_done(db, &id, no_auto_complete, json),
        TaskAction::Cancel { id, json } => cmd_cancel(db, &id, json),
        TaskAction::Breakdown { id, json } => cmd_breakdown(db, &id, json),
        TaskAction::Attach {
            task_id,
            path,
            name,
            mime_type,
            json,
        } => cmd_attach(db, &task_id, &path, name, mime_type, json),
        TaskAction::Detach {
            attachment_id,
            json,
        } => cmd_detach(db, &attachment_id, json),
        TaskAction::Attachments { task_id, json } => cmd_attachments(db, &task_id, json),
    }
}

fn cmd_add(db: &Database, params: AddParams) -> Result<(), FlowstateError> {
    let AddParams {
        title,
        schedule_type,
        due,
        recur,
        parent,
        tags,
        metadata,
        json,
    } = params;
    let stype = match schedule_type {
        Some(s) => s
            .parse::<ScheduleType>()
            .map_err(FlowstateError::Validation)?,
        None => ScheduleType::Once,
    };

    let due_at = due.map(|d| parse_datetime(&d)).transpose()?;

    if stype == ScheduleType::Deadline && due_at.is_none() {
        return Err(FlowstateError::Validation(
            "deadline tasks require --due".to_string(),
        ));
    }

    let parsed_metadata = parse_metadata_arg(metadata)?;

    let now = Utc::now();
    let task = Task {
        id: generate_task_id(),
        title,
        status: Status::Pending,
        schedule_type: stype,
        due_at,
        recur_rule: recur,
        parent_id: parent,
        tags,
        metadata: parsed_metadata,
        created_at: now,
        updated_at: now,
    };

    db.insert_task(&task)?;
    output::print_task(&task, json);
    Ok(())
}

fn cmd_get(db: &Database, id: &str, json: bool) -> Result<(), FlowstateError> {
    let task = db.get_task(id)?;
    output::print_task(&task, json);
    Ok(())
}

fn cmd_list(
    db: &Database,
    status: Option<String>,
    schedule_type: Option<String>,
    tag: Option<String>,
    due_before: Option<String>,
    json: bool,
) -> Result<(), FlowstateError> {
    let filters = TaskFilters {
        status: status
            .map(|s| s.parse::<Status>().map_err(FlowstateError::Validation))
            .transpose()?,
        schedule_type: schedule_type
            .map(|s| {
                s.parse::<ScheduleType>()
                    .map_err(FlowstateError::Validation)
            })
            .transpose()?,
        tag,
        due_before: due_before.map(|d| parse_datetime(&d)).transpose()?,
        parent_id: None,
    };
    let tasks = db.list_tasks(&filters)?;
    output::print_tasks(&tasks, json);
    Ok(())
}

fn cmd_update(db: &Database, id: &str, params: UpdateParams) -> Result<(), FlowstateError> {
    let UpdateParams {
        title,
        status,
        due,
        tags,
        metadata,
        json,
    } = params;

    let parsed_status = status
        .map(|s| s.parse::<Status>().map_err(FlowstateError::Validation))
        .transpose()?;

    let parsed_due = if let Some(d) = due {
        Some(Some(parse_datetime(&d)?))
    } else {
        None
    };

    let parsed_tags = if tags.is_empty() { None } else { Some(tags) };

    let parsed_metadata = if metadata.is_some() {
        Some(parse_metadata_arg(metadata)?)
    } else {
        None
    };

    let updates = TaskUpdates {
        title,
        status: parsed_status,
        due_at: parsed_due,
        tags: parsed_tags,
        recur_rule: None,
        metadata: parsed_metadata,
    };

    let task = db.update_task(id, &updates)?;
    output::print_task(&task, json);
    Ok(())
}

fn cmd_done(
    db: &Database,
    id: &str,
    no_auto_complete: bool,
    json: bool,
) -> Result<(), FlowstateError> {
    let task = db.get_task(id)?;

    if task.status == Status::Done {
        return Err(FlowstateError::Conflict(format!(
            "task {id} is already done"
        )));
    }

    let updated = db.update_task(
        id,
        &TaskUpdates {
            status: Some(Status::Done),
            title: None,
            due_at: None,
            tags: None,
            recur_rule: None,
            metadata: None,
        },
    )?;

    output::print_task(&updated, json);

    // Handle auto-complete for parent
    if !no_auto_complete {
        if let Some(ref parent_id) = updated.parent_id {
            if let Some(parent) = db.check_auto_complete(parent_id)? {
                output::print_message(&format!("Parent task {} auto-completed", parent.id), json);
            }
        }
    }

    // Handle recurring tasks
    if let Some(new_task) = recur::generate_next_instance(&updated, db)? {
        output::print_message(&format!("Next recurrence created: {}", new_task.id), json);
    }

    Ok(())
}

fn cmd_cancel(db: &Database, id: &str, json: bool) -> Result<(), FlowstateError> {
    let task = db.get_task(id)?;

    if task.status == Status::Cancelled {
        return Err(FlowstateError::Conflict(format!(
            "task {id} is already cancelled"
        )));
    }

    let updated = db.update_task(
        id,
        &TaskUpdates {
            status: Some(Status::Cancelled),
            title: None,
            due_at: None,
            tags: None,
            recur_rule: None,
            metadata: None,
        },
    )?;

    output::print_task(&updated, json);

    // Check parent auto-complete
    if let Some(ref parent_id) = updated.parent_id {
        if let Some(parent) = db.check_auto_complete(parent_id)? {
            output::print_message(&format!("Parent task {} auto-completed", parent.id), json);
        }
    }

    Ok(())
}

fn cmd_breakdown(db: &Database, id: &str, json: bool) -> Result<(), FlowstateError> {
    // Verify parent exists
    let _parent = db.get_task(id)?;
    let children = db.get_children(id)?;
    output::print_tasks(&children, json);
    Ok(())
}

fn parse_metadata_arg(metadata: Option<String>) -> Result<serde_json::Value, FlowstateError> {
    match metadata {
        Some(s) => {
            let val: serde_json::Value = serde_json::from_str(&s)
                .map_err(|e| FlowstateError::Validation(format!("invalid metadata JSON: {e}")))?;
            if !val.is_object() {
                return Err(FlowstateError::Validation(
                    "metadata must be a JSON object".to_string(),
                ));
            }
            Ok(val)
        }
        None => Ok(serde_json::json!({})),
    }
}

fn cmd_attach(
    db: &Database,
    task_id: &str,
    path: &str,
    name: Option<String>,
    mime_type: Option<String>,
    json: bool,
) -> Result<(), FlowstateError> {
    let file_path = Path::new(path);
    let attachment_name = name.unwrap_or_else(|| {
        file_path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| path.to_string())
    });

    let size_bytes = std::fs::metadata(file_path).ok().map(|m| m.len() as i64);

    let attachment = db.add_attachment(
        task_id,
        &attachment_name,
        path,
        mime_type.as_deref(),
        size_bytes,
    )?;
    output::print_attachment(&attachment, json);
    Ok(())
}

fn cmd_detach(db: &Database, attachment_id: &str, json: bool) -> Result<(), FlowstateError> {
    db.remove_attachment(attachment_id)?;
    output::print_message(&format!("Attachment {attachment_id} removed"), json);
    Ok(())
}

fn cmd_attachments(db: &Database, task_id: &str, json: bool) -> Result<(), FlowstateError> {
    let attachments = db.list_attachments(task_id)?;
    output::print_attachments(&attachments, json);
    Ok(())
}
