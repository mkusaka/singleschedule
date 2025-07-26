use super::state::{AppState, AppMode, NewTaskInput};
use crate::storage::Event;
use chrono::Utc;

#[cfg(test)]
fn create_test_event(slug: &str, active: bool) -> Event {
    Event {
        slug: slug.to_string(),
        cron: "0 0 * * * *".to_string(),  // Seconds Minutes Hours DayOfMonth Month DayOfWeek 
        command: "echo test".to_string(),
        pid: None,
        created_at: Utc::now(),
        last_run: None,
        active,
    }
}

#[cfg(test)]
mod state_tests {
    use super::*;

    #[test]
    fn test_default_app_state() {
        let state = AppState::default();
        assert!(state.tasks.is_empty());
        assert_eq!(state.selected_index, 0);
        assert_eq!(state.mode, AppMode::Normal);
        assert!(state.message.is_none());
        assert_eq!(state.new_task, NewTaskInput::default());
    }

    #[test]
    fn test_move_selection_up() {
        let mut state = AppState {
            tasks: vec![
                create_test_event("task1", true),
                create_test_event("task2", true),
                create_test_event("task3", true),
            ],
            selected_index: 2,
            ..Default::default()
        };

        state.move_selection_up();
        assert_eq!(state.selected_index, 1);

        state.move_selection_up();
        assert_eq!(state.selected_index, 0);

        // Should not go below 0
        state.move_selection_up();
        assert_eq!(state.selected_index, 0);
    }

    #[test]
    fn test_move_selection_down() {
        let mut state = AppState {
            tasks: vec![
                create_test_event("task1", true),
                create_test_event("task2", true),
                create_test_event("task3", true),
            ],
            selected_index: 0,
            ..Default::default()
        };

        state.move_selection_down();
        assert_eq!(state.selected_index, 1);

        state.move_selection_down();
        assert_eq!(state.selected_index, 2);

        // Should not go beyond last index
        state.move_selection_down();
        assert_eq!(state.selected_index, 2);
    }

    #[test]
    fn test_toggle_selected_task_active() {
        let mut state = AppState {
            tasks: vec![
                create_test_event("task1", true),
                create_test_event("task2", false),
            ],
            selected_index: 0,
            ..Default::default()
        };

        // Toggle first task
        assert_eq!(state.tasks[0].active, true);
        state.toggle_selected_task_active();
        assert_eq!(state.tasks[0].active, false);

        // Toggle it back
        state.toggle_selected_task_active();
        assert_eq!(state.tasks[0].active, true);

        // Toggle second task
        state.selected_index = 1;
        assert_eq!(state.tasks[1].active, false);
        state.toggle_selected_task_active();
        assert_eq!(state.tasks[1].active, true);
    }

    #[test]
    fn test_remove_selected_task() {
        let mut state = AppState {
            tasks: vec![
                create_test_event("task1", true),
                create_test_event("task2", true),
                create_test_event("task3", true),
            ],
            selected_index: 1,
            ..Default::default()
        };

        // Remove middle task
        state.remove_selected_task();
        assert_eq!(state.tasks.len(), 2);
        assert_eq!(state.tasks[0].slug, "task1");
        assert_eq!(state.tasks[1].slug, "task3");
        assert_eq!(state.selected_index, 1);

        // Remove last task
        state.remove_selected_task();
        assert_eq!(state.tasks.len(), 1);
        assert_eq!(state.tasks[0].slug, "task1");
        assert_eq!(state.selected_index, 0);

        // Remove remaining task
        state.remove_selected_task();
        assert_eq!(state.tasks.len(), 0);
        assert_eq!(state.selected_index, 0);
    }

    #[test]
    fn test_get_selected_task() {
        let state = AppState {
            tasks: vec![
                create_test_event("task1", true),
                create_test_event("task2", true),
            ],
            selected_index: 1,
            ..Default::default()
        };

        let selected = state.get_selected_task();
        assert!(selected.is_some());
        assert_eq!(selected.unwrap().slug, "task2");

        // Test with empty tasks
        let empty_state = AppState::default();
        assert!(empty_state.get_selected_task().is_none());
    }
}

#[cfg(test)]
mod new_task_input_tests {
    use super::*;

    #[test]
    fn test_default_new_task_input() {
        let input = NewTaskInput::default();
        assert_eq!(input.slug, "");
        assert_eq!(input.cron, "");
        assert_eq!(input.command, "");
        assert_eq!(input.current_field, 0);
    }

