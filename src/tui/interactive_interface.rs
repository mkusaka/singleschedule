use anyhow::Result;
use r3bl_tui::{
    choose, DefaultIoDevices, HowToChoose, height,
    StyleSheet,
};
use std::io::{self, Write};
use crate::storage::{Event, Storage};

pub async fn run_interactive_tui() -> Result<()> {
    let mut storage = Storage::load().await?;
    
    println!("\n🗓️  SingleSchedule - Interactive Task Management");
    println!("Use arrow keys to navigate, Enter to select, ESC/Ctrl+C to exit\n");
    
    loop {
        // Create menu options
        let menu_options = vec![
            "📋 View/Select Tasks",
            "➕ Add New Task",
            "🔄 Refresh Tasks",
            "❓ Help",
            "🚪 Exit",
        ];
        
        // Show main menu
        let mut default_io_devices = DefaultIoDevices::default();
        let selected = choose(
            "Main Menu:".to_string(),
            menu_options,
            Some(height(7)),
            None,
            HowToChoose::Single,
            StyleSheet::default(),
            default_io_devices.as_mut_tuple(),
        ).await.map_err(|e| anyhow::anyhow!("Choose error: {}", e))?;
        
        if selected.is_empty() {
            // User pressed ESC
            println!("👋 Goodbye!");
            break;
        }
        
        match selected[0].as_str() {
            "📋 View/Select Tasks" => {
                view_and_select_tasks(&mut storage).await?;
            }
            "➕ Add New Task" => {
                add_task_interactive(&mut storage).await?;
            }
            "🔄 Refresh Tasks" => {
                storage = Storage::load().await?;
                println!("✅ Tasks refreshed successfully!");
            }
            "❓ Help" => {
                display_help();
            }
            "🚪 Exit" => {
                println!("👋 Goodbye!");
                break;
            }
            _ => {}
        }
    }
    
    Ok(())
}

async fn view_and_select_tasks(storage: &mut Storage) -> Result<()> {
    if storage.events.is_empty() {
        println!("\n📭 No tasks found. Use 'Add New Task' to create one.");
        println!("Press Enter to continue...");
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        return Ok(());
    }
    
    loop {
        // Prepare task list for selection
        let task_strings: Vec<String> = storage.events.iter()
            .enumerate()
            .map(|(i, e)| {
                let status = if e.active { "✅" } else { "⏸️" };
                format!("{:2}. {} {:<20} {:<20} {}", 
                    i + 1, status, e.slug, e.cron, e.command)
            })
            .collect();
        
        let back_option = "⬅️  Back to Main Menu".to_string();
        let mut all_options = task_strings;
        all_options.push(back_option);
        
        let task_options: Vec<&str> = all_options.iter().map(|s| s.as_str()).collect();
        
        // Show task list with selection
        let mut default_io_devices = DefaultIoDevices::default();
        let selected = choose(
            "Tasks (Use arrow keys to select, Enter for actions):".to_string(),
            task_options,
            Some(height(15)),
            None,
            HowToChoose::Single,
            StyleSheet::default(),
            default_io_devices.as_mut_tuple(),
        ).await.map_err(|e| anyhow::anyhow!("Choose error: {}", e))?;
        
        if selected.is_empty() {
            // User pressed ESC
            break;
        }
        
        let selected_str = &selected[0];
        
        if selected_str.as_str().contains("Back to Main Menu") {
            break;
        }
        
        // Parse selected task index
        if let Some(dot_pos) = selected_str.as_str().find('.') {
            if let Ok(index) = selected_str.as_str()[..dot_pos].trim().parse::<usize>() {
                if index > 0 && index <= storage.events.len() {
                    // Show task actions
                    task_actions(storage, index - 1).await?;
                }
            }
        }
    }
    
    Ok(())
}

