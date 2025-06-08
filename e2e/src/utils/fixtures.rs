//! Test fixtures for schemas and ABIs

use crate::E2EError;
use anyhow::Result;
use serde_json::json;
use std::fs;
use std::path::Path;

/// Create test Valence Base Account schema
pub fn create_valence_base_account_schema<P: AsRef<Path>>(output_path: P) -> Result<(), E2EError> {
    let schema = json!({
        "contract_name": "valence-base-account",
        "contract_version": "0.2.0",
        "idl_version": "1.0.0",
        "instantiate": {
            "$schema": "http://json-schema.org/draft-07/schema#",
            "title": "InstantiateMsg",
            "type": "object",
            "required": ["admin", "approved_libraries"],
            "properties": {
                "admin": {"type": "string"},
                "approved_libraries": {
                    "type": "array",
                    "items": {"type": "string"}
                }
            },
            "additionalProperties": false
        },
        "execute": {
            "$schema": "http://json-schema.org/draft-07/schema#",
            "title": "ExecuteMsg",
            "oneOf": [
                {
                    "type": "object",
                    "required": ["approve_library"],
                    "properties": {
                        "approve_library": {
                            "type": "object",
                            "required": ["library"],
                            "properties": {"library": {"type": "string"}},
                            "additionalProperties": false
                        }
                    },
                    "additionalProperties": false
                },
                {
                    "type": "object",
                    "required": ["remove_library"],
                    "properties": {
                        "remove_library": {
                            "type": "object",
                            "required": ["library"],
                            "properties": {"library": {"type": "string"}},
                            "additionalProperties": false
                        }
                    },
                    "additionalProperties": false
                }
            ]
        },
        "query": {
            "$schema": "http://json-schema.org/draft-07/schema#",
            "title": "QueryMsg",
            "oneOf": [
                {
                    "type": "object",
                    "required": ["list_approved_libraries"],
                    "properties": {
                        "list_approved_libraries": {
                            "type": "object",
                            "additionalProperties": false
                        }
                    },
                    "additionalProperties": false
                },
                {
                    "type": "object",
                    "required": ["ownership"],
                    "properties": {
                        "ownership": {
                            "type": "object",
                            "additionalProperties": false
                        }
                    },
                    "additionalProperties": false
                }
            ]
        },
        "migrate": null,
        "sudo": null,
        "responses": {
            "list_approved_libraries": {
                "$schema": "http://json-schema.org/draft-07/schema#",
                "title": "Array_of_String",
                "type": "array",
                "items": {"type": "string"}
            },
            "ownership": {
                "$schema": "http://json-schema.org/draft-07/schema#",
                "title": "Ownership_for_String",
                "type": "object",
                "properties": {
                    "owner": {"type": ["string", "null"]},
                    "pending_owner": {"type": ["string", "null"]},
                    "pending_expiry": {"type": ["object", "null"]}
                },
                "additionalProperties": false
            }
        }
    });
    
    let content = serde_json::to_string_pretty(&schema)
        .map_err(|e| E2EError::SetupFailed {
            reason: format!("Failed to serialize schema: {}", e)
        })?;
    
    fs::write(output_path, content)
        .map_err(|e| E2EError::SetupFailed {
            reason: format!("Failed to write schema file: {}", e)
        })?;
    
    Ok(())
}

