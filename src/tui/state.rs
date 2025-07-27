use crate::storage::{Event, Storage};
use std::fmt::{self, Display, Formatter};
use std::str::FromStr;

#[derive(Debug, Clone, PartialEq)]
pub struct AppState {
    pub tasks: Vec<Event>,
    pub selected_index: usize,
    pub mode: AppMode,
    pub message: Option<String>,
    pub new_task: NewTaskInput,
    pub show_add_dialog: bool,
    pub show_delete_dialog: bool,
}

// Alias for R3BL TUI compatibility
pub type State = AppState;

// App signals for R3BL TUI event handling
#[derive(Debug, Clone, Default)]
#[allow(dead_code)]
pub enum AppSignal {
    #[default]
    RefreshTasks,
    SaveState,
    ToggleTask(usize),
    DeleteTask(usize),
    AddTask(Event),
    CloseDialog,
    ShowMessage(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum AppMode {
    Normal,
    #[cfg(test)]
    AddingTask,
    #[cfg(test)]
    ConfirmDelete(usize),
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct NewTaskInput {
    pub slug: String,
    pub cron: String,
    pub command: String,
    pub current_field: usize, // 0 = slug, 1 = cron, 2 = command
}

impl NewTaskInput {
    pub fn handle_char(&mut self, c: char) {
        match self.current_field {
            0 => self.slug.push(c),
            1 => self.cron.push(c),
            2 => self.command.push(c),
            _ => {}
        }
    }

    pub fn handle_backspace(&mut self) {
        match self.current_field {
            0 => {
                self.slug.pop();
            }
            1 => {
                self.cron.pop();
            }
            2 => {
                self.command.pop();
            }
            _ => {}
        }
    }

    pub fn create_task(&self) -> Option<Event> {
        if self.slug.is_empty() || self.cron.is_empty() || self.command.is_empty() {
            return None;
        }

        // Validate cron expression
        if cron::Schedule::from_str(&self.cron).is_err() {
            return None;
        }

        Some(Event {
            slug: self.slug.clone(),
            cron: self.cron.clone(),
            command: self.command.clone(),
            pid: None,
            created_at: chrono::Utc::now(),
            last_run: None,
            active: true,
        })
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            tasks: vec![],
            selected_index: 0,
            mode: AppMode::Normal,
            message: None,
            new_task: NewTaskInput::default(),
            show_add_dialog: false,
            show_delete_dialog: false,
        }
    }
}

impl Display for AppState {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "AppState {{ tasks: {} tasks, selected: {}, mode: {:?} }}",
            self.tasks.len(),
            self.selected_index,
            self.mode
        )
    }
}

impl AppState {
    pub async fn load_from_storage() -> anyhow::Result<Self> {
        let storage = Storage::load().await?;
        Ok(Self {
            tasks: storage.events,
            ..Default::default()
        })
    }

    pub async fn save_to_storage(&self) -> anyhow::Result<()> {
        let storage = Storage {
            events: self.tasks.clone(),
        };
        storage.save().await
    }

    #[cfg(test)]
    pub fn get_selected_task(&self) -> Option<&Event> {
        self.tasks.get(self.selected_index)
    }

    #[cfg(test)]
    pub fn get_selected_task_mut(&mut self) -> Option<&mut Event> {
        self.tasks.get_mut(self.selected_index)
    }

    #[cfg(test)]
    pub fn move_selection_up(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
        }
    }

    #[cfg(test)]
    pub fn move_selection_down(&mut self) {
        if self.selected_index < self.tasks.len().saturating_sub(1) {
            self.selected_index += 1;
        }
    }

    #[cfg(test)]
    pub fn toggle_selected_task_active(&mut self) {
        if let Some(task) = self.get_selected_task_mut() {
            task.active = !task.active;
        }
    }

    #[cfg(test)]
    pub fn remove_selected_task(&mut self) {
        if self.selected_index < self.tasks.len() {
            self.tasks.remove(self.selected_index);
            if self.selected_index > 0 && self.selected_index >= self.tasks.len() {
                self.selected_index = self.tasks.len().saturating_sub(1);
            }
        }
    }
}
