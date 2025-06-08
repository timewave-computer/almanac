//! Common utilities and types for Almanac CLI end-to-end tests

pub mod utils;

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum E2EError {
    #[error("CLI command failed: {command}")]
    CliCommandFailed { command: String },
    
    #[error("Generated code compilation failed: {details}")]
    CompilationFailed { details: String },
    
    #[error("File validation failed: {file} - {reason}")]
    FileValidationFailed { file: String, reason: String },
    
    #[error("Test setup failed: {reason}")]
    SetupFailed { reason: String },
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("JSON parsing error: {0}")]
    Json(#[from] serde_json::Error),
    
    #[error("TOML parsing error: {0}")]
    Toml(#[from] toml::de::Error),
}

/// Test configuration for CLI commands
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestConfig {
    /// Path to the almanac binary
    pub almanac_binary: PathBuf,
    
    /// Temporary directory for test outputs
    pub temp_dir: PathBuf,
    
    /// Timeout for CLI commands (in seconds)
    pub command_timeout: u64,
    
    /// Whether to clean up test artifacts
    pub cleanup: bool,
    
    /// Verbose output for debugging
    pub verbose: bool,
    
    /// Test fixtures directory
    pub fixtures_dir: PathBuf,
}

impl Default for TestConfig {
    fn default() -> Self {
        Self {
            almanac_binary: PathBuf::from("../target/debug/almanac"),
            temp_dir: std::env::temp_dir().join("almanac-e2e-tests"),
            command_timeout: 120, // 2 minutes
            cleanup: true,
            verbose: false,
            fixtures_dir: PathBuf::from("fixtures"),
        }
    }
}

/// Test result information
#[derive(Debug, Clone)]
pub struct TestResult {
    pub name: String,
    pub success: bool,
    pub duration: std::time::Duration,
    pub output: String,
    pub error: Option<String>,
}

impl TestResult {
    pub fn success(name: String, duration: std::time::Duration, output: String) -> Self {
        Self {
            name,
            success: true,
            duration,
            output,
            error: None,
        }
    }
    
    pub fn failure(name: String, duration: std::time::Duration, error: String) -> Self {
        Self {
            name,
            success: false,
            duration,
            output: String::new(),
            error: Some(error),
        }
    }
}

/// Test suite results
#[derive(Debug, Default)]
pub struct TestSuiteResults {
    pub results: Vec<TestResult>,
    pub total_duration: std::time::Duration,
}

impl TestSuiteResults {
    pub fn add_result(&mut self, result: TestResult) {
        self.total_duration += result.duration;
        self.results.push(result);
    }
    
    pub fn success_count(&self) -> usize {
        self.results.iter().filter(|r| r.success).count()
    }
    
    pub fn failure_count(&self) -> usize {
        self.results.iter().filter(|r| !r.success).count()
    }
    
    pub fn success_rate(&self) -> f64 {
        if self.results.is_empty() {
            0.0
        } else {
            self.success_count() as f64 / self.results.len() as f64
        }
    }
} 