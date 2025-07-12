//! Code generator for CosmWasm contracts
//! 
//! Generates Rust code for client interactions, storage models, APIs, and migrations
//! from parsed CosmWasm contract schemas.

use super::{CosmosCodegenConfig, parser::CosmWasmSchema};
use indexer_core::Result;
use std::path::{Path, PathBuf};
use convert_case::Casing;

/// Code generator for cosmos contracts
pub struct CosmosContractCodegen {
    config: CosmosCodegenConfig,
}

impl CosmosContractCodegen {
    /// Create a new code generator with the given configuration
    pub fn new(config: CosmosCodegenConfig) -> Self {
        Self { config }
    }

    /// Generate all enabled code components
    pub async fn generate_all(&self, schema: &CosmWasmSchema) -> Result<()> {
        let output_dir = Path::new(&self.config.output_dir);
        
        if !self.config.dry_run {
            tokio::fs::create_dir_all(output_dir).await
                .map_err(|e| indexer_core::Error::Config(format!("Failed to create output directory: {}", e)))?;
        }

        for feature in &self.config.features {
            match feature.as_str() {
                "client" => self.generate_client_code(schema, output_dir).await?,
                "storage" => self.generate_storage_code(schema, output_dir).await?,
                "api" => self.generate_api_code(schema, output_dir).await?,
                "migrations" => self.generate_migration_code(schema, output_dir).await?,
                _ => {
                    println!("Warning: Unknown feature '{}'", feature);
                }
            }
        }

        Ok(())
    }

    /// Generate client interaction code
    async fn generate_client_code(&self, _schema: &CosmWasmSchema, output_dir: &Path) -> Result<()> {
        println!("Generating client code for contract: {}", self.config.contract_address);
        
        let client_dir = output_dir.join("client");
        if !self.config.dry_run {
            tokio::fs::create_dir_all(&client_dir).await
                .map_err(|e| indexer_core::Error::Config(format!("Failed to create client directory: {}", e)))?;
        }

        // Generate basic client module
        let client_code = format!(
            r#"//! Generated client code for contract: {}

pub struct {}Client {{
    contract_address: String,
}}

impl {}Client {{
    pub fn new(contract_address: String) -> Self {{
        Self {{ contract_address }}
    }}
    
    pub fn contract_address(&self) -> &str {{
        &self.contract_address
    }}
}}
"#,
            self.config.contract_address,
            self.sanitize_contract_name(),
            self.sanitize_contract_name()
        );

        self.write_file(&client_dir.join("mod.rs"), &client_code).await?;
        Ok(())
    }

    /// Generate storage models and database schemas
    async fn generate_storage_code(&self, _schema: &CosmWasmSchema, output_dir: &Path) -> Result<()> {
        println!("Generating storage code for contract: {}", self.config.contract_address);
        
        let storage_dir = output_dir.join("storage");
        if !self.config.dry_run {
            tokio::fs::create_dir_all(&storage_dir).await
                .map_err(|e| indexer_core::Error::Config(format!("Failed to create storage directory: {}", e)))?;
        }

        let storage_code = format!(
            r#"//! Generated storage code for contract: {}

pub struct {}Storage {{
    // Storage implementation will be added here
}}
"#,
            self.config.contract_address,
            self.sanitize_contract_name()
        );

        self.write_file(&storage_dir.join("mod.rs"), &storage_code).await?;
        Ok(())
    }

    /// Generate API endpoints
    async fn generate_api_code(&self, _schema: &CosmWasmSchema, output_dir: &Path) -> Result<()> {
        println!("Generating API code for contract: {}", self.config.contract_address);
        
        let api_dir = output_dir.join("api");
        if !self.config.dry_run {
            tokio::fs::create_dir_all(&api_dir).await
                .map_err(|e| indexer_core::Error::Config(format!("Failed to create api directory: {}", e)))?;
        }

        let api_code = format!(
            r#"//! Generated API code for contract: {}

pub struct {}Api {{
    // API implementation will be added here
}}
"#,
            self.config.contract_address,
            self.sanitize_contract_name()
        );

        self.write_file(&api_dir.join("mod.rs"), &api_code).await?;
        Ok(())
    }

    /// Generate database migrations
    async fn generate_migration_code(&self, _schema: &CosmWasmSchema, output_dir: &Path) -> Result<()> {
        println!("Generating migration code for contract: {}", self.config.contract_address);
        
        let migrations_dir = output_dir.join("migrations");
        if !self.config.dry_run {
            tokio::fs::create_dir_all(&migrations_dir).await
                .map_err(|e| indexer_core::Error::Config(format!("Failed to create migrations directory: {}", e)))?;
        }

        let migration_sql = format!(
            r#"-- Migration for contract: {}
-- Generated at: {}

CREATE TABLE IF NOT EXISTS contract_state (
    id BIGSERIAL PRIMARY KEY,
    contract_address TEXT NOT NULL,
    block_height BIGINT NOT NULL,
    state_data JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
"#,
            self.config.contract_address,
            chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
        );

        let timestamp = chrono::Utc::now().format("%Y%m%d%H%M%S");
        let filename = format!("{}_{}_contract.sql", timestamp, self.sanitize_contract_name().to_lowercase());
        self.write_file(&migrations_dir.join(filename), &migration_sql).await?;

        Ok(())
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
} 