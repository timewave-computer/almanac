//! Common test teardown utilities

use std::process::Command;
use anyhow::Result;

/// Clean up test artifacts
pub fn cleanup_test_artifacts(dir: &std::path::Path) -> Result<()> {
    if dir.exists() {
        std::fs::remove_dir_all(dir)?;
    }
    Ok(())
}

/// Stop a process by PID file
pub fn stop_process_by_pid_file(pid_file: &str) -> Result<()> {
    if let Ok(pid_str) = std::fs::read_to_string(pid_file) {
        if let Ok(pid) = pid_str.trim().parse::<i32>() {
            Command::new("kill")
                .arg(pid.to_string())
                .output()?;
        }
        std::fs::remove_file(pid_file)?;
    }
    Ok(())
}

/// Kill processes matching a pattern
pub fn kill_processes_matching(pattern: &str) -> Result<()> {
    Command::new("pkill")
        .arg("-f")
        .arg(pattern)
        .output()?;
    Ok(())
}