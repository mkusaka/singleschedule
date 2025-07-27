use crate::storage::{Event, Storage};
use anyhow::Result;
use crossterm::terminal::{disable_raw_mode, enable_raw_mode};
use r3bl_tui::{
    ast, ast_lines, choose, height, inline_vec, new_style,
    readline_async::{Header, ReadlineAsyncContext},
    tui_color, AnsiStyledText, DefaultIoDevices, HowToChoose, InlineVec, InputDevice, OutputDevice,
    StyleSheet,
};

pub async fn run_simple_tui() -> Result<()> {
    let mut storage = Storage::load().await?;

    // Try to create readline context for better copy/paste support
    let readline_context = ReadlineAsyncContext::try_new(None::<String>)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to create readline context: {e}"))?;

    match readline_context {
        Some(rl_ctx) => {
            // Use readline-aware version with copy/paste support
            run_with_readline(&mut storage, rl_ctx).await
        }
        None => {
            // Fallback to standard version
            run_without_readline(&mut storage).await
        }
    }
}

async fn run_with_readline(storage: &mut Storage, mut rl_ctx: ReadlineAsyncContext) -> Result<()> {
    loop {
        // Create menu header with styling
        let header = create_main_menu_header();

        // Show task list as part of the header
        let task_list = create_task_list_display(storage);
        let mut full_header = header;
        full_header.extend(task_list);

        // Menu options
        let menu_options = [
            "📋 List tasks",
            "➕ Add new task",
            "🗑️  Delete task",
            "🔄 Toggle task active/inactive",
            "🔄 Refresh task list",
            "❓ Help",
            "👋 Exit",
        ];

        // Use readline-aware devices for better copy/paste support
        let sw = rl_ctx.clone_shared_writer();
        let mut output_device = rl_ctx.clone_output_device();
        let input_device = rl_ctx.mut_input_device();

        let selected = choose(
            full_header,
            &menu_options,
            Some(height(10)),
            None,
            HowToChoose::Single,
            StyleSheet::default(),
            (&mut output_device, input_device, Some(sw.clone())),
        )
        .await
        .map_err(|e| anyhow::anyhow!("Choose error: {e}"))?;

        if selected.is_empty() {
            // User pressed ESC or Ctrl+C
            break;
        }

        // Process selection
        match selected[0].as_ref() {
            "📋 List tasks" => {
                // Tasks are already shown in the header
                continue;
            }
            "➕ Add new task" => {
                add_task_with_readline(storage, &mut rl_ctx).await?;
            }
            "🗑️  Delete task" => {
                delete_task_with_readline(storage, &mut rl_ctx).await?;
            }
            "🔄 Toggle task active/inactive" => {
                toggle_task_with_readline(storage, &mut rl_ctx).await?;
            }
            "🔄 Refresh task list" => {
                *storage = Storage::load().await?;
                // Show refresh message in next iteration
            }
            "❓ Help" => {
                show_help_with_readline(&mut rl_ctx).await?;
            }
            "👋 Exit" => {
                break;
            }
            _ => {}
        }
    }

    // Shutdown readline context properly
    rl_ctx
        .request_shutdown(Some("Goodbye! 👋"))
        .await
        .map_err(|e| anyhow::anyhow!("Failed to shutdown readline: {e}"))?;
    rl_ctx.await_shutdown().await;

    Ok(())
}

async fn run_without_readline(storage: &mut Storage) -> Result<()> {
    // Note: Without readline context, we have limited terminal control
    // But we can still use choose() with proper setup

    // Try to enable raw mode for terminal control
    // If this fails, it's likely because we're not in a proper terminal
    if let Err(e) = enable_raw_mode() {
        // Fallback to simpler interface without raw mode
        eprintln!("Warning: Could not enable raw mode: {e}. Using simplified interface.");
        return run_simple_interface(storage).await;
    }

    let result = run_tui_loop(storage).await;

    // Always disable raw mode before returning
    let _ = disable_raw_mode(); // Ignore errors on cleanup

    result
}

async fn run_simple_interface(storage: &mut Storage) -> Result<()> {
    loop {
        // Clear screen
        print!("\x1B[2J\x1B[1;1H");

        // Show header
        println!("🗓️  SingleSchedule - Task Management");
        println!("=====================================\n");

        // Show task list
        if storage.events.is_empty() {
            println!("📭 No tasks found. Use 'Add new task' to create one.\n");
        } else {
            println!("📋 Current Tasks:");
            println!("{}", "-".repeat(60));
            for (index, event) in storage.events.iter().enumerate() {
                let status = if event.active { "✅" } else { "⏸️" };
                println!(
                    "{:2}. {} {:<20} {:<15} {}",
                    index + 1,
                    status,
                    truncate(&event.slug, 20),
                    truncate(&event.cron, 15),
                    truncate(&event.command, 25)
                );
            }
            println!("{}\n", "-".repeat(60));
        }

        // Show menu
        println!("Choose an option:");
        println!("1. 📋 List tasks");
        println!("2. ➕ Add new task");
        println!("3. 🗑️  Delete task");
        println!("4. 🔄 Toggle task active/inactive");
        println!("5. 🔄 Refresh task list");
        println!("6. ❓ Help");
        println!("7. 👋 Exit");

        // Get user choice
        print!("\nEnter your choice (1-7): ");
        std::io::Write::flush(&mut std::io::stdout())?;

        let mut choice = String::new();
        std::io::stdin().read_line(&mut choice)?;
        let choice = choice.trim();

        match choice {
            "1" => {
                // Tasks are already shown
                println!("\nTasks are shown above. Press Enter to continue...");
                let mut input = String::new();
                std::io::stdin().read_line(&mut input)?;
            }
            "2" => {
                add_task_interactive(storage).await?;
            }
            "3" => {
                delete_task_interactive(storage).await?;
            }
            "4" => {
                toggle_task_interactive(storage).await?;
            }
            "5" => {
                *storage = Storage::load().await?;
                println!("✅ Task list refreshed!");
                std::thread::sleep(std::time::Duration::from_secs(1));
            }
            "6" => {
                println!("\n📚 SingleSchedule Help");
                println!("====================\n");
                println!("💡 Tips:");
                println!("• Use number keys (1-7) to select menu options");
                println!("• Copy/paste works as expected in your terminal!");
                println!("• Tasks run automatically in the background via daemon");
                println!("• Use cron expressions like '0 * * * *' for hourly tasks");
                println!("\nPress Enter to continue...");

                let mut input = String::new();
                std::io::stdin().read_line(&mut input)?;
            }
            "7" | "exit" | "quit" => {
                println!("Goodbye! 👋");
                break;
            }
            _ => {
                println!("❌ Invalid choice. Please enter a number between 1 and 7.");
                std::thread::sleep(std::time::Duration::from_secs(1));
            }
        }
    }

    Ok(())
}

