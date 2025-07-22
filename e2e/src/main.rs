//! Main test runner for Almanac CLI end-to-end tests

use almanac_e2e_tests::utils::fixtures::setup_fixtures;
use almanac_e2e_tests::utils::print_test_summary;
use almanac_e2e_tests::utils::setup::{ensure_almanac_binary, TestEnvironment};
use almanac_e2e_tests::TestSuiteResults;
use colored::*;
use std::env;

mod tests;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    println!("{}", "Almanac CLI End-to-End Test Suite".bold().blue());
    println!("{}", "=".repeat(50));

    // Parse command line arguments
    let args: Vec<String> = env::args().collect();
    let verbose = args.contains(&"--verbose".to_string());
    let test_filter = args
        .iter()
        .find(|arg| arg.starts_with("--filter="))
        .map(|arg| arg.strip_prefix("--filter=").unwrap_or(""))
        .unwrap_or("");

    // Setup test environment
    let test_env =
        TestEnvironment::new().map_err(|e| format!("Failed to setup test environment: {}", e))?;

    let mut config = test_env.config.clone();
    config.verbose = verbose;

    // Ensure Almanac binary exists only for CLI tests
    // If no filter is specified, we only run integration tests (multi-chain, solana, unit)
    let needs_cli_binary = !test_filter.is_empty()
        && (should_run_test("basic", test_filter)
            || should_run_test("comprehensive", test_filter)
            || should_run_test("cosmos", test_filter)
            || should_run_test("ethereum", test_filter)
            || should_run_test("config", test_filter)
            || should_run_test("validation", test_filter));

    if needs_cli_binary {
        println!("Checking Almanac binary...");
        if let Err(e) = ensure_almanac_binary(&mut config).await {
            eprintln!("{}", format!("Failed to setup Almanac binary: {}", e).red());
            return Err(e.into());
        }
        println!("{}", "Almanac binary ready".green());
    } else {
        println!(
            "{}",
            "Skipping CLI binary check (not needed for selected tests)".yellow()
        );
    }

    // Setup test fixtures
    println!("Setting up test fixtures...");
    let fixtures_dir = test_env.fixtures_dir();
    if let Err(e) = setup_fixtures(&fixtures_dir) {
        eprintln!("{}", format!("Failed to setup fixtures: {}", e).red());
        return Err(e.into());
    }
    println!("{}", "Test fixtures ready".green());

    // Run all test suites
    let mut all_results = TestSuiteResults::default();

    println!("\n{}", "Running test suites...".bold());

    // Basic CLI tests
    if should_run_test("basic", test_filter) {
        println!("\n{}", "Basic CLI Tests".cyan().bold());
        let results = tests::basic::run_basic_cli_tests(&config).await;
        print_suite_results("Basic CLI", &results);
        for result in results.results {
            all_results.add_result(result);
        }
    }

    // Comprehensive CLI tests - covering all documented commands
    if should_run_test("comprehensive", test_filter) {
        println!(
            "\n{}",
            "Comprehensive CLI Tests (All Documented Commands)"
                .cyan()
                .bold()
        );
        let results =
            tests::cli_comprehensive::run_comprehensive_cli_tests(&config, &test_env).await;
        print_suite_results("Comprehensive CLI", &results);
        for result in results.results {
            all_results.add_result(result);
        }
    }

    // Cosmos CLI tests
    if should_run_test("cosmos", test_filter) {
        println!("\n{}", "Cosmos CLI Tests".cyan().bold());
        let results = tests::cosmos::run_cosmos_cli_tests(&config, &test_env).await;
        print_suite_results("Cosmos CLI", &results);
        for result in results.results {
            all_results.add_result(result);
        }
    }

    // Ethereum CLI tests
    if should_run_test("ethereum", test_filter) {
        println!("\n{}", "Ethereum CLI Tests".cyan().bold());
        let results = tests::ethereum::run_ethereum_cli_tests(&config, &test_env).await;
        print_suite_results("Ethereum CLI", &results);
        for result in results.results {
            all_results.add_result(result);
        }
    }

    // Configuration tests
    if should_run_test("config", test_filter) {
        println!("\n{}", "Configuration Tests".cyan().bold());
        let results = tests::config::run_config_tests(&config, &test_env).await;
        print_suite_results("Configuration", &results);
        for result in results.results {
            all_results.add_result(result);
        }
    }

    // Code generation validation tests
    if should_run_test("validation", test_filter) {
        println!("\n{}", "Code Generation Validation Tests".cyan().bold());
        let results = tests::validation::run_validation_tests(&config, &test_env).await;
        print_suite_results("Validation", &results);
        for result in results.results {
            all_results.add_result(result);
        }
    }

    // Multi-chain simulation E2E test
    if should_run_test("multi-chain-simulation", test_filter) {
        println!("\n{}", "Multi-Chain Simulation E2E Test".cyan().bold());
        println!("   This test requires a full simulation environment with:");
        println!("   - Solana test validator");
        println!("   - Ethereum/Anvil node");
        println!("   - Neutron Cosmos node");
        println!("   - FoundationDB cluster");
        println!("   - Almanac indexer and API");

        match tests::multi_chain_simulation_e2e::run_multi_chain_simulation_e2e_test().await {
            Ok(()) => {
                println!("  {}", "Multi-chain simulation test passed".green());
                all_results.add_result(almanac_e2e_tests::TestOutcome {
                    name: "multi_chain_simulation".to_string(),
                    success: true,
                    output: "Multi-chain simulation test completed successfully".to_string(),
                    duration: std::time::Duration::from_secs(0), // Placeholder
                    error: None,
                });
            }
            Err(e) => {
                println!(
                    "  {} Multi-chain simulation test failed: {}",
                    "FAILED".red(),
                    e
                );
                all_results.add_result(almanac_e2e_tests::TestOutcome {
                    name: "multi_chain_simulation".to_string(),
                    success: false,
                    output: String::new(),
                    duration: std::time::Duration::from_secs(0), // Placeholder
                    error: Some(format!("Multi-chain simulation test failed: {}", e)),
                });
            }
        }
    }

    // Unit tests (equivalent to cargo test)
    if should_run_test("unit", test_filter) {
        println!(
            "\n{}",
            "Unit Tests (FoundationDB Integration)".cyan().bold()
        );
        println!("   Running unit tests with FoundationDB integration:");
        println!("   - FDB data validation tests");
        println!("   - Cross-chain consistency tests");
        println!("   - Storage backend integration tests");

        match run_unit_tests().await {
            Ok(()) => {
                println!("  {}", "Unit tests passed".green());
                all_results.add_result(almanac_e2e_tests::TestOutcome {
                    name: "unit_tests".to_string(),
                    success: true,
                    output: "Unit tests completed successfully".to_string(),
                    duration: std::time::Duration::from_secs(0), // Placeholder
                    error: None,
                });
            }
            Err(e) => {
                println!("  {} Unit tests failed: {}", "FAILED".red(), e);
                all_results.add_result(almanac_e2e_tests::TestOutcome {
                    name: "unit_tests".to_string(),
                    success: false,
                    output: String::new(),
                    duration: std::time::Duration::from_secs(0), // Placeholder
                    error: Some(format!("Unit tests failed: {}", e)),
                });
            }
        }
    }

    // Solana Account Subscription E2E test
    if should_run_test("solana-subscription", test_filter) {
        println!("\n{}", "Solana Account Subscription E2E Test".cyan().bold());
        println!("   This test validates the complete account subscription system:");
        println!("   - Solana test validator");
        println!("   - Account subscription with bloom filters");
        println!("   - Sparse Merkle tree verification");
        println!("   - Geyser streaming simulation");
        println!("   - REST API endpoints");

        match tests::solana_real_e2e::run_real_solana_account_subscription_e2e_test().await {
            Ok(()) => {
                println!("  {}", "Solana account subscription test passed".green());
                all_results.add_result(almanac_e2e_tests::TestOutcome {
                    name: "solana_account_subscription".to_string(),
                    success: true,
                    output: "Solana account subscription test completed successfully".to_string(),
                    duration: std::time::Duration::from_secs(0), // Placeholder
                    error: None,
                });
            }
            Err(e) => {
                println!(
                    "  {} Solana account subscription test failed: {}",
                    "FAILED".red(),
                    e
                );
                all_results.add_result(almanac_e2e_tests::TestOutcome {
                    name: "solana_account_subscription".to_string(),
                    success: false,
                    output: String::new(),
                    duration: std::time::Duration::from_secs(0), // Placeholder
                    error: Some(format!("Solana account subscription test failed: {}", e)),
                });
            }
        }
    }

    // Print final summary
    print_test_summary(&all_results.results);

    // Exit with appropriate code
    if all_results.failure_count() > 0 {
        std::process::exit(1);
    } else {
        println!("\n{}", "All tests passed!".green().bold());
        std::process::exit(0);
    }
}

