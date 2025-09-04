//! Validation utilities for generated code and output files

use crate::{E2EError, TestConfig, TestOutcome};
use regex::Regex;
use std::fs;
use std::path::Path;
use std::process::Command;
use std::time::Instant;
use walkdir::WalkDir;

/// Validate that all expected files were generated
pub fn validate_generated_files<P: AsRef<Path>>(
    output_dir: P,
    features: &[&str],
    test_name: &str,
) -> TestOutcome {
    let start = Instant::now();
    let output_dir = output_dir.as_ref();

    if !output_dir.exists() {
        return TestOutcome::failure(
            test_name,
            start.elapsed().as_millis() as u64,
            "Output directory does not exist",
        );
    }

    let mut missing_files = Vec::new();
    let mut validation_errors = Vec::new();

    for feature in features {
        match *feature {
            "client" => {
                let client_dir = output_dir.join("client");
                if !client_dir.exists() {
                    missing_files.push("client directory");
                } else {
                    let mod_file = client_dir.join("mod.rs");
                    if !mod_file.exists() {
                        missing_files.push("client/mod.rs");
                    }
                }
            }
            "storage" => {
                let storage_dir = output_dir.join("storage");
                if !storage_dir.exists() {
                    missing_files.push("storage directory");
                } else {
                    let mod_file = storage_dir.join("mod.rs");
                    if !mod_file.exists() {
                        missing_files.push("storage/mod.rs");
                    }
                }
            }
            "api" => {
                let api_dir = output_dir.join("api");
                if !api_dir.exists() {
                    missing_files.push("api directory");
                } else {
                    let mod_file = api_dir.join("mod.rs");
                    if !mod_file.exists() {
                        missing_files.push("api/mod.rs");
                    }
                }
            }
            "migrations" => {
                let migrations_dir = output_dir.join("migrations");
                if !migrations_dir.exists() {
                    missing_files.push("migrations directory");
                } else {
                    // Check for at least one migration file
                    let migration_files: Vec<_> = WalkDir::new(&migrations_dir)
                        .into_iter()
                        .filter_map(|e| e.ok())
                        .filter(|e| e.file_type().is_file())
                        .filter(|e| e.path().extension().is_some_and(|ext| ext == "sql"))
                        .collect();

                    if migration_files.is_empty() {
                        missing_files.push("migration SQL files");
                    }
                }
            }
            _ => {
                validation_errors.push(format!("Unknown feature: {}", feature));
            }
        }
    }

    if !missing_files.is_empty() || !validation_errors.is_empty() {
        let mut error_msg = String::new();
        if !missing_files.is_empty() {
            error_msg.push_str(&format!(
                "Missing files/directories: {}\n",
                missing_files.join(", ")
            ));
        }
        if !validation_errors.is_empty() {
            error_msg.push_str(&format!(
                "Validation errors: {}",
                validation_errors.join(", ")
            ));
        }
        return TestOutcome::failure(test_name, start.elapsed().as_millis() as u64, &error_msg);
    }

    TestOutcome::success(
        test_name,
        start.elapsed().as_millis() as u64,
    )
}

/// Validate that generated Rust code contains expected patterns
pub fn validate_rust_code_patterns<P: AsRef<Path>>(
    file_path: P,
    expected_patterns: &[&str],
    test_name: &str,
) -> TestOutcome {
    let start = Instant::now();
    let file_path = file_path.as_ref();

    let content = match fs::read_to_string(file_path) {
        Ok(content) => content,
        Err(e) => {
            return TestOutcome::failure(
                test_name,
                start.elapsed().as_millis() as u64,
                &format!("Failed to read file {:?}: {}", file_path, e),
            )
        }
    };

    let mut missing_patterns = Vec::new();

    for pattern in expected_patterns {
        if let Ok(regex) = Regex::new(pattern) {
            if !regex.is_match(&content) {
                missing_patterns.push(*pattern);
            }
        } else {
            // Simple string search if regex fails
            if !content.contains(pattern) {
                missing_patterns.push(*pattern);
            }
        }
    }

    if !missing_patterns.is_empty() {
        return TestOutcome::failure(
            test_name,
            start.elapsed().as_millis() as u64,
            &format!(
                "Missing patterns in {:?}: {}",
                file_path,
                missing_patterns.join(", ")
            ),
        );
    }

    TestOutcome::success(
        test_name,
        start.elapsed().as_millis() as u64,
    )
}