async fn run_tui_loop(storage: &mut Storage) -> Result<()> {
    loop {
        // Create menu header with styling
        let header = create_main_menu_header();

        // Show task list as part of the header
        let task_list = create_task_list_display(storage);
        let mut full_header = header;
        full_header.extend(task_list);

        // Menu options
        let menu_options = [
            "📋 List tasks",
            "➕ Add new task",
            "🗑️  Delete task",
            "🔄 Toggle task active/inactive",
            "🔄 Refresh task list",
            "❓ Help",
            "👋 Exit",
        ];

        // Create IO devices for standalone usage
        let mut output_device = OutputDevice::new_stdout();
        let mut input_device = InputDevice::new_event_stream();

        let selected = choose(
            Header::MultiLine(full_header),
            &menu_options,
            Some(height(10)),
            None,
            HowToChoose::Single,
            StyleSheet::default(),
            (&mut output_device, &mut input_device, None),
        )
        .await
        .map_err(|e| anyhow::anyhow!("Choose error: {e}"))?;

        if selected.is_empty() {
            // User pressed ESC or Ctrl+C
            break;
        }

        // Process selection
        match selected[0].as_ref() {
            "📋 List tasks" => {
                // Tasks are already shown in the header
                continue;
            }
            "➕ Add new task" => {
                // Temporarily disable raw mode for input
                disable_raw_mode()?;
                add_task_interactive(storage).await?;
                enable_raw_mode()?;
            }
            "🗑️  Delete task" => {
                delete_task_interactive_with_choose(storage).await?;
            }
            "🔄 Toggle task active/inactive" => {
                toggle_task_interactive_with_choose(storage).await?;
            }
            "🔄 Refresh task list" => {
                *storage = Storage::load().await?;
                // Show refresh message in next iteration
            }
            "❓ Help" => {
                show_help_with_choose().await?;
            }
            "👋 Exit" => {
                break;
            }
            _ => {}
        }
    }

    Ok(())
}

fn create_main_menu_header() -> InlineVec<InlineVec<AnsiStyledText>> {
    let title = ast(
        "🗓️  SingleSchedule - Task Management",
        new_style!(
            color_fg: {tui_color!(171, 204, 242)}
            color_bg: {tui_color!(31, 36, 46)}
            bold
        ),
    );

    let subtitle = ast(
        "Use ↑/↓ to navigate, Enter to select, ESC to go back",
        new_style!(
            color_fg: {tui_color!(94, 103, 111)}
        ),
    );

    ast_lines![
        inline_vec![title],
        inline_vec![subtitle],
        inline_vec![] // Empty line
    ]
}

fn create_task_list_display(storage: &Storage) -> InlineVec<InlineVec<AnsiStyledText>> {
    let mut lines = InlineVec::new();

    if storage.events.is_empty() {
        let empty_msg = ast(
            "📭 No tasks found. Use 'Add new task' to create one.",
            new_style!(
                color_fg: {tui_color!(255, 216, 9)}
            ),
        );
        lines.push(inline_vec![empty_msg]);
    } else {
        // Task list header
        let header = ast(
            "📋 Current Tasks:",
            new_style!(
                color_fg: {tui_color!(9, 238, 211)}
                bold
            ),
        );
        lines.push(inline_vec![header]);

        // Task list separator
        let separator = ast(
            "─".repeat(60),
            new_style!(
                color_fg: {tui_color!(94, 103, 111)}
            ),
        );
        lines.push(inline_vec![separator.clone()]);

        // Tasks
        for (index, event) in storage.events.iter().enumerate() {
            let status = if event.active { "✅" } else { "⏸️" };
            let task_line = format!(
                "{:2}. {} {:<20} {:<15} {}",
                index + 1,
                status,
                truncate(&event.slug, 20),
                truncate(&event.cron, 15),
                truncate(&event.command, 25)
            );

            let task_ast = ast(
                &task_line,
                new_style!(
                    color_fg: {tui_color!(200, 200, 200)}
                ),
            );
            lines.push(inline_vec![task_ast]);
        }

        lines.push(inline_vec![separator]);
    }

    lines.push(inline_vec![]); // Empty line
    lines
}

