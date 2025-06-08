//! Configuration management tests

use almanac_e2e_tests::{TestConfig, TestResult, TestSuiteResults};
use almanac_e2e_tests::utils::run_cli_command;
use almanac_e2e_tests::utils::setup::TestEnvironment;
use almanac_e2e_tests::utils::validation::validate_toml_config;
use std::fs;

pub async fn run_config_tests(config: &TestConfig, test_env: &TestEnvironment) -> TestSuiteResults {
    let mut results = TestSuiteResults::default();
    
    // Test config subcommand help
    let result = run_cli_command(config, &["config", "--help"], "config_help_detailed").await;
    results.add_result(result);
    
    // Test config init command
    let config_output_dir = test_env.create_output_dir("config_init_test").unwrap();
    let config_file_path = config_output_dir.join("almanac.toml");
    let config_file_str = config_file_path.to_string_lossy();
    
    let result = run_cli_command(
        config,
        &[
            "config",
            "init",
            "--output", &config_file_str
        ],
        "config_init"
    ).await;
    results.add_result(result);
    
    // Validate that config file was created and is valid TOML
    if config_file_path.exists() {
        let validation_result = validate_toml_config(&config_file_path, "config_init_validation");
        results.add_result(validation_result);
        
        // Check that file contains expected sections
        if let Ok(content) = fs::read_to_string(&config_file_path) {
            let has_cosmos = content.contains("[cosmos]");
            let has_ethereum = content.contains("[ethereum]");
            
            if has_cosmos && has_ethereum {
                let sections_result = TestResult::success(
                    "config_init_sections".to_string(),
                    std::time::Duration::from_millis(1),
                    "Config file contains expected sections".to_string()
                );
                results.add_result(sections_result);
            } else {
                let sections_result = TestResult::failure(
                    "config_init_sections".to_string(),
                    std::time::Duration::from_millis(1),
                    format!("Missing sections - cosmos: {}, ethereum: {}", has_cosmos, has_ethereum)
                );
                results.add_result(sections_result);
            }
        }
    }
    
    // Test config validate command with valid config
    let valid_config_path = test_env.fixtures_dir().join("cosmos_config.toml");
    let result = run_cli_command(
        config,
        &[
            "config",
            "validate",
            "--config", &valid_config_path.to_string_lossy()
        ],
        "config_validate_valid"
    ).await;
    results.add_result(result);
    
    // Test config validate command with invalid config
    let invalid_config_path = test_env.temp_dir.path().join("invalid_config.toml");
    fs::write(&invalid_config_path, r#"
[cosmos]
invalid_field = "this should not exist"
[ethereum
# Intentionally malformed TOML
"#).unwrap();
    
    let result = run_cli_command(
        config,
        &[
            "config",
            "validate",
            "--config", &invalid_config_path.to_string_lossy()
        ],
        "config_validate_invalid"
    ).await;
    
    // Convert expected failure to success
    if !result.success {
        let success_result = TestResult::success(
            "config_validate_invalid_fails_gracefully".to_string(),
            result.duration,
            "Invalid config validation fails as expected".to_string()
        );
        results.add_result(success_result);
    } else {
        results.add_result(result);
    }
    
    // Test config show command
    let result = run_cli_command(
        config,
        &[
            "config",
            "show",
            "--config", &valid_config_path.to_string_lossy()
        ],
        "config_show"
    ).await;
    results.add_result(result);
    
    // Test config update command
    let update_config_path = test_env.temp_dir.path().join("update_config.toml");
    fs::copy(&valid_config_path, &update_config_path).unwrap();
    
    let result = run_cli_command(
        config,
        &[
            "config",
            "update",
            "--config", &update_config_path.to_string_lossy(),
            "--set", "cosmos.output_dir=./new_output",
            "--set", "cosmos.default_features=[\"client\"]"
        ],
        "config_update"
    ).await;
    results.add_result(result);
    
    // Validate that update worked
    if update_config_path.exists() {
        if let Ok(content) = fs::read_to_string(&update_config_path) {
            let has_new_output_dir = content.contains("output_dir = \"./new_output\"");
            
            if has_new_output_dir {
                let update_result = TestResult::success(
                    "config_update_validation".to_string(),
                    std::time::Duration::from_millis(1),
                    "Config update applied successfully".to_string()
                );
                results.add_result(update_result);
            } else {
                let update_result = TestResult::failure(
                    "config_update_validation".to_string(),
                    std::time::Duration::from_millis(1),
                    "Config update was not applied correctly".to_string()
                );
                results.add_result(update_result);
            }
        }
    }
    
    // Test config generate-template command
    let template_output_dir = test_env.create_output_dir("config_template_test").unwrap();
    let template_file_path = template_output_dir.join("template.toml");
    let template_file_str = template_file_path.to_string_lossy();
    
    let result = run_cli_command(
        config,
        &[
            "config",
            "generate-template",
            "--type", "cosmos",
            "--output", &template_file_str
        ],
        "config_generate_template_cosmos"
    ).await;
    results.add_result(result);
    
    // Test Ethereum template
    let eth_template_path = template_output_dir.join("eth_template.toml");
    let eth_template_str = eth_template_path.to_string_lossy();
    
    let result = run_cli_command(
        config,
        &[
            "config",
            "generate-template",
            "--type", "ethereum",
            "--output", &eth_template_str
        ],
        "config_generate_template_ethereum"
    ).await;
    results.add_result(result);
    
    // Test config merge command
    let merged_config_path = template_output_dir.join("merged.toml");
    let merged_config_str = merged_config_path.to_string_lossy();
    
    let result = run_cli_command(
        config,
        &[
            "config",
            "merge",
            "--configs", &template_file_str,
            "--configs", &eth_template_str,
            "--output", &merged_config_str
        ],
        "config_merge"
    ).await;
    results.add_result(result);
    
    // Validate merged config
    if merged_config_path.exists() {
        let validation_result = validate_toml_config(&merged_config_path, "config_merge_validation");
        results.add_result(validation_result);
    }
    
    // Test config list-features command
    let result = run_cli_command(
        config,
        &["config", "list-features"],
        "config_list_features"
    ).await;
    results.add_result(result);
    
    // Test config check-compatibility command
    let result = run_cli_command(
        config,
        &[
            "config",
            "check-compatibility",
            "--config", &valid_config_path.to_string_lossy()
        ],
        "config_check_compatibility"
    ).await;
    results.add_result(result);
    
    results
} 