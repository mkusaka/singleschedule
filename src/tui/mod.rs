// Full-screen TUI modules (commented out - not used)
// pub mod add_task_dialog;
// pub mod app_r3bl;
// pub mod delete_confirm_dialog;
// pub mod state;
// pub mod task_list_component;

// Interface modules
pub mod simple_interface;
// pub mod interactive_interface; // Unused - requires raw mode
// pub mod hybrid_interface; // Removed - requires crossterm

#[cfg(test)]
mod tests;

// Use the simple interface that already has arrow key selection for delete/toggle
// while preserving copy/paste functionality
pub async fn run_tui() -> anyhow::Result<()> {
    simple_interface::run_simple_tui().await
}
