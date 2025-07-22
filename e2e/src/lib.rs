//! Common utilities and types for Almanac CLI end-to-end tests

pub mod utils;

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use thiserror::Error;

/// Type alias for test results
pub type TestResult<T> = Result<T, E2EError>;

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

    /// FoundationDB cluster file path
    pub fdb_cluster_file: String,
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
            fdb_cluster_file: std::env::var("FDB_CLUSTER_FILE")
                .unwrap_or_else(|_| "/etc/foundationdb/fdb.cluster".to_string()),
        }
    }
}

impl TestConfig {
    /// Create test config from environment variables
    pub fn from_env() -> Result<Self, E2EError> {
        Ok(Self {
            fdb_cluster_file: std::env::var("FDB_CLUSTER_FILE")
                .unwrap_or_else(|_| "/etc/foundationdb/fdb.cluster".to_string()),
            command_timeout: std::env::var("TEST_TIMEOUT")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(300), // 5 minutes default for e2e tests
            verbose: std::env::var("RUST_LOG").is_ok(),
            ..Default::default()
        })
    }
}

/// Test execution result information
#[derive(Debug, Clone)]
pub struct TestOutcome {
    pub name: String,
    pub success: bool,
    pub duration: std::time::Duration,
    pub output: String,
    pub error: Option<String>,
}

impl TestOutcome {
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
    pub results: Vec<TestOutcome>,
    pub total_duration: std::time::Duration,
}

impl TestSuiteResults {
    pub fn add_result(&mut self, result: TestOutcome) {
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
