use crate::models::{Attachment, Task};

pub fn print_task(task: &Task, json: bool) {
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(task).expect("failed to serialize task")
        );
    } else {
        let due = task
            .due_at
            .map(|d| format!(" due:{}", d.format("%Y-%m-%d")))
            .unwrap_or_default();
        println!("[{}] {} ({}){}", task.id, task.title, task.status, due);
    }
}

pub fn print_tasks(tasks: &[Task], json: bool) {
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(tasks).expect("failed to serialize tasks")
        );
    } else if tasks.is_empty() {
        println!("No tasks found.");
    } else {
        for task in tasks {
            print_task(task, false);
        }
    }
}

pub fn print_attachment(attachment: &Attachment, json: bool) {
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(attachment).expect("failed to serialize attachment")
        );
    } else {
        let mime = attachment.mime_type.as_deref().unwrap_or("unknown");
        println!(
            "[{}] {} -> {} ({})",
            attachment.id, attachment.name, attachment.path, mime
        );
    }
}

pub fn print_attachments(attachments: &[Attachment], json: bool) {
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(attachments).expect("failed to serialize attachments")
        );
    } else if attachments.is_empty() {
        println!("No attachments found.");
    } else {
        for attachment in attachments {
            print_attachment(attachment, false);
        }
    }
}

pub fn print_message(msg: &str, json: bool) {
    if json {
        println!("{}", serde_json::json!({ "message": msg }));
    } else {
        println!("{msg}");
    }
}
