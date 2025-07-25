use anyhow::Result;
use log::{error, info};
use std::fs;
use std::path::PathBuf;
use tokio::signal;

use crate::scheduler::Scheduler;

pub async fn start_daemon() -> Result<()> {
    let pid_file = get_pid_file()?;

    // Check if daemon is already running
    if pid_file.exists() {
        let pid = fs::read_to_string(&pid_file)?.trim().parse::<u32>()?;
        if is_process_running(pid) {
            return Err(anyhow::anyhow!(
                "Daemon is already running with PID {}",
                pid
            ));
        }
        // Clean up stale PID file
        fs::remove_file(&pid_file)?;
    }

    // Fork the daemon process
    let daemon = daemonize::Daemonize::new()
        .pid_file(&pid_file)
        .working_directory("/tmp")
        .umask(0o027);

    match daemon.start() {
        Ok(_) => {
            info!("Daemon started successfully");
            run_scheduler().await?;
        }
        Err(e) => return Err(anyhow::anyhow!("Failed to start daemon: {}", e)),
    }

    Ok(())
}

pub async fn stop_daemon() -> Result<()> {
    let pid_file = get_pid_file()?;

    if !pid_file.exists() {
        return Err(anyhow::anyhow!("Daemon is not running"));
    }

    let pid = fs::read_to_string(&pid_file)?.trim().parse::<u32>()?;

    if !is_process_running(pid) {
        fs::remove_file(&pid_file)?;
        return Err(anyhow::anyhow!(
            "Daemon is not running (stale PID file removed)"
        ));
    }

    // Send SIGTERM to the daemon
    use nix::sys::signal::{self, Signal};
    use nix::unistd::Pid;

    signal::kill(Pid::from_raw(pid as i32), Signal::SIGTERM)
        .map_err(|e| anyhow::anyhow!("Failed to stop daemon: {}", e))?;

    // Wait a bit for the process to terminate
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    // Clean up PID file
    if pid_file.exists() {
        fs::remove_file(&pid_file)?;
    }

    println!("Daemon stopped successfully");
    Ok(())
}

pub async fn restart_daemon() -> Result<()> {
    // Try to stop existing daemon
    let _ = stop_daemon().await;

    // Start new daemon
    start_daemon().await
}

async fn run_scheduler() -> Result<()> {
    info!("Starting scheduler");

    let mut scheduler = Scheduler::new();
    scheduler.load_events().await?;

    // Set up signal handler for graceful shutdown
    let shutdown_signal = async {
        let _ = signal::ctrl_c().await;
        info!("Received shutdown signal");
    };

    tokio::select! {
        result = scheduler.run() => {
            if let Err(e) = result {
                error!("Scheduler error: {e}");
            }
        }
        _ = shutdown_signal => {
            info!("Shutting down scheduler");
        }
    }

    // Clean up PID file on exit
    let pid_file = get_pid_file()?;
    if pid_file.exists() {
        fs::remove_file(&pid_file)?;
    }

    Ok(())
}

fn get_pid_file() -> Result<PathBuf> {
    #[cfg(test)]
    {
        if let Ok(test_home) = std::env::var("SINGLESCHEDULE_TEST_HOME") {
            let dir = PathBuf::from(test_home).join(".singleschedule");
            fs::create_dir_all(&dir)?;
            return Ok(dir.join("daemon.pid"));
        }
    }

    let home = directories::UserDirs::new()
        .ok_or_else(|| anyhow::anyhow!("Failed to get home directory"))?
        .home_dir()
        .to_path_buf();

    let dir = home.join(".singleschedule");
    fs::create_dir_all(&dir)?;

    Ok(dir.join("daemon.pid"))
}

fn is_process_running(pid: u32) -> bool {
    // Try to send signal 0 to check if process exists
    use nix::sys::signal;
    use nix::unistd::Pid;

    // Signal 0 is used to check if process exists without sending actual signal
    signal::kill(Pid::from_raw(pid as i32), None).is_ok()
}
