//! Code generation for Ethereum contracts
//! 
//! This module provides functionality to automatically generate Rust code for interacting
//! with Ethereum contracts from their ABI JSON files.

pub mod parser;
pub mod generator;
pub mod templates;
pub mod cli;

#[cfg(test)]
mod tests;

pub use parser::AbiParser;
pub use generator::EthereumContractCodegen;

use indexer_core::Result;

/// Configuration for ethereum contract code generation
#[derive(Debug, Clone)]
pub struct EthereumCodegenConfig {
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

impl Default for EthereumCodegenConfig {
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

/// Main entry point for ethereum contract code generation
pub async fn generate_contract_code(
    abi_file_path: &str,
    config: EthereumCodegenConfig,
) -> Result<()> {
    let parser = AbiParser::new();
    let abi = parser.parse_file(abi_file_path)?;
    
    let codegen = EthereumContractCodegen::new(config);
    codegen.generate_all(&abi).await?;
    
    Ok(())
} 