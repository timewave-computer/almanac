//! Ethereum CLI functionality tests

use almanac_e2e_tests::{TestConfig, TestSuiteResults};
use almanac_e2e_tests::utils::run_cli_command;
use almanac_e2e_tests::utils::setup::TestEnvironment;
use almanac_e2e_tests::utils::validation::{validate_generated_files, validate_rust_code_patterns, validate_sql_migration};

pub async fn run_ethereum_cli_tests(config: &TestConfig, test_env: &TestEnvironment) -> TestSuiteResults {
    let mut results = TestSuiteResults::default();
    
    // Test ethereum subcommand help
    let result = run_cli_command(config, &["ethereum", "--help"], "ethereum_help").await;
    results.add_result(result);
    
    // Test ethereum generate-contract command with ERC20 ABI
    let output_dir = test_env.create_output_dir("ethereum_generate_contract_test").unwrap();
    let abi_path = test_env.fixtures_dir().join("abis/erc20.json");
    let output_dir_str = output_dir.to_string_lossy();
    let abi_path_str = abi_path.to_string_lossy();
    
    let result = run_cli_command(
        config,
        &[
            "ethereum",
            "generate-contract",
            &abi_path_str,
            "--address", "0xA0b86a33E6dc39C9c6D7C7CcF9C2e5C2c8C0b0",
            "--chain", "1",
            "--output-dir", &output_dir_str,
            "--features", "client,storage,api,migrations"
        ],
        "ethereum_generate_contract_basic"
    ).await;
    results.add_result(result);
    
    // Validate basic contract generation
    if output_dir.exists() {
        let validation_result = validate_generated_files(
            &output_dir,
            &["client", "storage", "api", "migrations"],
            "ethereum_generate_contract_validation"
        );
        results.add_result(validation_result);
        
        // Check for specific client code patterns
        let client_file = output_dir.join("client/mod.rs");
        if client_file.exists() {
            let patterns = &[
                "Erc20",
                "name",
                "symbol",
                "decimals",
                "totalSupply",
                "balanceOf",
                "transfer",
                "approve",
                "allowance"
            ];
            let pattern_result = validate_rust_code_patterns(
                &client_file,
                patterns,
                "ethereum_client_patterns"
            );
            results.add_result(pattern_result);
        }
    }
    
    // Test with USDC ABI (more complex contract)
    let usdc_output_dir = test_env.create_output_dir("ethereum_usdc_test").unwrap();
    let usdc_abi_path = test_env.fixtures_dir().join("abis/usdc.json");
    let usdc_output_str = usdc_output_dir.to_string_lossy();
    let usdc_abi_str = usdc_abi_path.to_string_lossy();
    
    let result = run_cli_command(
        config,
        &[
            "ethereum",
            "generate-contract",
            &usdc_abi_str,
            "--address", "0xA0b86a33E6411183EB78B0B35a6F4bB0B3B8b100",
            "--chain", "1",
            "--output-dir", &usdc_output_str,
            "--namespace", "usdc_token",
            "--features", "client,storage,api"
        ],
        "ethereum_generate_contract_usdc"
    ).await;
    results.add_result(result);
    
    // Validate USDC generation
    if usdc_output_dir.exists() {
        let validation_result = validate_generated_files(
            &usdc_output_dir,
            &["client", "storage", "api"],
            "ethereum_usdc_validation"
        );
        results.add_result(validation_result);
        
        // Check for USDC-specific patterns
        let client_file = usdc_output_dir.join("client/mod.rs");
        if client_file.exists() {
            let patterns = &[
                "mint",
                "burn",
                "blacklist",
                "unBlacklist",
                "isBlacklisted",
                "owner"
            ];
            let pattern_result = validate_rust_code_patterns(
                &client_file,
                patterns,
                "ethereum_usdc_patterns"
            );
            results.add_result(pattern_result);
        }
    }
    
    // Test dry-run functionality
    let dry_run_output_dir = test_env.create_output_dir("ethereum_dry_run_test").unwrap();
    let dry_run_output_str = dry_run_output_dir.to_string_lossy();
    
    let result = run_cli_command(
        config,
        &[
            "ethereum",
            "generate-contract",
            &abi_path_str,
            "--address", "0x1234567890123456789012345678901234567890",
            "--chain", "1",
            "--output-dir", &dry_run_output_str,
            "--dry-run",
            "--verbose"
        ],
        "ethereum_generate_contract_dry_run"
    ).await;
    results.add_result(result);
    
    // Test with different chain ID (Polygon)
    let polygon_output_dir = test_env.create_output_dir("ethereum_polygon_test").unwrap();
    let polygon_output_str = polygon_output_dir.to_string_lossy();
    
    let result = run_cli_command(
        config,
        &[
            "ethereum",
            "generate-contract",
            &abi_path_str,
            "--address", "0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174",
            "--chain", "137",
            "--output-dir", &polygon_output_str,
            "--features", "client,api"
        ],
        "ethereum_generate_contract_polygon"
    ).await;
    results.add_result(result);
    
    // Test with only client feature
    let client_only_output_dir = test_env.create_output_dir("ethereum_client_only_test").unwrap();
    let client_only_output_str = client_only_output_dir.to_string_lossy();
    
    let result = run_cli_command(
        config,
        &[
            "ethereum",
            "generate-contract",
            &abi_path_str,
            "--address", "0x1234567890123456789012345678901234567890",
            "--chain", "1",
            "--output-dir", &client_only_output_str,
            "--features", "client"
        ],
        "ethereum_generate_contract_client_only"
    ).await;
    results.add_result(result);
    
    // Validate client-only generation
    if client_only_output_dir.exists() {
        let validation_result = validate_generated_files(
            &client_only_output_dir,
            &["client"],
            "ethereum_client_only_validation"
        );
        results.add_result(validation_result);
    }
    
    // Test with only storage feature
    let storage_only_output_dir = test_env.create_output_dir("ethereum_storage_only_test").unwrap();
    let storage_only_output_str = storage_only_output_dir.to_string_lossy();
    
    let result = run_cli_command(
        config,
        &[
            "ethereum",
            "generate-contract",
            &abi_path_str,
            "--address", "0x1234567890123456789012345678901234567890",
            "--chain", "1",
            "--output-dir", &storage_only_output_str,
            "--features", "storage,migrations"
        ],
        "ethereum_generate_contract_storage_only"
    ).await;
    results.add_result(result);
    
    // Validate storage-only generation
    if storage_only_output_dir.exists() {
        let validation_result = validate_generated_files(
            &storage_only_output_dir,
            &["storage", "migrations"],
            "ethereum_storage_only_validation"
        );
        results.add_result(validation_result);
        
        // Check for SQL migrations
        let migrations_dir = storage_only_output_dir.join("migrations");
        if migrations_dir.exists() {
            let sql_result = validate_sql_migration(
                &migrations_dir,
                &["erc20"],
                "ethereum_storage_sql"
            );
            results.add_result(sql_result);
        }
    }
    
    // Test with custom namespace
    let namespace_output_dir = test_env.create_output_dir("ethereum_namespace_test").unwrap();
    let namespace_output_str = namespace_output_dir.to_string_lossy();
    
    let result = run_cli_command(
        config,
        &[
            "ethereum",
            "generate-contract",
            &abi_path_str,
            "--address", "0x1234567890123456789012345678901234567890",
            "--chain", "1",
            "--output-dir", &namespace_output_str,
            "--namespace", "my_custom_token",
            "--features", "client,storage"
        ],
        "ethereum_generate_contract_namespace"
    ).await;
    results.add_result(result);
    
    // Test with verbose output
    let verbose_output_dir = test_env.create_output_dir("ethereum_verbose_test").unwrap();
    let verbose_output_str = verbose_output_dir.to_string_lossy();
    
    let result = run_cli_command(
        config,
        &[
            "ethereum",
            "generate-contract",
            &abi_path_str,
            "--address", "0x1234567890123456789012345678901234567890",
            "--chain", "1",
            "--output-dir", &verbose_output_str,
            "--features", "client,storage,api",
            "--verbose"
        ],
        "ethereum_generate_contract_verbose"
    ).await;
    results.add_result(result);
    
    // Test invalid ABI file (should fail gracefully)
    let result = run_cli_command(
        config,
        &[
            "ethereum",
            "generate-contract",
            "nonexistent_abi.json",
            "--address", "0x1234567890123456789012345678901234567890",
            "--chain", "1"
        ],
        "ethereum_generate_contract_invalid_abi"
    ).await;
    // This should fail, so we expect a non-zero exit code
    let mut expected_failure = result;
    expected_failure.success = !expected_failure.success; // Flip the success expectation
    results.add_result(expected_failure);
    
    // Test invalid contract address format (should fail gracefully)
    let result = run_cli_command(
        config,
        &[
            "ethereum",
            "generate-contract",
            &abi_path_str,
            "--address", "invalid_address",
            "--chain", "1"
        ],
        "ethereum_generate_contract_invalid_address"
    ).await;
    // This should fail, so we expect a non-zero exit code
    let mut expected_failure = result;
    expected_failure.success = !expected_failure.success; // Flip the success expectation
    results.add_result(expected_failure);
    
    // Test invalid chain ID (should fail gracefully)
    let result = run_cli_command(
        config,
        &[
            "ethereum",
            "generate-contract",
            &abi_path_str,
            "--address", "0x1234567890123456789012345678901234567890",
            "--chain", "invalid_chain"
        ],
        "ethereum_generate_contract_invalid_chain"
    ).await;
    // This should fail, so we expect a non-zero exit code
    let mut expected_failure = result;
    expected_failure.success = !expected_failure.success; // Flip the success expectation
    results.add_result(expected_failure);
    
    // Test missing required arguments (should fail gracefully)
    let result = run_cli_command(
        config,
        &[
            "ethereum",
            "generate-contract",
            &abi_path_str
            // Missing --address and --chain
        ],
        "ethereum_generate_contract_missing_args"
    ).await;
    // This should fail, so we expect a non-zero exit code
    let mut expected_failure = result;
    expected_failure.success = !expected_failure.success; // Flip the success expectation
    results.add_result(expected_failure);
    
    results
} 