async fn show_help_with_readline(rl_ctx: &mut ReadlineAsyncContext) -> Result<()> {
    let help_header = ast_lines![
        inline_vec![ast(
            "📚 SingleSchedule Help",
            new_style!(
                color_fg: {tui_color!(171, 204, 242)}
                color_bg: {tui_color!(31, 36, 46)}
                bold
            ),
        )],
        inline_vec![],
        inline_vec![ast(
            "💡 Tips:",
            new_style!(
                color_fg: {tui_color!(9, 238, 211)}
                bold
            ),
        )],
        inline_vec![ast(
            "• Use arrow keys (↑/↓) to navigate menus",
            new_style!(color_fg: {tui_color!(200, 200, 200)}),
        )],
        inline_vec![ast(
            "• Press Enter to confirm selection",
            new_style!(color_fg: {tui_color!(200, 200, 200)}),
        )],
        inline_vec![ast(
            "• Press ESC to cancel or go back",
            new_style!(color_fg: {tui_color!(200, 200, 200)}),
        )],
        inline_vec![ast(
            "• ✨ Copy/paste works perfectly with readline support! ✨",
            new_style!(color_fg: {tui_color!(255, 216, 9)}),
        )],
        inline_vec![],
        inline_vec![ast(
            "Press Enter to continue...",
            new_style!(
                color_fg: {tui_color!(255, 216, 9)}
            ),
        )]
    ];

    let sw = rl_ctx.clone_shared_writer();
    let mut output_device = rl_ctx.clone_output_device();
    let input_device = rl_ctx.mut_input_device();

    let _ = choose(
        help_header,
        &["Continue"],
        Some(height(1)),
        None,
        HowToChoose::Single,
        StyleSheet::default(),
        (&mut output_device, input_device, Some(sw.clone())),
    )
    .await
    .map_err(|e| anyhow::anyhow!("Choose error: {}", e))?;

    Ok(())
}

async fn show_help_with_choose() -> Result<()> {
    let help_header = ast_lines![
        inline_vec![ast(
            "📚 SingleSchedule Help",
            new_style!(
                color_fg: {tui_color!(171, 204, 242)}
                color_bg: {tui_color!(31, 36, 46)}
                bold
            ),
        )],
        inline_vec![],
        inline_vec![ast(
            "💡 Tips:",
            new_style!(
                color_fg: {tui_color!(9, 238, 211)}
                bold
            ),
        )],
        inline_vec![ast(
            "• Use arrow keys (↑/↓) to navigate menus",
            new_style!(color_fg: {tui_color!(200, 200, 200)}),
        )],
        inline_vec![ast(
            "• Press Enter to confirm selection",
            new_style!(color_fg: {tui_color!(200, 200, 200)}),
        )],
        inline_vec![ast(
            "• Press ESC to cancel or go back",
            new_style!(color_fg: {tui_color!(200, 200, 200)}),
        )],
        inline_vec![ast(
            "• Copy/paste works in your terminal!",
            new_style!(color_fg: {tui_color!(200, 200, 200)}),
        )],
        inline_vec![],
        inline_vec![ast(
            "Press Enter to continue...",
            new_style!(
                color_fg: {tui_color!(255, 216, 9)}
            ),
        )]
    ];

    let mut output_device = OutputDevice::new_stdout();
    let mut input_device = InputDevice::new_event_stream();

    let _ = choose(
        Header::MultiLine(help_header),
        &["Continue"],
        Some(height(1)),
        None,
        HowToChoose::Single,
        StyleSheet::default(),
        (&mut output_device, &mut input_device, None),
    )
    .await
    .map_err(|e| anyhow::anyhow!("Choose error: {}", e))?;

    Ok(())
}

