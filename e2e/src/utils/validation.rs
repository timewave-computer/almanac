//! Validation utilities for generated code and output files

use crate::{E2EError, TestResult};
use std::path::Path;
use std::fs;
use walkdir::WalkDir;
use regex::Regex;
use std::time::Instant;

/// Validate that all expected files were generated
pub fn validate_generated_files<P: AsRef<Path>>(
    output_dir: P,
    features: &[&str],
    test_name: &str,
) -> TestResult {
    let start = Instant::now();
    let output_dir = output_dir.as_ref();
    
    if !output_dir.exists() {
        return TestResult::failure(
            test_name.to_string(),
            start.elapsed(),
            "Output directory does not exist".to_string()
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
            error_msg.push_str(&format!("Missing files/directories: {}\n", missing_files.join(", ")));
        }
        if !validation_errors.is_empty() {
            error_msg.push_str(&format!("Validation errors: {}", validation_errors.join(", ")));
        }
        return TestResult::failure(test_name.to_string(), start.elapsed(), error_msg);
    }
    
    TestResult::success(
        test_name.to_string(),
        start.elapsed(),
        format!("All expected files for features {} were generated", features.join(", "))
    )
}

/// Validate that generated Rust code contains expected patterns
pub fn validate_rust_code_patterns<P: AsRef<Path>>(
    file_path: P,
    expected_patterns: &[&str],
    test_name: &str,
) -> TestResult {
    let start = Instant::now();
    let file_path = file_path.as_ref();
    
    let content = match fs::read_to_string(file_path) {
        Ok(content) => content,
        Err(e) => return TestResult::failure(
            test_name.to_string(),
            start.elapsed(),
            format!("Failed to read file {:?}: {}", file_path, e)
        )
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
        return TestResult::failure(
            test_name.to_string(),
            start.elapsed(),
            format!("Missing patterns in {:?}: {}", file_path, missing_patterns.join(", "))
        );
    }
    
    TestResult::success(
        test_name.to_string(),
        start.elapsed(),
        format!("All expected patterns found in {:?}", file_path)
    )
}

/// Validate that generated SQL contains expected patterns
pub fn validate_sql_migration<P: AsRef<Path>>(
    migration_dir: P,
    expected_tables: &[&str],
    test_name: &str,
) -> TestResult {
    let start = Instant::now();
    let migration_dir = migration_dir.as_ref();
    
    if !migration_dir.exists() {
        return TestResult::failure(
            test_name.to_string(),
            start.elapsed(),
            "Migration directory does not exist".to_string()
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
        return TestResult::failure(
            test_name.to_string(),
            start.elapsed(),
            "No SQL migration files found".to_string()
        );
    }
    
    // Read all SQL content
    let mut all_sql_content = String::new();
    for sql_file in &sql_files {
        match fs::read_to_string(sql_file.path()) {
            Ok(content) => all_sql_content.push_str(&content),
            Err(e) => return TestResult::failure(
                test_name.to_string(),
                start.elapsed(),
                format!("Failed to read SQL file {:?}: {}", sql_file.path(), e)
            )
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
        return TestResult::failure(
            test_name.to_string(),
            start.elapsed(),
            format!("Missing table definitions: {}", missing_tables.join(", "))
        );
    }
    
    TestResult::success(
        test_name.to_string(),
        start.elapsed(),
        format!("All expected tables found in SQL migrations: {}", expected_tables.join(", "))
    )
}

/// Count lines of generated code
pub fn count_generated_code_lines<P: AsRef<Path>>(output_dir: P) -> Result<usize, E2EError> {
    let output_dir = output_dir.as_ref();
    let mut total_lines = 0;
    
    for entry in WalkDir::new(output_dir) {
        let entry = entry.map_err(|e| E2EError::FileValidationFailed {
            file: output_dir.display().to_string(),
            reason: format!("Failed to traverse directory: {}", e)
        })?;
        
        if entry.file_type().is_file() {
            let path = entry.path();
            if let Some(ext) = path.extension() {
                if ext == "rs" || ext == "sql" || ext == "toml" {
                    let content = fs::read_to_string(path)
                        .map_err(|e| E2EError::FileValidationFailed {
                            file: path.display().to_string(),
                            reason: format!("Failed to read file: {}", e)
                        })?;
                    total_lines += content.lines().count();
                }
            }
        }
    }
    
    Ok(total_lines)
}

/// Validate that generated configuration files are valid TOML
pub fn validate_toml_config<P: AsRef<Path>>(config_path: P, test_name: &str) -> TestResult {
    let start = Instant::now();
    let config_path = config_path.as_ref();
    
    let content = match fs::read_to_string(config_path) {
        Ok(content) => content,
        Err(e) => return TestResult::failure(
            test_name.to_string(),
            start.elapsed(),
            format!("Failed to read config file {:?}: {}", config_path, e)
        )
    };
    
    match toml::from_str::<toml::Value>(&content) {
        Ok(_) => TestResult::success(
            test_name.to_string(),
            start.elapsed(),
            format!("Valid TOML configuration: {:?}", config_path)
        ),
        Err(e) => TestResult::failure(
            test_name.to_string(),
            start.elapsed(),
            format!("Invalid TOML in {:?}: {}", config_path, e)
        )
    }
} 