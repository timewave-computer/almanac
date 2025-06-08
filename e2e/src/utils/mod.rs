//! Utility modules for end-to-end tests

pub mod setup;
pub mod fixtures;
pub mod validation;

use crate::{E2EError, TestConfig, TestResult};
use anyhow::Result;
use assert_cmd::Command;
use std::path::Path;
use std::time::Instant;

/// Execute a CLI command and return the result
pub async fn run_cli_command(
    config: &TestConfig,
    args: &[&str],
    test_name: &str,
) -> TestResult {
    let start = Instant::now();
    
    let mut cmd = Command::new(&config.almanac_binary);
    cmd.args(args);
    
    if config.verbose {
        eprintln!("Running: {:?} {}", config.almanac_binary, args.join(" "));
    }
    
    match cmd.timeout(std::time::Duration::from_secs(config.command_timeout)).output() {
        Ok(output) => {
            let duration = start.elapsed();
            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            
            if output.status.success() {
                TestResult::success(test_name.to_string(), duration, stdout)
            } else {
                TestResult::failure(
                    test_name.to_string(),
                    duration,
                    format!("Command failed with exit code {:?}\nStderr: {}", output.status.code(), stderr)
                )
            }
        }
        Err(e) => {
            let duration = start.elapsed();
            TestResult::failure(
                test_name.to_string(),
                duration,
                format!("Failed to execute command: {}", e)
            )
        }
    }
}

/// Check if a file exists and is not empty
pub fn file_exists_and_not_empty<P: AsRef<Path>>(path: P) -> bool {
    let path = path.as_ref();
    path.exists() && path.is_file() && path.metadata().is_ok_and(|m| m.len() > 0)
}

/// Check if a directory exists and is not empty
pub fn dir_exists_and_not_empty<P: AsRef<Path>>(path: P) -> bool {
    let path = path.as_ref();
    path.exists() && path.is_dir() && 
        path.read_dir().is_ok_and(|mut entries| entries.next().is_some())
}

/// Validate that generated Rust code compiles
pub async fn validate_rust_compilation<P: AsRef<Path>>(
    project_dir: P,
    test_name: &str,
) -> Result<TestResult, E2EError> {
    let start = Instant::now();
    let project_dir = project_dir.as_ref();
    
    // Check if Cargo.toml exists
    let cargo_toml = project_dir.join("Cargo.toml");
    if !cargo_toml.exists() {
        return Ok(TestResult::failure(
            test_name.to_string(),
            start.elapsed(),
            "No Cargo.toml found in generated project".to_string()
        ));
    }
    
    // Run cargo check
    let mut cmd = Command::new("cargo");
    cmd.arg("check")
       .current_dir(project_dir)
       .timeout(std::time::Duration::from_secs(120));
    
    match cmd.output() {
        Ok(output) => {
            let duration = start.elapsed();
            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            
            if output.status.success() {
                Ok(TestResult::success(test_name.to_string(), duration, stdout))
            } else {
                Ok(TestResult::failure(
                    test_name.to_string(),
                    duration,
                    format!("Compilation failed:\nStdout: {}\nStderr: {}", stdout, stderr)
                ))
            }
        }
        Err(e) => Ok(TestResult::failure(
            test_name.to_string(),
            start.elapsed(),
            format!("Failed to run cargo check: {}", e)
        ))
    }
}

/// Format test results for display
pub fn format_test_results(results: &[TestResult]) -> String {
    use colored::*;
    
    let mut output = String::new();
    
    for result in results {
        let status = if result.success {
            "PASS".green().bold()
        } else {
            "FAIL".red().bold()
        };
        
        let duration = format!("{:.2}s", result.duration.as_secs_f64());
        
        output.push_str(&format!(
            "[{}] {} ({})\n",
            status,
            result.name,
            duration.dimmed()
        ));
        
        if !result.success {
            if let Some(error) = &result.error {
                output.push_str(&format!("    Error: {}\n", error.red()));
            }
        }
    }
    
    output
}

/// Print test summary
pub fn print_test_summary(results: &[TestResult]) {
    use colored::*;
    
    let total = results.len();
    let passed = results.iter().filter(|r| r.success).count();
    let failed = total - passed;
    let total_duration: std::time::Duration = results.iter().map(|r| r.duration).sum();
    
    println!("\n{}", "=".repeat(60));
    println!("{}", "TEST SUMMARY".bold());
    println!("{}", "=".repeat(60));
    
    println!("Total tests: {}", total);
    println!("Passed: {}", passed.to_string().green());
    println!("Failed: {}", failed.to_string().red());
    println!("Success rate: {:.1}%", (passed as f64 / total as f64) * 100.0);
    println!("Total duration: {:.2}s", total_duration.as_secs_f64());
    
    if failed > 0 {
        println!("\n{}", "FAILED TESTS:".red().bold());
        for result in results.iter().filter(|r| !r.success) {
            println!("  - {}", result.name.red());
        }
    }
} 