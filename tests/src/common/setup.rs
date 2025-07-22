//! Common test setup utilities

use std::process::Command;
use anyhow::Result;

/// Common test infrastructure setup
pub struct TestInfrastructure {
    pub temp_dir: tempfile::TempDir,
    pub processes: Vec<std::process::Child>,
}

impl TestInfrastructure {
    /// Create new test infrastructure
    pub fn new() -> Result<Self> {
        let temp_dir = tempfile::tempdir()?;
        Ok(Self {
            temp_dir,
            processes: Vec::new(),
        })
    }
    
    /// Start a process and track it for cleanup
    pub fn start_process(&mut self, mut cmd: Command) -> Result<()> {
        let child = cmd.spawn()?;
        self.processes.push(child);
        Ok(())
    }
    
    /// Wait for a service to be ready
    pub async fn wait_for_service(url: &str, timeout_secs: u64) -> Result<()> {
        let start = std::time::Instant::now();
        let timeout = std::time::Duration::from_secs(timeout_secs);
        
        while start.elapsed() < timeout {
            if reqwest::get(url).await.is_ok() {
                return Ok(());
            }
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        }
        
        anyhow::bail!("Service at {} did not become ready within {} seconds", url, timeout_secs)
    }
}

impl Drop for TestInfrastructure {
    fn drop(&mut self) {
        // Clean up all processes
        for mut process in self.processes.drain(..) {
            let _ = process.kill();
            let _ = process.wait();
        }
    }
}