use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tokio::fs;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub slug: String,
    pub cron: String,
    pub command: String,
    pub pid: Option<u32>,
    pub created_at: DateTime<Utc>,
    pub last_run: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Storage {
    pub events: Vec<Event>,
}

impl Default for Storage {
    fn default() -> Self {
        Self::new()
    }
}

impl Storage {
    pub fn new() -> Self {
        Storage { events: Vec::new() }
    }

    pub async fn load() -> Result<Self> {
        let path = Self::get_path()?;

        if !path.exists() {
            // Create directory if it doesn't exist
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).await?;
            }
            return Ok(Self::new());
        }

        let content = fs::read_to_string(&path).await?;
        let storage: Storage = serde_json::from_str(&content)?;
        Ok(storage)
    }

    pub async fn save(&self) -> Result<()> {
        let path = Self::get_path()?;

        // Create directory if it doesn't exist
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).await?;
        }

        let content = serde_json::to_string_pretty(self)?;
        fs::write(&path, content).await?;
        Ok(())
    }

    fn get_path() -> Result<PathBuf> {
        #[cfg(test)]
        {
            if let Ok(test_home) = std::env::var("SINGLESCHEDULE_TEST_HOME") {
                return Ok(PathBuf::from(test_home)
                    .join(".singleschedule")
                    .join("events.json"));
            }
        }

        let home = directories::UserDirs::new()
            .ok_or_else(|| anyhow::anyhow!("Failed to get home directory"))?
            .home_dir()
            .to_path_buf();

        Ok(home.join(".singleschedule").join("events.json"))
    }
}