    #[test]
    fn test_handle_char() {
        let mut input = NewTaskInput::default();

        // Add to slug field
        input.handle_char('t');
        input.handle_char('e');
        input.handle_char('s');
        input.handle_char('t');
        assert_eq!(input.slug, "test");

        // Add to cron field
        input.current_field = 1;
        input.handle_char('*');
        input.handle_char(' ');
        input.handle_char('*');
        assert_eq!(input.cron, "* *");

        // Add to command field
        input.current_field = 2;
        input.handle_char('e');
        input.handle_char('c');
        input.handle_char('h');
        input.handle_char('o');
        assert_eq!(input.command, "echo");
    }

    #[test]
    fn test_handle_backspace() {
        let mut input = NewTaskInput {
            slug: "test".to_string(),
            cron: "* * * * *".to_string(),
            command: "echo test".to_string(),
            current_field: 0,
        };

        // Remove from slug
        input.handle_backspace();
        assert_eq!(input.slug, "tes");

        // Remove from cron
        input.current_field = 1;
        input.handle_backspace();
        assert_eq!(input.cron, "* * * * ");

        // Remove from command
        input.current_field = 2;
        input.handle_backspace();
        input.handle_backspace();
        assert_eq!(input.command, "echo te");

        // Test backspace on empty field
        input.slug = "".to_string();
        input.current_field = 0;
        input.handle_backspace(); // Should not panic
        assert_eq!(input.slug, "");
    }

    #[test]
    fn test_create_task_valid() {
        let input = NewTaskInput {
            slug: "test-task".to_string(),
            cron: "0 0 * * * *".to_string(),
            command: "echo hello".to_string(),
            current_field: 0,
        };

        let task = input.create_task();
        assert!(task.is_some());

        let task = task.unwrap();
        assert_eq!(task.slug, "test-task");
        assert_eq!(task.cron, "0 0 * * * *");
        assert_eq!(task.command, "echo hello");
        assert!(task.pid.is_none());
        assert!(task.last_run.is_none());
        assert_eq!(task.active, true);
    }

    #[test]
    fn test_create_task_invalid() {
        // Empty slug
        let input = NewTaskInput {
            slug: "".to_string(),
            cron: "0 0 * * * *".to_string(),
            command: "echo test".to_string(),
            current_field: 0,
        };
        assert!(input.create_task().is_none());

        // Empty cron
        let input = NewTaskInput {
            slug: "test".to_string(),
            cron: "".to_string(),
            command: "echo test".to_string(),
            current_field: 0,
        };
        assert!(input.create_task().is_none());

        // Empty command
        let input = NewTaskInput {
            slug: "test".to_string(),
            cron: "0 0 * * * *".to_string(),
            command: "".to_string(),
            current_field: 0,
        };
        assert!(input.create_task().is_none());

        // Invalid cron expression
        let input = NewTaskInput {
            slug: "test".to_string(),
            cron: "invalid cron".to_string(),
            command: "echo test".to_string(),
            current_field: 0,
        };
        assert!(input.create_task().is_none());
    }
}

#[cfg(test)]
mod app_mode_tests {
    use super::*;

    #[test]
    fn test_app_mode_equality() {
        assert_eq!(AppMode::Normal, AppMode::Normal);
        assert_eq!(AppMode::AddingTask, AppMode::AddingTask);
        assert_eq!(AppMode::ConfirmDelete(5), AppMode::ConfirmDelete(5));
        assert_ne!(AppMode::ConfirmDelete(5), AppMode::ConfirmDelete(6));
        assert_ne!(AppMode::Normal, AppMode::AddingTask);
    }

    #[test]
    fn test_app_mode_clone() {
        let mode1 = AppMode::ConfirmDelete(10);
        let mode2 = mode1.clone();
        assert_eq!(mode1, mode2);
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;
    use tokio::test;

    #[test]
    async fn test_state_save_and_load() {
        // Set up temporary directory for test
        let temp_dir = tempfile::TempDir::new().unwrap();
        unsafe {
            std::env::set_var("SINGLESCHEDULE_TEST_HOME", temp_dir.path());
        }

        // Create a state with some tasks
        let state = AppState {
            tasks: vec![
                create_test_event("task1", true),
                create_test_event("task2", false),
            ],
            selected_index: 1,
            mode: AppMode::Normal,
            message: Some("Test message".to_string()),
            new_task: NewTaskInput::default(),
        };

        // Save state
        let result = state.save_to_storage().await;
        assert!(result.is_ok());

        // Load state
        let loaded_state = AppState::load_from_storage().await;
        assert!(loaded_state.is_ok());

        let loaded_state = loaded_state.unwrap();
        assert_eq!(loaded_state.tasks.len(), state.tasks.len());
        
        // Check that tasks were saved correctly
        for (i, task) in loaded_state.tasks.iter().enumerate() {
            assert_eq!(task.slug, state.tasks[i].slug);
            assert_eq!(task.active, state.tasks[i].active);
            assert_eq!(task.cron, state.tasks[i].cron);
            assert_eq!(task.command, state.tasks[i].command);
        }
    }
}