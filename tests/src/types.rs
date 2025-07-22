//! Common types for the test suite

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Test error type
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

/// Test configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestConfig {
    pub binary_path: String,
    pub test_name: String,
    pub timeout_seconds: u64,
    pub extra_args: Vec<String>,
}

impl TestConfig {
    pub fn from_env() -> anyhow::Result<Self> {
        Ok(Self {
            binary_path: std::env::var("ALMANAC_BINARY")
                .unwrap_or_else(|_| "target/debug/almanac".to_string()),
            test_name: "default".to_string(),
            timeout_seconds: 30,
            extra_args: vec![],
        })
    }
}

/// Individual test result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestOutcome {
    pub name: String,
    pub success: bool,
    pub duration_ms: u64,
    pub error_message: Option<String>,
    pub stdout: Option<String>,
    pub stderr: Option<String>,
}

impl TestOutcome {
    /// Create a successful test outcome
    pub fn success(name: &str, duration_ms: u64) -> Self {
        Self {
            name: name.to_string(),
            success: true,
            duration_ms,
            error_message: None,
            stdout: None,
            stderr: None,
        }
    }
    
    /// Create a failed test outcome
    pub fn failure(name: &str, duration_ms: u64, error: &str) -> Self {
        Self {
            name: name.to_string(),
            success: false,
            duration_ms,
            error_message: Some(error.to_string()),
            stdout: None,
            stderr: None,
        }
    }
    
    /// Get duration in milliseconds (compatibility)
    pub fn duration(&self) -> u64 {
        self.duration_ms
    }
}

/// Test suite results
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct TestSuiteResults {
    pub passed: Vec<TestOutcome>,
    pub failed: Vec<TestOutcome>,
    pub total_duration_ms: u64,
}

impl TestSuiteResults {
    pub fn add_result(&mut self, result: TestOutcome) {
        if result.success {
            self.passed.push(result);
        } else {
            self.failed.push(result);
        }
    }

    pub fn success_rate(&self) -> f64 {
        let total = self.passed.len() + self.failed.len();
        if total == 0 {
            0.0
        } else {
            self.passed.len() as f64 / total as f64
        }
    }
}