/// Run unit tests programmatically (equivalent to cargo test)
async fn run_unit_tests() -> Result<(), Box<dyn std::error::Error>> {
    use std::process::Command;

    println!("Running unit tests via cargo test...");

    let mut cmd = Command::new("cargo");
    cmd.args([
        "test",
        "-p",
        "almanac-e2e-tests",
        "--release",
        "--all-features",
        "--",
        "--include-ignored",
    ]);

    // Pass through essential FDB environment variables
    if let Ok(fdb_cluster_file) = std::env::var("FDB_CLUSTER_FILE") {
        cmd.env("FDB_CLUSTER_FILE", fdb_cluster_file);
    }
    if let Ok(fdb_include_dir) = std::env::var("FDB_INCLUDE_DIR") {
        cmd.env("FDB_INCLUDE_DIR", fdb_include_dir);
    }
    if let Ok(dyld_path) = std::env::var("DYLD_LIBRARY_PATH") {
        cmd.env("DYLD_LIBRARY_PATH", dyld_path);
    }
    if let Ok(ld_path) = std::env::var("LD_LIBRARY_PATH") {
        cmd.env("LD_LIBRARY_PATH", ld_path);
    }

    // Pass through FDB TLS options
    cmd.env("FDB_NETWORK_OPTION_TLS_CERT_PATH", "");
    cmd.env("FDB_NETWORK_OPTION_TLS_KEY_PATH", "");
    cmd.env("FDB_NETWORK_OPTION_TLS_CA_PATH", "");

    let output = cmd
        .output()
        .map_err(|e| format!("Failed to run cargo test: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Unit tests failed:\n{}", stderr).into());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    println!("{}", stdout);

    Ok(())
}

fn should_run_test(test_name: &str, filter: &str) -> bool {
    if filter.is_empty() {
        // When no filter is specified, only run integration tests (not CLI tests)
        matches!(
            test_name,
            "multi-chain-simulation" | "solana-subscription" | "unit"
        )
    } else {
        test_name.contains(filter)
    }
}

fn print_suite_results(_suite_name: &str, results: &TestSuiteResults) {
    let passed = results.success_count();
    let failed = results.failure_count();
    let total = results.results.len();

    if failed == 0 {
        println!(
            "  {} {}/{} tests passed ({:.1}%)",
            "PASSED".green(),
            passed,
            total,
            results.success_rate() * 100.0
        );
    } else {
        println!(
            "  {} {}/{} tests passed, {} failed ({:.1}%)",
            "FAILED".yellow(),
            passed,
            total,
            failed,
            results.success_rate() * 100.0
        );
    }

    if results.results.iter().any(|r| !r.success) {
        println!("    Failed tests:");
        for result in &results.results {
            if !result.success {
                println!("      - {}", result.name.red());
            }
        }
    }
}
