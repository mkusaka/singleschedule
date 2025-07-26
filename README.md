# singleschedule

A simple cron-based task scheduler daemon for running commands at specified intervals.

## Features

- Schedule commands using standard cron expressions
- Run tasks as a background daemon
- Manage tasks with add/remove/list commands
- Tasks are persisted in `~/.singleschedule/events.json`
- Each task has a unique slug identifier
- Selective task control - start/stop individual tasks without affecting others
- Task status tracking (active/inactive)

## Installation

```bash
cargo install --path .
```

## Usage

### Start the daemon

```bash
# Start daemon with all tasks active
singleschedule start

# Start daemon with specific tasks active
singleschedule start task1 task2

# Explicitly start all tasks
singleschedule start --all
```

### Add a scheduled task

```bash
# Run a command every minute
singleschedule add --slug my-task --cron "0 * * * * *" -- echo "Hello, World!"

# Run a backup script daily at 2 AM
singleschedule add --slug daily-backup --cron "0 0 2 * * *" -- /path/to/backup.sh

# Send a webhook every 30 minutes
singleschedule add --slug webhook --cron "0 */30 * * * *" -- curl -X POST https://example.com/webhook
```

### List scheduled tasks

```bash
singleschedule list
```

Output shows task status (Active/Inactive):
```
SLUG                 CRON                 COMMAND                                  STATUS     LAST RUN       
---------------------------------------------------------------------------------------------------------
task1                */10 * * * * *       echo Task 1                              Active     2025-01-25 12:00
task2                */15 * * * * *       echo Task 2                              Inactive   Never
```

### Remove a task

```bash
singleschedule remove --slug my-task
```

### Stop the daemon

```bash
# Stop the entire daemon
singleschedule stop

# Stop specific tasks (keeps daemon running if other tasks are active)
singleschedule stop task1 task2

# Explicitly stop all tasks and daemon
singleschedule stop --all
```

## Selective Task Control

The scheduler supports fine-grained control over individual tasks without affecting others:

### Starting specific tasks
```bash
# Start only task1 and task3
singleschedule start task1 task3

# This will:
# - Mark task1 and task3 as active
# - Start the daemon if not running
# - Only these tasks will execute on their schedules
```

### Stopping specific tasks
```bash
# Stop only task2
singleschedule stop task2

# This will:
# - Mark task2 as inactive
# - Keep daemon running if other tasks are active
# - task2 will not execute until reactivated
```

### Use cases
- **Maintenance**: Temporarily disable a task without removing it
- **Testing**: Run only specific tasks during development
- **Resource management**: Control which tasks run based on system load
- **Debugging**: Isolate problematic tasks

## Cron Expression Format

The cron expression follows the format (including seconds):

```
* * * * * *
│ │ │ │ │ │
│ │ │ │ │ └─── Day of week (0-6, Sunday = 0)
│ │ │ │ └───── Month (1-12)
│ │ │ └─────── Day of month (1-31)
│ │ └───────── Hour (0-23)
│ └─────────── Minute (0-59)
└───────────── Second (0-59)
```

Examples:
- `* * * * * *` - Every second
- `0 * * * * *` - Every minute
- `0 0 * * * *` - Every hour
- `0 0 0 * * *` - Daily at midnight
- `0 0 9-17 * * MON-FRI` - Every hour from 9 AM to 5 PM on weekdays
- `0 */5 * * * *` - Every 5 minutes

## Data Storage

Tasks are stored in `~/.singleschedule/events.json` with the following structure:

```json
{
  "events": [
    {
      "slug": "my-task",
      "cron": "0 * * * * *",
      "command": "echo Hello",
      "pid": null,
      "created_at": "2025-01-25T12:00:00Z",
      "last_run": "2025-01-25T12:01:00Z",
      "active": true
    }
  ]
}
```

## Development

### Running tests

```bash
cargo test
```

### Building

```bash
cargo build --release
```

## License

MIT