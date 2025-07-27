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
    Start {
        /// Slugs of tasks to start (if not specified, starts daemon for all tasks)
        #[arg(value_name = "SLUG")]
        slugs: Vec<String>,

        /// Start all tasks explicitly
        #[arg(short, long, conflicts_with = "slugs")]
        all: bool,
    },

    /// Stop the scheduler daemon
    Stop {
        /// Slugs of tasks to stop (if not specified, stops entire daemon)
        #[arg(value_name = "SLUG")]
        slugs: Vec<String>,

        /// Stop all tasks explicitly
        #[arg(short, long, conflicts_with = "slugs")]
        all: bool,
    },

    /// Launch the interactive TUI
    Tui,
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
        active: true,
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
        "{:<20} {:<20} {:<40} {:<10} {:<15}",
        "SLUG", "CRON", "COMMAND", "STATUS", "LAST RUN"
    );
    println!("{}", "-".repeat(105));

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

        let status = if event.active { "Active" } else { "Inactive" };

        println!(
            "{:<20} {:<20} {:<40} {:<10} {:<15}",
            event.slug, event.cron, command, status, last_run
        );
    }

    Ok(())
}

pub async fn handle_start(slugs: Vec<String>, all: bool) -> Result<()> {
    let mut storage = Storage::load().await?;

    if !slugs.is_empty() {
        // Start specific tasks
        let mut found_count = 0;
        for slug in &slugs {
            if let Some(event) = storage.events.iter_mut().find(|e| &e.slug == slug) {
                event.active = true;
                found_count += 1;
            } else {
                eprintln!("Warning: Task with slug '{slug}' not found");
            }
        }

        if found_count == 0 {
            return Err(anyhow::anyhow!("No valid tasks found to start"));
        }

        storage.save().await?;
        println!("Started {found_count} task(s)");
    } else if all || slugs.is_empty() {
        // Start all tasks (explicit --all or no arguments)
        let inactive_count = storage.events.iter_mut().filter(|e| !e.active).count();
        for event in &mut storage.events {
            event.active = true;
        }

        if inactive_count > 0 {
            storage.save().await?;
            println!("Started all {inactive_count} inactive task(s)");
        } else {
            println!("All tasks are already active");
        }
    }

    // Start or restart the daemon
    crate::daemon::start_daemon().await?;

    Ok(())
}

pub async fn handle_stop(slugs: Vec<String>, all: bool) -> Result<()> {
    let mut storage = Storage::load().await?;

    if !slugs.is_empty() {
        // Stop specific tasks
        let mut found_count = 0;
        for slug in &slugs {
            if let Some(event) = storage.events.iter_mut().find(|e| &e.slug == slug) {
                event.active = false;
                found_count += 1;
            } else {
                eprintln!("Warning: Task with slug '{slug}' not found");
            }
        }

        if found_count == 0 {
            return Err(anyhow::anyhow!("No valid tasks found to stop"));
        }

        storage.save().await?;
        println!("Stopped {found_count} task(s)");

        // Check if any tasks are still active
        if storage.events.iter().any(|e| e.active) {
            // Some tasks still active, restart daemon
            if let Err(e) = crate::daemon::restart_daemon().await {
                eprintln!("Warning: Failed to restart daemon: {e}");
            }
        } else {
            // No active tasks, stop daemon
            crate::daemon::stop_daemon().await?;
        }
    } else if all || slugs.is_empty() {
        // Stop all tasks (explicit --all or no arguments means stop daemon entirely)
        crate::daemon::stop_daemon().await?;
    }

    Ok(())
}

pub async fn handle_tui() -> Result<()> {
    crate::tui::run_tui()
        .await
        .map_err(|e| anyhow::anyhow!("TUI error: {}", e))
}
