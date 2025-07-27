// Tests for the old full-screen TUI implementation have been disabled
// as we're now using a simple command-based interface that preserves
// terminal features like copy/paste.

// The old tests were for:
// - state management (AppState, AppMode, NewTaskInput)
// - TUI components (TaskListComponent, AddTaskDialog, DeleteConfirmDialog)
// - keyboard navigation and event handling
//
// These are no longer relevant with the simple interface.
//
// TODO: Write new tests for the simple_interface module that test:
// - Command parsing
// - Task selection with choose()
// - Add/delete/toggle operations