async fn add_task_with_readline(
    storage: &mut Storage,
    rl_ctx: &mut ReadlineAsyncContext,
) -> Result<()> {
    // Show add task instructions
    let header = ast_lines![
        inline_vec![ast(
            "➕ Add New Task",
            new_style!(
                color_fg: {tui_color!(171, 204, 242)}
                color_bg: {tui_color!(31, 36, 46)}
                bold
            ),
        )],
        inline_vec![],
        inline_vec![ast(
            "✨ Good news! With readline support, you can use arrow keys and copy/paste! ✨",
            new_style!(color_fg: {tui_color!(9, 238, 211)}),
        )],
        inline_vec![],
        inline_vec![ast(
            "Task details needed:",
            new_style!(
                color_fg: {tui_color!(9, 238, 211)}
                bold
            ),
        )],
        inline_vec![ast(
            "• Slug (unique identifier)",
            new_style!(color_fg: {tui_color!(200, 200, 200)}),
        )],
        inline_vec![ast(
            "• Cron expression (e.g., '0 * * * *' for hourly)",
            new_style!(color_fg: {tui_color!(200, 200, 200)}),
        )],
        inline_vec![ast(
            "• Command to execute",
            new_style!(color_fg: {tui_color!(200, 200, 200)}),
        )],
        inline_vec![]
    ];

    let sw = rl_ctx.clone_shared_writer();
    let mut output_device = rl_ctx.clone_output_device();
    let input_device = rl_ctx.mut_input_device();

    let selected = choose(
        header,
        &["Continue to add task", "Cancel"],
        Some(height(2)),
        None,
        HowToChoose::Single,
        StyleSheet::default(),
        (&mut output_device, input_device, Some(sw.clone())),
    )
    .await
    .map_err(|e| anyhow::anyhow!("Choose error: {}", e))?;

    if selected.is_empty() || selected[0] == "Cancel" {
        return Ok(());
    }

    // Use readline for input with full editing support
    println!("\n--- Add New Task ---");

    // Get slug
    print!("Enter task slug: ");
    std::io::Write::flush(&mut std::io::stdout())?;
    let mut slug = String::new();
    std::io::stdin().read_line(&mut slug)?;
    let slug = slug.trim().to_string();

    if slug.is_empty() {
        println!("Task creation cancelled");
        std::thread::sleep(std::time::Duration::from_secs(1));
        return Ok(());
    }

    // Check if slug already exists
    if storage.events.iter().any(|e| e.slug == slug) {
        println!("Error: Task with slug '{slug}' already exists");
        std::thread::sleep(std::time::Duration::from_secs(2));
        return Ok(());
    }

    // Get cron expression
    print!("Enter cron expression (e.g., '0 * * * *'): ");
    std::io::Write::flush(&mut std::io::stdout())?;
    let mut cron = String::new();
    std::io::stdin().read_line(&mut cron)?;
    let cron = cron.trim().to_string();

    // Validate cron
    if let Err(e) = cron::Schedule::from_str(&cron) {
        println!("Error: Invalid cron expression: {e}");
        std::thread::sleep(std::time::Duration::from_secs(2));
        return Ok(());
    }

    // Get command
    print!("Enter command to execute: ");
    std::io::Write::flush(&mut std::io::stdout())?;
    let mut command = String::new();
    std::io::stdin().read_line(&mut command)?;
    let command = command.trim().to_string();

    if command.is_empty() {
        println!("Task creation cancelled");
        std::thread::sleep(std::time::Duration::from_secs(1));
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

    std::thread::sleep(std::time::Duration::from_secs(2));

    Ok(())
}

async fn add_task_interactive(storage: &mut Storage) -> Result<()> {
    // Show add task instructions
    let header = ast_lines![
        inline_vec![ast(
            "➕ Add New Task",
            new_style!(
                color_fg: {tui_color!(171, 204, 242)}
                color_bg: {tui_color!(31, 36, 46)}
                bold
            ),
        )],
        inline_vec![],
        inline_vec![ast(
            "Note: Due to technical limitations, you'll need to enter task details",
            new_style!(color_fg: {tui_color!(255, 216, 9)}),
        )],
        inline_vec![ast(
            "in the terminal after selecting this option.",
            new_style!(color_fg: {tui_color!(255, 216, 9)}),
        )],
        inline_vec![],
        inline_vec![ast(
            "Task details needed:",
            new_style!(
                color_fg: {tui_color!(9, 238, 211)}
                bold
            ),
        )],
        inline_vec![ast(
            "• Slug (unique identifier)",
            new_style!(color_fg: {tui_color!(200, 200, 200)}),
        )],
        inline_vec![ast(
            "• Cron expression (e.g., '0 * * * *' for hourly)",
            new_style!(color_fg: {tui_color!(200, 200, 200)}),
        )],
        inline_vec![ast(
            "• Command to execute",
            new_style!(color_fg: {tui_color!(200, 200, 200)}),
        )],
        inline_vec![]
    ];

    let mut default_io_devices = DefaultIoDevices::default();
    let selected = choose(
        header,
        &["Continue to add task", "Cancel"],
        Some(height(2)),
        None,
        HowToChoose::Single,
        StyleSheet::default(),
        default_io_devices.as_mut_tuple(),
    )
    .await
    .map_err(|e| anyhow::anyhow!("Choose error: {}", e))?;

    if selected.is_empty() || selected[0] == "Cancel" {
        return Ok(());
    }

    // Temporarily exit TUI mode for input
    println!("\n--- Add New Task ---");

    // Get slug
    print!("Enter task slug: ");
    std::io::Write::flush(&mut std::io::stdout())?;
    let mut slug = String::new();
    std::io::stdin().read_line(&mut slug)?;
    let slug = slug.trim().to_string();

    if slug.is_empty() {
        println!("Task creation cancelled");
        std::thread::sleep(std::time::Duration::from_secs(1));
        return Ok(());
    }

    // Check if slug already exists
    if storage.events.iter().any(|e| e.slug == slug) {
        println!("Error: Task with slug '{slug}' already exists");
        std::thread::sleep(std::time::Duration::from_secs(2));
        return Ok(());
    }

    // Get cron expression
    print!("Enter cron expression (e.g., '0 * * * *'): ");
    std::io::Write::flush(&mut std::io::stdout())?;
    let mut cron = String::new();
    std::io::stdin().read_line(&mut cron)?;
    let cron = cron.trim().to_string();

    // Validate cron
    if let Err(e) = cron::Schedule::from_str(&cron) {
        println!("Error: Invalid cron expression: {e}");
        std::thread::sleep(std::time::Duration::from_secs(2));
        return Ok(());
    }

    // Get command
    print!("Enter command to execute: ");
    std::io::Write::flush(&mut std::io::stdout())?;
    let mut command = String::new();
    std::io::stdin().read_line(&mut command)?;
    let command = command.trim().to_string();

    if command.is_empty() {
        println!("Task creation cancelled");
        std::thread::sleep(std::time::Duration::from_secs(1));
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

    std::thread::sleep(std::time::Duration::from_secs(2));

    Ok(())
}

async fn delete_task_with_readline(
    storage: &mut Storage,
    rl_ctx: &mut ReadlineAsyncContext,
) -> Result<()> {
    if storage.events.is_empty() {
        let header = ast_lines![inline_vec![ast(
            "❌ No tasks to delete",
            new_style!(
                color_fg: {tui_color!(255, 132, 18)}
                bold
            ),
        )]];

        let sw = rl_ctx.clone_shared_writer();
        let mut output_device = rl_ctx.clone_output_device();
        let input_device = rl_ctx.mut_input_device();

        let _ = choose(
            header,
            &["OK"],
            Some(height(1)),
            None,
            HowToChoose::Single,
            StyleSheet::default(),
            (&mut output_device, input_device, Some(sw.clone())),
        )
        .await
        .map_err(|e| anyhow::anyhow!("Choose error: {e}"))?;

        return Ok(());
    }

    // Prepare choices for selection
    let mut choices = Vec::new();
    choices.push("❌ Cancel".to_string());

    for (i, event) in storage.events.iter().enumerate() {
        choices.push(format!(
            "{:2}. {} - {}",
            i + 1,
            event.slug,
            truncate(&event.command, 40)
        ));
    }

    let choice_refs: Vec<&str> = choices.iter().map(|s| s.as_str()).collect();

    let header = ast_lines![
        inline_vec![ast(
            "🗑️  Select task to delete",
            new_style!(
                color_fg: {tui_color!(255, 132, 18)}
                color_bg: {tui_color!(31, 36, 46)}
                bold
            ),
        )],
        inline_vec![],
        inline_vec![ast(
            "Use ↑/↓ to select, Enter to confirm, ESC to cancel",
            new_style!(color_fg: {tui_color!(94, 103, 111)}),
        )]
    ];

    let sw = rl_ctx.clone_shared_writer();
    let mut output_device = rl_ctx.clone_output_device();
    let input_device = rl_ctx.mut_input_device();

    let selected = choose(
        header,
        &choice_refs[..],
        Some(height(10)),
        None,
        HowToChoose::Single,
        StyleSheet::default(),
        (&mut output_device, input_device, Some(sw.clone())),
    )
    .await
    .map_err(|e| anyhow::anyhow!("Choose error: {}", e))?;

    if selected.is_empty() || selected[0] == "❌ Cancel" {
        return Ok(());
    }

    // Parse the selected index
    let selected_str = &selected[0];
    if let Some(dot_pos) = selected_str.find('.') {
        if let Ok(index) = selected_str[..dot_pos].trim().parse::<usize>() {
            if index > 0 && index <= storage.events.len() {
                let task = storage.events.remove(index - 1);
                storage.save().await?;

                // Show success message
                let success_header = ast_lines![inline_vec![ast(
                    format!("✅ Task '{}' deleted successfully!", task.slug),
                    new_style!(
                        color_fg: {tui_color!(9, 238, 211)}
                        bold
                    ),
                )]];

                let _ = choose(
                    success_header,
                    &["OK"],
                    Some(height(1)),
                    None,
                    HowToChoose::Single,
                    StyleSheet::default(),
                    (&mut output_device, input_device, Some(sw.clone())),
                )
                .await
                .map_err(|e| anyhow::anyhow!("Choose error: {e}"))?;

                // Restart daemon if needed
                if storage.events.iter().any(|e| e.active) {
                    if let Err(e) = crate::daemon::restart_daemon().await {
                        eprintln!("Warning: Failed to restart daemon: {e}");
                    }
                } else if let Err(e) = crate::daemon::stop_daemon().await {
                    eprintln!("Warning: Failed to stop daemon: {e}");
                }
            }
        }
    }

    Ok(())
}

async fn delete_task_interactive_with_choose(storage: &mut Storage) -> Result<()> {
    if storage.events.is_empty() {
        let header = ast_lines![inline_vec![ast(
            "❌ No tasks to delete",
            new_style!(
                color_fg: {tui_color!(255, 132, 18)}
                bold
            ),
        )]];

        let mut output_device = OutputDevice::new_stdout();
        let mut input_device = InputDevice::new_event_stream();

        let _ = choose(
            Header::MultiLine(header),
            &["OK"],
            Some(height(1)),
            None,
            HowToChoose::Single,
            StyleSheet::default(),
            (&mut output_device, &mut input_device, None),
        )
        .await
        .map_err(|e| anyhow::anyhow!("Choose error: {e}"))?;

        return Ok(());
    }

    // Prepare choices for selection
    let mut choices = Vec::new();
    choices.push("❌ Cancel".to_string());

    for (i, event) in storage.events.iter().enumerate() {
        choices.push(format!(
            "{:2}. {} - {}",
            i + 1,
            event.slug,
            truncate(&event.command, 40)
        ));
    }

    let choice_refs: Vec<&str> = choices.iter().map(|s| s.as_str()).collect();

    let header = ast_lines![
        inline_vec![ast(
            "🗑️  Select task to delete",
            new_style!(
                color_fg: {tui_color!(255, 132, 18)}
                color_bg: {tui_color!(31, 36, 46)}
                bold
            ),
        )],
        inline_vec![],
        inline_vec![ast(
            "Use ↑/↓ to select, Enter to confirm, ESC to cancel",
            new_style!(color_fg: {tui_color!(94, 103, 111)}),
        )]
    ];

    let mut output_device = OutputDevice::new_stdout();
    let mut input_device = InputDevice::new_event_stream();

    let selected = choose(
        Header::MultiLine(header),
        &choice_refs[..],
        Some(height(10)),
        None,
        HowToChoose::Single,
        StyleSheet::default(),
        (&mut output_device, &mut input_device, None),
    )
    .await
    .map_err(|e| anyhow::anyhow!("Choose error: {}", e))?;

    if selected.is_empty() || selected[0] == "❌ Cancel" {
        return Ok(());
    }

    // Parse the selected index
    let selected_str = &selected[0];
    if let Some(dot_pos) = selected_str.find('.') {
        if let Ok(index) = selected_str[..dot_pos].trim().parse::<usize>() {
            if index > 0 && index <= storage.events.len() {
                let task = storage.events.remove(index - 1);
                storage.save().await?;

                // Show success message
                let success_header = ast_lines![inline_vec![ast(
                    format!("✅ Task '{}' deleted successfully!", task.slug),
                    new_style!(
                        color_fg: {tui_color!(9, 238, 211)}
                        bold
                    ),
                )]];

                let mut output_device2 = OutputDevice::new_stdout();
                let mut input_device2 = InputDevice::new_event_stream();

                let _ = choose(
                    Header::MultiLine(success_header),
                    &["OK"],
                    Some(height(1)),
                    None,
                    HowToChoose::Single,
                    StyleSheet::default(),
                    (&mut output_device2, &mut input_device2, None),
                )
                .await
                .map_err(|e| anyhow::anyhow!("Choose error: {e}"))?;

                // Restart daemon if needed
                if storage.events.iter().any(|e| e.active) {
                    if let Err(e) = crate::daemon::restart_daemon().await {
                        eprintln!("Warning: Failed to restart daemon: {e}");
                    }
                } else if let Err(e) = crate::daemon::stop_daemon().await {
                    eprintln!("Warning: Failed to stop daemon: {e}");
                }
            }
        }
    }

    Ok(())
}

async fn delete_task_interactive(storage: &mut Storage) -> Result<()> {
    if storage.events.is_empty() {
        let header = ast_lines![inline_vec![ast(
            "❌ No tasks to delete",
            new_style!(
                color_fg: {tui_color!(255, 132, 18)}
                bold
            ),
        )]];

        let mut output_device = OutputDevice::new_stdout();
        let mut input_device = InputDevice::new_event_stream();
        let _ = choose(
            header,
            &["OK"],
            Some(height(1)),
            None,
            HowToChoose::Single,
            StyleSheet::default(),
            (&mut output_device, &mut input_device, None),
        )
        .await
        .map_err(|e| anyhow::anyhow!("Choose error: {e}"))?;

        return Ok(());
    }

    // Prepare choices for selection
    let mut choices = Vec::new();
    choices.push("❌ Cancel".to_string());

    for (i, event) in storage.events.iter().enumerate() {
        choices.push(format!(
            "{:2}. {} - {}",
            i + 1,
            event.slug,
            truncate(&event.command, 40)
        ));
    }

    let choice_refs: Vec<&str> = choices.iter().map(|s| s.as_str()).collect();

    let header = ast_lines![
        inline_vec![ast(
            "🗑️  Select task to delete",
            new_style!(
                color_fg: {tui_color!(255, 132, 18)}
                color_bg: {tui_color!(31, 36, 46)}
                bold
            ),
        )],
        inline_vec![],
        inline_vec![ast(
            "Use ↑/↓ to select, Enter to confirm, ESC to cancel",
            new_style!(color_fg: {tui_color!(94, 103, 111)}),
        )]
    ];

    let mut output_device = OutputDevice::new_stdout();
    let mut input_device = InputDevice::new_event_stream();
    let selected = choose(
        header,
        &choice_refs[..],
        Some(height(10)),
        None,
        HowToChoose::Single,
        StyleSheet::default(),
        (&mut output_device, &mut input_device, None),
    )
    .await
    .map_err(|e| anyhow::anyhow!("Choose error: {}", e))?;

    if selected.is_empty() || selected[0] == "❌ Cancel" {
        return Ok(());
    }

    // Parse the selected index
    let selected_str = &selected[0];
    if let Some(dot_pos) = selected_str.find('.') {
        if let Ok(index) = selected_str[..dot_pos].trim().parse::<usize>() {
            if index > 0 && index <= storage.events.len() {
                let task = storage.events.remove(index - 1);
                storage.save().await?;

                // Show success message
                let success_header = ast_lines![inline_vec![ast(
                    format!("✅ Task '{}' deleted successfully!", task.slug),
                    new_style!(
                        color_fg: {tui_color!(9, 238, 211)}
                        bold
                    ),
                )]];

                let mut output_device2 = OutputDevice::new_stdout();
                let mut input_device2 = InputDevice::new_event_stream();
                let _ = choose(
                    success_header,
                    &["OK"],
                    Some(height(1)),
                    None,
                    HowToChoose::Single,
                    StyleSheet::default(),
                    (&mut output_device2, &mut input_device2, None),
                )
                .await
                .map_err(|e| anyhow::anyhow!("Choose error: {e}"))?;

                // Restart daemon if needed
                if storage.events.iter().any(|e| e.active) {
                    if let Err(e) = crate::daemon::restart_daemon().await {
                        eprintln!("Warning: Failed to restart daemon: {e}");
                    }
                } else if let Err(e) = crate::daemon::stop_daemon().await {
                    eprintln!("Warning: Failed to stop daemon: {e}");
                }
            }
        }
    }

    Ok(())
}

async fn toggle_task_with_readline(
    storage: &mut Storage,
    rl_ctx: &mut ReadlineAsyncContext,
) -> Result<()> {
    if storage.events.is_empty() {
        let header = ast_lines![inline_vec![ast(
            "❌ No tasks to toggle",
            new_style!(
                color_fg: {tui_color!(255, 132, 18)}
                bold
            ),
        )]];

        let sw = rl_ctx.clone_shared_writer();
        let mut output_device = rl_ctx.clone_output_device();
        let input_device = rl_ctx.mut_input_device();

        let _ = choose(
            header,
            &["OK"],
            Some(height(1)),
            None,
            HowToChoose::Single,
            StyleSheet::default(),
            (&mut output_device, input_device, Some(sw.clone())),
        )
        .await
        .map_err(|e| anyhow::anyhow!("Choose error: {e}"))?;

        return Ok(());
    }

    // Prepare choices with current status
    let mut choices = Vec::new();
    choices.push("❌ Cancel".to_string());

    for (i, event) in storage.events.iter().enumerate() {
        let status = if event.active { "✅" } else { "⏸️" };
        choices.push(format!(
            "{:2}. {} {} - {}",
            i + 1,
            status,
            event.slug,
            truncate(&event.command, 35)
        ));
    }

    let choice_refs: Vec<&str> = choices.iter().map(|s| s.as_str()).collect();

    let header = ast_lines![
        inline_vec![ast(
            "🔄 Select task to toggle active/inactive",
            new_style!(
                color_fg: {tui_color!(255, 216, 9)}
                color_bg: {tui_color!(31, 36, 46)}
                bold
            ),
        )],
        inline_vec![],
        inline_vec![ast(
            "Use ↑/↓ to select, Enter to confirm, ESC to cancel",
            new_style!(color_fg: {tui_color!(94, 103, 111)}),
        )]
    ];

    let sw = rl_ctx.clone_shared_writer();
    let mut output_device = rl_ctx.clone_output_device();
    let input_device = rl_ctx.mut_input_device();

    let selected = choose(
        header,
        &choice_refs[..],
        Some(height(10)),
        None,
        HowToChoose::Single,
        StyleSheet::default(),
        (&mut output_device, input_device, Some(sw.clone())),
    )
    .await
    .map_err(|e| anyhow::anyhow!("Choose error: {}", e))?;

    if selected.is_empty() || selected[0] == "❌ Cancel" {
        return Ok(());
    }

    // Parse the selected index
    let selected_str = &selected[0];
    if let Some(dot_pos) = selected_str.find('.') {
        if let Ok(index) = selected_str[..dot_pos].trim().parse::<usize>() {
            if index > 0 && index <= storage.events.len() {
                let task = &mut storage.events[index - 1];
                task.active = !task.active;
                let new_status = if task.active {
                    "activated"
                } else {
                    "deactivated"
                };
                let slug = task.slug.clone();

                storage.save().await?;

                // Show success message
                let success_header = ast_lines![inline_vec![ast(
                    format!("✅ Task '{slug}' {new_status}!"),
                    new_style!(
                        color_fg: {tui_color!(9, 238, 211)}
                        bold
                    ),
                )]];

                let _ = choose(
                    success_header,
                    &["OK"],
                    Some(height(1)),
                    None,
                    HowToChoose::Single,
                    StyleSheet::default(),
                    (&mut output_device, input_device, Some(sw.clone())),
                )
                .await
                .map_err(|e| anyhow::anyhow!("Choose error: {e}"))?;

                // Restart daemon
                if let Err(e) = crate::daemon::restart_daemon().await {
                    eprintln!("Warning: Failed to restart daemon: {e}");
                }
            }
        }
    }

    Ok(())
}

async fn toggle_task_interactive_with_choose(storage: &mut Storage) -> Result<()> {
    if storage.events.is_empty() {
        let header = ast_lines![inline_vec![ast(
            "❌ No tasks to toggle",
            new_style!(
                color_fg: {tui_color!(255, 132, 18)}
                bold
            ),
        )]];

        let mut output_device = OutputDevice::new_stdout();
        let mut input_device = InputDevice::new_event_stream();

        let _ = choose(
            Header::MultiLine(header),
            &["OK"],
            Some(height(1)),
            None,
            HowToChoose::Single,
            StyleSheet::default(),
            (&mut output_device, &mut input_device, None),
        )
        .await
        .map_err(|e| anyhow::anyhow!("Choose error: {e}"))?;

        return Ok(());
    }

    // Prepare choices with current status
    let mut choices = Vec::new();
    choices.push("❌ Cancel".to_string());

    for (i, event) in storage.events.iter().enumerate() {
        let status = if event.active { "✅" } else { "⏸️" };
        choices.push(format!(
            "{:2}. {} {} - {}",
            i + 1,
            status,
            event.slug,
            truncate(&event.command, 35)
        ));
    }

    let choice_refs: Vec<&str> = choices.iter().map(|s| s.as_str()).collect();

    let header = ast_lines![
        inline_vec![ast(
            "🔄 Select task to toggle active/inactive",
            new_style!(
                color_fg: {tui_color!(255, 216, 9)}
                color_bg: {tui_color!(31, 36, 46)}
                bold
            ),
        )],
        inline_vec![],
        inline_vec![ast(
            "Use ↑/↓ to select, Enter to confirm, ESC to cancel",
            new_style!(color_fg: {tui_color!(94, 103, 111)}),
        )]
    ];

    let mut output_device = OutputDevice::new_stdout();
    let mut input_device = InputDevice::new_event_stream();

    let selected = choose(
        Header::MultiLine(header),
        &choice_refs[..],
        Some(height(10)),
        None,
        HowToChoose::Single,
        StyleSheet::default(),
        (&mut output_device, &mut input_device, None),
    )
    .await
    .map_err(|e| anyhow::anyhow!("Choose error: {}", e))?;

    if selected.is_empty() || selected[0] == "❌ Cancel" {
        return Ok(());
    }

    // Parse the selected index
    let selected_str = &selected[0];
    if let Some(dot_pos) = selected_str.find('.') {
        if let Ok(index) = selected_str[..dot_pos].trim().parse::<usize>() {
            if index > 0 && index <= storage.events.len() {
                let task = &mut storage.events[index - 1];
                task.active = !task.active;
                let new_status = if task.active {
                    "activated"
                } else {
                    "deactivated"
                };
                let slug = task.slug.clone();

                storage.save().await?;

                // Show success message
                let success_header = ast_lines![inline_vec![ast(
                    format!("✅ Task '{slug}' {new_status}!"),
                    new_style!(
                        color_fg: {tui_color!(9, 238, 211)}
                        bold
                    ),
                )]];

                let mut output_device2 = OutputDevice::new_stdout();
                let mut input_device2 = InputDevice::new_event_stream();

                let _ = choose(
                    Header::MultiLine(success_header),
                    &["OK"],
                    Some(height(1)),
                    None,
                    HowToChoose::Single,
                    StyleSheet::default(),
                    (&mut output_device2, &mut input_device2, None),
                )
                .await
                .map_err(|e| anyhow::anyhow!("Choose error: {e}"))?;

                // Restart daemon
                if let Err(e) = crate::daemon::restart_daemon().await {
                    eprintln!("Warning: Failed to restart daemon: {e}");
                }
            }
        }
    }

    Ok(())
}

async fn toggle_task_interactive(storage: &mut Storage) -> Result<()> {
    if storage.events.is_empty() {
        let header = ast_lines![inline_vec![ast(
            "❌ No tasks to toggle",
            new_style!(
                color_fg: {tui_color!(255, 132, 18)}
                bold
            ),
        )]];

        let mut output_device = OutputDevice::new_stdout();
        let mut input_device = InputDevice::new_event_stream();
        let _ = choose(
            header,
            &["OK"],
            Some(height(1)),
            None,
            HowToChoose::Single,
            StyleSheet::default(),
            (&mut output_device, &mut input_device, None),
        )
        .await
        .map_err(|e| anyhow::anyhow!("Choose error: {e}"))?;

        return Ok(());
    }

    // Prepare choices with current status
    let mut choices = Vec::new();
    choices.push("❌ Cancel".to_string());

    for (i, event) in storage.events.iter().enumerate() {
        let status = if event.active { "✅" } else { "⏸️" };
        choices.push(format!(
            "{:2}. {} {} - {}",
            i + 1,
            status,
            event.slug,
            truncate(&event.command, 35)
        ));
    }

    let choice_refs: Vec<&str> = choices.iter().map(|s| s.as_str()).collect();

    let header = ast_lines![
        inline_vec![ast(
            "🔄 Select task to toggle active/inactive",
            new_style!(
                color_fg: {tui_color!(255, 216, 9)}
                color_bg: {tui_color!(31, 36, 46)}
                bold
            ),
        )],
        inline_vec![],
        inline_vec![ast(
            "Use ↑/↓ to select, Enter to confirm, ESC to cancel",
            new_style!(color_fg: {tui_color!(94, 103, 111)}),
        )]
    ];

    let mut output_device = OutputDevice::new_stdout();
    let mut input_device = InputDevice::new_event_stream();
    let selected = choose(
        header,
        &choice_refs[..],
        Some(height(10)),
        None,
        HowToChoose::Single,
        StyleSheet::default(),
        (&mut output_device, &mut input_device, None),
    )
    .await
    .map_err(|e| anyhow::anyhow!("Choose error: {}", e))?;

    if selected.is_empty() || selected[0] == "❌ Cancel" {
        return Ok(());
    }

    // Parse the selected index
    let selected_str = &selected[0];
    if let Some(dot_pos) = selected_str.find('.') {
        if let Ok(index) = selected_str[..dot_pos].trim().parse::<usize>() {
            if index > 0 && index <= storage.events.len() {
                let task = &mut storage.events[index - 1];
                task.active = !task.active;
                let new_status = if task.active {
                    "activated"
                } else {
                    "deactivated"
                };
                let slug = task.slug.clone();

                storage.save().await?;

                // Show success message
                let success_header = ast_lines![inline_vec![ast(
                    format!("✅ Task '{slug}' {new_status}!"),
                    new_style!(
                        color_fg: {tui_color!(9, 238, 211)}
                        bold
                    ),
                )]];

                let mut output_device2 = OutputDevice::new_stdout();
                let mut input_device2 = InputDevice::new_event_stream();
                let _ = choose(
                    success_header,
                    &["OK"],
                    Some(height(1)),
                    None,
                    HowToChoose::Single,
                    StyleSheet::default(),
                    (&mut output_device2, &mut input_device2, None),
                )
                .await
                .map_err(|e| anyhow::anyhow!("Choose error: {e}"))?;

                // Restart daemon
                if let Err(e) = crate::daemon::restart_daemon().await {
                    eprintln!("Warning: Failed to restart daemon: {e}");
                }
            }
        }
    }

    Ok(())
}

fn truncate(s: &str, max_len: usize) -> String {
    if s.len() > max_len {
        format!("{}...", &s[..max_len.saturating_sub(3)])
    } else {
        s.to_string()
    }
}

use std::str::FromStr;
