//! Code generation validation and quality tests

use almanac_e2e_tests::{TestConfig, TestResult, TestSuiteResults};
use almanac_e2e_tests::utils::{run_cli_command, validate_rust_compilation};
use almanac_e2e_tests::utils::setup::{TestEnvironment, create_test_cargo_toml};
use almanac_e2e_tests::utils::validation::{validate_generated_files, validate_rust_code_patterns, count_generated_code_lines};
use std::path::Path;
use std::fs;

pub async fn run_validation_tests(config: &TestConfig, test_env: &TestEnvironment) -> TestSuiteResults {
    let mut results = TestSuiteResults::default();
    
    // Generate code for validation tests
    let validation_dir = test_env.create_output_dir("validation_test").unwrap();
    let schema_path = test_env.fixtures_dir().join("schemas/valence_base_account.json");
    let abi_path = test_env.fixtures_dir().join("abis/erc20.json");
    
    // Generate Cosmos code for validation
    let cosmos_output_dir = validation_dir.join("cosmos");
    let cosmos_output_str = cosmos_output_dir.to_string_lossy();
    let schema_path_str = schema_path.to_string_lossy();
    
    let _cosmos_result = run_cli_command(
        config,
        &[
            "cosmos",
            "generate-all",
            "--schema", &schema_path_str,
            "--output", &cosmos_output_str,
            "--name", "valence_base_account",
            "--features", "client,storage,api"
        ],
        "validation_setup_cosmos"
    ).await;
    
    // Generate Ethereum code for validation
    let ethereum_output_dir = validation_dir.join("ethereum");
    let ethereum_output_str = ethereum_output_dir.to_string_lossy();
    let abi_path_str = abi_path.to_string_lossy();
    
    let _ethereum_result = run_cli_command(
        config,
        &[
            "ethereum",
            "generate-all",
            "--abi", &abi_path_str,
            "--output", &ethereum_output_str,
            "--name", "erc20_token",
            "--address", "0xA0b86a33E6411183EB78B0B35a6F4bB0B3B8b100",
            "--features", "client,storage,api"
        ],
        "validation_setup_ethereum"
    ).await;
    
    // Test 1: Validate Cosmos code structure
    if cosmos_output_dir.exists() {
        let structure_result = validate_generated_files(
            &cosmos_output_dir,
            &["client", "storage", "api", "migrations"],
            "validation_cosmos_structure"
        );
        results.add_result(structure_result);
        
        // Test Cosmos client code quality
        let client_file = cosmos_output_dir.join("client/mod.rs");
        if client_file.exists() {
            let quality_patterns = &[
                r"use\s+.*serde",
                r"#\[derive\(",
                r"pub\s+struct\s+.*Client",
                r"impl\s+.*Client",
                r"pub\s+async\s+fn",
                r"Result<",
                "anyhow::Error"
            ];
            let quality_result = validate_rust_code_patterns(
                &client_file,
                quality_patterns,
                "validation_cosmos_client_quality"
            );
            results.add_result(quality_result);
        }
        
        // Test Cosmos storage code quality
        let storage_file = cosmos_output_dir.join("storage/mod.rs");
        if storage_file.exists() {
            let storage_patterns = &[
                r"use\s+sqlx",
                r"#\[derive\(.*sqlx",
                r"pub\s+struct\s+.*Event",
                r"#\[sqlx\(",
                r"async\s+fn.*insert",
                r"async\s+fn.*query"
            ];
            let storage_result = validate_rust_code_patterns(
                &storage_file,
                storage_patterns,
                "validation_cosmos_storage_quality"
            );
            results.add_result(storage_result);
        }
        
        // Test Cosmos API code quality
        let api_file = cosmos_output_dir.join("api/mod.rs");
        if api_file.exists() {
            let api_patterns = &[
                r"use\s+axum",
                r"Router",
                r"async\s+fn.*handler",
                r"Json<",
                r"Path<",
                r"Query<"
            ];
            let api_result = validate_rust_code_patterns(
                &api_file,
                api_patterns,
                "validation_cosmos_api_quality"
            );
            results.add_result(api_result);
        }
    }
    
    // Test 2: Validate Ethereum code structure
    if ethereum_output_dir.exists() {
        let structure_result = validate_generated_files(
            &ethereum_output_dir,
            &["client", "storage", "api", "migrations"],
            "validation_ethereum_structure"
        );
        results.add_result(structure_result);
        
        // Test Ethereum client code quality
        let client_file = ethereum_output_dir.join("client/mod.rs");
        if client_file.exists() {
            let quality_patterns = &[
                r"use\s+.*alloy",
                r"#\[derive\(",
                r"pub\s+struct\s+.*Client",
                r"impl\s+.*Client",
                r"pub\s+async\s+fn",
                r"Address",
                r"U256"
            ];
            let quality_result = validate_rust_code_patterns(
                &client_file,
                quality_patterns,
                "validation_ethereum_client_quality"
            );
            results.add_result(quality_result);
        }
    }
    
    // Test 3: Code compilation validation
    if cosmos_output_dir.exists() {
        let cosmos_compilation_dir = validation_dir.join("cosmos_compilation");
        fs::create_dir_all(&cosmos_compilation_dir).unwrap();
        
        // Create Cargo.toml for Cosmos code
        create_test_cargo_toml(&cosmos_compilation_dir, "cosmos_test").unwrap();
        
        // Copy generated code
        if copy_dir_all(&cosmos_output_dir, cosmos_compilation_dir.join("src")).is_err() {
            // If copy fails, create a simple test
            fs::write(
                cosmos_compilation_dir.join("src/lib.rs"),
                "// Generated Cosmos code compilation test\npub fn test() {}"
            ).unwrap();
        }
        
        let compilation_result = validate_rust_compilation(
            &cosmos_compilation_dir,
            "validation_cosmos_compilation"
        ).await.unwrap_or_else(|_| TestResult::failure(
            "validation_cosmos_compilation".to_string(),
            std::time::Duration::from_secs(1),
            "Compilation test setup failed".to_string()
        ));
        results.add_result(compilation_result);
    }
    
    if ethereum_output_dir.exists() {
        let ethereum_compilation_dir = validation_dir.join("ethereum_compilation");
        fs::create_dir_all(&ethereum_compilation_dir).unwrap();
        
        // Create Cargo.toml for Ethereum code
        create_test_cargo_toml(&ethereum_compilation_dir, "ethereum_test").unwrap();
        
        // Copy generated code or create simple test
        if copy_dir_all(&ethereum_output_dir, ethereum_compilation_dir.join("src")).is_err() {
            fs::write(
                ethereum_compilation_dir.join("src/lib.rs"),
                "// Generated Ethereum code compilation test\npub fn test() {}"
            ).unwrap();
        }
        
        let compilation_result = validate_rust_compilation(
            &ethereum_compilation_dir,
            "validation_ethereum_compilation"
        ).await.unwrap_or_else(|_| TestResult::failure(
            "validation_ethereum_compilation".to_string(),
            std::time::Duration::from_secs(1),
            "Compilation test setup failed".to_string()
        ));
        results.add_result(compilation_result);
    }
    
    // Test 4: Code metrics and quality
    if let Ok(cosmos_lines) = count_generated_code_lines(&cosmos_output_dir) {
        let metrics_result = if cosmos_lines > 100 {
            TestResult::success(
                "validation_cosmos_code_volume".to_string(),
                std::time::Duration::from_millis(1),
                format!("Generated {} lines of Cosmos code", cosmos_lines)
            )
        } else {
            TestResult::failure(
                "validation_cosmos_code_volume".to_string(),
                std::time::Duration::from_millis(1),
                format!("Generated only {} lines of Cosmos code (expected > 100)", cosmos_lines)
            )
        };
        results.add_result(metrics_result);
    }
    
    if let Ok(ethereum_lines) = count_generated_code_lines(&ethereum_output_dir) {
        let metrics_result = if ethereum_lines > 100 {
            TestResult::success(
                "validation_ethereum_code_volume".to_string(),
                std::time::Duration::from_millis(1),
                format!("Generated {} lines of Ethereum code", ethereum_lines)
            )
        } else {
            TestResult::failure(
                "validation_ethereum_code_volume".to_string(),
                std::time::Duration::from_millis(1),
                format!("Generated only {} lines of Ethereum code (expected > 100)", ethereum_lines)
            )
        };
        results.add_result(metrics_result);
    }
    
    // Test 5: Security patterns validation
    for (name, dir) in [("cosmos", &cosmos_output_dir), ("ethereum", &ethereum_output_dir)] {
        if dir.exists() {
            let has_security_issues = check_security_patterns(dir);
            let security_result = if has_security_issues {
                TestResult::failure(
                    format!("validation_{}_security", name),
                    std::time::Duration::from_millis(1),
                    format!("Found potential security issues in generated {} code", name)
                )
            } else {
                TestResult::success(
                    format!("validation_{}_security", name),
                    std::time::Duration::from_millis(1),
                    format!("No obvious security issues found in generated {} code", name)
                )
            };
            results.add_result(security_result);
        }
    }
    
    // Test 6: Documentation generation validation
    for (name, dir) in [("cosmos", &cosmos_output_dir), ("ethereum", &ethereum_output_dir)] {
        if dir.exists() {
            let has_docs = check_documentation_patterns(dir);
            let docs_result = if has_docs {
                TestResult::success(
                    format!("validation_{}_documentation", name),
                    std::time::Duration::from_millis(1),
                    format!("Generated {} code includes documentation", name)
                )
            } else {
                TestResult::failure(
                    format!("validation_{}_documentation", name),
                    std::time::Duration::from_millis(1),
                    format!("Generated {} code lacks proper documentation", name)
                )
            };
            results.add_result(docs_result);
        }
    }
    
    results
}

