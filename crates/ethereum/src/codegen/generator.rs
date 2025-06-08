//! Code generator for Ethereum contracts
//! 
//! Generates Rust code for client interactions, storage models, APIs, and migrations
//! from parsed Ethereum contract ABIs.

use super::{EthereumCodegenConfig, parser::EthereumAbi};
use indexer_core::Result;
use std::path::{Path, PathBuf};
use convert_case::Casing;

/// Code generator for ethereum contracts
pub struct EthereumContractCodegen {
    config: EthereumCodegenConfig,
}

impl EthereumContractCodegen {
    /// Create a new code generator with the given configuration
    pub fn new(config: EthereumCodegenConfig) -> Self {
        Self { config }
    }

    /// Generate all enabled code components
    pub async fn generate_all(&self, abi: &EthereumAbi) -> Result<()> {
        let output_dir = Path::new(&self.config.output_dir);
        
        if !self.config.dry_run {
            tokio::fs::create_dir_all(output_dir).await
                .map_err(|e| indexer_core::Error::Config(format!("Failed to create output directory: {}", e)))?;
        }

        for feature in &self.config.features {
            match feature.as_str() {
                "client" => self.generate_client_code(abi, output_dir).await?,
                "storage" => self.generate_storage_code(abi, output_dir).await?,
                "api" => self.generate_api_code(abi, output_dir).await?,
                "migrations" => self.generate_migration_code(abi, output_dir).await?,
                _ => {
                    println!("Warning: Unknown feature '{}'", feature);
                }
            }
        }

        Ok(())
    }

    /// Generate client interaction code
    async fn generate_client_code(&self, abi: &EthereumAbi, output_dir: &Path) -> Result<()> {
        println!("Generating client code for contract: {}", self.config.contract_address);
        
        let client_dir = output_dir.join("client");
        if !self.config.dry_run {
            tokio::fs::create_dir_all(&client_dir).await
                .map_err(|e| indexer_core::Error::Config(format!("Failed to create client directory: {}", e)))?;
        }

        // Generate client module
        let client_code = self.generate_client_module(abi)?;
        self.write_file(&client_dir.join("mod.rs"), &client_code).await?;

        // Generate view functions
        let view_functions = abi.functions.iter().filter(|f| f.state_mutability == "view" || f.state_mutability == "pure").collect::<Vec<_>>();
        if !view_functions.is_empty() {
            let view_code = self.generate_view_methods(&view_functions)?;
            self.write_file(&client_dir.join("view.rs"), &view_code).await?;
        }

        // Generate transaction functions
        let tx_functions = abi.functions.iter().filter(|f| f.state_mutability == "nonpayable" || f.state_mutability == "payable").collect::<Vec<_>>();
        if !tx_functions.is_empty() {
            let tx_code = self.generate_transaction_methods(&tx_functions)?;
            self.write_file(&client_dir.join("transactions.rs"), &tx_code).await?;
        }

        // Generate deployment method
        if let Some(constructor) = &abi.constructor {
            let deploy_code = self.generate_deployment_method(Some(constructor))?;
            self.write_file(&client_dir.join("deploy.rs"), &deploy_code).await?;
        } else {
            let deploy_code = self.generate_deployment_method(None)?;
            self.write_file(&client_dir.join("deploy.rs"), &deploy_code).await?;
        }

        // Generate types
        let types_code = self.generate_types(abi)?;
        self.write_file(&client_dir.join("types.rs"), &types_code).await?;

        // Generate ABI helpers
        let abi_helpers_code = self.generate_abi_helpers(abi)?;
        self.write_file(&client_dir.join("abi_helpers.rs"), &abi_helpers_code).await?;

        // Generate event methods if events exist
        if !abi.events.is_empty() {
            let event_methods_code = self.generate_event_methods(&abi.events)?;
            self.write_file(&client_dir.join("events.rs"), &event_methods_code).await?;
        }

        Ok(())
    }

    /// Generate storage models and database schemas
    async fn generate_storage_code(&self, abi: &EthereumAbi, output_dir: &Path) -> Result<()> {
        println!("Generating storage code for contract: {}", self.config.contract_address);
        
        let storage_dir = output_dir.join("storage");
        if !self.config.dry_run {
            tokio::fs::create_dir_all(&storage_dir).await
                .map_err(|e| indexer_core::Error::Config(format!("Failed to create storage directory: {}", e)))?;
        }

        // Generate storage module file
        let storage_module = self.generate_storage_module(abi)?;
        self.write_file(&storage_dir.join("mod.rs"), &storage_module).await?;

        // Generate PostgreSQL schema
        let postgres_schema = self.generate_postgres_schema(abi)?;
        self.write_file(&storage_dir.join("postgres_schema.sql"), &postgres_schema).await?;

        // Generate RocksDB schemas
        let rocksdb_code = self.generate_rocksdb_schemas(abi)?;
        self.write_file(&storage_dir.join("rocksdb.rs"), &rocksdb_code).await?;

        // Generate storage traits
        let storage_traits = self.generate_storage_traits(abi)?;
        self.write_file(&storage_dir.join("traits.rs"), &storage_traits).await?;

        Ok(())
    }

    /// Generate API endpoints
    async fn generate_api_code(&self, abi: &EthereumAbi, output_dir: &Path) -> Result<()> {
        println!("Generating API code for contract: {}", self.config.contract_address);
        
        let api_dir = output_dir.join("api");
        if !self.config.dry_run {
            tokio::fs::create_dir_all(&api_dir).await
                .map_err(|e| indexer_core::Error::Config(format!("Failed to create api directory: {}", e)))?;
        }

        // Generate REST endpoints
        let rest_code = self.generate_rest_endpoints(abi)?;
        self.write_file(&api_dir.join("rest.rs"), &rest_code).await?;

        // Generate GraphQL schema
        let graphql_code = self.generate_graphql_schema(abi)?;
        self.write_file(&api_dir.join("graphql.rs"), &graphql_code).await?;

        // Generate WebSocket handlers
        let websocket_code = self.generate_websocket_handlers(abi)?;
        self.write_file(&api_dir.join("websocket.rs"), &websocket_code).await?;

        // Generate OpenAPI documentation
        let openapi_code = self.generate_openapi_documentation(abi)?;
        self.write_file(&api_dir.join("openapi.rs"), &openapi_code).await?;

        // Generate authentication and rate limiting
        let auth_code = self.generate_auth_and_rate_limiting(abi)?;
        self.write_file(&api_dir.join("auth.rs"), &auth_code).await?;

        Ok(())
    }

    /// Generate database migrations
    async fn generate_migration_code(&self, abi: &EthereumAbi, output_dir: &Path) -> Result<()> {
        println!("Generating migration code for contract: {}", self.config.contract_address);
        
        let migrations_dir = output_dir.join("migrations");
        if !self.config.dry_run {
            tokio::fs::create_dir_all(&migrations_dir).await
                .map_err(|e| indexer_core::Error::Config(format!("Failed to create migrations directory: {}", e)))?;
        }

        // Generate migration file
        let migration_sql = self.generate_migration_sql(abi)?;
        let timestamp = chrono::Utc::now().format("%Y%m%d%H%M%S");
        let filename = format!("{}_{}_contract.sql", timestamp, self.sanitize_contract_name());
        self.write_file(&migrations_dir.join(filename), &migration_sql).await?;

        Ok(())
    }

    // Helper methods for generating specific code components

    fn generate_storage_module(&self, _abi: &EthereumAbi) -> Result<String> {
        Ok(format!(
            r#"//! Generated storage module for contract: {}
//! Chain: {}

pub mod postgres_schema;
pub mod rocksdb;
pub mod traits;

use indexer_core::Result;

/// Storage layer for the {} contract
pub struct {}Storage {{
    // Storage implementation details would be added here
}}

impl {}Storage {{
    /// Create a new storage instance
    pub fn new() -> Self {{
        Self {{
            // Initialize storage
        }}
    }}
}}
"#,
            self.config.contract_address,
            self.config.chain_id,
            self.config.contract_address,
            self.sanitize_contract_name(),
            self.sanitize_contract_name()
        ))
    }

    fn generate_client_module(&self, _abi: &EthereumAbi) -> Result<String> {
        Ok(format!(
            r#"//! Generated client code for contract: {}
//! Chain: {}

pub mod view;
pub mod transactions;
pub mod deploy;
pub mod types;
pub mod abi_helpers;
pub mod events;

use indexer_core::Result;
use indexer_ethereum::EthereumClient;

/// Client for interacting with the {} contract
pub struct {}Client {{
    ethereum_client: EthereumClient,
    contract_address: String,
}}

impl {}Client {{
    /// Create a new client instance
    pub fn new(ethereum_client: EthereumClient, contract_address: String) -> Self {{
        Self {{
            ethereum_client,
            contract_address,
        }}
    }}

    /// Get the contract address
    pub fn contract_address(&self) -> &str {{
        &self.contract_address
    }}

    /// Get the underlying ethereum client
    pub fn ethereum_client(&self) -> &EthereumClient {{
        &self.ethereum_client
    }}
}}
"#,
            self.config.contract_address,
            self.config.chain_id,
            self.config.contract_address,
            self.sanitize_contract_name(),
            self.sanitize_contract_name()
        ))
    }

