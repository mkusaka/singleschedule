# TUI Limitations and Known Issues

## Text Selection and Copy/Paste

R3BL TUI operates in terminal raw mode, which disables standard text selection and copy/paste functionality. This is a fundamental limitation of terminal-based user interfaces in raw mode:

- **Why it happens**: Raw mode gives the application full control over the terminal, but this means the terminal's native text selection is disabled.
- **Workaround**: To copy text from the TUI:
  1. Exit the TUI (ESC, x, Ctrl+Q, or Ctrl+C)
  2. Run `singleschedule list` to see your tasks in standard output
  3. Copy text normally from the terminal

## Alternative Solutions

If you need to frequently copy task information:

1. **Use the CLI commands**: 
   ```bash
   singleschedule list               # List all tasks
   singleschedule list --active      # List only active tasks
   singleschedule list --json        # JSON output for scripting
   ```

2. **Export to file**:
   ```bash
   singleschedule list > tasks.txt   # Save to file
   ```

3. **Use terminal multiplexers**: Tools like `tmux` or `screen` have their own copy modes that work with TUI applications.

## Exit Key Functionality

The TUI supports multiple exit methods:
- **ESC**: Exit the application
- **x**: Exit the application
- **Ctrl+Q**: Exit the application
- **Ctrl+C**: Force quit (interrupt signal)

If these keys don't work, ensure your terminal emulator properly passes these key combinations to the application.