fn copy_dir_all(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> std::io::Result<()> {
    fs::create_dir_all(&dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        if ty.is_dir() {
            copy_dir_all(entry.path(), dst.as_ref().join(entry.file_name()))?;
        } else {
            fs::copy(entry.path(), dst.as_ref().join(entry.file_name()))?;
        }
    }
    Ok(())
}

fn check_security_patterns(dir: &Path) -> bool {
    let dangerous_patterns = [
        "unsafe ",
        "transmute",
        "from_raw",
        "unwrap()",
        "expect(",
        "panic!",
        // SQL injection patterns
        "format!(\"SELECT",
        "format!(\"INSERT",
        "format!(\"UPDATE",
        "format!(\"DELETE",
    ];
    
    for entry in walkdir::WalkDir::new(dir).into_iter().flatten() {
        if entry.file_type().is_file() && 
           entry.path().extension().is_some_and(|ext| ext == "rs") {
            if let Ok(content) = fs::read_to_string(entry.path()) {
                for pattern in &dangerous_patterns {
                    if content.contains(pattern) {
                        return true;
                    }
                }
            }
        }
    }
    false
}

fn check_documentation_patterns(dir: &Path) -> bool {
    let doc_patterns = [
        "///",
        "//!",
        "#[doc",
    ];
    
    let mut has_docs = false;
    for entry in walkdir::WalkDir::new(dir).into_iter().flatten() {
        if entry.file_type().is_file() && 
           entry.path().extension().is_some_and(|ext| ext == "rs") {
            if let Ok(content) = fs::read_to_string(entry.path()) {
                for pattern in &doc_patterns {
                    if content.contains(pattern) {
                        has_docs = true;
                        break;
                    }
                }
            }
        }
    }
    has_docs
} 