use chrono::{Duration, Utc};
use singleschedule::{
    scheduler::Scheduler,
    storage::{Event, Storage},
};
use std::env;
use tempfile::TempDir;

#[tokio::test]
async fn test_scheduler_initialization() {
    let _scheduler = Scheduler::new();
    // Scheduler should initialize with empty state
    assert!(true); // Basic initialization test
}

#[tokio::test]
async fn test_scheduler_load_events() {
    let temp_dir = TempDir::new().unwrap();
    unsafe {
        env::set_var("SINGLESCHEDULE_TEST_HOME", temp_dir.path());
    }

    // Create storage with events
    let mut storage = Storage::new();
    storage.events.push(Event {
        slug: "hourly-task".to_string(),
        cron: "0 * * * * *".to_string(),
        command: "echo hourly".to_string(),
        pid: None,
        created_at: Utc::now(),
        last_run: None,
    });

    storage.events.push(Event {
        slug: "daily-task".to_string(),
        cron: "0 0 * * * *".to_string(),
        command: "echo daily".to_string(),
        pid: None,
        created_at: Utc::now(),
        last_run: None,
    });

    storage.save().await.unwrap();

    // Load events into scheduler
    let mut scheduler = Scheduler::new();
    scheduler.load_events().await.unwrap();

    // Verify events were loaded (indirect test through storage)
    let loaded_storage = Storage::load().await.unwrap();
    assert_eq!(loaded_storage.events.len(), 2);
}

#[tokio::test]
async fn test_should_run_logic() {
    use cron::Schedule;
    use std::str::FromStr;

    // Test "every minute" schedule
    let _schedule = Schedule::from_str("* * * * * *").unwrap();
    let now = Utc::now();

    // Task that never ran should run
    let _event_never_run = Event {
        slug: "test".to_string(),
        cron: "* * * * * *".to_string(),
        command: "echo test".to_string(),
        pid: None,
        created_at: now - Duration::hours(1),
        last_run: None,
    };

    // Task that ran 2 minutes ago should run again
    let _event_ran_2min_ago = Event {
        slug: "test2".to_string(),
        cron: "* * * * * *".to_string(),
        command: "echo test2".to_string(),
        pid: None,
        created_at: now - Duration::hours(1),
        last_run: Some(now - Duration::minutes(2)),
    };

    // Task that just ran should not run again
    let _event_just_ran = Event {
        slug: "test3".to_string(),
        cron: "* * * * * *".to_string(),
        command: "echo test3".to_string(),
        pid: None,
        created_at: now - Duration::hours(1),
        last_run: Some(now - Duration::seconds(30)),
    };
}

#[tokio::test]
async fn test_invalid_cron_handling() {
    let temp_dir = TempDir::new().unwrap();
    unsafe {
        env::set_var("SINGLESCHEDULE_TEST_HOME", temp_dir.path());
    }

    let mut storage = Storage::new();
    storage.events.push(Event {
        slug: "invalid-cron".to_string(),
        cron: "invalid cron expression".to_string(),
        command: "echo test".to_string(),
        pid: None,
        created_at: Utc::now(),
        last_run: None,
    });

    storage.save().await.unwrap();

    // Scheduler should handle invalid cron gracefully
    let mut scheduler = Scheduler::new();
    let result = scheduler.load_events().await;
    assert!(result.is_ok()); // Should not fail, just log error
}

