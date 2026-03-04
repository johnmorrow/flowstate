use crate::models::Task;

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

pub fn print_message(msg: &str, json: bool) {
    if json {
        println!("{}", serde_json::json!({ "message": msg }));
    } else {
        println!("{msg}");
    }
}
