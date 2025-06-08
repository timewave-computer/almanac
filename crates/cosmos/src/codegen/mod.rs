//! Code generation for CosmWasm contracts
//! 
//! This module provides functionality to automatically generate Rust code for interacting
//! with CosmWasm contracts from their message schema files (*_msg.json).

pub mod parser;
pub mod generator;
pub mod templates;
pub mod cli;

#[cfg(test)]
mod tests;

pub use parser::CosmWasmMsgParser;
pub use generator::CosmosContractCodegen;

use indexer_core::Result;

/// Configuration for cosmos contract code generation
#[derive(Debug, Clone)]
pub struct CosmosCodegenConfig {
    /// Contract address on the chain
    pub contract_address: String,
    /// Chain ID where the contract is deployed
    pub chain_id: String,
    /// Output directory for generated code
    pub output_dir: String,
    /// Namespace for generated code
    pub namespace: Option<String>,
    /// Features to enable in generation
    pub features: Vec<String>,
    /// Whether this is a dry run
    pub dry_run: bool,
}

impl Default for CosmosCodegenConfig {
    fn default() -> Self {
        Self {
            contract_address: String::new(),
            chain_id: String::new(),
            output_dir: "./generated".to_string(),
            namespace: None,
            features: vec![
                "client".to_string(),
                "storage".to_string(),
                "api".to_string(),
                "migrations".to_string(),
            ],
            dry_run: false,
        }
    }
}

/// Main entry point for cosmos contract code generation
pub async fn generate_contract_code(
    msg_file_path: &str,
    config: CosmosCodegenConfig,
) -> Result<()> {
    let parser = CosmWasmMsgParser::new();
    let schema = parser.parse_file(msg_file_path)?;
    
    let codegen = CosmosContractCodegen::new(config);
    codegen.generate_all(&schema).await?;
    
    Ok(())
} 