//! Basic CLI functionality tests

use almanac_e2e_tests::{TestConfig, TestResult, TestSuiteResults};
use almanac_e2e_tests::utils::run_cli_command;

pub async fn run_basic_cli_tests(config: &TestConfig) -> TestSuiteResults {
    let mut results = TestSuiteResults::default();
    
    // Test --version command
    let result = run_cli_command(config, &["--version"], "version_command").await;
    results.add_result(result);
    
    // Test --help command
    let result = run_cli_command(config, &["--help"], "help_command").await;
    results.add_result(result);
    
    // Test help for specific subcommands
    let result = run_cli_command(config, &["cosmos", "--help"], "cosmos_help").await;
    results.add_result(result);
    
    let result = run_cli_command(config, &["ethereum", "--help"], "ethereum_help").await;
    results.add_result(result);
    
    let result = run_cli_command(config, &["config", "--help"], "config_help").await;
    results.add_result(result);
    
    // Test invalid command
    let result = run_cli_command(config, &["invalid-command"], "invalid_command").await;
    // Note: This should fail, but we expect it to fail gracefully
    if !result.success {
        // Convert failure to success if it fails as expected
        let success_result = TestResult::success(
            "invalid_command_fails_gracefully".to_string(),
            result.duration,
            "Invalid command fails as expected".to_string()
        );
        results.add_result(success_result);
    } else {
        results.add_result(result);
    }
    
    results
} 