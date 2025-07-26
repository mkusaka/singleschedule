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

#[tokio::test]
async fn test_selective_start_stop() {
    let temp_dir = TempDir::new().unwrap();
    unsafe {
        env::set_var("SINGLESCHEDULE_TEST_HOME", temp_dir.path());
    }

    // Add multiple tasks
    for i in 1..=3 {
        cli::handle_add(
            format!("task-{}", i),
            "0 * * * * *".to_string(),
            vec!["echo".to_string(), format!("task {}", i)],
        )
        .await
        .unwrap();
    }

    // All tasks should be active initially
    let storage = Storage::load().await.unwrap();
    assert_eq!(storage.events.len(), 3);
    assert!(storage.events.iter().all(|e| e.active));

    // Stop specific tasks
    cli::handle_stop(vec!["task-1".to_string(), "task-3".to_string()], false)
        .await
        .unwrap();

    // Check that only task-2 is active
    let storage = Storage::load().await.unwrap();
    assert!(
        !storage
            .events
            .iter()
            .find(|e| e.slug == "task-1")
            .unwrap()
            .active
    );
    assert!(
        storage
            .events
            .iter()
            .find(|e| e.slug == "task-2")
            .unwrap()
            .active
    );
    assert!(
        !storage
            .events
            .iter()
            .find(|e| e.slug == "task-3")
            .unwrap()
            .active
    );

    // Start task-1 only
    cli::handle_start(vec!["task-1".to_string()], false)
        .await
        .unwrap();

    // Check that task-1 and task-2 are active
    let storage = Storage::load().await.unwrap();
    assert!(
        storage
            .events
            .iter()
            .find(|e| e.slug == "task-1")
            .unwrap()
            .active
    );
    assert!(
        storage
            .events
            .iter()
            .find(|e| e.slug == "task-2")
            .unwrap()
            .active
    );
    assert!(
        !storage
            .events
            .iter()
            .find(|e| e.slug == "task-3")
            .unwrap()
            .active
    );

    // Start all tasks using --all flag
    cli::handle_start(vec![], true).await.unwrap();

    // Check that all tasks are active
    let storage = Storage::load().await.unwrap();
    assert!(storage.events.iter().all(|e| e.active));
}

#[tokio::test]
async fn test_start_stop_nonexistent_task() {
    let temp_dir = TempDir::new().unwrap();
    unsafe {
        env::set_var("SINGLESCHEDULE_TEST_HOME", temp_dir.path());
    }

    // Add a task
    cli::handle_add(
        "existing-task".to_string(),
        "0 * * * * *".to_string(),
        vec!["echo".to_string(), "hello".to_string()],
    )
    .await
    .unwrap();

    // Try to stop a non-existent task along with existing one
    cli::handle_stop(
        vec!["existing-task".to_string(), "nonexistent".to_string()],
        false,
    )
    .await
    .unwrap();

    // The existing task should be stopped
    let storage = Storage::load().await.unwrap();
    assert!(
        !storage
            .events
            .iter()
            .find(|e| e.slug == "existing-task")
            .unwrap()
            .active
    );

    // Try to start only non-existent tasks - should fail
    let result = cli::handle_start(
        vec!["nonexistent1".to_string(), "nonexistent2".to_string()],
        false,
    )
    .await;
    assert!(result.is_err());
}