async fn task_actions(storage: &mut Storage, task_index: usize) -> Result<()> {
    let task = &storage.events[task_index];
    let status = if task.active { "Active ✅" } else { "Inactive ⏸️" };
    
    let action_options = vec![
        if task.active { "⏸️  Deactivate Task" } else { "✅ Activate Task" },
        "🗑️  Delete Task",
        "📋 View Details",
        "⬅️  Back to Task List",
    ];
    
    let mut default_io_devices = DefaultIoDevices::default();
    let selected = choose(
        format!("Actions for '{}' ({})", task.slug, status),
        action_options,
        Some(height(6)),
        None,
        HowToChoose::Single,
        StyleSheet::default(),
        default_io_devices.as_mut_tuple(),
    ).await.map_err(|e| anyhow::anyhow!("Choose error: {}", e))?;
    
    if selected.is_empty() {
        return Ok(());
    }
    
    match selected[0].as_str() {
        "⏸️  Deactivate Task" | "✅ Activate Task" => {
            storage.events[task_index].active = !storage.events[task_index].active;
            let new_status = if storage.events[task_index].active { "activated" } else { "deactivated" };
            storage.save().await?;
            println!("✅ Task '{}' {new_status}!", storage.events[task_index].slug);
            
            // Restart daemon
            if let Err(e) = crate::daemon::restart_daemon().await {
                println!("⚠️  Warning: Failed to restart daemon: {e}");
            }
        }
        "🗑️  Delete Task" => {
            // Confirm deletion
            let confirm_options = vec!["❌ Yes, Delete", "✅ No, Keep Task"];
            let mut default_io_devices = DefaultIoDevices::default();
            let confirmed = choose(
                format!("Are you sure you want to delete '{}'?", storage.events[task_index].slug),
                confirm_options,
                Some(height(4)),
                None,
                HowToChoose::Single,
                StyleSheet::default(),
                default_io_devices.as_mut_tuple(),
            ).await.map_err(|e| anyhow::anyhow!("Choose error: {}", e))?;
            
            if !confirmed.is_empty() && confirmed[0].as_str() == "❌ Yes, Delete" {
                let removed_task = storage.events.remove(task_index);
                storage.save().await?;
                println!("✅ Task '{}' deleted successfully!", removed_task.slug);
                
                // Restart daemon if needed
                if storage.events.iter().any(|e| e.active) {
                    if let Err(e) = crate::daemon::restart_daemon().await {
                        println!("⚠️  Warning: Failed to restart daemon: {e}");
                    }
                } else if let Err(e) = crate::daemon::stop_daemon().await {
                    println!("⚠️  Warning: Failed to stop daemon: {e}");
                }
            }
        }
        "📋 View Details" => {
            let task = &storage.events[task_index];
            println!("\n📋 Task Details:");
            println!("  Slug: {}", task.slug);
            println!("  Cron: {}", task.cron);
            println!("  Command: {}", task.command);
            println!("  Status: {}", if task.active { "Active ✅" } else { "Inactive ⏸️" });
            println!("  Created: {}", task.created_at.format("%Y-%m-%d %H:%M:%S"));
            if let Some(last_run) = task.last_run {
                println!("  Last Run: {}", last_run.format("%Y-%m-%d %H:%M:%S"));
            } else {
                println!("  Last Run: Never");
            }
            println!("\nPress Enter to continue...");
            let mut input = String::new();
            io::stdin().read_line(&mut input)?;
        }
        _ => {}
    }
    
    Ok(())
}

async fn add_task_interactive(storage: &mut Storage) -> Result<()> {
    println!("\n➕ Add New Task");
    
    // Get slug
    print!("Enter task slug: ");
    io::stdout().flush()?;
    let mut slug = String::new();
    io::stdin().read_line(&mut slug)?;
    let slug = slug.trim().to_string();
    
    if slug.is_empty() {
        println!("❌ Task creation cancelled");
        return Ok(());
    }
    
    // Check if slug already exists
    if storage.events.iter().any(|e| e.slug == slug) {
        println!("❌ Task with slug '{slug}' already exists");
        return Ok(());
    }
    
    // Get cron expression
    print!("Enter cron expression (e.g., '0 * * * * *' for every hour): ");
    io::stdout().flush()?;
    let mut cron_input = String::new();
    io::stdin().read_line(&mut cron_input)?;
    let cron = cron_input.trim().to_string();
    
    // Validate cron
    if let Err(e) = cron::Schedule::from_str(&cron) {
        println!("❌ Invalid cron expression: {e}");
        return Ok(());
    }
    
    // Get command
    print!("Enter command to execute: ");
    io::stdout().flush()?;
    let mut command = String::new();
    io::stdin().read_line(&mut command)?;
    let command = command.trim().to_string();
    
    if command.is_empty() {
        println!("❌ Task creation cancelled");
        return Ok(());
    }
    
    // Create and save task
    let event = Event {
        slug: slug.clone(),
        cron,
        command,
        pid: None,
        created_at: chrono::Utc::now(),
        last_run: None,
        active: true,
    };
    
    storage.events.push(event);
    storage.save().await?;
    
    println!("✅ Task '{slug}' added successfully!");
    
    // Restart daemon
    if let Err(e) = crate::daemon::restart_daemon().await {
        println!("⚠️  Warning: Failed to restart daemon: {e}");
    }
    
    println!("\nPress Enter to continue...");
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    
    Ok(())
}

fn display_help() {
    println!("\n📋 SingleSchedule Help:");
    println!();
    println!("📌 Navigation:");
    println!("  • Use UP/DOWN arrow keys to navigate menus");
    println!("  • Press ENTER to select an option");
    println!("  • Press ESC to go back or exit");
    println!();
    println!("📌 Task Management:");
    println!("  • Select a task from the list to view actions");
    println!("  • You can activate/deactivate tasks");
    println!("  • Delete tasks with confirmation");
    println!("  • View detailed task information");
    println!();
    println!("📌 Cron Expression Examples:");
    println!("  • '0 * * * * *' - Every hour at minute 0");
    println!("  • '0 0 * * * *' - Daily at midnight");
    println!("  • '0 0 * * 0 *' - Weekly on Sunday");
    println!("  • '0 0 1 * * *' - Monthly on the 1st");
    println!();
    println!("Press Enter to continue...");
    let mut input = String::new();
    let _ = io::stdin().read_line(&mut input);
}

use std::str::FromStr;