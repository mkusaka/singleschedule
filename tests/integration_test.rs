use singleschedule::{cli, storage::Storage};
use std::env;
use std::process::Command;
use tempfile::TempDir;

#[tokio::test]
async fn test_end_to_end_workflow() {
    let temp_dir = TempDir::new().unwrap();
    unsafe {
        env::set_var("SINGLESCHEDULE_TEST_HOME", temp_dir.path());
    }

    // Add a task
    cli::handle_add(
        "test-task".to_string(),
        "* * * * * *".to_string(),
        vec!["echo".to_string(), "hello world".to_string()],
    )
    .await
    .unwrap();

    // Verify it was added
    let storage = Storage::load().await.unwrap();
    assert_eq!(storage.events.len(), 1);
    assert_eq!(storage.events[0].slug, "test-task");

    // List tasks
    cli::handle_list().await.unwrap();

    // Remove the task
    cli::handle_remove("test-task".to_string()).await.unwrap();

    // Verify it was removed
    let storage = Storage::load().await.unwrap();
    assert_eq!(storage.events.len(), 0);
}

#[tokio::test]
async fn test_multiple_tasks() {
    let temp_dir = TempDir::new().unwrap();
    unsafe {
        env::set_var("SINGLESCHEDULE_TEST_HOME", temp_dir.path());
    }

    // Add multiple tasks
    for i in 1..=5 {
        cli::handle_add(
            format!("task-{}", i),
            "0 * * * * *".to_string(),
            vec!["echo".to_string(), format!("task {}", i)],
        )
        .await
        .unwrap();
    }

    // Verify all were added
    let storage = Storage::load().await.unwrap();
    assert_eq!(storage.events.len(), 5);

    // Remove specific task
    cli::handle_remove("task-3".to_string()).await.unwrap();

    let storage = Storage::load().await.unwrap();
    assert_eq!(storage.events.len(), 4);
    assert!(!storage.events.iter().any(|e| e.slug == "task-3"));
}

#[tokio::test]
async fn test_complex_commands() {
    let temp_dir = TempDir::new().unwrap();
    unsafe {
        env::set_var("SINGLESCHEDULE_TEST_HOME", temp_dir.path());
    }

    // Test command with multiple arguments and flags
    cli::handle_add(
        "complex-command".to_string(),
        "*/30 * * * * *".to_string(),
        vec![
            "curl".to_string(),
            "-X".to_string(),
            "POST".to_string(),
            "-H".to_string(),
            "Content-Type: application/json".to_string(),
            "-d".to_string(),
            "{\"status\": \"ok\"}".to_string(),
            "http://example.com/webhook".to_string(),
        ],
    )
    .await
    .unwrap();

    let storage = Storage::load().await.unwrap();
    assert_eq!(storage.events.len(), 1);
    assert!(storage.events[0].command.contains("curl"));
    assert!(storage.events[0].command.contains("POST"));
    assert!(storage.events[0].command.contains("example.com"));
}

#[test]
fn test_cli_binary() {
    // Build the binary first
    let output = Command::new("cargo")
        .args(&["build", "--quiet"])
        .output()
        .expect("Failed to build");

    assert!(output.status.success(), "Build failed");

    // Test help command
    let output = Command::new("cargo")
        .args(&["run", "--quiet", "--", "--help"])
        .output()
        .expect("Failed to run help");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("singleschedule"));
    assert!(stdout.contains("add"));
    assert!(stdout.contains("remove"));
    assert!(stdout.contains("list"));
}