/// Create test ERC20 ABI
pub fn create_erc20_abi<P: AsRef<Path>>(output_path: P) -> Result<(), E2EError> {
    let abi = json!([
        {
            "type": "function",
            "name": "name",
            "inputs": [],
            "outputs": [{"name": "", "type": "string"}],
            "stateMutability": "view"
        },
        {
            "type": "function",
            "name": "symbol",
            "inputs": [],
            "outputs": [{"name": "", "type": "string"}],
            "stateMutability": "view"
        },
        {
            "type": "function",
            "name": "decimals",
            "inputs": [],
            "outputs": [{"name": "", "type": "uint8"}],
            "stateMutability": "view"
        },
        {
            "type": "function",
            "name": "totalSupply",
            "inputs": [],
            "outputs": [{"name": "", "type": "uint256"}],
            "stateMutability": "view"
        },
        {
            "type": "function",
            "name": "balanceOf",
            "inputs": [{"name": "account", "type": "address"}],
            "outputs": [{"name": "", "type": "uint256"}],
            "stateMutability": "view"
        },
        {
            "type": "function",
            "name": "transfer",
            "inputs": [
                {"name": "to", "type": "address"},
                {"name": "amount", "type": "uint256"}
            ],
            "outputs": [{"name": "", "type": "bool"}],
            "stateMutability": "nonpayable"
        },
        {
            "type": "function",
            "name": "allowance",
            "inputs": [
                {"name": "owner", "type": "address"},
                {"name": "spender", "type": "address"}
            ],
            "outputs": [{"name": "", "type": "uint256"}],
            "stateMutability": "view"
        },
        {
            "type": "function",
            "name": "approve",
            "inputs": [
                {"name": "spender", "type": "address"},
                {"name": "amount", "type": "uint256"}
            ],
            "outputs": [{"name": "", "type": "bool"}],
            "stateMutability": "nonpayable"
        },
        {
            "type": "function",
            "name": "transferFrom",
            "inputs": [
                {"name": "from", "type": "address"},
                {"name": "to", "type": "address"},
                {"name": "amount", "type": "uint256"}
            ],
            "outputs": [{"name": "", "type": "bool"}],
            "stateMutability": "nonpayable"
        },
        {
            "type": "event",
            "name": "Transfer",
            "inputs": [
                {"name": "from", "type": "address", "indexed": true},
                {"name": "to", "type": "address", "indexed": true},
                {"name": "value", "type": "uint256", "indexed": false}
            ]
        },
        {
            "type": "event",
            "name": "Approval",
            "inputs": [
                {"name": "owner", "type": "address", "indexed": true},
                {"name": "spender", "type": "address", "indexed": true},
                {"name": "value", "type": "uint256", "indexed": false}
            ]
        }
    ]);
    
    let content = serde_json::to_string_pretty(&abi)
        .map_err(|e| E2EError::SetupFailed {
            reason: format!("Failed to serialize ABI: {}", e)
        })?;
    
    fs::write(output_path, content)
        .map_err(|e| E2EError::SetupFailed {
            reason: format!("Failed to write ABI file: {}", e)
        })?;
    
    Ok(())
}

/// Create test configuration files
pub fn create_test_config<P: AsRef<Path>>(output_path: P, config_type: &str) -> Result<(), E2EError> {
    let config_content = match config_type {
        "cosmos" => r#"[cosmos]
output_dir = "./generated"
default_features = ["client", "storage", "api"]

[cosmos.templates]
template_dir = "./templates"

[cosmos.database]
url = "postgresql://localhost/test_db"
schema_prefix = "contract_"

[cosmos.api]
base_path = "/api/v1/cosmos"
rate_limiting = true
cors = true

[[cosmos.contracts]]
name_pattern = "valence_base_account_*"
features = ["client", "storage", "api"]
namespace_prefix = "valence_"
"#,
        "ethereum" => r#"[ethereum]
output_dir = "./generated"
default_features = ["client", "storage", "api"]

[ethereum.templates]
template_dir = "./templates"

[ethereum.database]
url = "postgresql://localhost/test_db"
schema_prefix = "contract_"

[ethereum.api]
base_path = "/api/v1/ethereum"
rate_limiting = true
cors = true

[[ethereum.contracts]]
name_pattern = "erc20_*"
features = ["client", "storage", "api"]
namespace_prefix = "token_"
"#,
        _ => return Err(E2EError::SetupFailed {
            reason: format!("Unknown config type: {}", config_type)
        })
    };
    
    fs::write(output_path, config_content)
        .map_err(|e| E2EError::SetupFailed {
            reason: format!("Failed to write config file: {}", e)
        })?;
    
    Ok(())
}

/// Initialize all test fixtures
pub fn setup_fixtures(fixtures_dir: &Path) -> Result<(), E2EError> {
    // Create directories
    let schemas_dir = fixtures_dir.join("schemas");
    let abis_dir = fixtures_dir.join("abis");
    fs::create_dir_all(&schemas_dir)
        .map_err(|e| E2EError::SetupFailed {
            reason: format!("Failed to create schemas directory: {}", e)
        })?;
    fs::create_dir_all(&abis_dir)
        .map_err(|e| E2EError::SetupFailed {
            reason: format!("Failed to create abis directory: {}", e)
        })?;
    
    // Create schema files
    create_valence_base_account_schema(schemas_dir.join("valence_base_account.json"))?;
    
    // Create ABI files
    create_erc20_abi(abis_dir.join("erc20.json"))?;
    
    // Create config files
    create_test_config(fixtures_dir.join("cosmos_config.toml"), "cosmos")?;
    create_test_config(fixtures_dir.join("ethereum_config.toml"), "ethereum")?;
    
    Ok(())
} 