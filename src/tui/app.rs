use std::io::{Stdout, Write};
use anyhow::Result;
use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    execute,
    style::{Color, Print, ResetColor, SetBackgroundColor, SetForegroundColor},
    terminal::{self, Clear, ClearType},
};

use crate::tui::state::{AppState, AppMode, NewTaskInput};

pub async fn run_app(stdout: &mut Stdout) -> Result<()> {
    let mut state = AppState::load_from_storage().await?;
    
    loop {
        // Clear screen and draw UI
        draw_ui(stdout, &state)?;
        
        // Handle input
        if let Event::Key(key) = event::read()? {
            match handle_input(&mut state, key).await? {
                InputResult::Exit => break,
                InputResult::Continue => {},
            }
        }
    }
    
    Ok(())
}

enum InputResult {
    Exit,
    Continue,
}

async fn handle_input(state: &mut AppState, key: KeyEvent) -> Result<InputResult> {
    match state.mode {
        AppMode::Normal => match key.code {
            KeyCode::Esc => {
                return Ok(InputResult::Exit);
            }
            KeyCode::Char('q') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                return Ok(InputResult::Exit);
            }
            KeyCode::Up => {
                state.move_selection_up();
            }
            KeyCode::Down => {
                state.move_selection_down();
            }
            KeyCode::Char(' ') => {
                state.toggle_selected_task_active();
                state.save_to_storage().await?;
            }
            KeyCode::Char('a') => {
                state.mode = AppMode::AddingTask;
                state.new_task = NewTaskInput::default();
            }
            KeyCode::Char('d') => {
                if !state.tasks.is_empty() {
                    state.mode = AppMode::ConfirmDelete(state.selected_index);
                }
            }
            KeyCode::Char('r') => {
                // Refresh from storage
                *state = AppState::load_from_storage().await?;
            }
            KeyCode::Enter => {
                // Toggle daemon for selected task
                if let Some(task) = state.get_selected_task() {
                    let task_slug = task.slug.clone();
                    let is_active = task.active;
                    tokio::spawn(async move {
                        let _ = if is_active {
                            crate::cli::handle_stop(vec![task_slug], false).await
                        } else {
                            crate::cli::handle_start(vec![task_slug], false).await
                        };
                    });
                }
            }
            _ => {}
        },
        AppMode::AddingTask => match key.code {
            KeyCode::Esc => {
                state.mode = AppMode::Normal;
            }
            KeyCode::Tab => {
                state.new_task.current_field = (state.new_task.current_field + 1) % 3;
            }
            KeyCode::Enter => {
                if state.new_task.current_field < 2 {
                    state.new_task.current_field += 1;
                } else {
                    // Try to create task
                    if let Some(task) = state.new_task.create_task() {
                        state.tasks.push(task);
                        state.save_to_storage().await?;
                        state.mode = AppMode::Normal;
                        state.message = Some("Task added successfully".to_string());
                    } else {
                        state.message = Some("Invalid input. Please check all fields.".to_string());
                    }
                }
            }
            KeyCode::Backspace => {
                state.new_task.handle_backspace();
            }
            KeyCode::Char(c) => {
                state.new_task.handle_char(c);
            }
            _ => {}
        },
        AppMode::ConfirmDelete(index) => match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') => {
                if index < state.tasks.len() {
                    state.tasks.remove(index);
                    if state.selected_index >= state.tasks.len() && state.selected_index > 0 {
                        state.selected_index = state.tasks.len() - 1;
                    }
                    state.save_to_storage().await?;
                    state.message = Some("Task deleted successfully".to_string());
                }
                state.mode = AppMode::Normal;
            }
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                state.mode = AppMode::Normal;
            }
            _ => {}
        }
    }
    
    Ok(InputResult::Continue)
}

fn draw_ui(stdout: &mut Stdout, state: &AppState) -> Result<()> {
    execute!(stdout, Clear(ClearType::All), cursor::MoveTo(0, 0))?;
    
    // Header
    execute!(
        stdout,
        SetForegroundColor(Color::Blue),
        Print("SingleSchedule TUI"),
        ResetColor,
        Print(" | Press "),
        SetForegroundColor(Color::Yellow),
        Print("ESC"),
        ResetColor,
        Print(" to quit | "),
        SetForegroundColor(Color::Yellow),
        Print("a"),
        ResetColor,
        Print(" to add | "),
        SetForegroundColor(Color::Yellow),
        Print("d"),
        ResetColor,
        Print(" to delete | "),
        SetForegroundColor(Color::Yellow),
        Print("Space"),
        ResetColor,
        Print(" to toggle\n\n"),
    )?;
    
    // Task list
    if state.tasks.is_empty() {
        execute!(
            stdout,
            SetForegroundColor(Color::DarkGrey),
            Print("No tasks scheduled. Press 'a' to add a task.\n"),
            ResetColor,
        )?;
    } else {
        // Headers
        execute!(
            stdout,
            Print(format!("{:<8} {:<20} {:<20} {:<30} {:<15}\n",
                "Status", "Slug", "Cron", "Command", "Last Run")),
            Print(format!("{}\n", "-".repeat(95))),
        )?;
        
        // Tasks
        for (index, task) in state.tasks.iter().enumerate() {
            let is_selected = index == state.selected_index;
            
            if is_selected {
                execute!(stdout, SetBackgroundColor(Color::DarkGrey))?;
            }
            
            let status_icon = if task.active { "●" } else { "○" };
            let status_color = if task.active { Color::Green } else { Color::Red };
            let last_run = task
                .last_run
                .map(|dt| dt.format("%Y-%m-%d %H:%M").to_string())
                .unwrap_or_else(|| "Never".to_string());
            
            let command_display = if task.command.len() > 27 {
                format!("{}...", &task.command[..27])
            } else {
                task.command.clone()
            };
            
            execute!(
                stdout,
                SetForegroundColor(status_color),
                Print(format!("{status_icon:<8}")),
                SetForegroundColor(Color::Cyan),
                Print(format!("{:<20}", task.slug)),
                SetForegroundColor(Color::Yellow),
                Print(format!("{:<20}", task.cron)),
                SetForegroundColor(Color::White),
                Print(format!("{command_display:<30}")),
                SetForegroundColor(Color::Magenta),
                Print(format!("{last_run:<15}")),
                ResetColor,
                Print("\n"),
            )?;
        }
    }
    
    // Status message
    if let Some(message) = &state.message {
        execute!(
            stdout,
            cursor::MoveTo(0, terminal::size()?.1 - 2),
            SetForegroundColor(Color::Yellow),
            Print(message),
            ResetColor,
        )?;
    }
    
    // Modal dialogs
    match &state.mode {
        AppMode::AddingTask => draw_add_dialog(stdout, state)?,
        AppMode::ConfirmDelete(index) => draw_confirm_dialog(stdout, state, *index)?,
        AppMode::Normal => {}
    }
    
    stdout.flush()?;
    Ok(())
}