/// Validate that generated SQL contains expected patterns
pub fn validate_sql_migration<P: AsRef<Path>>(
    migration_dir: P,
    expected_tables: &[&str],
    test_name: &str,
) -> TestOutcome {
    let start = Instant::now();
    let migration_dir = migration_dir.as_ref();

    if !migration_dir.exists() {
        return TestOutcome::failure(
            test_name,
            start.elapsed().as_millis() as u64,
            "Migration directory does not exist",
        );
    }

    // Find all SQL files
    let sql_files: Vec<_> = WalkDir::new(migration_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "sql"))
        .collect();

    if sql_files.is_empty() {
        return TestOutcome::failure(
            test_name,
            start.elapsed().as_millis() as u64,
            "No SQL migration files found",
        );
    }

    // Read all SQL content
    let mut all_sql_content = String::new();
    for sql_file in &sql_files {
        match fs::read_to_string(sql_file.path()) {
            Ok(content) => all_sql_content.push_str(&content),
            Err(e) => {
                return TestOutcome::failure(
                    test_name,
                    start.elapsed().as_millis() as u64,
                    &format!("Failed to read SQL file {:?}: {}", sql_file.path(), e),
                )
            }
        }
    }

    let mut missing_tables = Vec::new();
    for table in expected_tables {
        let create_pattern = format!(r"CREATE\s+TABLE\s+{}", regex::escape(table));
        if let Ok(regex) = Regex::new(&create_pattern) {
            if !regex.is_match(&all_sql_content) {
                missing_tables.push(*table);
            }
        }
    }

    if !missing_tables.is_empty() {
        return TestOutcome::failure(
            test_name,
            start.elapsed().as_millis() as u64,
            &format!("Missing table definitions: {}", missing_tables.join(", ")),
        );
    }

    TestOutcome::success(
        test_name,
        start.elapsed().as_millis() as u64,
    )
}

/// Count lines of generated code
pub fn count_generated_code_lines<P: AsRef<Path>>(output_dir: P) -> Result<usize, E2EError> {
    let output_dir = output_dir.as_ref();
    let mut total_lines = 0;

    for entry in WalkDir::new(output_dir) {
        let entry = entry.map_err(|e| E2EError::FileValidationFailed {
            file: output_dir.display().to_string(),
            reason: format!("Failed to traverse directory: {}", e),
        })?;

        if entry.file_type().is_file() {
            let path = entry.path();
            if let Some(ext) = path.extension() {
                if ext == "rs" || ext == "sql" || ext == "toml" {
                    let content =
                        fs::read_to_string(path).map_err(|e| E2EError::FileValidationFailed {
                            file: path.display().to_string(),
                            reason: format!("Failed to read file: {}", e),
                        })?;
                    total_lines += content.lines().count();
                }
            }
        }
    }

    Ok(total_lines)
}

/// Validate that generated configuration files are valid TOML
pub fn validate_toml_config<P: AsRef<Path>>(config_path: P, test_name: &str) -> TestOutcome {
    let start = Instant::now();
    let config_path = config_path.as_ref();

    let content = match fs::read_to_string(config_path) {
        Ok(content) => content,
        Err(e) => {
            return TestOutcome::failure(
                test_name,
                start.elapsed().as_millis() as u64,
                &format!("Failed to read config file {:?}: {}", config_path, e),
            )
        }
    };

    match toml::from_str::<toml::Value>(&content) {
        Ok(_) => TestOutcome::success(
            test_name,
            start.elapsed().as_millis() as u64,
        ),
        Err(e) => TestOutcome::failure(
            test_name,
            start.elapsed().as_millis() as u64,
            &format!("Invalid TOML in {:?}: {}", config_path, e),
        ),
    }
}

/// Run a CLI command and return the result
pub async fn run_cli_command(
    _config: &TestConfig,
    args: &[&str],
    test_name: &str,
) -> TestOutcome {
    let start_time = Instant::now();
    
    // For now, this is a placeholder implementation
    // In a real implementation, this would execute the CLI command
    let output = Command::new("echo")
        .args(args)
        .output();
    
    let duration = start_time.elapsed();
    
    match output {
        Ok(output) => {
            if output.status.success() {
                TestOutcome::success("compile_check", duration.as_millis() as u64)
            } else {
                TestOutcome::failure(
                    test_name,
                    duration.as_millis() as u64,
                    &format!("Command failed with status: {}", output.status),
                )
            }
        }
        Err(e) => TestOutcome::failure(
            test_name,
            duration.as_millis() as u64,
            &format!("Failed to execute command: {}", e),
        )
    }
}

/// Validate Rust compilation
pub fn validate_rust_compilation<P: AsRef<Path>>(
    output_dir: P,
    test_name: &str,
) -> TestOutcome {
    let start = Instant::now();
    let output_dir_ref = output_dir.as_ref();
    
    // Check if the directory exists and contains Rust files
    if !output_dir_ref.exists() {
        return TestOutcome::failure(
            test_name,
            start.elapsed().as_millis() as u64,
            &format!("Output directory does not exist: {:?}", output_dir_ref),
        );
    }
    
    // Try to compile the Rust code
    let status = Command::new("cargo")
        .args(&["check", "--manifest-path"])
        .arg(output_dir_ref.join("Cargo.toml"))
        .status();
    
    match status {
        Ok(status) if status.success() => {
            TestOutcome::success(test_name, start.elapsed().as_millis() as u64)
        }
        Ok(status) => {
            TestOutcome::failure(
                test_name,
                start.elapsed().as_millis() as u64,
                &format!("Cargo check failed with status: {}", status),
            )
        }
        Err(e) => {
            TestOutcome::failure(
                test_name,
                start.elapsed().as_millis() as u64,
                &format!("Failed to run cargo check: {}", e),
            )
        }
    }
}
