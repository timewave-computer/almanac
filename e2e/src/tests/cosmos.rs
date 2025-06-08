//! Cosmos CLI functionality tests

use almanac_e2e_tests::{TestConfig, TestSuiteResults};
use almanac_e2e_tests::utils::run_cli_command;
use almanac_e2e_tests::utils::setup::TestEnvironment;
use almanac_e2e_tests::utils::validation::{validate_generated_files, validate_rust_code_patterns, validate_sql_migration};

pub async fn run_cosmos_cli_tests(config: &TestConfig, test_env: &TestEnvironment) -> TestSuiteResults {
    let mut results = TestSuiteResults::default();
    
    // Test cosmos subcommand help
    let result = run_cli_command(config, &["cosmos", "--help"], "cosmos_help").await;
    results.add_result(result);
    
    // Test cosmos generate-contract command with Valence Base Account schema
    let output_dir = test_env.create_output_dir("cosmos_generate_contract_test").unwrap();
    let schema_path = test_env.fixtures_dir().join("schemas/valence_base_account.json");
    let output_dir_str = output_dir.to_string_lossy();
    let schema_path_str = schema_path.to_string_lossy();
    
    let result = run_cli_command(
        config,
        &[
            "cosmos",
            "generate-contract",
            &schema_path_str,
            "--address", "cosmos1valencebaseaccountexample123456789",
            "--chain", "cosmoshub-4",
            "--output-dir", &output_dir_str,
            "--features", "client,storage,api,migrations"
        ],
        "cosmos_generate_contract_basic"
    ).await;
    results.add_result(result);
    
    // Validate basic contract generation
    if output_dir.exists() {
        let validation_result = validate_generated_files(
            &output_dir,
            &["client", "storage", "api", "migrations"],
            "cosmos_generate_contract_validation"
        );
        results.add_result(validation_result);
        
        // Check for specific client code patterns
        let client_file = output_dir.join("client/mod.rs");
        if client_file.exists() {
            let patterns = &[
                "ValenceBaseAccount",
                "approve_library",
                "remove_library",
                "list_approved_libraries",
                "ownership"
            ];
            let pattern_result = validate_rust_code_patterns(
                &client_file,
                patterns,
                "cosmos_client_patterns"
            );
            results.add_result(pattern_result);
        }
    }
    
    // Test dry-run functionality
    let dry_run_output_dir = test_env.create_output_dir("cosmos_dry_run_test").unwrap();
    let dry_run_output_str = dry_run_output_dir.to_string_lossy();
    
    let result = run_cli_command(
        config,
        &[
            "cosmos",
            "generate-contract",
            &schema_path_str,
            "--address", "cosmos1test123456789",
            "--chain", "cosmoshub-4",
            "--output-dir", &dry_run_output_str,
            "--dry-run",
            "--verbose"
        ],
        "cosmos_generate_contract_dry_run"
    ).await;
    results.add_result(result);
    
    // Test with custom namespace
    let namespace_output_dir = test_env.create_output_dir("cosmos_namespace_test").unwrap();
    let namespace_output_str = namespace_output_dir.to_string_lossy();
    
    let result = run_cli_command(
        config,
        &[
            "cosmos",
            "generate-contract",
            &schema_path_str,
            "--address", "cosmos1test123456789",
            "--chain", "cosmoshub-4",
            "--output-dir", &namespace_output_str,
            "--namespace", "my_custom_contract",
            "--features", "client,storage"
        ],
        "cosmos_generate_contract_namespace"
    ).await;
    results.add_result(result);
    
    // Test with different chain ID
    let chain_output_dir = test_env.create_output_dir("cosmos_chain_test").unwrap();
    let chain_output_str = chain_output_dir.to_string_lossy();
    
    let result = run_cli_command(
        config,
        &[
            "cosmos",
            "generate-contract",
            &schema_path_str,
            "--address", "juno1test123456789",
            "--chain", "juno-1",
            "--output-dir", &chain_output_str,
            "--features", "client,api"
        ],
        "cosmos_generate_contract_juno"
    ).await;
    results.add_result(result);
    
    // Test with only client feature
    let client_only_output_dir = test_env.create_output_dir("cosmos_client_only_test").unwrap();
    let client_only_output_str = client_only_output_dir.to_string_lossy();
    
    let result = run_cli_command(
        config,
        &[
            "cosmos",
            "generate-contract",
            &schema_path_str,
            "--address", "cosmos1clientonly123456789",
            "--chain", "cosmoshub-4",
            "--output-dir", &client_only_output_str,
            "--features", "client"
        ],
        "cosmos_generate_contract_client_only"
    ).await;
    results.add_result(result);
    
    // Validate client-only generation
    if client_only_output_dir.exists() {
        let validation_result = validate_generated_files(
            &client_only_output_dir,
            &["client"],
            "cosmos_client_only_validation"
        );
        results.add_result(validation_result);
    }
    
    // Test with only storage feature
    let storage_only_output_dir = test_env.create_output_dir("cosmos_storage_only_test").unwrap();
    let storage_only_output_str = storage_only_output_dir.to_string_lossy();
    
    let result = run_cli_command(
        config,
        &[
            "cosmos",
            "generate-contract",
            &schema_path_str,
            "--address", "cosmos1storageonly123456789",
            "--chain", "cosmoshub-4",
            "--output-dir", &storage_only_output_str,
            "--features", "storage,migrations"
        ],
        "cosmos_generate_contract_storage_only"
    ).await;
    results.add_result(result);
    
    // Validate storage-only generation
    if storage_only_output_dir.exists() {
        let validation_result = validate_generated_files(
            &storage_only_output_dir,
            &["storage", "migrations"],
            "cosmos_storage_only_validation"
        );
        results.add_result(validation_result);
        
        // Check for SQL migrations
        let migrations_dir = storage_only_output_dir.join("migrations");
        if migrations_dir.exists() {
            let sql_result = validate_sql_migration(
                &migrations_dir,
                &["valence_base_account"],
                "cosmos_storage_sql"
            );
            results.add_result(sql_result);
        }
    }
    
    // Test with verbose output
    let verbose_output_dir = test_env.create_output_dir("cosmos_verbose_test").unwrap();
    let verbose_output_str = verbose_output_dir.to_string_lossy();
    
    let result = run_cli_command(
        config,
        &[
            "cosmos",
            "generate-contract",
            &schema_path_str,
            "--address", "cosmos1verbose123456789",
            "--chain", "cosmoshub-4",
            "--output-dir", &verbose_output_str,
            "--features", "client,storage,api",
            "--verbose"
        ],
        "cosmos_generate_contract_verbose"
    ).await;
    results.add_result(result);
    
    // Test invalid schema file (should fail gracefully)
    let result = run_cli_command(
        config,
        &[
            "cosmos",
            "generate-contract",
            "nonexistent_schema.json",
            "--address", "cosmos1test123456789",
            "--chain", "cosmoshub-4"
        ],
        "cosmos_generate_contract_invalid_schema"
    ).await;
    // This should fail, so we expect a non-zero exit code
    let mut expected_failure = result;
    expected_failure.success = !expected_failure.success; // Flip the success expectation
    results.add_result(expected_failure);
    
    // Test invalid contract address format (should fail gracefully)
    let result = run_cli_command(
        config,
        &[
            "cosmos",
            "generate-contract",
            &schema_path_str,
            "--address", "invalid_address",
            "--chain", "cosmoshub-4"
        ],
        "cosmos_generate_contract_invalid_address"
    ).await;
    // This should fail, so we expect a non-zero exit code
    let mut expected_failure = result;
    expected_failure.success = !expected_failure.success; // Flip the success expectation
    results.add_result(expected_failure);
    
    // Test empty chain ID (should fail gracefully)
    let result = run_cli_command(
        config,
        &[
            "cosmos",
            "generate-contract",
            &schema_path_str,
            "--address", "cosmos1test123456789",
            "--chain", ""
        ],
        "cosmos_generate_contract_empty_chain"
    ).await;
    // This should fail, so we expect a non-zero exit code
    let mut expected_failure = result;
    expected_failure.success = !expected_failure.success; // Flip the success expectation
    results.add_result(expected_failure);
    
    results
} 