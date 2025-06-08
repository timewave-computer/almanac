//! Main test runner for Almanac CLI end-to-end tests

use almanac_e2e_tests::TestSuiteResults;
use almanac_e2e_tests::utils::print_test_summary;
use almanac_e2e_tests::utils::setup::{TestEnvironment, ensure_almanac_binary};
use almanac_e2e_tests::utils::fixtures::setup_fixtures;
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
    let test_filter = args.iter()
        .find(|arg| arg.starts_with("--filter="))
        .map(|arg| arg.strip_prefix("--filter=").unwrap_or(""))
        .unwrap_or("");

    // Setup test environment
    let test_env = TestEnvironment::new()
        .map_err(|e| format!("Failed to setup test environment: {}", e))?;
    
    let mut config = test_env.config.clone();
    config.verbose = verbose;

    // Ensure Almanac binary exists
    println!("Checking Almanac binary...");
    if let Err(e) = ensure_almanac_binary(&mut config).await {
        eprintln!("❌ {}", format!("Failed to setup Almanac binary: {}", e).red());
        return Err(e.into());
    }
    println!("✓ {}", "Almanac binary ready".green());

    // Setup test fixtures
    println!("Setting up test fixtures...");
    let fixtures_dir = test_env.fixtures_dir();
    if let Err(e) = setup_fixtures(&fixtures_dir) {
        eprintln!("❌ {}", format!("Failed to setup fixtures: {}", e).red());
        return Err(e.into());
    }
    println!("✓ {}", "Test fixtures ready".green());

    // Run all test suites
    let mut all_results = TestSuiteResults::default();

    println!("\n{}", "Running test suites...".bold());

    // Basic CLI tests
    if should_run_test("basic", test_filter) {
        println!("\n{}", "🧪 Basic CLI Tests".cyan().bold());
        let results = tests::basic::run_basic_cli_tests(&config).await;
        print_suite_results("Basic CLI", &results);
        for result in results.results {
            all_results.add_result(result);
        }
    }

    // Comprehensive CLI tests - covering all documented commands
    if should_run_test("comprehensive", test_filter) {
        println!("\n{}", "📚 Comprehensive CLI Tests (All Documented Commands)".cyan().bold());
        let results = tests::cli_comprehensive::run_comprehensive_cli_tests(&config, &test_env).await;
        print_suite_results("Comprehensive CLI", &results);
        for result in results.results {
            all_results.add_result(result);
        }
    }

    // Cosmos CLI tests
    if should_run_test("cosmos", test_filter) {
        println!("\n{}", "🌌 Cosmos CLI Tests".cyan().bold());
        let results = tests::cosmos::run_cosmos_cli_tests(&config, &test_env).await;
        print_suite_results("Cosmos CLI", &results);
        for result in results.results {
            all_results.add_result(result);
        }
    }

    // Ethereum CLI tests
    if should_run_test("ethereum", test_filter) {
        println!("\n{}", "⚡ Ethereum CLI Tests".cyan().bold());
        let results = tests::ethereum::run_ethereum_cli_tests(&config, &test_env).await;
        print_suite_results("Ethereum CLI", &results);
        for result in results.results {
            all_results.add_result(result);
        }
    }

    // Configuration tests
    if should_run_test("config", test_filter) {
        println!("\n{}", "⚙️  Configuration Tests".cyan().bold());
        let results = tests::config::run_config_tests(&config, &test_env).await;
        print_suite_results("Configuration", &results);
        for result in results.results {
            all_results.add_result(result);
        }
    }

    // Code generation validation tests
    if should_run_test("validation", test_filter) {
        println!("\n{}", "✅ Code Generation Validation Tests".cyan().bold());
        let results = tests::validation::run_validation_tests(&config, &test_env).await;
        print_suite_results("Validation", &results);
        for result in results.results {
            all_results.add_result(result);
        }
    }

    // Print final summary
    print_test_summary(&all_results.results);

    // Exit with appropriate code
    if all_results.failure_count() > 0 {
        std::process::exit(1);
    } else {
        println!("\n🎉 {}", "All tests passed!".green().bold());
        std::process::exit(0);
    }
}

fn should_run_test(test_name: &str, filter: &str) -> bool {
    if filter.is_empty() {
        true
    } else {
        test_name.contains(filter)
    }
}

fn print_suite_results(_suite_name: &str, results: &TestSuiteResults) {
    let passed = results.success_count();
    let failed = results.failure_count();
    let total = results.results.len();
    
    if failed == 0 {
        println!("  {} {}/{} tests passed ({:.1}%)", 
                "✓".green(), passed, total, results.success_rate() * 100.0);
    } else {
        println!("  {} {}/{} tests passed, {} failed ({:.1}%)", 
                "⚠".yellow(), passed, total, failed, results.success_rate() * 100.0);
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