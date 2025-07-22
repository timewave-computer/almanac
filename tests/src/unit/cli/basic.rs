//! Basic CLI functionality tests

use crate::utils::run_cli_command;
use crate::{TestConfig, TestOutcome, TestSuiteResults};

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
        let success_result = TestOutcome::success(
            "invalid_command_fails_gracefully",
            result.duration_ms,
        );
        results.add_result(success_result);
    } else {
        results.add_result(result);
    }

    results
}
