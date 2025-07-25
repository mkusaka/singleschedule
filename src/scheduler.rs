use anyhow::Result;
use chrono::{DateTime, Utc};
use cron::Schedule;
use log::{debug, error, info};
use std::collections::HashMap;
use std::process::Stdio;
use std::str::FromStr;
use std::sync::Arc;
use tokio::process::Command;
use tokio::sync::Mutex;
use tokio::time::{self, Duration};

use crate::storage::Storage;

pub struct Scheduler {
    storage: Arc<Mutex<Storage>>,
    schedules: HashMap<String, Schedule>,
}

impl Default for Scheduler {
    fn default() -> Self {
        Self::new()
    }
}

impl Scheduler {
    pub fn new() -> Self {
        Scheduler {
            storage: Arc::new(Mutex::new(Storage::new())),
            schedules: HashMap::new(),
        }
    }

    pub async fn load_events(&mut self) -> Result<()> {
        let storage = Storage::load().await?;

        // Parse cron expressions
        for event in &storage.events {
            match Schedule::from_str(&event.cron) {
                Ok(schedule) => {
                    self.schedules.insert(event.slug.clone(), schedule);
                    info!("Loaded schedule for task '{}'", event.slug);
                }
                Err(e) => {
                    error!(
                        "Failed to parse cron expression for task '{}': {}",
                        event.slug, e
                    );
                }
            }
        }

        *self.storage.lock().await = storage;
        Ok(())
    }

    pub async fn run(&mut self) -> Result<()> {
        info!("Scheduler running");

        // Check every 10 seconds since cron expressions support seconds
        let mut interval = time::interval(Duration::from_secs(10));

        loop {
            interval.tick().await;

            // Reload events in case they changed
            if let Err(e) = self.reload_events().await {
                error!("Failed to reload events: {e}");
            }

            let now = Utc::now();
            self.check_and_run_tasks(now).await;
        }
    }

    async fn reload_events(&mut self) -> Result<()> {
        let storage = Storage::load().await?;
        let mut schedules = HashMap::new();

        for event in &storage.events {
            match Schedule::from_str(&event.cron) {
                Ok(schedule) => {
                    schedules.insert(event.slug.clone(), schedule);
                }
                Err(e) => {
                    error!(
                        "Failed to parse cron expression for task '{}': {}",
                        event.slug, e
                    );
                }
            }
        }

        self.schedules = schedules;
        *self.storage.lock().await = storage;
        Ok(())
    }

    async fn check_and_run_tasks(&self, now: DateTime<Utc>) {
        let mut storage = self.storage.lock().await;
        let mut tasks_to_update = Vec::new();

        for (idx, event) in storage.events.iter().enumerate() {
            if let Some(schedule) = self.schedules.get(&event.slug) {
                if self.should_run(schedule, &event.last_run, now) {
                    info!("Running task '{}'", event.slug);

                    match self.run_command(&event.command).await {
                        Ok(output) => {
                            if output.success {
                                info!("Task '{}' completed successfully", event.slug);
                            } else {
                                error!("Task '{}' failed with exit code", event.slug);
                            }

                            // Mark task for update
                            tasks_to_update.push(idx);
                        }
                        Err(e) => {
                            error!("Failed to run task '{}': {}", event.slug, e);
                        }
                    }
                }
            }
        }

        // Update last run times for executed tasks
        let should_save = !tasks_to_update.is_empty();

        for idx in tasks_to_update {
            storage.events[idx].last_run = Some(now);
        }

        // Save storage once after all updates
        if should_save {
            if let Err(e) = storage.save().await {
                error!("Failed to save storage: {e}");
            }
        }
    }

    fn should_run(
        &self,
        schedule: &Schedule,
        last_run: &Option<DateTime<Utc>>,
        now: DateTime<Utc>,
    ) -> bool {
        // Get the next scheduled time after the last run (or epoch if never run)
        let last = last_run.unwrap_or_else(|| DateTime::from_timestamp(0, 0).unwrap());

        // Check if there's a scheduled time between last run and now
        if let Some(next) = schedule.after(&last).next() {
            // Allow 1 minute tolerance for missed schedules
            next <= now + chrono::Duration::seconds(30)
        } else {
            false
        }
    }

    async fn run_command(&self, command: &str) -> Result<CommandOutput> {
        debug!("Executing command: {command}");

        // Split command into program and args
        let parts: Vec<&str> = command.split_whitespace().collect();
        if parts.is_empty() {
            return Err(anyhow::anyhow!("Empty command"));
        }

        let output = Command::new(parts[0])
            .args(&parts[1..])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?
            .wait_with_output()
            .await?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        if !stdout.is_empty() {
            debug!("Command stdout: {stdout}");
        }
        if !stderr.is_empty() {
            debug!("Command stderr: {stderr}");
        }

        Ok(CommandOutput {
            success: output.status.success(),
            _stdout: stdout.to_string(),
            _stderr: stderr.to_string(),
        })
    }
}

struct CommandOutput {
    success: bool,
    _stdout: String,
    _stderr: String,
}

