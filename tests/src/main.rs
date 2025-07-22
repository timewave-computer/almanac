//! Test runner for Almanac test suite

use almanac_tests::{TestConfig, TestSuiteResults};
use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "test-runner")]
#[command(about = "Almanac test suite runner", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run unit tests
    Unit {
        /// Specific test to run
        #[arg(short, long)]
        test: Option<String>,
    },
    /// Run integration tests
    Integration {
        /// Specific test to run
        #[arg(short, long)]
        test: Option<String>,
    },
    /// Run end-to-end tests
    E2e {
        /// Specific test to run
        #[arg(short, long)]
        test: Option<String>,
    },
    /// Run all tests
    All,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    
    match cli.command {
        Commands::Unit { test } => {
            println!("Running unit tests...");
            run_unit_tests(test).await?;
        }
        Commands::Integration { test } => {
            println!("Running integration tests...");
            run_integration_tests(test).await?;
        }
        Commands::E2e { test } => {
            println!("Running E2E tests...");
            run_e2e_tests(test).await?;
        }
        Commands::All => {
            println!("Running all tests...");
            run_unit_tests(None).await?;
            run_integration_tests(None).await?;
            run_e2e_tests(None).await?;
        }
    }
    
    Ok(())
}

async fn run_unit_tests(test: Option<String>) -> Result<()> {
    use almanac_tests::unit::cli;
    use almanac_tests::utils::setup::TestEnvironment;
    
    let config = TestConfig::from_env()?;
    let test_env = TestEnvironment::new()?;
    
    if let Some(specific_test) = test {
        match specific_test.as_str() {
            "basic" => {
                let results = cli::basic::run_basic_cli_tests(&config).await;
                print_results("Basic CLI", &results);
            }
            "config" => {
                let results = cli::config::run_config_tests(&config, &test_env).await;
                print_results("Config CLI", &results);
            }
            _ => println!("Unknown unit test: {}", specific_test),
        }
    } else {
        // Run all unit tests
        let basic_results = cli::basic::run_basic_cli_tests(&config).await;
        print_results("Basic CLI", &basic_results);
        
        let config_results = cli::config::run_config_tests(&config, &test_env).await;
        print_results("Config CLI", &config_results);
    }
    
    Ok(())
}

async fn run_integration_tests(test: Option<String>) -> Result<()> {
    if let Some(specific_test) = test {
        println!("Running integration test: {}", specific_test);
    } else {
        println!("Running all integration tests...");
    }
    // Integration tests would be run here
    Ok(())
}

async fn run_e2e_tests(test: Option<String>) -> Result<()> {
    if let Some(specific_test) = test {
        println!("Running E2E test: {}", specific_test);
    } else {
        println!("Running all E2E tests...");
    }
    // E2E tests would be run here
    Ok(())
}

fn print_results(test_name: &str, results: &TestSuiteResults) {
    println!("\n{} Test Results:", test_name);
    println!("  Passed: {}", results.passed.len());
    println!("  Failed: {}", results.failed.len());
    println!("  Success Rate: {:.1}%", results.success_rate() * 100.0);
    
    if !results.failed.is_empty() {
        println!("\n  Failed tests:");
        for test in &results.failed {
            println!("    - {}: {}", test.name, test.error_message.as_ref().unwrap_or(&"Unknown error".to_string()));
        }
    }
}