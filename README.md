# singleschedule

A simple cron-based task scheduler daemon for running commands at specified intervals.

## Features

- Schedule commands using standard cron expressions
- Run tasks as a background daemon
- Manage tasks with add/remove/list commands
- Tasks are persisted in `~/.singleschedule/events.json`
- Each task has a unique slug identifier

## Installation

```bash
cargo install --path .
```

## Usage

### Start the daemon

```bash
singleschedule start
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

### Remove a task

```bash
singleschedule remove --slug my-task
```

### Stop the daemon

```bash
singleschedule stop
```

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
      "last_run": "2025-01-25T12:01:00Z"
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