use clap::Subcommand;
use std::fs;
use std::path::{Path, PathBuf};

use crate::db::{Database, TaskFilters, TaskUpdates};
use crate::errors::FlowstateError;
use crate::models::{Status, Task};
use crate::output;

const DEFAULT_DIR: &str = ".flowstate/tasks";

#[derive(Subcommand)]
pub enum StateAction {
    /// Export tasks to files for version control
    Export {
        /// Output directory (default: .flowstate/tasks)
        #[arg(long, default_value = DEFAULT_DIR)]
        dir: String,
        /// Include metadata in exported files (excluded by default for security)
        #[arg(long)]
        include_metadata: bool,
        /// Filter by status
        #[arg(long)]
        status: Option<String>,
        /// Filter by tag
        #[arg(long)]
        tag: Option<String>,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Import tasks from exported files
    Import {
        /// Input directory (default: .flowstate/tasks)
        #[arg(long, default_value = DEFAULT_DIR)]
        dir: String,
        /// Merge strategy: overwrite, skip, or update-newer
        #[arg(long, default_value = "skip")]
        strategy: String,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
}

pub fn handle(action: StateAction, db: &Database) -> Result<(), FlowstateError> {
    match action {
        StateAction::Export {
            dir,
            include_metadata,
            status,
            tag,
            json,
        } => cmd_export(db, &dir, include_metadata, status, tag, json),
        StateAction::Import {
            dir,
            strategy,
            json,
        } => cmd_import(db, &dir, &strategy, json),
    }
}

fn cmd_export(
    db: &Database,
    dir: &str,
    include_metadata: bool,
    status: Option<String>,
    tag: Option<String>,
    json: bool,
) -> Result<(), FlowstateError> {
    let filters = TaskFilters {
        status: status
            .map(|s| s.parse::<Status>().map_err(FlowstateError::Validation))
            .transpose()?,
        tag,
        ..Default::default()
    };

    let tasks = db.list_tasks(&filters)?;
    let dir_path = Path::new(dir);
    fs::create_dir_all(dir_path)?;

    let mut exported = 0u32;
    for task in &tasks {
        let mut task_value =
            serde_json::to_value(task).map_err(|e| FlowstateError::Validation(e.to_string()))?;

        if !include_metadata {
            if let Some(obj) = task_value.as_object_mut() {
                obj.remove("metadata");
            }
        }

        let file_path = dir_path.join(format!("{}.json", task.id));
        let content = serde_json::to_string_pretty(&task_value)
            .map_err(|e| FlowstateError::Validation(e.to_string()))?;
        fs::write(&file_path, format!("{content}\n"))?;
        exported += 1;
    }

    // Remove files for tasks that no longer exist in the DB
    let mut removed = 0u32;
    if dir_path.is_dir() {
        let all_ids: std::collections::HashSet<&str> =
            tasks.iter().map(|t| t.id.as_str()).collect();
        for entry in fs::read_dir(dir_path)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("json") {
                if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                    if stem.starts_with("tk_") && !all_ids.contains(stem) {
                        fs::remove_file(&path)?;
                        removed += 1;
                    }
                }
            }
        }
    }

    let msg = format!("Exported {exported} tasks to {dir} (removed {removed} stale files)");
    output::print_message(&msg, json);
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum MergeStrategy {
    Overwrite,
    Skip,
    UpdateNewer,
}

impl std::str::FromStr for MergeStrategy {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "overwrite" => Ok(MergeStrategy::Overwrite),
            "skip" => Ok(MergeStrategy::Skip),
            "update-newer" => Ok(MergeStrategy::UpdateNewer),
            _ => Err(format!(
                "invalid strategy: {s} (expected overwrite, skip, or update-newer)"
            )),
        }
    }
}

fn cmd_import(db: &Database, dir: &str, strategy: &str, json: bool) -> Result<(), FlowstateError> {
    let strategy: MergeStrategy = strategy.parse().map_err(FlowstateError::Validation)?;

    let dir_path = Path::new(dir);
    if !dir_path.is_dir() {
        return Err(FlowstateError::NotFound(format!(
            "directory not found: {dir}"
        )));
    }

    let mut files: Vec<PathBuf> = Vec::new();
    for entry in fs::read_dir(dir_path)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("json") {
            if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                if stem.starts_with("tk_") {
                    files.push(path);
                }
            }
        }
    }

    let mut created = 0u32;
    let mut updated = 0u32;
    let mut skipped = 0u32;

    for file_path in &files {
        let content = fs::read_to_string(file_path)?;
        let task: Task = serde_json::from_str(&content).map_err(|e| {
            FlowstateError::Validation(format!("invalid task file {}: {e}", file_path.display()))
        })?;

        match db.get_task(&task.id) {
            Ok(existing) => match strategy {
                MergeStrategy::Skip => {
                    skipped += 1;
                }
                MergeStrategy::Overwrite => {
                    apply_full_update(db, &task)?;
                    updated += 1;
                }
                MergeStrategy::UpdateNewer => {
                    if task.updated_at > existing.updated_at {
                        apply_full_update(db, &task)?;
                        updated += 1;
                    } else {
                        skipped += 1;
                    }
                }
            },
            Err(FlowstateError::NotFound(_)) => {
                db.insert_task(&task)?;
                created += 1;
            }
            Err(e) => return Err(e),
        }
    }

    let msg =
        format!("Imported from {dir}: {created} created, {updated} updated, {skipped} skipped");
    output::print_message(&msg, json);
    Ok(())
}

fn apply_full_update(db: &Database, task: &Task) -> Result<(), FlowstateError> {
    let updates = TaskUpdates {
        title: Some(task.title.clone()),
        status: Some(task.status),
        due_at: Some(task.due_at),
        tags: Some(task.tags.clone()),
        recur_rule: Some(task.recur_rule.clone()),
        metadata: Some(task.metadata.clone()),
    };
    db.update_task(&task.id, &updates)?;
    Ok(())
}
