//! Test utilities

pub mod fixtures;
pub mod setup;
pub mod validation;

use crate::{TestConfig, TestOutcome};
use std::process::Stdio;
use tokio::process::Command;
use std::time::Instant;

/// Run a CLI command and capture its output
pub async fn run_cli_command(
    config: &TestConfig,
    args: &[&str],
    test_name: &str,
) -> TestOutcome {
    let start = Instant::now();
    
    let mut command = Command::new(&config.binary_path);
    command.args(args);
    command.stdout(Stdio::piped());
    command.stderr(Stdio::piped());
    
    let output = match command.output().await {
        Ok(output) => output,
        Err(e) => {
            return TestOutcome {
                name: test_name.to_string(),
                success: false,
                duration_ms: start.elapsed().as_millis() as u64,
                error_message: Some(format!("Failed to execute command: {}", e)),
                stdout: None,
                stderr: None,
            };
        }
    };
    
    TestOutcome {
        name: test_name.to_string(),
        success: output.status.success(),
        duration_ms: start.elapsed().as_millis() as u64,
        error_message: if !output.status.success() {
            Some(format!("Command exited with status: {}", output.status))
        } else {
            None
        },
        stdout: Some(String::from_utf8_lossy(&output.stdout).to_string()),
        stderr: Some(String::from_utf8_lossy(&output.stderr).to_string()),
    }
}

/// Validate Rust compilation
pub async fn validate_rust_compilation(project_dir: &str) -> anyhow::Result<()> {
    let output = Command::new("cargo")
        .arg("check")
        .current_dir(project_dir)
        .output()
        .await?;
        
    if !output.status.success() {
        anyhow::bail!("Cargo check failed: {}", String::from_utf8_lossy(&output.stderr));
    }
    
    Ok(())
}