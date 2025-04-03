/// Contract schema repository implementation for PostgreSQL
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use async_trait::async_trait;
use sqlx::{Pool, Postgres, Row};
use sqlx::FromRow;
use tracing::{debug, info, warn};

use indexer_core::Result;

use crate::{ContractSchemaVersion, EventSchema, FieldSchema, FunctionSchema};

/// Contract schema record as stored in the database
#[derive(Debug, FromRow)]
pub struct ContractSchemaRecord {
    /// Schema version ID
    pub version: String,
    
    /// Contract address
    pub contract_address: String,
    
    /// Chain ID
    pub chain_id: String,
    
    /// ABI JSON
    pub abi_json: String,
    
    /// Created timestamp
    pub created_at: i64,
}

/// Event schema record as stored in the database
#[derive(Debug, FromRow)]
pub struct EventSchemaRecord {
    /// Schema version ID
    pub version: String,
    
    /// Contract address
    pub contract_address: String,
    
    /// Chain ID
    pub chain_id: String,
    
    /// Event name
    pub event_name: String,
    
    /// Event signature
    pub event_signature: String,
    
    /// Schema JSON
    pub schema_json: String,
}

/// Function schema record as stored in the database
#[derive(Debug, FromRow)]
pub struct FunctionSchemaRecord {
    /// Schema version ID
    pub version: String,
    
    /// Contract address
    pub contract_address: String,
    
    /// Chain ID
    pub chain_id: String,
    
    /// Function name
    pub function_name: String,
    
    /// Function signature
    pub function_signature: String,
    
    /// Schema JSON
    pub schema_json: String,
}

/// Repository for contract schema data
#[async_trait]
pub trait ContractSchemaRepository: Send + Sync + 'static {
    /// Register a new contract schema version
    async fn register_schema(&self, schema: ContractSchemaVersion) -> Result<()>;
    
    /// Get a contract schema version
    async fn get_schema(&self, version: &str, contract_address: &str, chain_id: &str) -> Result<Option<ContractSchemaVersion>>;
    
    /// Get the latest contract schema version
    async fn get_latest_schema(&self, contract_address: &str, chain_id: &str) -> Result<Option<ContractSchemaVersion>>;
    
    /// List all schema versions for a contract
    async fn list_schema_versions(&self, contract_address: &str, chain_id: &str) -> Result<Vec<String>>;
    
    /// List all contracts with schemas
    async fn list_contracts(&self) -> Result<Vec<(String, String)>>;
}

/// PostgreSQL implementation of the contract schema repository
pub struct PostgresContractSchemaRepository {
    /// Connection pool
    pool: Pool<Postgres>,
}

