pub mod app;
pub mod state;

#[cfg(test)]
mod tests;

use anyhow::Result;
use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::io::{self, Write};

pub async fn run_tui() -> Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;

    // Run the app
    let result = app::run_app(&mut stdout).await;

    // Restore terminal
    disable_raw_mode()?;
    execute!(stdout, LeaveAlternateScreen)?;
    stdout.flush()?;

    result
}