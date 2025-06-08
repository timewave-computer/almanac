//! Test environment setup and cleanup utilities

use crate::{E2EError, TestConfig};
use anyhow::Result;
use std::fs;
use std::path::Path;
use tempfile::TempDir;

/// Test environment setup
pub struct TestEnvironment {
    pub temp_dir: TempDir,
    pub config: TestConfig,
}

impl TestEnvironment {
    /// Create a new test environment
    pub fn new() -> Result<Self> {
        let temp_dir = TempDir::new()?;
        let config = TestConfig {
            temp_dir: temp_dir.path().to_path_buf(),
            ..Default::default()
        };
        
        // Ensure fixtures directory exists
        let fixtures_dir = Path::new("fixtures");
        if !fixtures_dir.exists() {
            fs::create_dir_all(fixtures_dir)
                .map_err(|e| E2EError::SetupFailed {
                    reason: format!("Failed to create fixtures directory: {}", e)
                })?;
        }
        
        Ok(Self { temp_dir, config })
    }
    
    /// Create a test output directory
    pub fn create_output_dir(&self, name: &str) -> Result<std::path::PathBuf, E2EError> {
        let output_dir = self.temp_dir.path().join(name);
        fs::create_dir_all(&output_dir)
            .map_err(|e| E2EError::SetupFailed {
                reason: format!("Failed to create output directory {}: {}", name, e)
            })?;
        Ok(output_dir)
    }
    
    /// Get path to fixtures directory
    pub fn fixtures_dir(&self) -> std::path::PathBuf {
        Path::new("fixtures").to_path_buf()
    }
    
    /// Cleanup test environment (called automatically on drop)
    pub fn cleanup(&self) -> Result<(), E2EError> {
        // TempDir handles cleanup automatically, but we can do additional cleanup here
        Ok(())
    }
}

impl Drop for TestEnvironment {
    fn drop(&mut self) {
        if let Err(e) = self.cleanup() {
            eprintln!("Warning: Failed to cleanup test environment: {}", e);
        }
    }
}

/// Check if Almanac binary exists and is executable
pub fn check_almanac_binary(config: &TestConfig) -> Result<(), E2EError> {
    let binary_path = &config.almanac_binary;
    
    if !binary_path.exists() {
        return Err(E2EError::SetupFailed {
            reason: format!("Almanac binary not found at {:?}", binary_path)
        });
    }
    
    // Try to run --version to verify it's executable
    let output = std::process::Command::new(binary_path)
        .arg("--version")
        .output()
        .map_err(|e| E2EError::SetupFailed {
            reason: format!("Failed to execute almanac binary: {}", e)
        })?;
    
    if !output.status.success() {
        return Err(E2EError::SetupFailed {
            reason: "Almanac binary is not executable or failed version check".to_string()
        });
    }
    
    Ok(())
}

/// Build Almanac binary if it doesn't exist
pub async fn ensure_almanac_binary(config: &mut TestConfig) -> Result<(), E2EError> {
    if check_almanac_binary(config).is_ok() {
        return Ok(());
    }
    
    println!("Building Almanac binary...");
    
    // Try to build the binary using cargo
    let output = std::process::Command::new("cargo")
        .args(["build", "--bin", "almanac"])
        .current_dir("..")  // Assuming we're in e2e directory
        .output()
        .map_err(|e| E2EError::SetupFailed {
            reason: format!("Failed to run cargo build: {}", e)
        })?;
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(E2EError::SetupFailed {
            reason: format!("Failed to build almanac binary:\n{}", stderr)
        });
    }
    
    // Verify the binary was created
    check_almanac_binary(config)?;
    
    println!("✓ Almanac binary built successfully");
    Ok(())
}

/// Create a minimal Cargo.toml for testing generated code compilation
pub fn create_test_cargo_toml<P: AsRef<Path>>(project_dir: P, name: &str) -> Result<(), E2EError> {
    let project_dir = project_dir.as_ref();
    let cargo_toml_path = project_dir.join("Cargo.toml");
    
    let cargo_toml_content = format!(r#"[package]
name = "{}"
version = "0.1.0"
edition = "2021"

[dependencies]
serde = {{ version = "1.0", features = ["derive"] }}
serde_json = "1.0"
cosmwasm-std = "2.0"
anyhow = "1.0"
tokio = {{ version = "1.0", features = ["full"] }}

# Indexer dependencies (these might not exist in a real test, but we'll make them optional)
indexer-core = {{ path = "../../crates/core", optional = true }}
indexer-cosmos = {{ path = "../../crates/cosmos", optional = true }}
indexer-ethereum = {{ path = "../../crates/ethereum", optional = true }}

[features]
default = []
indexer = ["indexer-core", "indexer-cosmos", "indexer-ethereum"]
"#, name);
    
    fs::write(&cargo_toml_path, cargo_toml_content)
        .map_err(|e| E2EError::SetupFailed {
            reason: format!("Failed to create Cargo.toml: {}", e)
        })?;
    
    // Create src/lib.rs
    let src_dir = project_dir.join("src");
    fs::create_dir_all(&src_dir)
        .map_err(|e| E2EError::SetupFailed {
            reason: format!("Failed to create src directory: {}", e)
        })?;
    
    let lib_rs_content = r#"//! Generated test library

// Re-export generated modules if they exist
#[cfg(feature = "client")]
pub mod client;

#[cfg(feature = "storage")]
pub mod storage;

#[cfg(feature = "api")]
pub mod api;

// Provide a simple test to verify compilation
#[cfg(test)]
mod tests {
    #[test]
    fn test_compilation() {
        // This test just verifies that the code compiles
        assert!(true);
    }
}
"#;
    
    fs::write(src_dir.join("lib.rs"), lib_rs_content)
        .map_err(|e| E2EError::SetupFailed {
            reason: format!("Failed to create lib.rs: {}", e)
        })?;
    
    Ok(())
} 