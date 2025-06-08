//! Comprehensive CLI tests that cover all documented commands

use almanac_e2e_tests::{TestConfig, TestSuiteResults};
use almanac_e2e_tests::utils::run_cli_command;
use almanac_e2e_tests::utils::setup::TestEnvironment;

pub async fn run_comprehensive_cli_tests(config: &TestConfig, test_env: &TestEnvironment) -> TestSuiteResults {
    let mut results = TestSuiteResults::default();

    // Test main almanac help
    let result = run_cli_command(config, &["--help"], "almanac_help").await;
    results.add_result(result);

    // Test version command
    let result = run_cli_command(config, &["--version"], "almanac_version").await;
    results.add_result(result);

    // === Cosmos Documentation Examples ===
    
    // Example from cosmos_codegen.md: Basic command
    let _cosmos_basic_output = test_env.create_output_dir("cosmos_doc_basic").unwrap();
    let schema_path = test_env.fixtures_dir().join("schemas/valence_base_account.json");
    
    let result = run_cli_command(
        config,
        &[
            "cosmos",
            "generate-contract",
            &schema_path.to_string_lossy(),
            "--address", "cosmos1abc123456789",
            "--chain", "cosmoshub-4"
        ],
        "cosmos_doc_basic_example"
    ).await;
    results.add_result(result);

    // Example from README.md: Valence Base Account generation
    let cosmos_valence_output = test_env.create_output_dir("cosmos_doc_valence").unwrap();
    
    let result = run_cli_command(
        config,
        &[
            "cosmos",
            "generate-contract",
            &schema_path.to_string_lossy(),
            "--address", "cosmos1valencebaseaccountexample...",
            "--chain", "cosmoshub-4",
            "--features", "client,storage,api",
            "--output-dir", &cosmos_valence_output.to_string_lossy()
        ],
        "cosmos_doc_valence_example"
    ).await;
    results.add_result(result);

    // Example from cosmos_cli_reference.md: Dry run
    let result = run_cli_command(
        config,
        &[
            "cosmos",
            "generate-contract",
            &schema_path.to_string_lossy(),
            "--address", "cosmos1contract...",
            "--chain", "juno-1",
            "--dry-run",
            "--verbose"
        ],
        "cosmos_doc_dry_run_example"
    ).await;
    results.add_result(result);

    // Example from cosmos_cli_reference.md: Custom namespace
    let cosmos_namespace_output = test_env.create_output_dir("cosmos_doc_namespace").unwrap();
    
    let result = run_cli_command(
        config,
        &[
            "cosmos",
            "generate-contract",
            &schema_path.to_string_lossy(),
            "--address", "cosmos1nft...",
            "--chain", "stargaze-1",
            "--namespace", "my_nft_collection",
            "--features", "client,storage,api,migrations",
            "--output-dir", &cosmos_namespace_output.to_string_lossy()
        ],
        "cosmos_doc_namespace_example"
    ).await;
    results.add_result(result);

    // === Ethereum Documentation Examples ===
    
    // Example from README.md: USDC generation
    let eth_usdc_output = test_env.create_output_dir("eth_doc_usdc").unwrap();
    let usdc_abi_path = test_env.fixtures_dir().join("abis/usdc.json");
    
    let result = run_cli_command(
        config,
        &[
            "ethereum",
            "generate-contract",
            &usdc_abi_path.to_string_lossy(),
            "--address", "0xA0b86a33E6dc39C9c6D7C7CcF9C2e5C2c8C0b0",
            "--chain", "1",
            "--features", "client,storage,api",
            "--output-dir", &eth_usdc_output.to_string_lossy()
        ],
        "eth_doc_usdc_example"
    ).await;
    results.add_result(result);

    // Example from ethereum_cli_reference.md: Basic generation
    let eth_basic_output = test_env.create_output_dir("eth_doc_basic").unwrap();
    let erc20_abi_path = test_env.fixtures_dir().join("abis/erc20.json");
    
    let result = run_cli_command(
        config,
        &[
            "ethereum",
            "generate-contract",
            &erc20_abi_path.to_string_lossy(),
            "--address", "0xA0b86a33E6dc39C9c6D7C7CcF9C2e5C2c8C0b0",
            "--chain", "1",
            "--output-dir", &eth_basic_output.to_string_lossy()
        ],
        "eth_doc_basic_example"
    ).await;
    results.add_result(result);

    // Example from ethereum_cli_reference.md: Full feature generation
    let eth_full_output = test_env.create_output_dir("eth_doc_full").unwrap();
    
    let result = run_cli_command(
        config,
        &[
            "ethereum",
            "generate-contract",
            &erc20_abi_path.to_string_lossy(),
            "--address", "0x8ad599c3A0ff1De082011EFDDc58f1908eb6e6D8",
            "--chain", "1",
            "--features", "client,storage,api,migrations",
            "--output-dir", &eth_full_output.to_string_lossy(),
            "--namespace", "uniswap_pool"
        ],
        "eth_doc_full_example"
    ).await;
    results.add_result(result);

    // Example from ethereum_cli_reference.md: Dry run with JSON format
    let result = run_cli_command(
        config,
        &[
            "ethereum",
            "generate-contract",
            &erc20_abi_path.to_string_lossy(),
            "--address", "0x123...",
            "--chain", "5",
            "--dry-run"
        ],
        "eth_doc_dry_run_example"
    ).await;
    results.add_result(result);

    // Example from ethereum_cli_reference.md: Polygon network
    let eth_polygon_output = test_env.create_output_dir("eth_doc_polygon").unwrap();
    
    let result = run_cli_command(
        config,
        &[
            "ethereum",
            "generate-contract",
            &erc20_abi_path.to_string_lossy(),
            "--address", "0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174",
            "--chain", "137",
            "--output-dir", &eth_polygon_output.to_string_lossy(),
            "--features", "client,api"
        ],
        "eth_doc_polygon_example"
    ).await;
    results.add_result(result);

    // === Feature-specific tests ===
    
    // Test all individual features for cosmos
    for feature in &["client", "storage", "api", "migrations"] {
        let feature_output = test_env.create_output_dir(&format!("cosmos_feature_{}", feature)).unwrap();
        
        let result = run_cli_command(
            config,
            &[
                "cosmos",
                "generate-contract",
                &schema_path.to_string_lossy(),
                "--address", &format!("cosmos1{}123456789", feature),
                "--chain", "cosmoshub-4",
                "--features", feature,
                "--output-dir", &feature_output.to_string_lossy()
            ],
            &format!("cosmos_feature_{}", feature)
        ).await;
        results.add_result(result);
    }

    // Test all individual features for ethereum
    for feature in &["client", "storage", "api", "migrations"] {
        let feature_output = test_env.create_output_dir(&format!("eth_feature_{}", feature)).unwrap();
        
        let result = run_cli_command(
            config,
            &[
                "ethereum",
                "generate-contract",
                &erc20_abi_path.to_string_lossy(),
                "--address", "0x1234567890123456789012345678901234567890",
                "--chain", "1",
                "--features", feature,
                "--output-dir", &feature_output.to_string_lossy()
            ],
            &format!("eth_feature_{}", feature)
        ).await;
        results.add_result(result);
    }

    // === Chain ID variation tests ===
    
    // Test different Cosmos chains from documentation
    let cosmos_chains = &[
        ("cosmoshub-4", "cosmos1test123456789"),
        ("juno-1", "juno1test123456789"),
        ("stargaze-1", "stars1test123456789"),
        ("osmosis-1", "osmo1test123456789")
    ];

    for (chain_id, address) in cosmos_chains {
        let chain_output = test_env.create_output_dir(&format!("cosmos_chain_{}", chain_id.replace("-", "_"))).unwrap();
        
        let result = run_cli_command(
            config,
            &[
                "cosmos",
                "generate-contract",
                &schema_path.to_string_lossy(),
                "--address", address,
                "--chain", chain_id,
                "--features", "client",
                "--output-dir", &chain_output.to_string_lossy(),
                "--dry-run"
            ],
            &format!("cosmos_chain_{}", chain_id.replace("-", "_"))
        ).await;
        results.add_result(result);
    }

    // Test different Ethereum chains from documentation
    let ethereum_chains = &[
        ("1", "0x1234567890123456789012345678901234567890"), // Mainnet
        ("5", "0x1234567890123456789012345678901234567890"), // Goerli
        ("11155111", "0x1234567890123456789012345678901234567890"), // Sepolia
        ("137", "0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174"), // Polygon
        ("80001", "0x1234567890123456789012345678901234567890") // Mumbai
    ];

    for (chain_id, address) in ethereum_chains {
        let chain_output = test_env.create_output_dir(&format!("eth_chain_{}", chain_id)).unwrap();
        
        let result = run_cli_command(
            config,
            &[
                "ethereum",
                "generate-contract",
                &erc20_abi_path.to_string_lossy(),
                "--address", address,
                "--chain", chain_id,
                "--features", "client",
                "--output-dir", &chain_output.to_string_lossy(),
                "--dry-run"
            ],
            &format!("eth_chain_{}", chain_id)
        ).await;
        results.add_result(result);
    }

    // === Combined feature tests ===
    
    // Test common feature combinations
    let feature_combinations = &[
        "client,storage",
        "client,api",
        "storage,migrations",
        "client,storage,api",
        "storage,api,migrations",
        "client,storage,api,migrations"
    ];

    for (i, features) in feature_combinations.iter().enumerate() {
        // Test cosmos
        let cosmos_combo_output = test_env.create_output_dir(&format!("cosmos_combo_{}", i)).unwrap();
        
        let result = run_cli_command(
            config,
            &[
                "cosmos",
                "generate-contract",
                &schema_path.to_string_lossy(),
                "--address", &format!("cosmos1combo{}123456789", i),
                "--chain", "cosmoshub-4",
                "--features", features,
                "--output-dir", &cosmos_combo_output.to_string_lossy()
            ],
            &format!("cosmos_combo_{}", i)
        ).await;
        results.add_result(result);

        // Test ethereum
        let eth_combo_output = test_env.create_output_dir(&format!("eth_combo_{}", i)).unwrap();
        
        let result = run_cli_command(
            config,
            &[
                "ethereum",
                "generate-contract",
                &erc20_abi_path.to_string_lossy(),
                "--address", "0x1234567890123456789012345678901234567890",
                "--chain", "1",
                "--features", features,
                "--output-dir", &eth_combo_output.to_string_lossy()
            ],
            &format!("eth_combo_{}", i)
        ).await;
        results.add_result(result);
    }

    // === Error condition tests ===
    
    // Test various error conditions documented in troubleshooting guides
    
    // Missing required arguments
    let result = run_cli_command(
        config,
        &["cosmos", "generate-contract"],
        "cosmos_missing_args"
    ).await;
    let mut expected_failure = result;
    expected_failure.success = !expected_failure.success;
    results.add_result(expected_failure);

    let result = run_cli_command(
        config,
        &["ethereum", "generate-contract"],
        "eth_missing_args"
    ).await;
    let mut expected_failure = result;
    expected_failure.success = !expected_failure.success;
    results.add_result(expected_failure);

    // Invalid file paths
    let result = run_cli_command(
        config,
        &[
            "cosmos",
            "generate-contract",
            "/nonexistent/path/schema.json",
            "--address", "cosmos1test123456789",
            "--chain", "cosmoshub-4"
        ],
        "cosmos_invalid_file"
    ).await;
    let mut expected_failure = result;
    expected_failure.success = !expected_failure.success;
    results.add_result(expected_failure);

    let result = run_cli_command(
        config,
        &[
            "ethereum",
            "generate-contract",
            "/nonexistent/path/abi.json",
            "--address", "0x1234567890123456789012345678901234567890",
            "--chain", "1"
        ],
        "eth_invalid_file"
    ).await;
    let mut expected_failure = result;
    expected_failure.success = !expected_failure.success;
    results.add_result(expected_failure);

    // Invalid addresses
    let result = run_cli_command(
        config,
        &[
            "cosmos",
            "generate-contract",
            &schema_path.to_string_lossy(),
            "--address", "invalid_cosmos_address",
            "--chain", "cosmoshub-4"
        ],
        "cosmos_invalid_address"
    ).await;
    let mut expected_failure = result;
    expected_failure.success = !expected_failure.success;
    results.add_result(expected_failure);

    let result = run_cli_command(
        config,
        &[
            "ethereum",
            "generate-contract",
            &erc20_abi_path.to_string_lossy(),
            "--address", "not_an_ethereum_address",
            "--chain", "1"
        ],
        "eth_invalid_address"
    ).await;
    let mut expected_failure = result;
    expected_failure.success = !expected_failure.success;
    results.add_result(expected_failure);

    // Invalid chain IDs
    let result = run_cli_command(
        config,
        &[
            "cosmos",
            "generate-contract",
            &schema_path.to_string_lossy(),
            "--address", "cosmos1test123456789",
            "--chain", ""
        ],
        "cosmos_empty_chain"
    ).await;
    let mut expected_failure = result;
    expected_failure.success = !expected_failure.success;
    results.add_result(expected_failure);

    let result = run_cli_command(
        config,
        &[
            "ethereum",
            "generate-contract",
            &erc20_abi_path.to_string_lossy(),
            "--address", "0x1234567890123456789012345678901234567890",
            "--chain", "not_a_number"
        ],
        "eth_invalid_chain"
    ).await;
    let mut expected_failure = result;
    expected_failure.success = !expected_failure.success;
    results.add_result(expected_failure);

    // Invalid features
    let result = run_cli_command(
        config,
        &[
            "cosmos",
            "generate-contract",
            &schema_path.to_string_lossy(),
            "--address", "cosmos1test123456789",
            "--chain", "cosmoshub-4",
            "--features", "invalid_feature"
        ],
        "cosmos_invalid_feature"
    ).await;
    let mut expected_failure = result;
    expected_failure.success = !expected_failure.success;
    results.add_result(expected_failure);

    let result = run_cli_command(
        config,
        &[
            "ethereum",
            "generate-contract",
            &erc20_abi_path.to_string_lossy(),
            "--address", "0x1234567890123456789012345678901234567890",
            "--chain", "1",
            "--features", "nonexistent_feature"
        ],
        "eth_invalid_feature"
    ).await;
    let mut expected_failure = result;
    expected_failure.success = !expected_failure.success;
    results.add_result(expected_failure);

    results
} 