fn draw_add_dialog(stdout: &mut Stdout, state: &AppState) -> Result<()> {
    let (width, height) = terminal::size()?;
    let dialog_width = 60;
    let dialog_height = 12;
    let x = (width.saturating_sub(dialog_width)) / 2;
    let y = (height.saturating_sub(dialog_height)) / 2;
    
    // Clear dialog area with border
    for row in 0..dialog_height {
        execute!(stdout, cursor::MoveTo(x, y + row))?;
        if row == 0 {
            execute!(stdout, Print(format!("╔{}╗", "═".repeat(dialog_width as usize - 2))))?;
        } else if row == dialog_height - 1 {
            execute!(stdout, Print(format!("╚{}╝", "═".repeat(dialog_width as usize - 2))))?;
        } else {
            execute!(stdout, Print(format!("║{}║", " ".repeat(dialog_width as usize - 2))))?;
        }
    }
    
    // Title
    execute!(
        stdout,
        cursor::MoveTo(x + 2, y + 1),
        SetForegroundColor(Color::Cyan),
        Print("Add New Task"),
        ResetColor,
    )?;
    
    // Fields
    let fields = [
        ("Slug:", &state.new_task.slug, 0),
        ("Cron:", &state.new_task.cron, 1),
        ("Command:", &state.new_task.command, 2),
    ];
    
    for (i, (label, value, field_index)) in fields.iter().enumerate() {
        let field_y = y + 3 + (i * 2) as u16;
        let is_active = state.new_task.current_field == *field_index;
        
        execute!(
            stdout,
            cursor::MoveTo(x + 2, field_y),
            Print(label),
            cursor::MoveTo(x + 12, field_y),
        )?;
        
        if is_active {
            execute!(stdout, SetBackgroundColor(Color::DarkGrey))?;
        }
        
        execute!(
            stdout,
            Print(format!("{value:<45}")),
            ResetColor,
        )?;
        
        if is_active {
            execute!(
                stdout,
                cursor::MoveTo(x + 12 + value.len() as u16, field_y),
                Print("█"),
            )?;
        }
    }
    
    // Instructions
    execute!(
        stdout,
        cursor::MoveTo(x + 2, y + 10),
        SetForegroundColor(Color::DarkGrey),
        Print("Tab: Next field | Enter: Submit | Esc: Cancel"),
        ResetColor,
    )?;
    
    Ok(())
}

fn draw_confirm_dialog(stdout: &mut Stdout, state: &AppState, task_index: usize) -> Result<()> {
    let (width, height) = terminal::size()?;
    let dialog_width = 50;
    let dialog_height = 8;
    let x = (width.saturating_sub(dialog_width)) / 2;
    let y = (height.saturating_sub(dialog_height)) / 2;
    
    // Clear dialog area with border
    for row in 0..dialog_height {
        execute!(stdout, cursor::MoveTo(x, y + row))?;
        if row == 0 {
            execute!(
                stdout,
                SetForegroundColor(Color::Red),
                Print(format!("╔{}╗", "═".repeat(dialog_width as usize - 2))),
                ResetColor,
            )?;
        } else if row == dialog_height - 1 {
            execute!(
                stdout,
                SetForegroundColor(Color::Red),
                Print(format!("╚{}╝", "═".repeat(dialog_width as usize - 2))),
                ResetColor,
            )?;
        } else {
            execute!(
                stdout,
                SetForegroundColor(Color::Red),
                Print(format!("║{}║", " ".repeat(dialog_width as usize - 2))),
                ResetColor,
            )?;
        }
    }
    
    // Title
    execute!(
        stdout,
        cursor::MoveTo(x + 2, y + 1),
        SetForegroundColor(Color::Red),
        Print("Confirm Delete"),
        ResetColor,
    )?;
    
    // Message
    if let Some(task) = state.tasks.get(task_index) {
        execute!(
            stdout,
            cursor::MoveTo(x + 2, y + 3),
            Print(format!("Delete task '{}'?", task.slug)),
        )?;
    }
    
    // Buttons
    execute!(
        stdout,
        cursor::MoveTo(x + 15, y + 5),
        Print("Press "),
        SetForegroundColor(Color::Red),
        Print("Y"),
        ResetColor,
        Print(" to confirm, "),
        SetForegroundColor(Color::Green),
        Print("N"),
        ResetColor,
        Print(" to cancel"),
    )?;
    
    Ok(())
}