impl PostgresContractSchemaRepository {
    /// Create a new PostgreSQL contract schema repository
    pub fn new(pool: Pool<Postgres>) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl ContractSchemaRepository for PostgresContractSchemaRepository {
    async fn register_schema(&self, schema: ContractSchemaVersion) -> Result<()> {
        // Start a transaction
        let mut tx = self.pool.begin().await?;
        
        // Insert the contract schema version
        sqlx::query!(
            r#"
            INSERT INTO contract_schema_versions (version, contract_address, chain_id, abi_json, created_at)
            VALUES ($1, $2, $3, $4, $5)
            ON CONFLICT (version, contract_address, chain_id) DO UPDATE SET
                abi_json = EXCLUDED.abi_json,
                created_at = EXCLUDED.created_at
            "#,
            schema.version,
            schema.contract_address,
            schema.chain_id,
            schema.abi_json,
            schema.created_at as i64
        )
        .execute(&mut *tx)
        .await?;
        
        // Insert event schemas
        for (event_name, event_schema) in &schema.event_schemas {
            let schema_json = serde_json::to_string(event_schema)?;
            
            sqlx::query!(
                r#"
                INSERT INTO contract_event_schemas (
                    version, contract_address, chain_id, event_name, event_signature, schema_json
                )
                VALUES ($1, $2, $3, $4, $5, $6)
                ON CONFLICT (version, contract_address, chain_id, event_name) DO UPDATE SET
                    event_signature = EXCLUDED.event_signature,
                    schema_json = EXCLUDED.schema_json
                "#,
                schema.version,
                schema.contract_address,
                schema.chain_id,
                event_name,
                event_schema.signature,
                schema_json
            )
            .execute(&mut *tx)
            .await?;
        }
        
        // Insert function schemas
        for (function_name, function_schema) in &schema.function_schemas {
            let schema_json = serde_json::to_string(function_schema)?;
            
            sqlx::query!(
                r#"
                INSERT INTO contract_function_schemas (
                    version, contract_address, chain_id, function_name, function_signature, schema_json
                )
                VALUES ($1, $2, $3, $4, $5, $6)
                ON CONFLICT (version, contract_address, chain_id, function_name) DO UPDATE SET
                    function_signature = EXCLUDED.function_signature,
                    schema_json = EXCLUDED.schema_json
                "#,
                schema.version,
                schema.contract_address,
                schema.chain_id,
                function_name,
                function_schema.signature,
                schema_json
            )
            .execute(&mut *tx)
            .await?;
        }
        
        // Commit the transaction
        tx.commit().await?;
        
        Ok(())
    }
    
    async fn get_schema(&self, version: &str, contract_address: &str, chain_id: &str) -> Result<Option<ContractSchemaVersion>> {
        // Get the schema version record
        let schema_record = sqlx::query_as::<_, ContractSchemaRecord>(
            r#"
            SELECT version, contract_address, chain_id, abi_json, created_at
            FROM contract_schema_versions
            WHERE version = $1 AND contract_address = $2 AND chain_id = $3
            "#
        )
        .bind(version)
        .bind(contract_address)
        .bind(chain_id)
        .fetch_optional(&self.pool)
        .await?;
        
        let Some(record) = schema_record else {
            return Ok(None);
        };
        
        // Get event schemas
        let event_records = sqlx::query_as::<_, EventSchemaRecord>(
            r#"
            SELECT version, contract_address, chain_id, event_name, event_signature, schema_json
            FROM contract_event_schemas
            WHERE version = $1 AND contract_address = $2 AND chain_id = $3
            "#
        )
        .bind(version)
        .bind(contract_address)
        .bind(chain_id)
        .fetch_all(&self.pool)
        .await?;
        
        let mut event_schemas = HashMap::new();
        for event_record in event_records {
            let event_schema: EventSchema = serde_json::from_str(&event_record.schema_json)?;
            event_schemas.insert(event_record.event_name, event_schema);
        }
        
        // Get function schemas
        let function_records = sqlx::query_as::<_, FunctionSchemaRecord>(
            r#"
            SELECT version, contract_address, chain_id, function_name, function_signature, schema_json
            FROM contract_function_schemas
            WHERE version = $1 AND contract_address = $2 AND chain_id = $3
            "#
        )
        .bind(version)
        .bind(contract_address)
        .bind(chain_id)
        .fetch_all(&self.pool)
        .await?;
        
        let mut function_schemas = HashMap::new();
        for function_record in function_records {
            let function_schema: FunctionSchema = serde_json::from_str(&function_record.schema_json)?;
            function_schemas.insert(function_record.function_name, function_schema);
        }
        
        // Create the contract schema version
        let schema_version = ContractSchemaVersion {
            version: record.version,
            contract_address: record.contract_address,
            chain_id: record.chain_id,
            abi_json: record.abi_json,
            event_schemas,
            function_schemas,
            created_at: record.created_at as u64,
        };
        
        Ok(Some(schema_version))
    }
    
    async fn get_latest_schema(&self, contract_address: &str, chain_id: &str) -> Result<Option<ContractSchemaVersion>> {
        // Get the latest schema version
        let latest_version = sqlx::query!(
            r#"
            SELECT version
            FROM contract_schema_versions
            WHERE contract_address = $1 AND chain_id = $2
            ORDER BY created_at DESC
            LIMIT 1
            "#,
            contract_address,
            chain_id
        )
        .fetch_optional(&self.pool)
        .await?;
        
        let Some(record) = latest_version else {
            return Ok(None);
        };
        
        // Get the full schema version
        self.get_schema(&record.version, contract_address, chain_id).await
    }
    
    async fn list_schema_versions(&self, contract_address: &str, chain_id: &str) -> Result<Vec<String>> {
        // List all schema versions for a contract
        let versions = sqlx::query!(
            r#"
            SELECT version
            FROM contract_schema_versions
            WHERE contract_address = $1 AND chain_id = $2
            ORDER BY created_at ASC
            "#,
            contract_address,
            chain_id
        )
        .fetch_all(&self.pool)
        .await?;
        
        Ok(versions.into_iter().map(|r| r.version).collect())
    }
    
    async fn list_contracts(&self) -> Result<Vec<(String, String)>> {
        // List all contracts with schemas
        let contracts = sqlx::query!(
            r#"
            SELECT DISTINCT contract_address, chain_id
            FROM contract_schema_versions
            "#
        )
        .fetch_all(&self.pool)
        .await?;
        
        Ok(contracts.into_iter().map(|r| (r.contract_address, r.chain_id)).collect())
    }
} 