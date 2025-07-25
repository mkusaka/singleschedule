use anyhow::Result;
use clap::{Parser, Subcommand};
use std::str::FromStr;

use crate::storage::{Event, Storage};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Add a new scheduled task
    Add {
        /// Unique identifier for the task
        #[arg(short, long)]
        slug: String,

        /// Cron expression for scheduling
        #[arg(short, long)]
        cron: String,

        /// Command to execute (everything after --)
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        command: Vec<String>,
    },

    /// Remove a scheduled task
    Remove {
        /// Slug of the task to remove
        #[arg(short, long)]
        slug: String,
    },

    /// List all scheduled tasks
    List,

    /// Start the scheduler daemon
    Start,

    /// Stop the scheduler daemon
    Stop,
}

pub async fn handle_add(slug: String, cron_expr: String, command: Vec<String>) -> Result<()> {
    // Validate cron expression
    let _schedule = cron::Schedule::from_str(&cron_expr)
        .map_err(|e| anyhow::anyhow!("Invalid cron expression: {}", e))?;

    let mut storage = Storage::load().await?;

    // Check if slug already exists
    if storage.events.iter().any(|e| e.slug == slug) {
        return Err(anyhow::anyhow!("Task with slug '{}' already exists", slug));
    }

    let event = Event {
        slug: slug.clone(),
        cron: cron_expr,
        command: command.join(" "),
        pid: None,
        created_at: chrono::Utc::now(),
        last_run: None,
    };

    storage.events.push(event);
    storage.save().await?;

    println!("Task '{slug}' added successfully");

    // Restart daemon to pick up new task
    if let Err(e) = crate::daemon::restart_daemon().await {
        eprintln!("Warning: Failed to restart daemon: {e}");
        eprintln!("Please restart the daemon manually with 'singleschedule start'");
    }

    Ok(())
}

pub async fn handle_remove(slug: String) -> Result<()> {
    let mut storage = Storage::load().await?;

    let initial_count = storage.events.len();
    storage.events.retain(|e| e.slug != slug);

    if storage.events.len() == initial_count {
        return Err(anyhow::anyhow!("Task with slug '{}' not found", slug));
    }

    storage.save().await?;
    println!("Task '{slug}' removed successfully");

    // Restart daemon to update tasks
    if let Err(e) = crate::daemon::restart_daemon().await {
        eprintln!("Warning: Failed to restart daemon: {e}");
        eprintln!("Please restart the daemon manually with 'singleschedule start'");
    }

    Ok(())
}

pub async fn handle_list() -> Result<()> {
    let storage = Storage::load().await?;

    if storage.events.is_empty() {
        println!("No scheduled tasks");
        return Ok(());
    }

    println!(
        "{:<20} {:<20} {:<40} {:<15}",
        "SLUG", "CRON", "COMMAND", "LAST RUN"
    );
    println!("{}", "-".repeat(95));

    for event in &storage.events {
        let last_run = event
            .last_run
            .map(|dt| dt.format("%Y-%m-%d %H:%M").to_string())
            .unwrap_or_else(|| "Never".to_string());

        let command = if event.command.len() > 37 {
            format!("{}...", &event.command[..37])
        } else {
            event.command.clone()
        };

        println!(
            "{:<20} {:<20} {:<40} {:<15}",
            event.slug, event.cron, command, last_run
        );
    }

    Ok(())
}