    fn generate_view_methods(&self, functions: &[&super::parser::AbiFunction]) -> Result<String> {
        let mut methods = String::new();
        let client_name = self.sanitize_contract_name();

        // Generate header
        methods.push_str(&format!(
            r#"//! Generated view methods for contract: {}

use super::types::*;
use super::{}Client;
use indexer_core::Result;
use alloy_primitives::{{Address, U256, Bytes}};

impl {}Client {{
"#,
            self.config.contract_address,
            client_name,
            client_name
        ));

        // Generate methods for each view function
        for function in functions {
            let method_name = &function.name;
            
            // Generate parameters string
            let mut params = Vec::new();
            for (i, input) in function.inputs.iter().enumerate() {
                let param_name = if input.name.is_empty() {
                    format!("param_{}", i)
                } else {
                    input.name.clone()
                };
                let rust_type = self.convert_abi_type_to_rust(&input.param_type);
                params.push(format!("{}: {}", param_name, rust_type));
            }
            let params_str = if params.is_empty() {
                String::new()
            } else {
                format!(", {}", params.join(", "))
            };

            // Generate return type
            let return_type = if function.outputs.is_empty() {
                "()".to_string()
            } else if function.outputs.len() == 1 {
                self.convert_abi_type_to_rust(&function.outputs[0].param_type)
            } else {
                // Multiple outputs - create tuple
                let output_types: Vec<String> = function.outputs.iter()
                    .map(|output| self.convert_abi_type_to_rust(&output.param_type))
                    .collect();
                format!("({})", output_types.join(", "))
            };

            // Generate method documentation
            methods.push_str(&format!("    /// Call view function: {}\n", method_name));
            if let Some(ref sig) = function.signature {
                methods.push_str(&format!("    /// Function selector: {}\n", sig));
            }

            // Generate method signature and body
            methods.push_str(&format!(
                r#"    pub async fn {}(&self{}) -> Result<{}> {{
        // Encode function call
        let call_data = self.encode_function_call("{}", &[
"#,
                method_name, params_str, return_type, method_name
            ));

            // Add parameters to function call
            for (i, input) in function.inputs.iter().enumerate() {
                let param_name = if input.name.is_empty() {
                    format!("param_{}", i)
                } else {
                    input.name.clone()
                };
                methods.push_str(&format!("            {},\n", param_name));
            }

            methods.push_str(&format!(
                r#"        ])?;

        // Execute view call using ethereum client
        // TODO: Implement actual ethereum contract view call
        // This would use self.ethereum_client().call(&self.contract_address, &call_data).await
        todo!("Implement ethereum contract view call for {}")
    }}

"#,
                method_name
            ));
        }

        methods.push_str("}\n");
        Ok(methods)
    }

    /// Convert ABI type to Rust type
    fn convert_abi_type_to_rust(&self, abi_type: &str) -> String {
        Self::convert_abi_type_to_rust_static(abi_type)
    }

    /// Convert ABI type to Rust type (static implementation)
    fn convert_abi_type_to_rust_static(abi_type: &str) -> String {
        // Handle array types first
        if abi_type.ends_with("[]") {
            let base_type = &abi_type[..abi_type.len() - 2];
            if let Some(bracket_pos) = base_type.rfind('[') {
                // Fixed-size array like uint256[4][]
                let inner_base = &base_type[..bracket_pos];
                let size = &base_type[bracket_pos + 1..base_type.len() - 1];
                format!("Vec<[{}; {}]>", Self::convert_abi_type_to_rust_static(inner_base), size)
            } else {
                // Dynamic array like uint256[]
                format!("Vec<{}>", Self::convert_abi_type_to_rust_static(base_type))
            }
        }
        // Handle fixed-size arrays
        else if let Some(bracket_pos) = abi_type.rfind('[') {
            let base_type = &abi_type[..bracket_pos];
            let size = &abi_type[bracket_pos + 1..abi_type.len() - 1];
            format!("[{}; {}]", Self::convert_abi_type_to_rust_static(base_type), size)
        }
        // Handle basic types
        else {
            match abi_type {
                "bool" => "bool".to_string(),
                "address" => "Address".to_string(),
                "string" => "String".to_string(),
                "bytes" => "Bytes".to_string(),
                _ if abi_type.starts_with("uint") => {
                    // All uint types mapped to U256 for simplicity
                    "U256".to_string()
                }
                _ if abi_type.starts_with("int") => {
                    // All int types mapped to U256 for simplicity (when available)
                    "U256".to_string() // For now, use U256
                }
                _ if abi_type.starts_with("bytes") && abi_type.len() > 5 => {
                    // Fixed-size bytes
                    format!("[u8; {}]", &abi_type[5..])
                }
                _ if abi_type.starts_with("tuple") => {
                    // Tuple type - for now just use generic Value
                    "serde_json::Value".to_string()
                }
                _ => {
                    // Unknown type, use generic
                    "serde_json::Value".to_string()
                }
            }
        }
    }

    fn generate_transaction_methods(&self, functions: &[&super::parser::AbiFunction]) -> Result<String> {
        let mut methods = String::new();
        let client_name = self.sanitize_contract_name();

        // Generate header
        methods.push_str(&format!(
            r#"//! Generated transaction methods for contract: {}

use super::types::*;
use super::{}Client;
use indexer_core::Result;
use alloy_primitives::{{Address, U256, Bytes}};

impl {}Client {{
"#,
            self.config.contract_address,
            client_name,
            client_name
        ));

        // Generate methods for each transaction function
        for function in functions {
            let method_name = &function.name;
            
            // Generate parameters string
            let mut params = Vec::new();
            for (i, input) in function.inputs.iter().enumerate() {
                let param_name = if input.name.is_empty() {
                    format!("param_{}", i)
                } else {
                    input.name.clone()
                };
                let rust_type = self.convert_abi_type_to_rust(&input.param_type);
                params.push(format!("{}: {}", param_name, rust_type));
            }
            
            // Add value parameter for payable functions
            if function.payable {
                params.push("value: U256".to_string());
            }
            
            let params_str = if params.is_empty() {
                String::new()
            } else {
                format!(", {}", params.join(", "))
            };

            // Generate method documentation
            methods.push_str(&format!("    /// Call transaction function: {}\n", method_name));
            if let Some(ref sig) = function.signature {
                methods.push_str(&format!("    /// Function selector: {}\n", sig));
            }
            if function.payable {
                methods.push_str("    /// This function is payable and can receive ETH\n");
            }

            // Generate method signature and body
            methods.push_str(&format!(
                r#"    pub async fn {}(&self{}) -> Result<String> {{
        // Encode function call
        let call_data = self.encode_function_call("{}", &[
"#,
                method_name, params_str, method_name
            ));

            // Add parameters to function call (excluding value parameter)
            for (i, input) in function.inputs.iter().enumerate() {
                let param_name = if input.name.is_empty() {
                    format!("param_{}", i)
                } else {
                    input.name.clone()
                };
                methods.push_str(&format!("            {},\n", param_name));
            }

            methods.push_str("        ])?;\n\n");

            if function.payable {
                methods.push_str(&format!(
                    r#"        // Execute transaction with value using ethereum client
        // TODO: Implement actual ethereum contract transaction
        // This would use self.ethereum_client().send_transaction(&self.contract_address, &call_data, value).await
        todo!("Implement ethereum contract transaction for {} (payable)")
    }}

"#,
                    method_name
                ));
            } else {
                methods.push_str(&format!(
                    r#"        // Execute transaction using ethereum client
        // TODO: Implement actual ethereum contract transaction
        // This would use self.ethereum_client().send_transaction(&self.contract_address, &call_data, U256::ZERO).await
        todo!("Implement ethereum contract transaction for {}")
    }}

"#,
                    method_name
                ));
            }
        }

        methods.push_str("}\n");
        Ok(methods)
    }

    fn generate_deployment_method(&self, constructor: Option<&super::parser::AbiFunction>) -> Result<String> {
        let client_name = self.sanitize_contract_name();
        let mut method = String::new();

        // Generate header
        method.push_str(&format!(
            r#"//! Generated deployment method for contract: {}

use super::types::*;
use super::{}Client;
use indexer_core::Result;
use alloy_primitives::{{Address, U256, Bytes}};

impl {}Client {{
"#,
            self.config.contract_address,
            client_name,
            client_name
        ));

        if let Some(constructor) = constructor {
            // Generate parameters string
            let mut params = Vec::new();
            for (i, input) in constructor.inputs.iter().enumerate() {
                let param_name = if input.name.is_empty() {
                    format!("param_{}", i)
                } else {
                    input.name.clone()
                };
                let rust_type = self.convert_abi_type_to_rust(&input.param_type);
                params.push(format!("{}: {}", param_name, rust_type));
            }
            
            // Add standard deployment parameters
            params.push("bytecode: Bytes".to_string());
            if constructor.payable {
                params.push("value: U256".to_string());
            }
            
            let params_str = params.join(", ");

            // Generate method documentation
            method.push_str("    /// Deploy a new contract instance\n");
            if constructor.payable {
                method.push_str("    /// This constructor is payable and can receive ETH\n");
            }

            // Generate method signature and body
            method.push_str(&format!(
                r#"    pub async fn deploy(&self, {}) -> Result<Address> {{
        // Encode constructor arguments
        let constructor_args = self.encode_constructor_args(&[
"#,
                params_str
            ));

            // Add parameters to constructor call (excluding bytecode and value)
            for (i, input) in constructor.inputs.iter().enumerate() {
                let param_name = if input.name.is_empty() {
                    format!("param_{}", i)
                } else {
                    input.name.clone()
                };
                method.push_str(&format!("            {},\n", param_name));
            }

            method.push_str("        ])?;\n\n");
            method.push_str("        // Combine bytecode with constructor arguments\n");
            method.push_str("        let mut deployment_data = bytecode.to_vec();\n");
            method.push_str("        deployment_data.extend_from_slice(&constructor_args);\n\n");

            if constructor.payable {
                method.push_str(r#"        // Deploy contract with value using ethereum client
        // TODO: Implement actual ethereum contract deployment
        // This would use self.ethereum_client().deploy_contract(&deployment_data, value).await
        todo!("Implement ethereum contract deployment (payable)")
    }
"#);
            } else {
                method.push_str(r#"        // Deploy contract using ethereum client
        // TODO: Implement actual ethereum contract deployment
        // This would use self.ethereum_client().deploy_contract(&deployment_data, U256::ZERO).await
        todo!("Implement ethereum contract deployment")
    }
"#);
            }
        } else {
            // No constructor - simple deployment
            method.push_str(r#"    /// Deploy a new contract instance (no constructor)
    pub async fn deploy(&self, bytecode: Bytes) -> Result<Address> {
        // Deploy contract using ethereum client
        // TODO: Implement actual ethereum contract deployment
        // This would use self.ethereum_client().deploy_contract(&bytecode, U256::ZERO).await
        todo!("Implement ethereum contract deployment")
    }
"#);
        }

        method.push_str("\n}\n");
        Ok(method)
    }

    fn generate_types(&self, _abi: &EthereumAbi) -> Result<String> {
        Ok(format!(
            r#"//! Generated types for contract: {}

use serde::{{Deserialize, Serialize}};
use alloy_primitives::{{Address, U256, Bytes}};

// Re-export alloy types that might be used
pub use alloy_primitives::*;

// Event types will be generated here based on ABI event definitions
// TODO: Implement type generation from ABI

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaceholderType {{
    // Placeholder - will be replaced with actual generated types
}}

/// Contract execution errors
#[derive(Debug, thiserror::Error)]
pub enum ContractError {{
    #[error("Serialization error: {{0}}")]
    Serialization(#[from] serde_json::Error),
    
    #[error("Core indexer error: {{0}}")]
    Core(#[from] indexer_core::Error),
    
    #[error("Contract call failed: {{0}}")]
    CallFailed(String),
    
    #[error("Contract transaction failed: {{0}}")]
    TransactionFailed(String),
    
    #[error("Contract deployment failed: {{0}}")]
    DeploymentFailed(String),
    
    #[error("ABI encoding failed: {{0}}")]
    AbiEncodingFailed(String),
    
    #[error("ABI decoding failed: {{0}}")]
    AbiDecodingFailed(String),
    
    #[error("Event parsing failed: {{0}}")]
    EventParsingFailed(String),
    
    #[error("Gas estimation failed: {{0}}")]
    GasEstimationFailed(String),
    
    #[error("Invalid contract address: {{0}}")]
    InvalidAddress(String),
}}

impl From<ContractError> for indexer_core::Error {{
    fn from(err: ContractError) -> Self {{
        indexer_core::Error::generic(err.to_string())
    }}
}}

/// ABI encoding/decoding utilities
pub mod abi_types {{
    use super::*;
    use indexer_core::Result;

    /// Encode ABI parameters to bytes
    pub fn encode_abi_params(types: &[&str], values: &[serde_json::Value]) -> Result<Vec<u8>> {{
        // TODO: Implement actual ABI encoding
        // This would use ethabi or alloy to encode parameters according to ABI spec
        todo!("Implement ABI parameter encoding")
    }}

    /// Decode ABI parameters from bytes
    pub fn decode_abi_params(types: &[&str], data: &[u8]) -> Result<Vec<serde_json::Value>> {{
        // TODO: Implement actual ABI decoding
        // This would use ethabi or alloy to decode parameters according to ABI spec
        todo!("Implement ABI parameter decoding")
    }}

    /// Convert Rust value to ABI-compatible JSON value
    pub fn to_abi_value<T: Serialize>(value: T) -> Result<serde_json::Value> {{
        serde_json::to_value(value)
            .map_err(|e| indexer_core::Error::Serialization(format!("Failed to convert to ABI value: {{}}", e)))
    }}

    /// Convert ABI JSON value to Rust type
    pub fn from_abi_value<T: for<'de> Deserialize<'de>>(value: serde_json::Value) -> Result<T> {{
        serde_json::from_value(value)
            .map_err(|e| indexer_core::Error::Serialization(format!("Failed to convert from ABI value: {{}}", e)))
    }}
}}

/// Gas estimation utilities
pub mod gas_utils {{
    use super::*;
    use indexer_core::Result;

    /// Estimate gas with safety margin
    pub fn add_gas_margin(gas_estimate: U256, margin_percent: u32) -> U256 {{
        let margin = gas_estimate * U256::from(margin_percent) / U256::from(100);
        gas_estimate + margin
    }}

    /// Convert gas price from gwei to wei
    pub fn gwei_to_wei(gwei: u64) -> U256 {{
        U256::from(gwei) * U256::from(1_000_000_000u64)
    }}

    /// Convert wei to gwei
    pub fn wei_to_gwei(wei: U256) -> u64 {{
        (wei / U256::from(1_000_000_000u64)).try_into().unwrap_or(0)
    }}
}}
"#,
            self.config.contract_address
        ))
    }

    fn generate_postgres_schema(&self, abi: &EthereumAbi) -> Result<String> {
        let contract_name = self.sanitize_contract_name();
        let mut sql = String::new();

        sql.push_str(&format!(
            r#"-- Generated PostgreSQL schema for contract: {}
-- Chain: {}
-- Generated at: {}

-- Extension for JSON operations
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

-- Main contract state table
CREATE TABLE IF NOT EXISTS {}_state (
    id BIGSERIAL PRIMARY KEY,
    contract_address TEXT NOT NULL,
    block_number BIGINT NOT NULL,
    block_timestamp TIMESTAMPTZ NOT NULL,
    transaction_hash TEXT NOT NULL,
    state_data JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Contract transactions table
CREATE TABLE IF NOT EXISTS {}_transactions (
    id BIGSERIAL PRIMARY KEY,
    transaction_hash TEXT NOT NULL UNIQUE,
    contract_address TEXT NOT NULL,
    block_number BIGINT NOT NULL,
    block_timestamp TIMESTAMPTZ NOT NULL,
    from_address TEXT NOT NULL,
    to_address TEXT,
    value NUMERIC(78, 0) NOT NULL DEFAULT 0,
    function_name TEXT,
    function_data JSONB,
    gas_used BIGINT,
    gas_price NUMERIC(78, 0),
    status INTEGER NOT NULL,
    error_message TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Contract events table
CREATE TABLE IF NOT EXISTS {}_events (
    id BIGSERIAL PRIMARY KEY,
    transaction_hash TEXT NOT NULL,
    contract_address TEXT NOT NULL,
    block_number BIGINT NOT NULL,
    block_timestamp TIMESTAMPTZ NOT NULL,
    event_name TEXT NOT NULL,
    event_data JSONB NOT NULL,
    log_index INTEGER NOT NULL,
    transaction_index INTEGER NOT NULL,
    removed BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Contract deployments table
CREATE TABLE IF NOT EXISTS {}_deployments (
    id BIGSERIAL PRIMARY KEY,
    contract_address TEXT NOT NULL UNIQUE,
    deployer_address TEXT NOT NULL,
    constructor_args JSONB,
    bytecode TEXT NOT NULL,
    block_number BIGINT NOT NULL,
    block_timestamp TIMESTAMPTZ NOT NULL,
    transaction_hash TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

"#,
            self.config.contract_address,
            self.config.chain_id,
            chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC"),
            contract_name.to_lowercase(),
            contract_name.to_lowercase(),
            contract_name.to_lowercase(),
            contract_name.to_lowercase()
        ));

        // Generate function-specific tables for each ABI function
        for function in &abi.functions {
            sql.push_str(&format!(
                r#"-- Function-specific table for {} calls
CREATE TABLE IF NOT EXISTS {}_{}_calls (
    id BIGSERIAL PRIMARY KEY,
    transaction_hash TEXT NOT NULL,
    contract_address TEXT NOT NULL,
    block_number BIGINT NOT NULL,
    block_timestamp TIMESTAMPTZ NOT NULL,
    from_address TEXT NOT NULL,
    function_name TEXT NOT NULL DEFAULT '{}',
"#,
                function.name,
                contract_name.to_lowercase(),
                function.name.to_lowercase(),
                function.name
            ));

            // Add columns for each function input
            for input in &function.inputs {
                let param_name = if input.name.is_empty() {
                    format!("param_{}", input.name)
                } else {
                    input.name.clone()
                };
                let sql_type = self.convert_abi_type_to_postgres(&input.param_type);
                sql.push_str(&format!("    {} {},\n", param_name, sql_type));
            }

            sql.push_str(r#"    gas_used BIGINT,
    gas_price NUMERIC(78, 0),
    status INTEGER NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

"#);
        }

        // Generate event-specific tables
        for event in &abi.events {
            sql.push_str(&format!(
                r#"-- Event-specific table for {} events
CREATE TABLE IF NOT EXISTS {}_{}_events (
    id BIGSERIAL PRIMARY KEY,
    transaction_hash TEXT NOT NULL,
    contract_address TEXT NOT NULL,
    block_number BIGINT NOT NULL,
    block_timestamp TIMESTAMPTZ NOT NULL,
    log_index INTEGER NOT NULL,
    transaction_index INTEGER NOT NULL,
"#,
                event.name,
                contract_name.to_lowercase(),
                event.name.to_lowercase()
            ));

            // Add columns for each event input
            for input in &event.inputs {
                let param_name = if input.name.is_empty() {
                    format!("param_{}", input.name)
                } else {
                    input.name.clone()
                };
                let sql_type = self.convert_abi_type_to_postgres(&input.param_type);
                sql.push_str(&format!("    {} {},\n", param_name, sql_type));
            }

            sql.push_str(r#"    removed BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

"#);
        }

        // Add indexes for performance
        let contract_lower = contract_name.to_lowercase();
        
        sql.push_str("\n-- Indexes for performance\n");
        
        // State table indexes
        sql.push_str(&format!("CREATE INDEX IF NOT EXISTS idx_{}_state_contract ON {}_state(contract_address);\n", contract_lower, contract_lower));
        sql.push_str(&format!("CREATE INDEX IF NOT EXISTS idx_{}_state_block ON {}_state(block_number);\n", contract_lower, contract_lower));
        sql.push_str(&format!("CREATE INDEX IF NOT EXISTS idx_{}_state_timestamp ON {}_state(block_timestamp);\n", contract_lower, contract_lower));
        
        // Transaction table indexes
        sql.push_str(&format!("CREATE INDEX IF NOT EXISTS idx_{}_tx_hash ON {}_transactions(transaction_hash);\n", contract_lower, contract_lower));
        sql.push_str(&format!("CREATE INDEX IF NOT EXISTS idx_{}_tx_contract ON {}_transactions(contract_address);\n", contract_lower, contract_lower));
        sql.push_str(&format!("CREATE INDEX IF NOT EXISTS idx_{}_tx_block ON {}_transactions(block_number);\n", contract_lower, contract_lower));
        sql.push_str(&format!("CREATE INDEX IF NOT EXISTS idx_{}_tx_from ON {}_transactions(from_address);\n", contract_lower, contract_lower));
        sql.push_str(&format!("CREATE INDEX IF NOT EXISTS idx_{}_tx_function ON {}_transactions(function_name);\n", contract_lower, contract_lower));
        
        // Event table indexes
        sql.push_str(&format!("CREATE INDEX IF NOT EXISTS idx_{}_events_tx ON {}_events(transaction_hash);\n", contract_lower, contract_lower));
        sql.push_str(&format!("CREATE INDEX IF NOT EXISTS idx_{}_events_contract ON {}_events(contract_address);\n", contract_lower, contract_lower));
        sql.push_str(&format!("CREATE INDEX IF NOT EXISTS idx_{}_events_block ON {}_events(block_number);\n", contract_lower, contract_lower));
        sql.push_str(&format!("CREATE INDEX IF NOT EXISTS idx_{}_events_name ON {}_events(event_name);\n", contract_lower, contract_lower));
        
        // Deployment table indexes
        sql.push_str(&format!("CREATE INDEX IF NOT EXISTS idx_{}_deploy_contract ON {}_deployments(contract_address);\n", contract_lower, contract_lower));
        sql.push_str(&format!("CREATE INDEX IF NOT EXISTS idx_{}_deploy_deployer ON {}_deployments(deployer_address);\n", contract_lower, contract_lower));
        sql.push_str(&format!("CREATE INDEX IF NOT EXISTS idx_{}_deploy_block ON {}_deployments(block_number);\n", contract_lower, contract_lower));

        // Function-specific indexes
        for function in &abi.functions {
            let function_lower = function.name.to_lowercase();
            sql.push_str(&format!("CREATE INDEX IF NOT EXISTS idx_{}_{}_calls_contract ON {}_{}_calls(contract_address);\n", contract_lower, function_lower, contract_lower, function_lower));
            sql.push_str(&format!("CREATE INDEX IF NOT EXISTS idx_{}_{}_calls_block ON {}_{}_calls(block_number);\n", contract_lower, function_lower, contract_lower, function_lower));
            sql.push_str(&format!("CREATE INDEX IF NOT EXISTS idx_{}_{}_calls_tx ON {}_{}_calls(transaction_hash);\n", contract_lower, function_lower, contract_lower, function_lower));
        }

        // Event-specific indexes
        for event in &abi.events {
            let event_lower = event.name.to_lowercase();
            sql.push_str(&format!("CREATE INDEX IF NOT EXISTS idx_{}_{}_events_contract ON {}_{}_events(contract_address);\n", contract_lower, event_lower, contract_lower, event_lower));
            sql.push_str(&format!("CREATE INDEX IF NOT EXISTS idx_{}_{}_events_block ON {}_{}_events(block_number);\n", contract_lower, event_lower, contract_lower, event_lower));
            sql.push_str(&format!("CREATE INDEX IF NOT EXISTS idx_{}_{}_events_tx ON {}_{}_events(transaction_hash);\n", contract_lower, event_lower, contract_lower, event_lower));
        }

        Ok(sql)
    }

    /// Convert ABI type to PostgreSQL type
    fn convert_abi_type_to_postgres(&self, abi_type: &str) -> String {
        match abi_type {
            "bool" => "BOOLEAN".to_string(),
            "address" => "TEXT".to_string(),
            "string" => "TEXT".to_string(),
            "bytes" => "BYTEA".to_string(),
            _ if abi_type.starts_with("uint") => "NUMERIC(78, 0)".to_string(), // Large enough for uint256
            _ if abi_type.starts_with("int") => "NUMERIC(78, 0)".to_string(),
            _ if abi_type.starts_with("bytes") && abi_type.len() > 5 => "BYTEA".to_string(),
            _ if abi_type.ends_with("[]") => "JSONB".to_string(), // Arrays as JSON
            _ if abi_type.contains('[') && abi_type.ends_with(']') => "JSONB".to_string(), // Fixed arrays as JSON
            _ if abi_type.starts_with("tuple") => "JSONB".to_string(),
            _ => "JSONB".to_string(), // Default to JSON for complex types
        }
    }

    fn generate_rocksdb_schemas(&self, _abi: &EthereumAbi) -> Result<String> {
        let contract_name = self.sanitize_contract_name();
        
        Ok(format!(
            r#"//! Generated RocksDB schemas for contract: {}

use indexer_core::Result;
use rocksdb::{{DB, Options, ColumnFamily}};
use serde::{{Serialize, Deserialize}};
use std::path::Path;
use alloy_primitives::{{Address, U256, Bytes}};

/// RocksDB storage for {} contract
pub struct {}RocksDB {{
    db: DB,
}}

impl {}RocksDB {{
    /// Column families for organized data storage
    pub const CF_STATE: &'static str = "state";
    pub const CF_TRANSACTIONS: &'static str = "transactions";
    pub const CF_EVENTS: &'static str = "events";
    pub const CF_CALLS: &'static str = "calls";
    pub const CF_DEPLOYMENTS: &'static str = "deployments";
    pub const CF_METADATA: &'static str = "metadata";

    /// Initialize RocksDB with contract-specific column families
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {{
        let mut opts = Options::default();
        opts.create_if_missing(true);
        opts.create_missing_column_families(true);

        let column_families = vec![
            Self::CF_STATE,
            Self::CF_TRANSACTIONS,
            Self::CF_EVENTS,
            Self::CF_CALLS,
            Self::CF_DEPLOYMENTS,
            Self::CF_METADATA,
        ];

        let db = DB::open_cf(&opts, path, &column_families)
            .map_err(|e| indexer_core::Error::Storage(format!("Failed to open RocksDB: {{}}", e)))?;

        Ok(Self {{ db }})
    }}

    /// Store contract state at specific block
    pub fn store_state(&self, contract_address: &Address, block_number: u64, state: &serde_json::Value) -> Result<()> {{
        let cf = self.db.cf_handle(Self::CF_STATE)
            .ok_or_else(|| indexer_core::Error::Storage("State column family not found".to_string()))?;

        let key = format!("{{}}:{{:020}}", contract_address, block_number);
        let value = serde_json::to_vec(state)
            .map_err(|e| indexer_core::Error::Serialization(format!("Failed to serialize state: {{}}", e)))?;

        self.db.put_cf(cf, key.as_bytes(), &value)
            .map_err(|e| indexer_core::Error::Storage(format!("Failed to store state: {{}}", e)))?;

        // Update latest state pointer
        let latest_key = format!("{{}}_latest", contract_address);
        self.db.put_cf(cf, latest_key.as_bytes(), &value)
            .map_err(|e| indexer_core::Error::Storage(format!("Failed to update latest state: {{}}", e)))?;

        Ok(())
    }}

    /// Get contract state at specific block (or latest)
    pub fn get_state(&self, contract_address: &Address, block_number: Option<u64>) -> Result<Option<serde_json::Value>> {{
        let cf = self.db.cf_handle(Self::CF_STATE)
            .ok_or_else(|| indexer_core::Error::Storage("State column family not found".to_string()))?;

        let key = match block_number {{
            Some(height) => format!("{{}}:{{:020}}", contract_address, height),
            None => format!("{{}}_latest", contract_address),
        }};

        match self.db.get_cf(cf, key.as_bytes()) {{
            Ok(Some(data)) => {{
                let state = serde_json::from_slice(&data)
                    .map_err(|e| indexer_core::Error::Serialization(format!("Failed to deserialize state: {{}}", e)))?;
                Ok(Some(state))
            }},
            Ok(None) => Ok(None),
            Err(e) => Err(indexer_core::Error::Storage(format!("Failed to get state: {{}}", e)))
        }}
    }}

    /// Store transaction data
    pub fn store_transaction(&self, tx_hash: &str, tx_data: &TransactionData) -> Result<()> {{
        let cf = self.db.cf_handle(Self::CF_TRANSACTIONS)
            .ok_or_else(|| indexer_core::Error::Storage("Transactions column family not found".to_string()))?;

        let value = serde_json::to_vec(tx_data)
            .map_err(|e| indexer_core::Error::Serialization(format!("Failed to serialize transaction: {{}}", e)))?;

        self.db.put_cf(cf, tx_hash.as_bytes(), &value)
            .map_err(|e| indexer_core::Error::Storage(format!("Failed to store transaction: {{}}", e)))?;

        // Create block number index
        let block_key = format!("block:{{:020}}:{{}}", tx_data.block_number, tx_hash);
        self.db.put_cf(cf, block_key.as_bytes(), tx_hash.as_bytes())
            .map_err(|e| indexer_core::Error::Storage(format!("Failed to store block index: {{}}", e)))?;

        // Create contract address index
        let contract_key = format!("contract:{{}}:{{}}", tx_data.contract_address, tx_hash);
        self.db.put_cf(cf, contract_key.as_bytes(), tx_hash.as_bytes())
            .map_err(|e| indexer_core::Error::Storage(format!("Failed to store contract index: {{}}", e)))?;

        Ok(())
    }}

    /// Store event data with multiple indexes
    pub fn store_event(&self, event_id: &str, event_data: &EventData) -> Result<()> {{
        let cf = self.db.cf_handle(Self::CF_EVENTS)
            .ok_or_else(|| indexer_core::Error::Storage("Events column family not found".to_string()))?;

        let value = serde_json::to_vec(event_data)
            .map_err(|e| indexer_core::Error::Serialization(format!("Failed to serialize event: {{}}", e)))?;

        // Primary key: event_id
        self.db.put_cf(cf, event_id.as_bytes(), &value)
            .map_err(|e| indexer_core::Error::Storage(format!("Failed to store event: {{}}", e)))?;

        // Secondary indexes for efficient queries
        let contract_key = format!("contract:{{}}:{{:020}}:{{}}", event_data.contract_address, event_data.block_number, event_id);
        self.db.put_cf(cf, contract_key.as_bytes(), event_id.as_bytes())
            .map_err(|e| indexer_core::Error::Storage(format!("Failed to store contract index: {{}}", e)))?;

        let block_key = format!("block:{{:020}}:{{}}", event_data.block_number, event_id);
        self.db.put_cf(cf, block_key.as_bytes(), event_id.as_bytes())
            .map_err(|e| indexer_core::Error::Storage(format!("Failed to store block index: {{}}", e)))?;

        let event_type_key = format!("event_type:{{}}:{{}}", event_data.event_name, event_id);
        self.db.put_cf(cf, event_type_key.as_bytes(), event_id.as_bytes())
            .map_err(|e| indexer_core::Error::Storage(format!("Failed to store event type index: {{}}", e)))?;

        Ok(())
    }}

    /// Store function call data
    pub fn store_call(&self, call_id: &str, call_data: &CallData) -> Result<()> {{
        let cf = self.db.cf_handle(Self::CF_CALLS)
            .ok_or_else(|| indexer_core::Error::Storage("Calls column family not found".to_string()))?;

        let value = serde_json::to_vec(call_data)
            .map_err(|e| indexer_core::Error::Serialization(format!("Failed to serialize call: {{}}", e)))?;

        self.db.put_cf(cf, call_id.as_bytes(), &value)
            .map_err(|e| indexer_core::Error::Storage(format!("Failed to store call: {{}}", e)))?;

        // Function name index
        let function_key = format!("function:{{}}:{{}}", call_data.function_name, call_id);
        self.db.put_cf(cf, function_key.as_bytes(), call_id.as_bytes())
            .map_err(|e| indexer_core::Error::Storage(format!("Failed to store function index: {{}}", e)))?;

        Ok(())
    }}

    /// Store contract deployment data
    pub fn store_deployment(&self, contract_address: &Address, deployment_data: &DeploymentData) -> Result<()> {{
        let cf = self.db.cf_handle(Self::CF_DEPLOYMENTS)
            .ok_or_else(|| indexer_core::Error::Storage("Deployments column family not found".to_string()))?;

        let value = serde_json::to_vec(deployment_data)
            .map_err(|e| indexer_core::Error::Serialization(format!("Failed to serialize deployment: {{}}", e)))?;

        self.db.put_cf(cf, contract_address.to_string().as_bytes(), &value)
            .map_err(|e| indexer_core::Error::Storage(format!("Failed to store deployment: {{}}", e)))?;

        // Deployer index
        let deployer_key = format!("deployer:{{}}:{{}}", deployment_data.deployer_address, contract_address);
        self.db.put_cf(cf, deployer_key.as_bytes(), contract_address.to_string().as_bytes())
            .map_err(|e| indexer_core::Error::Storage(format!("Failed to store deployer index: {{}}", e)))?;

        Ok(())
    }}

    /// Get events for a contract within block range
    pub fn get_contract_events(&self, contract_address: &Address, from_block: u64, to_block: u64) -> Result<Vec<EventData>> {{
        let cf = self.db.cf_handle(Self::CF_EVENTS)
            .ok_or_else(|| indexer_core::Error::Storage("Events column family not found".to_string()))?;

        let mut events = Vec::new();
        
        // Iterate through contract events in block range
        for block_num in from_block..=to_block {{
            let start_key = format!("contract:{{}}:{{:020}}:", contract_address, block_num);
            let end_key = format!("contract:{{}}:{{:020}}~", contract_address, block_num);

            let iterator = self.db.iterator_cf(cf, rocksdb::IteratorMode::From(start_key.as_bytes(), rocksdb::Direction::Forward));

            for item in iterator {{
                let (key, value) = item.map_err(|e| indexer_core::Error::Storage(format!("Iterator error: {{}}", e)))?;
                
                if key > end_key.as_bytes() {{
                    break;
                }}

                // Get the actual event data
                let event_id = String::from_utf8_lossy(&value);
                if let Ok(Some(event_data_bytes)) = self.db.get_cf(cf, event_id.as_bytes()) {{
                    if let Ok(event_data) = serde_json::from_slice::<EventData>(&event_data_bytes) {{
                        events.push(event_data);
                    }}
                }}
            }}
        }}

        // Sort by block number and log index
        events.sort_by(|a, b| {{
            a.block_number.cmp(&b.block_number)
                .then(a.log_index.cmp(&b.log_index))
        }});

        Ok(events)
    }}

    /// Get transactions for a contract within block range
    pub fn get_contract_transactions(&self, contract_address: &Address, from_block: u64, to_block: u64) -> Result<Vec<TransactionData>> {{
        let cf = self.db.cf_handle(Self::CF_TRANSACTIONS)
            .ok_or_else(|| indexer_core::Error::Storage("Transactions column family not found".to_string()))?;

        let mut transactions = Vec::new();
        let start_key = format!("contract:{{}}:", contract_address);
        let end_key = format!("contract:{{}}~", contract_address);

        let iterator = self.db.iterator_cf(cf, rocksdb::IteratorMode::From(start_key.as_bytes(), rocksdb::Direction::Forward));

        for item in iterator {{
            let (key, value) = item.map_err(|e| indexer_core::Error::Storage(format!("Iterator error: {{}}", e)))?;
            
            if !key.starts_with(start_key.as_bytes()) {{
                break;
            }}

            // Get the actual transaction data
            let tx_hash = String::from_utf8_lossy(&value);
            if let Ok(Some(tx_data_bytes)) = self.db.get_cf(cf, tx_hash.as_bytes()) {{
                if let Ok(tx_data) = serde_json::from_slice::<TransactionData>(&tx_data_bytes) {{
                    if tx_data.block_number >= from_block && tx_data.block_number <= to_block {{
                        transactions.push(tx_data);
                    }}
                }}
            }}
        }}

        Ok(transactions)
    }}
}}

/// Transaction data structure for RocksDB storage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionData {{
    pub hash: String,
    pub contract_address: Address,
    pub block_number: u64,
    pub block_timestamp: i64,
    pub from_address: Address,
    pub to_address: Option<Address>,
    pub value: U256,
    pub function_name: Option<String>,
    pub function_data: Option<serde_json::Value>,
    pub gas_used: Option<u64>,
    pub gas_price: Option<U256>,
    pub status: u32,
    pub error_message: Option<String>,
}}

/// Event data structure for RocksDB storage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventData {{
    pub transaction_hash: String,
    pub contract_address: Address,
    pub block_number: u64,
    pub block_timestamp: i64,
    pub event_name: String,
    pub event_data: serde_json::Value,
    pub log_index: u32,
    pub transaction_index: u32,
    pub removed: bool,
}}

/// Function call data structure for RocksDB storage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallData {{
    pub transaction_hash: String,
    pub contract_address: Address,
    pub block_number: u64,
    pub block_timestamp: i64,
    pub from_address: Address,
    pub function_name: String,
    pub input_data: serde_json::Value,
    pub output_data: Option<serde_json::Value>,
    pub gas_used: Option<u64>,
    pub gas_price: Option<U256>,
    pub status: u32,
}}

/// Contract deployment data structure for RocksDB storage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeploymentData {{
    pub contract_address: Address,
    pub deployer_address: Address,
    pub constructor_args: Option<serde_json::Value>,
    pub bytecode: Bytes,
    pub block_number: u64,
    pub block_timestamp: i64,
    pub transaction_hash: String,
}}
"#,
            self.config.contract_address,
            self.config.contract_address,
            contract_name,
            contract_name
        ))
    }

    fn generate_storage_traits(&self, _abi: &EthereumAbi) -> Result<String> {
        Ok(format!(
            r#"//! Generated storage traits for contract: {}

use indexer_core::Result;
use async_trait::async_trait;

#[async_trait]
pub trait {}Storage: Send + Sync {{
    // Storage trait methods will be generated based on contract ABI
    // TODO: Implement storage trait generation from ABI
}}
"#,
            self.config.contract_address,
            self.sanitize_contract_name()
        ))
    }

    fn generate_rest_endpoints(&self, abi: &EthereumAbi) -> Result<String> {
        let contract_name = self.sanitize_contract_name();
        let mut endpoints_code = String::new();

        endpoints_code.push_str(&format!(
            r#"//! Generated REST endpoints for contract: {}

use axum::{{
    extract::{{Path, Query, State}},
    http::StatusCode,
    response::Json,
    routing::{{get, post}},
    Router,
}};
use serde::{{Deserialize, Serialize}};
use serde_json::Value;
use std::collections::HashMap;
use indexer_core::Result;
use super::types::*;
use super::{}Client;

/// Create routes for {} contract API
pub fn {}_routes() -> Router<AppState> {{
    Router::new()
        .route("/contract/{}/info", get(get_contract_info))
        .route("/contract/{}/state", get(get_contract_state))
        .route("/contract/{}/transactions", get(get_contract_transactions))
        .route("/contract/{}/events", get(get_contract_events))
        .route("/contract/{}/logs", get(get_contract_logs))
"#,
            self.config.contract_address,
            contract_name,
            self.config.contract_address,
            contract_name.to_lowercase(),
            self.config.contract_address,
            self.config.contract_address,
            self.config.contract_address,
            self.config.contract_address,
            self.config.contract_address
        ));

        // Add view function endpoints
        for function in &abi.functions {
            if function.state_mutability == "view" || function.state_mutability == "pure" {
                endpoints_code.push_str(&format!(
                    r#"        .route("/contract/{}/call/{}", get(call_{}))
"#,
                    self.config.contract_address,
                    function.name.to_lowercase(),
                    function.name.to_lowercase()
                ));
            }
        }

        // Add transaction endpoints for state-changing functions
        for function in &abi.functions {
            if function.state_mutability == "nonpayable" || function.state_mutability == "payable" {
                endpoints_code.push_str(&format!(
                    r#"        .route("/contract/{}/execute/{}", post(execute_{}))
"#,
                    self.config.contract_address,
                    function.name.to_lowercase(),
                    function.name.to_lowercase()
                ));
            }
        }

        endpoints_code.push_str("}\n\n");

        // Generate request/response types
        endpoints_code.push_str(&format!(
            r#"/// Application state for API handlers
#[derive(Clone)]
pub struct AppState {{
    pub client: {}Client,
}}

/// Standard API response wrapper
#[derive(Debug, Serialize)]
pub struct ApiResponse<T> {{
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
    pub metadata: Option<ApiMetadata>,
}}

/// API response metadata
#[derive(Debug, Serialize)]
pub struct ApiMetadata {{
    pub block_number: Option<u64>,
    pub timestamp: Option<String>,
    pub gas_used: Option<u64>,
    pub transaction_hash: Option<String>,
}}

/// Query parameters for pagination
#[derive(Debug, Deserialize)]
pub struct PaginationQuery {{
    pub page: Option<u32>,
    pub limit: Option<u32>,
    pub from_block: Option<u64>,
    pub to_block: Option<u64>,
}}

/// Query parameters for event filtering
#[derive(Debug, Deserialize)]
pub struct EventFilterQuery {{
    pub event_name: Option<String>,
    pub from_block: Option<u64>,
    pub to_block: Option<u64>,
    pub topics: Option<Vec<String>>,
    pub limit: Option<u32>,
}}

"#,
            contract_name
        ));

        // Generate basic endpoint handlers
        endpoints_code.push_str(&format!(
            r#"/// Get contract information
pub async fn get_contract_info(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<ContractInfo>>, StatusCode> {{
    let contract_info = ContractInfo {{
        address: state.client.contract_address().to_string(),
        chain_id: "{}".to_string(),
        contract_type: "Ethereum".to_string(),
    }};

    Ok(Json(ApiResponse {{
        success: true,
        data: Some(contract_info),
        error: None,
        metadata: None,
    }}))
}}

/// Get current contract state
pub async fn get_contract_state(
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<ApiResponse<Value>>, StatusCode> {{
    let block_number = params.get("block_number")
        .and_then(|h| h.parse::<u64>().ok());

    // TODO: Implement actual state retrieval
    // This would use the storage layer to get contract state
    let placeholder_state = serde_json::json!({{
        "message": "Contract state retrieval not yet implemented",
        "block_number": block_number,
        "contract_address": state.client.contract_address()
    }});

    Ok(Json(ApiResponse {{
        success: true,
        data: Some(placeholder_state),
        error: None,
        metadata: Some(ApiMetadata {{
            block_number,
            timestamp: Some(chrono::Utc::now().to_rfc3339()),
            gas_used: None,
            transaction_hash: None,
        }}),
    }}))
}}

/// Get contract transactions
pub async fn get_contract_transactions(
    State(state): State<AppState>,
    Query(params): Query<PaginationQuery>,
) -> Result<Json<ApiResponse<Vec<TransactionData>>>, StatusCode> {{
    // TODO: Implement actual transaction retrieval
    let placeholder_transactions = Vec::<TransactionData>::new();

    Ok(Json(ApiResponse {{
        success: true,
        data: Some(placeholder_transactions),
        error: None,
        metadata: Some(ApiMetadata {{
            block_number: params.to_block,
            timestamp: Some(chrono::Utc::now().to_rfc3339()),
            gas_used: None,
            transaction_hash: None,
        }}),
    }}))
}}

/// Get contract events
pub async fn get_contract_events(
    State(state): State<AppState>,
    Query(params): Query<EventFilterQuery>,
) -> Result<Json<ApiResponse<Vec<EventData>>>, StatusCode> {{
    // TODO: Implement actual event retrieval
    let placeholder_events = Vec::<EventData>::new();

    Ok(Json(ApiResponse {{
        success: true,
        data: Some(placeholder_events),
        error: None,
        metadata: Some(ApiMetadata {{
            block_number: params.to_block,
            timestamp: Some(chrono::Utc::now().to_rfc3339()),
            gas_used: None,
            transaction_hash: None,
        }}),
    }}))
}}

/// Get contract logs
pub async fn get_contract_logs(
    State(state): State<AppState>,
    Query(params): Query<EventFilterQuery>,
) -> Result<Json<ApiResponse<Vec<LogData>>>, StatusCode> {{
    // TODO: Implement actual log retrieval
    let placeholder_logs = Vec::<LogData>::new();

    Ok(Json(ApiResponse {{
        success: true,
        data: Some(placeholder_logs),
        error: None,
        metadata: Some(ApiMetadata {{
            block_number: params.to_block,
            timestamp: Some(chrono::Utc::now().to_rfc3339()),
            gas_used: None,
            transaction_hash: None,
        }}),
    }}))
}}

"#,
            self.config.chain_id
        ));

        // Generate view function handlers
        for function in &abi.functions {
            if function.state_mutability == "view" || function.state_mutability == "pure" {
                let handler_name = format!("call_{}", function.name.to_lowercase());
                endpoints_code.push_str(&format!(
                    r#"/// Call {} view function
pub async fn {}(
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<ApiResponse<Value>>, StatusCode> {{
    // TODO: Parse function parameters and call view function
    // This would extract parameters and call state.client.call_{}(params).await
    
    let placeholder_result = serde_json::json!({{
        "function_name": "{}",
        "message": "Function call not yet implemented",
        "parameters": params
    }});

    Ok(Json(ApiResponse {{
        success: true,
        data: Some(placeholder_result),
        error: None,
        metadata: Some(ApiMetadata {{
            block_number: None,
            timestamp: Some(chrono::Utc::now().to_rfc3339()),
            gas_used: None,
            transaction_hash: None,
        }}),
    }}))
}}

"#,
                    function.name,
                    handler_name,
                    function.name.to_lowercase(),
                    function.name
                ));
            }
        }

        // Generate transaction function handlers
        for function in &abi.functions {
            if function.state_mutability == "nonpayable" || function.state_mutability == "payable" {
                let handler_name = format!("execute_{}", function.name.to_lowercase());
                endpoints_code.push_str(&format!(
                    r#"/// Execute {} transaction
pub async fn {}(
    State(state): State<AppState>,
    Json(payload): Json<Value>,
) -> Result<Json<ApiResponse<Value>>, StatusCode> {{
    // TODO: Parse transaction parameters and execute function
    // This would extract parameters and call state.client.execute_{}(params).await
    
    let placeholder_result = serde_json::json!({{
        "function_name": "{}",
        "message": "Transaction execution not yet implemented",
        "payload": payload
    }});

    Ok(Json(ApiResponse {{
        success: true,
        data: Some(placeholder_result),
        error: None,
        metadata: Some(ApiMetadata {{
            block_number: None,
            timestamp: Some(chrono::Utc::now().to_rfc3339()),
            gas_used: None,
            transaction_hash: Some("placeholder_hash".to_string()),
        }}),
    }}))
}}

"#,
                    function.name,
                    handler_name,
                    function.name.to_lowercase(),
                    function.name
                ));
            }
        }

        // Add supporting types
        endpoints_code.push_str(r#"/// Contract information response
#[derive(Debug, Serialize)]
pub struct ContractInfo {
    pub address: String,
    pub chain_id: String,
    pub contract_type: String,
}

/// Transaction data for responses
#[derive(Debug, Serialize)]
pub struct TransactionData {
    pub hash: String,
    pub block_number: u64,
    pub block_hash: String,
    pub transaction_index: u32,
    pub from: String,
    pub to: String,
    pub value: String,
    pub gas: u64,
    pub gas_price: String,
    pub input: String,
    pub status: u32,
}

/// Event data for responses
#[derive(Debug, Serialize)]
pub struct EventData {
    pub transaction_hash: String,
    pub block_number: u64,
    pub log_index: u32,
    pub event_name: String,
    pub event_data: Value,
}

/// Log data for responses
#[derive(Debug, Serialize)]
pub struct LogData {
    pub address: String,
    pub topics: Vec<String>,
    pub data: String,
    pub block_number: u64,
    pub transaction_hash: String,
    pub log_index: u32,
}

/// Error handling for API endpoints
impl From<indexer_core::Error> for StatusCode {
    fn from(err: indexer_core::Error) -> Self {
        match err {
            indexer_core::Error::NotFound(_) => StatusCode::NOT_FOUND,
            indexer_core::Error::Validation(_) => StatusCode::BAD_REQUEST,
            indexer_core::Error::Unauthorized(_) => StatusCode::UNAUTHORIZED,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}
"#);

        Ok(endpoints_code)
    }

    fn generate_migration_sql(&self, _abi: &EthereumAbi) -> Result<String> {
        Ok(format!(
            r#"-- Migration for contract: {}
-- Chain: {}

-- TODO: Generate actual migration SQL based on contract ABI

-- Add contract-specific tables and indexes
"#,
            self.config.contract_address,
            self.config.chain_id
        ))
    }

    /// Sanitize contract name for use in code identifiers
    fn sanitize_contract_name(&self) -> String {
        self.config.contract_address
            .chars()
            .filter(|c| c.is_alphanumeric())
            .collect::<String>()
            .to_case(convert_case::Case::Pascal)
    }

    /// Write content to file (or just print if dry run)
    async fn write_file(&self, path: &PathBuf, content: &str) -> Result<()> {
        if self.config.dry_run {
            println!("\n--- {} ---", path.display());
            println!("{}", content);
        } else {
            if let Some(parent) = path.parent() {
                tokio::fs::create_dir_all(parent).await
                    .map_err(|e| indexer_core::Error::Config(format!("Failed to create directory: {}", e)))?;
            }
            tokio::fs::write(path, content).await
                .map_err(|e| indexer_core::Error::Config(format!("Failed to write file {}: {}", path.display(), e)))?;
        }
        Ok(())
    }

    fn generate_abi_helpers(&self, _abi: &super::parser::EthereumAbi) -> Result<String> {
        let client_name = self.sanitize_contract_name();
        
        Ok(format!(
            r#"//! Generated ABI encoding/decoding helpers for contract: {}

use super::types::*;
use super::{}Client;
use indexer_core::Result;
use alloy_primitives::{{Address, U256, Bytes}};
use serde_json::Value;

impl {}Client {{
    /// Encode function call data
    pub fn encode_function_call(&self, function_name: &str, params: &[Value]) -> Result<Bytes> {{
        // TODO: Implement actual ABI encoding for function calls
        // This would use the ABI to encode the function selector and parameters
        todo!("Implement ABI function call encoding for {{}}", function_name)
    }}

    /// Encode constructor arguments
    pub fn encode_constructor_args(&self, params: &[Value]) -> Result<Vec<u8>> {{
        // TODO: Implement actual ABI encoding for constructor arguments
        // This would use the ABI to encode the constructor parameters
        todo!("Implement ABI constructor encoding")
    }}

    /// Decode function call result
    pub fn decode_function_result<T>(&self, function_name: &str, data: &[u8]) -> Result<T>
    where
        T: serde::de::DeserializeOwned,
    {{
        // TODO: Implement actual ABI decoding for function results
        // This would use the ABI to decode the function return values
        todo!("Implement ABI function result decoding for {{}}", function_name)
    }}

    /// Decode event log data
    pub fn decode_event_log(&self, event_name: &str, topics: &[String], data: &[u8]) -> Result<Value> {{
        // TODO: Implement actual ABI decoding for event logs
        // This would use the ABI to decode the event parameters from topics and data
        todo!("Implement ABI event log decoding for {{}}", event_name)
    }}

    /// Get function signature (4-byte selector)
    pub fn get_function_signature(&self, function_name: &str) -> Result<[u8; 4]> {{
        // TODO: Implement function signature lookup
        // This would return the 4-byte selector for the given function name
        todo!("Implement function signature lookup for {{}}", function_name)
    }}

    /// Get event signature hash
    pub fn get_event_signature(&self, event_name: &str) -> Result<[u8; 32]> {{
        // TODO: Implement event signature lookup
        // This would return the 32-byte topic hash for the given event name
        todo!("Implement event signature lookup for {{}}", event_name)
    }}

    /// Estimate gas for function call
    pub async fn estimate_gas_for_function(&self, function_name: &str, params: &[Value]) -> Result<U256> {{
        let call_data = self.encode_function_call(function_name, params)?;
        
        // TODO: Implement actual gas estimation
        // This would use the ethereum client to estimate gas for the function call
        todo!("Implement gas estimation for function: {{}}", function_name)
    }}

    /// Get current gas price
    pub async fn get_gas_price(&self) -> Result<U256> {{
        // TODO: Implement actual gas price fetching
        // This would use the ethereum client to get current gas price
        todo!("Implement gas price fetching")
    }}
}}

/// ABI encoding/decoding utilities
pub mod abi_utils {{
    use super::*;
    
    /// Encode parameters according to ABI specification
    pub fn encode_parameters(types: &[&str], values: &[Value]) -> Result<Vec<u8>> {{
        // TODO: Implement parameter encoding
        todo!("Implement ABI parameter encoding")
    }}
    
    /// Decode parameters according to ABI specification
    pub fn decode_parameters(types: &[&str], data: &[u8]) -> Result<Vec<Value>> {{
        // TODO: Implement parameter decoding
        todo!("Implement ABI parameter decoding")
    }}
    
    /// Calculate function selector from signature
    pub fn calculate_function_selector(signature: &str) -> [u8; 4] {{
        use sha3::{{Digest, Keccak256}};
        let mut hasher = Keccak256::new();
        hasher.update(signature.as_bytes());
        let hash = hasher.finalize();
        [hash[0], hash[1], hash[2], hash[3]]
    }}
    
    /// Calculate event topic hash from signature
    pub fn calculate_event_topic(signature: &str) -> [u8; 32] {{
        use sha3::{{Digest, Keccak256}};
        let mut hasher = Keccak256::new();
        hasher.update(signature.as_bytes());
        hasher.finalize().into()
    }}
}}
"#,
            self.config.contract_address,
            client_name,
            client_name
        ))
    }

    fn generate_event_methods(&self, events: &[super::parser::AbiEvent]) -> Result<String> {
        let mut methods = String::new();
        let client_name = self.sanitize_contract_name();

        // Generate header
        methods.push_str(&format!(
            r#"//! Generated event parsing methods for contract: {}

use super::types::*;
use super::{}Client;
use indexer_core::Result;
use alloy_primitives::{{Address, U256, Bytes}};
use serde_json::Value;

impl {}Client {{
"#,
            self.config.contract_address,
            client_name,
            client_name
        ));

        // Generate methods for each event
        for event in events {
            let event_name = &event.name;
            let method_name = format!("parse_{}_event", event_name.to_lowercase());
            
            // Generate event struct if it has parameters
            if !event.inputs.is_empty() {
                methods.push_str(&format!(
                    r#"    /// Parse {} event from transaction log
    pub fn {}(&self, topics: &[String], data: &[u8]) -> Result<{}Event> {{
        // TODO: Implement actual event parsing from log data
        // This would decode the event parameters from topics and data according to ABI
        todo!("Implement event parsing for {}")
    }}

"#,
                    event_name, method_name, event_name, event_name
                ));
            } else {
                methods.push_str(&format!(
                    r#"    /// Parse {} event from transaction log (no parameters)
    pub fn {}(&self, topics: &[String], data: &[u8]) -> Result<()> {{
        // Verify this is the correct event type
        if topics.is_empty() {{
            return Err(indexer_core::Error::generic("Missing event topic"));
        }}
        
        // TODO: Verify event signature matches {}
        // let expected_topic = self.get_event_signature("{}")?;
        
        Ok(())
    }}

"#,
                    event_name, method_name, event_name, event_name
                ));
            }
        }

        methods.push_str("}\n");
        Ok(methods)
    }

    /// Generate GraphQL schema for Ethereum contracts
    fn generate_graphql_schema(&self, abi: &EthereumAbi) -> Result<String> {
        let contract_name = self.sanitize_contract_name();
        let mut graphql_code = String::new();

        graphql_code.push_str(&format!(
            r#"//! Generated GraphQL schema for contract: {}

use async_graphql::{{
    Context, Object, Result as GqlResult, Schema, Subscription, Union,
    SimpleObject, Enum, InputObject, ID, FieldResult,
}};
use serde_json::Value;
use std::collections::HashMap;
use tokio_stream::Stream;
use super::types::*;
use super::{}Client;

/// Root Query type for {} contract
pub struct {}Query;

#[Object]
impl {}Query {{
    /// Get contract information
    async fn contract_info(&self, ctx: &Context<'_>) -> GqlResult<ContractInfo> {{
        let client = ctx.data::<{}Client>()?;
        Ok(ContractInfo {{
            address: client.contract_address().to_string(),
            chain_id: "{}".to_string(),
            contract_type: "Ethereum".to_string(),
        }})
    }}

    /// Get current contract state
    async fn contract_state(
        &self,
        ctx: &Context<'_>,
        block_number: Option<u64>,
    ) -> GqlResult<Option<Value>> {{
        let _client = ctx.data::<{}Client>()?;
        // TODO: Implement actual state retrieval
        Ok(Some(serde_json::json!({{
            "message": "Contract state retrieval not yet implemented",
            "block_number": block_number
        }})))
    }}

    /// Get contract transactions
    async fn transactions(
        &self,
        ctx: &Context<'_>,
        from_block: Option<u64>,
        to_block: Option<u64>,
        limit: Option<u32>,
        offset: Option<u32>,
    ) -> GqlResult<Vec<TransactionData>> {{
        let _client = ctx.data::<{}Client>()?;
        // TODO: Implement actual transaction retrieval
        Ok(Vec::new())
    }}

    /// Get contract events
    async fn events(
        &self,
        ctx: &Context<'_>,
        from_block: Option<u64>,
        to_block: Option<u64>,
        event_name: Option<String>,
        limit: Option<u32>,
        offset: Option<u32>,
    ) -> GqlResult<Vec<EventData>> {{
        let _client = ctx.data::<{}Client>()?;
        // TODO: Implement actual event retrieval
        Ok(Vec::new())
    }}
"#,
            self.config.contract_address,
            contract_name,
            self.config.contract_address,
            contract_name,
            contract_name,
            contract_name,
            self.config.chain_id,
            contract_name,
            contract_name,
            contract_name
        ));

        // Generate view function resolvers
        for function in &abi.functions {
            if function.state_mutability == "view" || function.state_mutability == "pure" {
                let field_name = function.name.to_lowercase();
                graphql_code.push_str(&format!(
                    r#"
    /// Call {} view function
    async fn {}(
        &self,
        ctx: &Context<'_>,
"#,
                    function.name,
                    field_name
                ));

                // Add parameters
                for input in &function.inputs {
                    let gql_type = self.convert_abi_type_to_graphql(&input.param_type);
                    graphql_code.push_str(&format!("        {}: {},\n", input.name, gql_type));
                }

                graphql_code.push_str(&format!(
                    r#"    ) -> GqlResult<Option<Value>> {{
        let client = ctx.data::<{}Client>()?;
        // TODO: Call actual view function
        // let result = client.call_{}(...).await?;
        Ok(Some(serde_json::json!({{
            "function_name": "{}",
            "message": "Function call not yet implemented"
        }})))
    }}
"#,
                    contract_name,
                    field_name,
                    function.name
                ));
            }
        }

        graphql_code.push_str("}\n\n");

        // Generate schema builder
        graphql_code.push_str(&format!(
            r#"/// Ethereum contract GraphQL schema
pub type {}Schema = Schema<{}Query, async_graphql::EmptyMutation, async_graphql::EmptySubscription>;

/// Build the GraphQL schema
pub fn build_schema() -> {}Schema {{
    Schema::build({}Query, async_graphql::EmptyMutation, async_graphql::EmptySubscription)
        .finish()
}}

/// Build the GraphQL schema with client context
pub fn build_schema_with_client(client: {}Client) -> {}Schema {{
    Schema::build({}Query, async_graphql::EmptyMutation, async_graphql::EmptySubscription)
        .data(client)
        .finish()
}}
"#,
            contract_name,
            contract_name,
            contract_name,
            contract_name,
            contract_name,
            contract_name,
            contract_name
        ));

        Ok(graphql_code)
    }

    /// Convert Ethereum ABI type to GraphQL type
    fn convert_abi_type_to_graphql(&self, abi_type: &str) -> String {
        match abi_type {
            "address" => "String".to_string(),
            "bool" => "bool".to_string(),
            "string" => "String".to_string(),
            "bytes" => "String".to_string(),
            t if t.starts_with("uint") || t.starts_with("int") => "String".to_string(), // Use String for big integers
            t if t.starts_with("bytes") => "String".to_string(),
            t if t.contains("[") => "Vec<String>".to_string(), // Arrays as strings for simplicity
            _ => "String".to_string(),
        }
    }

    /// Generate WebSocket handlers for Ethereum contracts
    fn generate_websocket_handlers(&self, _abi: &EthereumAbi) -> Result<String> {
        let contract_name = self.sanitize_contract_name();
        
        Ok(format!(
            r#"//! Generated WebSocket handlers for contract: {}

use axum::{{
    extract::{{ws::{{Message, WebSocket, WebSocketUpgrade}}, State, Path}},
    response::Response,
}};
use serde::{{Deserialize, Serialize}};
use serde_json::Value;
use std::collections::HashMap;
use tokio::sync::broadcast;
use tokio_stream::{{Stream, StreamExt}};
use super::types::*;
use super::{}Client;

/// WebSocket message types for contract events
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum WebSocketMessage {{
    /// Subscription request
    Subscribe {{
        subscription_id: String,
        event_name: Option<String>,
        filters: Option<HashMap<String, Value>>,
    }},
    /// Unsubscribe request
    Unsubscribe {{
        subscription_id: String,
    }},
    /// Event notification
    Event {{
        subscription_id: String,
        event_data: EventData,
    }},
    /// Transaction notification
    Transaction {{
        subscription_id: String,
        transaction_data: TransactionData,
    }},
    /// Error message
    Error {{
        subscription_id: Option<String>,
        error: String,
    }},
    /// Acknowledgment
    Ack {{
        subscription_id: String,
        message: String,
    }},
}}

/// WebSocket upgrade handler for contract events
pub async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(state): State<{}WebSocketState>,
) -> Response {{
    ws.on_upgrade(move |socket| handle_websocket(socket, state))
}}

/// Handle WebSocket connection
async fn handle_websocket(mut socket: WebSocket, state: {}WebSocketState) {{
    let mut subscriptions: HashMap<String, EventSubscription> = HashMap::new();

    loop {{
        tokio::select! {{
            // Handle incoming WebSocket messages
            msg = socket.recv() => {{
                match msg {{
                    Some(Ok(Message::Text(text))) => {{
                        match serde_json::from_str::<WebSocketMessage>(&text) {{
                            Ok(ws_msg) => {{
                                handle_websocket_message(ws_msg, &mut subscriptions, &mut socket).await;
                            }},
                            Err(e) => {{
                                let error_msg = WebSocketMessage::Error {{
                                    subscription_id: None,
                                    error: format!("Invalid message format: {{}}", e),
                                }};
                                if let Ok(error_text) = serde_json::to_string(&error_msg) {{
                                    let _ = socket.send(Message::Text(error_text)).await;
                                }}
                            }}
                        }}
                    }},
                    Some(Ok(Message::Close(_))) | None => {{
                        break;
                    }},
                    Some(Err(e)) => {{
                        eprintln!("WebSocket error: {{}}", e);
                        break;
                    }},
                    _ => {{}}
                }}
            }},
        }}
    }}
}}

/// Handle individual WebSocket messages
async fn handle_websocket_message(
    message: WebSocketMessage,
    subscriptions: &mut HashMap<String, EventSubscription>,
    socket: &mut WebSocket,
) {{
    match message {{
        WebSocketMessage::Subscribe {{ subscription_id, event_name, filters }} => {{
            let subscription = EventSubscription {{
                event_name: event_name.clone(),
                filters: filters.unwrap_or_default(),
            }};
            
            subscriptions.insert(subscription_id.clone(), subscription);
            
            let ack = WebSocketMessage::Ack {{
                subscription_id,
                message: format!("Subscribed to events: {{}}", event_name.unwrap_or_else(|| "all".to_string())),
            }};
            
            if let Ok(ack_text) = serde_json::to_string(&ack) {{
                let _ = socket.send(Message::Text(ack_text)).await;
            }}
        }},
        
        WebSocketMessage::Unsubscribe {{ subscription_id }} => {{
            subscriptions.remove(&subscription_id);
            
            let ack = WebSocketMessage::Ack {{
                subscription_id,
                message: "Unsubscribed successfully".to_string(),
            }};
            
            if let Ok(ack_text) = serde_json::to_string(&ack) {{
                let _ = socket.send(Message::Text(ack_text)).await;
            }}
        }},
        
        _ => {{
            let error = WebSocketMessage::Error {{
                subscription_id: None,
                error: "Unsupported message type".to_string(),
            }};
            
            if let Ok(error_text) = serde_json::to_string(&error) {{
                let _ = socket.send(Message::Text(error_text)).await;
            }}
        }}
    }}
}}

/// Event subscription configuration
#[derive(Debug, Clone)]
pub struct EventSubscription {{
    pub event_name: Option<String>,
    pub filters: HashMap<String, Value>,
}}

/// WebSocket state
#[derive(Clone)]
pub struct {}WebSocketState {{
    pub client: {}Client,
}}
"#,
            self.config.contract_address,
            contract_name,
            contract_name,
            contract_name,
            contract_name,
            contract_name
        ))
    }

    /// Generate OpenAPI documentation for Ethereum contracts
    fn generate_openapi_documentation(&self, abi: &EthereumAbi) -> Result<String> {
        let contract_name = self.sanitize_contract_name();
        let mut openapi_code = String::new();

        openapi_code.push_str(&format!(
            r#"//! Generated OpenAPI documentation for contract: {}

use utoipa::{{OpenApi, ToSchema}};
use serde::{{Deserialize, Serialize}};
use serde_json::Value;
use super::types::*;

/// OpenAPI specification for {} contract
#[derive(OpenApi)]
#[openapi(
    paths(
        get_contract_info,
        get_contract_state,
        get_contract_transactions,
        get_contract_events,
        get_contract_logs,
"#,
            self.config.contract_address,
            self.config.contract_address
        ));

        // Add function paths
        for function in &abi.functions {
            if function.state_mutability == "view" || function.state_mutability == "pure" {
                openapi_code.push_str(&format!(
                    "        call_{},\n",
                    function.name.to_lowercase()
                ));
            } else {
                openapi_code.push_str(&format!(
                    "        execute_{},\n",
                    function.name.to_lowercase()
                ));
            }
        }

        openapi_code.push_str(&format!(
            r#"    ),
    components(
        schemas(
            ApiResponse<Value>,
            ApiMetadata,
            ContractInfo,
            TransactionData,
            EventData,
            LogData,
            PaginationQuery,
            EventFilterQuery,
        )
    ),
    tags(
        (name = "{}", description = "Ethereum smart contract API endpoints")
    ),
    info(
        title = "{} Contract API",
        version = "1.0.0",
        description = "Generated API for {} smart contract on {} chain",
        contact(
            name = "Almanac Indexer",
            url = "https://github.com/timewave-ai/almanac"
        ),
        license(
            name = "MIT",
            url = "https://opensource.org/licenses/MIT"
        )
    ),
    servers(
        (url = "/api/v1", description = "Local development server")
    )
)]
pub struct {}ApiDoc;

/// Generate OpenAPI JSON specification
pub fn generate_openapi_spec() -> Result<String, serde_json::Error> {{
    let doc = {}ApiDoc::openapi();
    serde_json::to_string_pretty(&doc)
}}
"#,
            self.config.contract_address,
            self.config.contract_address,
            self.config.contract_address,
            self.config.chain_id,
            contract_name,
            contract_name
        ));

        Ok(openapi_code)
    }

    /// Generate authentication and rate limiting for Ethereum contracts
    fn generate_auth_and_rate_limiting(&self, _abi: &EthereumAbi) -> Result<String> {
        let contract_name = self.sanitize_contract_name();
        
        Ok(format!(
            r#"//! Generated authentication and rate limiting for contract: {}

use axum::{{
    extract::{{Request, State}},
    http::{{HeaderMap, StatusCode}},
    middleware::Next,
    response::Response,
}};
use std::{{
    collections::HashMap,
    sync::{{Arc, Mutex}},
    time::{{Duration, Instant}},
}};
use tokio::time::interval;

/// Authentication configuration
#[derive(Debug, Clone)]
pub struct AuthConfig {{
    pub require_api_key: bool,
    pub valid_api_keys: Vec<String>,
    pub admin_keys: Vec<String>,
}}

impl Default for AuthConfig {{
    fn default() -> Self {{
        Self {{
            require_api_key: false,
            valid_api_keys: vec![],
            admin_keys: vec![],
        }}
    }}
}}

/// Rate limiting configuration
#[derive(Debug, Clone)]
pub struct RateLimitConfig {{
    pub requests_per_minute: u32,
    pub burst_size: u32,
    pub window_size: Duration,
}}

impl Default for RateLimitConfig {{
    fn default() -> Self {{
        Self {{
            requests_per_minute: 60,
            burst_size: 10,
            window_size: Duration::from_secs(60),
        }}
    }}
}}

/// Application state with auth and rate limiting for Ethereum contracts
#[derive(Clone)]
pub struct {}AuthState {{
    pub auth_config: AuthConfig,
    pub rate_limiter: Arc<RateLimiter>,
}}

/// Rate limiter implementation
#[derive(Debug)]
pub struct RateLimiter {{
    requests: Arc<Mutex<HashMap<String, Vec<Instant>>>>,
    config: RateLimitConfig,
}}

impl RateLimiter {{
    /// Create a new rate limiter
    pub fn new(config: RateLimitConfig) -> Self {{
        Self {{
            requests: Arc::new(Mutex::new(HashMap::new())),
            config,
        }}
    }}

    /// Check if request is allowed for the given client
    pub fn is_allowed(&self, client_id: &str) -> bool {{
        let now = Instant::now();
        let mut requests = self.requests.lock().unwrap();
        
        let client_requests = requests.entry(client_id.to_string()).or_insert_with(Vec::new);
        
        // Remove old requests outside the window
        client_requests.retain(|&time| now.duration_since(time) < self.config.window_size);
        
        // Check if we're under the limit
        if client_requests.len() < self.config.requests_per_minute as usize {{
            client_requests.push(now);
            true
        }} else {{
            false
        }}
    }}
}}

/// Authentication middleware
pub async fn auth_middleware(
    State(auth_state): State<{}AuthState>,
    headers: HeaderMap,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {{
    // Skip auth if not required
    if !auth_state.auth_config.require_api_key {{
        return Ok(next.run(request).await);
    }}

    // Check for API key
    let api_key = headers
        .get("x-api-key")
        .or_else(|| headers.get("authorization"))
        .and_then(|h| h.to_str().ok())
        .map(|s| s.trim_start_matches("Bearer ").to_string());

    if let Some(key) = api_key {{
        if auth_state.auth_config.valid_api_keys.contains(&key) 
            || auth_state.auth_config.admin_keys.contains(&key) {{
            Ok(next.run(request).await)
        }} else {{
            Err(StatusCode::UNAUTHORIZED)
        }}
    }} else {{
        Err(StatusCode::UNAUTHORIZED)
    }}
}}

/// Rate limiting middleware
pub async fn rate_limit_middleware(
    State(auth_state): State<{}AuthState>,
    headers: HeaderMap,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {{
    // Get client identifier (IP or API key)
    let client_id = headers
        .get("x-api-key")
        .and_then(|h| h.to_str().ok())
        .or_else(|| {{
            headers
                .get("x-forwarded-for")
                .or_else(|| headers.get("x-real-ip"))
                .and_then(|h| h.to_str().ok())
        }})
        .unwrap_or("unknown")
        .to_string();

    // Check rate limit
    if auth_state.rate_limiter.is_allowed(&client_id) {{
        Ok(next.run(request).await)
    }} else {{
        Err(StatusCode::TOO_MANY_REQUESTS)
    }}
}}
"#,
            self.config.contract_address,
            contract_name,
            contract_name,
            contract_name
        ))
    }
} 