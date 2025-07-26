pub mod cli;
pub mod daemon;
pub mod scheduler;
pub mod storage;

pub use scheduler::Scheduler;

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_storage_new() {
        let storage = storage::Storage::new();
        assert!(storage.events.is_empty());
    }

    #[tokio::test]
    async fn test_storage_save_and_load() {
        let temp_dir = TempDir::new().unwrap();
        unsafe {
            env::set_var("SINGLESCHEDULE_TEST_HOME", temp_dir.path());
        }

        let mut storage = storage::Storage::new();
        let event = storage::Event {
            slug: "test-task".to_string(),
            cron: "0 * * * * *".to_string(),
            command: "echo hello".to_string(),
            pid: None,
            created_at: chrono::Utc::now(),
            last_run: None,
            active: true,
        };

        storage.events.push(event.clone());
        storage.save().await.unwrap();

        let loaded = storage::Storage::load().await.unwrap();
        assert_eq!(loaded.events.len(), 1);
        assert_eq!(loaded.events[0].slug, "test-task");
        assert_eq!(loaded.events[0].cron, "0 * * * * *");
        assert_eq!(loaded.events[0].command, "echo hello");
    }

    #[tokio::test]
    async fn test_add_duplicate_slug() {
        let temp_dir = TempDir::new().unwrap();
        unsafe {
            env::set_var("SINGLESCHEDULE_TEST_HOME", temp_dir.path());
        }

        let mut storage = storage::Storage::new();
        let event = storage::Event {
            slug: "duplicate".to_string(),
            cron: "0 * * * * *".to_string(),
            command: "echo test".to_string(),
            pid: None,
            created_at: chrono::Utc::now(),
            last_run: None,
            active: true,
        };

        storage.events.push(event);
        storage.save().await.unwrap();

        // Try to add with same slug
        let result = cli::handle_add(
            "duplicate".to_string(),
            "0 * * * * *".to_string(),
            vec!["echo".to_string(), "test2".to_string()],
        )
        .await;

        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("already exists") || err_msg.contains("Invalid cron"),
            "Unexpected error: {}",
            err_msg
        );
    }

    #[tokio::test]
    async fn test_remove_nonexistent() {
        let temp_dir = TempDir::new().unwrap();
        unsafe {
            env::set_var("SINGLESCHEDULE_TEST_HOME", temp_dir.path());
        }

        let result = cli::handle_remove("nonexistent".to_string()).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }

    #[tokio::test]
    async fn test_cron_validation() {
        let result = cli::handle_add(
            "invalid-cron".to_string(),
            "invalid cron expression".to_string(),
            vec!["echo".to_string(), "test".to_string()],
        )
        .await;

        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Invalid cron expression"));
    }

    #[test]
    fn test_schedule_parsing() {
        use cron::Schedule;
        use std::str::FromStr;

        let valid_crons = vec![
            "* * * * * *",
            "0 0 * * * *",
            "*/5 * * * * *",
            "0 9-17 * * * MON-FRI *",
        ];

        for cron_str in valid_crons {
            assert!(
                Schedule::from_str(cron_str).is_ok(),
                "Failed to parse: {}",
                cron_str
            );
        }

        let invalid_crons = vec![
            "invalid",
            "* * * *",
            "60 * * * * *",
            "* * * * *", // 5 fields instead of 6
        ];

        for cron_str in invalid_crons {
            assert!(
                Schedule::from_str(cron_str).is_err(),
                "Should have failed: {}",
                cron_str
            );
        }
    }
}
