use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

#[allow(deprecated)]
fn flowstate(tmp: &TempDir) -> Command {
    let mut cmd = Command::cargo_bin("flowstate").unwrap();
    cmd.env("FLOWSTATE_DB", tmp.path().join("test.db").to_str().unwrap());
    cmd
}

#[test]
fn test_add_and_get_task() {
    let tmp = TempDir::new().unwrap();

    // Add a task
    let output = flowstate(&tmp)
        .args(["task", "add", "Write API docs", "--json"])
        .output()
        .unwrap();
    assert!(output.status.success());

    let task: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(task["title"], "Write API docs");
    assert_eq!(task["status"], "pending");
    assert_eq!(task["schedule_type"], "once");
    let id = task["id"].as_str().unwrap().to_string();
    assert!(id.starts_with("tk_"));
    assert_eq!(id.len(), 11); // "tk_" + 8 chars

    // Get the task
    flowstate(&tmp)
        .args(["task", "get", &id, "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains(&id));
}

#[test]
fn test_add_with_options() {
    let tmp = TempDir::new().unwrap();

    let output = flowstate(&tmp)
        .args([
            "task",
            "add",
            "Deploy v2",
            "--type",
            "deadline",
            "--due",
            "2026-03-10T17:00:00Z",
            "--tag",
            "project:flowstate",
            "--tag",
            "agent:claude",
            "--json",
        ])
        .output()
        .unwrap();
    assert!(output.status.success());

    let task: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(task["schedule_type"], "deadline");
    assert_eq!(task["due_at"], "2026-03-10T17:00:00Z");
    assert_eq!(
        task["tags"],
        serde_json::json!(["project:flowstate", "agent:claude"])
    );
}

#[test]
fn test_deadline_requires_due() {
    let tmp = TempDir::new().unwrap();

    flowstate(&tmp)
        .args(["task", "add", "No due date", "--type", "deadline"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("deadline tasks require --due"));
}

#[test]
fn test_json_output_format() {
    let tmp = TempDir::new().unwrap();

    let output = flowstate(&tmp)
        .args(["task", "add", "Test task", "--json"])
        .output()
        .unwrap();

    let task: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    // Verify all expected fields exist
    assert!(task["id"].is_string());
    assert!(task["title"].is_string());
    assert!(task["status"].is_string());
    assert!(task["schedule_type"].is_string());
    assert!(task["tags"].is_array());
    assert!(task["created_at"].is_string());
    assert!(task["updated_at"].is_string());
}

#[test]
fn test_list_with_filters() {
    let tmp = TempDir::new().unwrap();

    // Add tasks with different statuses and tags
    let out1 = flowstate(&tmp)
        .args(["task", "add", "Task A", "--tag", "alpha", "--json"])
        .output()
        .unwrap();
    let task_a: serde_json::Value = serde_json::from_slice(&out1.stdout).unwrap();
    let id_a = task_a["id"].as_str().unwrap();

    flowstate(&tmp)
        .args(["task", "add", "Task B", "--tag", "beta", "--json"])
        .output()
        .unwrap();

    // Mark A as done
    flowstate(&tmp)
        .args(["task", "done", id_a])
        .assert()
        .success();

    // List pending only
    let output = flowstate(&tmp)
        .args(["task", "list", "--status", "pending", "--json"])
        .output()
        .unwrap();
    let tasks: Vec<serde_json::Value> = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(tasks.len(), 1);
    assert_eq!(tasks[0]["title"], "Task B");

    // List by tag
    let output = flowstate(&tmp)
        .args(["task", "list", "--tag", "alpha", "--json"])
        .output()
        .unwrap();
    let tasks: Vec<serde_json::Value> = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(tasks.len(), 1);
    assert_eq!(tasks[0]["title"], "Task A");
}

#[test]
fn test_done_and_cancel() {
    let tmp = TempDir::new().unwrap();

    let out = flowstate(&tmp)
        .args(["task", "add", "To complete", "--json"])
        .output()
        .unwrap();
    let task: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let id = task["id"].as_str().unwrap();

    // Mark done
    let out = flowstate(&tmp)
        .args(["task", "done", id, "--json"])
        .output()
        .unwrap();
    assert!(out.status.success());
    let done_task: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(done_task["status"], "done");

    // Already done — should fail with exit code 3
    flowstate(&tmp)
        .args(["task", "done", id])
        .assert()
        .failure()
        .stderr(predicate::str::contains("already done"));
}

#[test]
fn test_cancel() {
    let tmp = TempDir::new().unwrap();

    let out = flowstate(&tmp)
        .args(["task", "add", "To cancel", "--json"])
        .output()
        .unwrap();
    let task: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let id = task["id"].as_str().unwrap();

    let out = flowstate(&tmp)
        .args(["task", "cancel", id, "--json"])
        .output()
        .unwrap();
    assert!(out.status.success());
    let cancelled: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(cancelled["status"], "cancelled");
}

#[test]
fn test_auto_complete_parent() {
    let tmp = TempDir::new().unwrap();

    // Create parent
    let out = flowstate(&tmp)
        .args(["task", "add", "Parent task", "--json"])
        .output()
        .unwrap();
    let parent: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let parent_id = parent["id"].as_str().unwrap().to_string();

    // Create children
    let out = flowstate(&tmp)
        .args(["task", "add", "Child 1", "--parent", &parent_id, "--json"])
        .output()
        .unwrap();
    let child1: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let child1_id = child1["id"].as_str().unwrap().to_string();

    let out = flowstate(&tmp)
        .args(["task", "add", "Child 2", "--parent", &parent_id, "--json"])
        .output()
        .unwrap();
    let child2: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let child2_id = child2["id"].as_str().unwrap().to_string();

    // Complete child 1
    flowstate(&tmp)
        .args(["task", "done", &child1_id])
        .assert()
        .success();

    // Parent should still be pending
    let out = flowstate(&tmp)
        .args(["task", "get", &parent_id, "--json"])
        .output()
        .unwrap();
    let parent_state: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(parent_state["status"], "pending");

    // Complete child 2
    flowstate(&tmp)
        .args(["task", "done", &child2_id])
        .assert()
        .success()
        .stdout(predicate::str::contains("auto-completed"));

    // Parent should now be done
    let out = flowstate(&tmp)
        .args(["task", "get", &parent_id, "--json"])
        .output()
        .unwrap();
    let parent_state: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(parent_state["status"], "done");
}

#[test]
fn test_no_auto_complete_flag() {
    let tmp = TempDir::new().unwrap();

    // Create parent + single child
    let out = flowstate(&tmp)
        .args(["task", "add", "Parent", "--json"])
        .output()
        .unwrap();
    let parent: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let parent_id = parent["id"].as_str().unwrap().to_string();

    let out = flowstate(&tmp)
        .args([
            "task",
            "add",
            "Only child",
            "--parent",
            &parent_id,
            "--json",
        ])
        .output()
        .unwrap();
    let child: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let child_id = child["id"].as_str().unwrap().to_string();

    // Complete child with --no-auto-complete
    flowstate(&tmp)
        .args(["task", "done", &child_id, "--no-auto-complete"])
        .assert()
        .success();

    // Parent should still be pending
    let out = flowstate(&tmp)
        .args(["task", "get", &parent_id, "--json"])
        .output()
        .unwrap();
    let parent_state: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(parent_state["status"], "pending");
}

#[test]
fn test_meta_tag_bypasses_auto_complete() {
    let tmp = TempDir::new().unwrap();

    // Create meta-tagged parent
    let out = flowstate(&tmp)
        .args(["task", "add", "Meta parent", "--tag", "meta", "--json"])
        .output()
        .unwrap();
    let parent: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let parent_id = parent["id"].as_str().unwrap().to_string();

    let out = flowstate(&tmp)
        .args(["task", "add", "Child", "--parent", &parent_id, "--json"])
        .output()
        .unwrap();
    let child: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let child_id = child["id"].as_str().unwrap().to_string();

    // Complete child
    flowstate(&tmp)
        .args(["task", "done", &child_id])
        .assert()
        .success();

    // Meta parent should NOT be auto-completed
    let out = flowstate(&tmp)
        .args(["task", "get", &parent_id, "--json"])
        .output()
        .unwrap();
    let parent_state: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(parent_state["status"], "pending");
}

#[test]
fn test_breakdown() {
    let tmp = TempDir::new().unwrap();

    let out = flowstate(&tmp)
        .args(["task", "add", "Parent", "--json"])
        .output()
        .unwrap();
    let parent: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let parent_id = parent["id"].as_str().unwrap().to_string();

    flowstate(&tmp)
        .args(["task", "add", "Sub 1", "--parent", &parent_id])
        .assert()
        .success();
    flowstate(&tmp)
        .args(["task", "add", "Sub 2", "--parent", &parent_id])
        .assert()
        .success();

    let out = flowstate(&tmp)
        .args(["task", "breakdown", &parent_id, "--json"])
        .output()
        .unwrap();
    let children: Vec<serde_json::Value> = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(children.len(), 2);
}

#[test]
fn test_agenda() {
    let tmp = TempDir::new().unwrap();

    // Add a daily task
    flowstate(&tmp)
        .args(["task", "add", "Daily standup", "--type", "daily"])
        .assert()
        .success();

    // Add a task due today
    flowstate(&tmp)
        .args([
            "task",
            "add",
            "Due today",
            "--type",
            "deadline",
            "--due",
            &chrono::Utc::now().format("%Y-%m-%d").to_string(),
        ])
        .assert()
        .success();

    let out = flowstate(&tmp).args(["agenda", "--json"]).output().unwrap();
    assert!(out.status.success());
    let tasks: Vec<serde_json::Value> = serde_json::from_slice(&out.stdout).unwrap();
    assert!(tasks.len() >= 2);
}

#[test]
fn test_overdue() {
    let tmp = TempDir::new().unwrap();

    // Add an overdue task
    flowstate(&tmp)
        .args([
            "task",
            "add",
            "Overdue task",
            "--type",
            "deadline",
            "--due",
            "2020-01-01",
        ])
        .assert()
        .success();

    // Add a future task
    flowstate(&tmp)
        .args([
            "task",
            "add",
            "Future task",
            "--type",
            "deadline",
            "--due",
            "2030-01-01",
        ])
        .assert()
        .success();

    let out = flowstate(&tmp)
        .args(["overdue", "--json"])
        .output()
        .unwrap();
    let tasks: Vec<serde_json::Value> = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(tasks.len(), 1);
    assert_eq!(tasks[0]["title"], "Overdue task");
}

#[test]
fn test_recurring_daily() {
    let tmp = TempDir::new().unwrap();

    let out = flowstate(&tmp)
        .args([
            "task",
            "add",
            "Daily standup",
            "--type",
            "daily",
            "--due",
            "2026-03-03T09:00:00Z",
            "--json",
        ])
        .output()
        .unwrap();
    let task: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let id = task["id"].as_str().unwrap();

    // Complete it — should generate next instance
    flowstate(&tmp)
        .args(["task", "done", id])
        .assert()
        .success()
        .stdout(predicate::str::contains("Next recurrence created"));

    // List pending — should have the new instance
    let out = flowstate(&tmp)
        .args(["task", "list", "--status", "pending", "--json"])
        .output()
        .unwrap();
    let tasks: Vec<serde_json::Value> = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(tasks.len(), 1);
    assert_eq!(tasks[0]["title"], "Daily standup");
    assert_eq!(tasks[0]["due_at"], "2026-03-04T09:00:00Z");
}

#[test]
fn test_nonexistent_task() {
    let tmp = TempDir::new().unwrap();

    flowstate(&tmp)
        .args(["task", "get", "tk_nonexist"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("not found"));
}

#[test]
fn test_invalid_status() {
    let tmp = TempDir::new().unwrap();

    flowstate(&tmp)
        .args(["task", "list", "--status", "bogus"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("invalid status"));
}

#[test]
fn test_update_task() {
    let tmp = TempDir::new().unwrap();

    let out = flowstate(&tmp)
        .args(["task", "add", "Original title", "--json"])
        .output()
        .unwrap();
    let task: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let id = task["id"].as_str().unwrap();

    let out = flowstate(&tmp)
        .args([
            "task",
            "update",
            id,
            "--title",
            "Updated title",
            "--status",
            "in_progress",
            "--json",
        ])
        .output()
        .unwrap();
    assert!(out.status.success());
    let updated: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(updated["title"], "Updated title");
    assert_eq!(updated["status"], "in_progress");
}

#[test]
fn test_plaintext_output() {
    let tmp = TempDir::new().unwrap();

    // Without --json, should get plaintext
    flowstate(&tmp)
        .args(["task", "add", "Plain task"])
        .assert()
        .success()
        .stdout(predicate::str::contains("[tk_"))
        .stdout(predicate::str::contains("Plain task"))
        .stdout(predicate::str::contains("(pending)"));
}

#[test]
fn test_list_empty() {
    let tmp = TempDir::new().unwrap();

    flowstate(&tmp)
        .args(["task", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("No tasks found"));
}

#[test]
fn test_list_by_schedule_type() {
    let tmp = TempDir::new().unwrap();

    flowstate(&tmp)
        .args(["task", "add", "Once task", "--type", "once"])
        .assert()
        .success();
    flowstate(&tmp)
        .args(["task", "add", "Daily task", "--type", "daily"])
        .assert()
        .success();

    let out = flowstate(&tmp)
        .args(["task", "list", "--type", "daily", "--json"])
        .output()
        .unwrap();
    let tasks: Vec<serde_json::Value> = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(tasks.len(), 1);
    assert_eq!(tasks[0]["title"], "Daily task");
}

#[test]
fn test_invalid_parent() {
    let tmp = TempDir::new().unwrap();

    flowstate(&tmp)
        .args(["task", "add", "Orphan", "--parent", "tk_nonexist"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("not found"));
}

#[test]
fn test_recurring_with_recur_rule() {
    let tmp = TempDir::new().unwrap();

    let out = flowstate(&tmp)
        .args([
            "task",
            "add",
            "Biweekly review",
            "--type",
            "recurring",
            "--recur",
            "every:2w",
            "--due",
            "2026-03-03T10:00:00Z",
            "--json",
        ])
        .output()
        .unwrap();
    let task: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let id = task["id"].as_str().unwrap();

    flowstate(&tmp)
        .args(["task", "done", id])
        .assert()
        .success()
        .stdout(predicate::str::contains("Next recurrence created"));

    let out = flowstate(&tmp)
        .args(["task", "list", "--status", "pending", "--json"])
        .output()
        .unwrap();
    let tasks: Vec<serde_json::Value> = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(tasks.len(), 1);
    assert_eq!(tasks[0]["due_at"], "2026-03-17T10:00:00Z");
}

#[test]
fn test_cancel_auto_completes_parent() {
    let tmp = TempDir::new().unwrap();

    // Parent with one child; cancel the child -> parent auto-completes
    let out = flowstate(&tmp)
        .args(["task", "add", "Parent", "--json"])
        .output()
        .unwrap();
    let parent: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let parent_id = parent["id"].as_str().unwrap().to_string();

    let out = flowstate(&tmp)
        .args(["task", "add", "Child", "--parent", &parent_id, "--json"])
        .output()
        .unwrap();
    let child: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let child_id = child["id"].as_str().unwrap().to_string();

    flowstate(&tmp)
        .args(["task", "cancel", &child_id])
        .assert()
        .success()
        .stdout(predicate::str::contains("auto-completed"));

    let out = flowstate(&tmp)
        .args(["task", "get", &parent_id, "--json"])
        .output()
        .unwrap();
    let p: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(p["status"], "done");
}
