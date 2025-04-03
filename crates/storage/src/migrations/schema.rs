/// Contract schema migration support
use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use sqlx::{Pool, Postgres};

use indexer_core::{Result, Error};

use super::{Migration, SqlMigration};

/// Contract schema version
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractSchemaVersion {
    /// Schema version ID
    pub version: String,
    
    /// Contract address
    pub contract_address: String,
    
    /// Chain ID
    pub chain_id: String,
    
    /// ABI JSON
    pub abi_json: String,
    
    /// Event schemas
    pub event_schemas: HashMap<String, EventSchema>,
    
    /// Function schemas
    pub function_schemas: HashMap<String, FunctionSchema>,
    
    /// Created timestamp
    pub created_at: u64,
}

/// Event schema
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventSchema {
    /// Event name
    pub name: String,
    
    /// Event signature
    pub signature: String,
    
    /// Fields
    pub fields: Vec<FieldSchema>,
}

/// Function schema
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionSchema {
    /// Function name
    pub name: String,
    
    /// Function signature
    pub signature: String,
    
    /// Input fields
    pub inputs: Vec<FieldSchema>,
    
    /// Output fields
    pub outputs: Vec<FieldSchema>,
}

/// Field schema
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldSchema {
    /// Field name
    pub name: String,
    
    /// Field type
    pub field_type: String,
    
    /// Whether the field is indexed
    pub indexed: bool,
}

/// Contract schema registry
pub struct ContractSchemaRegistry {
    /// PostgreSQL pool
    pool: Pool<Postgres>,
    
    /// Schema migration registry
    schemas: HashMap<String, ContractSchemaVersion>,
}

impl ContractSchemaRegistry {
    /// Create a new contract schema registry
    pub fn new(pool: Pool<Postgres>) -> Self {
        Self {
            pool,
            schemas: HashMap::new(),
        }
    }
    
    /// Initialize the schema registry
    pub async fn initialize(&mut self) -> Result<()> {
        // Create schema registry table if it doesn't exist
        self.create_tables().await?;
        
        // Load existing schemas
        self.load_schemas().await?;
        
        Ok(())
    }
    
    /// Create schema registry tables
    async fn create_tables(&self) -> Result<()> {
        // Create initial migration
        let migration = SqlMigration::new(
            "contract_schema_registry_001",
            "Create contract schema registry tables",
            r#"
            CREATE TABLE IF NOT EXISTS contract_schema_versions (
                version TEXT NOT NULL,
                contract_address TEXT NOT NULL,
                chain_id TEXT NOT NULL,
                abi_json TEXT NOT NULL,
                created_at BIGINT NOT NULL,
                PRIMARY KEY (version, contract_address, chain_id)
            );
            
            CREATE TABLE IF NOT EXISTS contract_event_schemas (
                version TEXT NOT NULL,
                contract_address TEXT NOT NULL,
                chain_id TEXT NOT NULL,
                event_name TEXT NOT NULL,
                event_signature TEXT NOT NULL,
                schema_json TEXT NOT NULL,
                PRIMARY KEY (version, contract_address, chain_id, event_name),
                FOREIGN KEY (version, contract_address, chain_id) 
                    REFERENCES contract_schema_versions(version, contract_address, chain_id)
            );
            
            CREATE TABLE IF NOT EXISTS contract_function_schemas (
                version TEXT NOT NULL,
                contract_address TEXT NOT NULL,
                chain_id TEXT NOT NULL,
                function_name TEXT NOT NULL,
                function_signature TEXT NOT NULL,
                schema_json TEXT NOT NULL,
                PRIMARY KEY (version, contract_address, chain_id, function_name),
                FOREIGN KEY (version, contract_address, chain_id) 
                    REFERENCES contract_schema_versions(version, contract_address, chain_id)
            );
            
            CREATE INDEX IF NOT EXISTS idx_contract_schema_versions_contract 
                ON contract_schema_versions(contract_address, chain_id);
            "#,
            // Down migration
            r#"
            DROP TABLE IF EXISTS contract_function_schemas;
            DROP TABLE IF EXISTS contract_event_schemas;
            DROP TABLE IF EXISTS contract_schema_versions;
            "#,
            self.pool.clone(),
        );
        
        // Apply migration
        migration.up().await?;
        
        Ok(())
    }
    
    /// Load schemas from database
    async fn load_schemas(&mut self) -> Result<()> {
        // Load schema versions
        let versions = sqlx::query!(
            r#"
            SELECT 
                version, 
                contract_address, 
                chain_id, 
                abi_json, 
                created_at
            FROM contract_schema_versions
            "#
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| Error::Storage(format!("Failed to load schema versions: {}", e)))?;
        
        // Process each schema version
        for version in versions {
            let key = format!("{}:{}:{}", version.version, version.contract_address, version.chain_id);
            
            // Load event schemas
            let events = sqlx::query!(
                r#"
                SELECT 
                    event_name, 
                    event_signature, 
                    schema_json
                FROM contract_event_schemas
                WHERE 
                    version = $1 AND 
                    contract_address = $2 AND 
                    chain_id = $3
                "#,
                version.version,
                version.contract_address,
                version.chain_id
            )
            .fetch_all(&self.pool)
            .await
            .map_err(|e| Error::Storage(format!("Failed to load event schemas: {}", e)))?;
            
            let mut event_schemas = HashMap::new();
            for event in events {
                let schema: EventSchema = serde_json::from_str(&event.schema_json)
                    .map_err(|e| Error::Serialization(format!("Failed to deserialize event schema: {}", e)))?;
                
                event_schemas.insert(event.event_name, schema);
            }
            
            // Load function schemas
            let functions = sqlx::query!(
                r#"
                SELECT 
                    function_name, 
                    function_signature, 
                    schema_json
                FROM contract_function_schemas
                WHERE 
                    version = $1 AND 
                    contract_address = $2 AND 
                    chain_id = $3
                "#,
                version.version,
                version.contract_address,
                version.chain_id
            )
            .fetch_all(&self.pool)
            .await
            .map_err(|e| Error::Storage(format!("Failed to load function schemas: {}", e)))?;
            
            let mut function_schemas = HashMap::new();
            for function in functions {
                let schema: FunctionSchema = serde_json::from_str(&function.schema_json)
                    .map_err(|e| Error::Serialization(format!("Failed to deserialize function schema: {}", e)))?;
                
                function_schemas.insert(function.function_name, schema);
            }
            
            // Create schema version
            let schema_version = ContractSchemaVersion {
                version: version.version,
                contract_address: version.contract_address,
                chain_id: version.chain_id,
                abi_json: version.abi_json,
                event_schemas,
                function_schemas,
                created_at: version.created_at as u64,
            };
            
            // Store in memory
            self.schemas.insert(key, schema_version);
        }
        
        Ok(())
    }
    
    /// Register a new contract schema version
    pub async fn register_schema(&mut self, schema: ContractSchemaVersion) -> Result<()> {
        // Store in database
        sqlx::query!(
            r#"
            INSERT INTO contract_schema_versions (
                version, 
                contract_address, 
                chain_id, 
                abi_json, 
                created_at
            ) VALUES ($1, $2, $3, $4, $5)
            ON CONFLICT (version, contract_address, chain_id) DO UPDATE
            SET 
                abi_json = $4,
                created_at = $5
            "#,
            schema.version,
            schema.contract_address,
            schema.chain_id,
            schema.abi_json,
            schema.created_at as i64
        )
        .execute(&self.pool)
        .await
        .map_err(|e| Error::Storage(format!("Failed to register schema version: {}", e)))?;
        
        // Store event schemas
        for (name, event_schema) in &schema.event_schemas {
            let schema_json = serde_json::to_string(event_schema)
                .map_err(|e| Error::Serialization(format!("Failed to serialize event schema: {}", e)))?;
            
            sqlx::query!(
                r#"
                INSERT INTO contract_event_schemas (
                    version, 
                    contract_address, 
                    chain_id, 
                    event_name, 
                    event_signature, 
                    schema_json
                ) VALUES ($1, $2, $3, $4, $5, $6)
                ON CONFLICT (version, contract_address, chain_id, event_name) DO UPDATE
                SET 
                    event_signature = $5,
                    schema_json = $6
                "#,
                schema.version,
                schema.contract_address,
                schema.chain_id,
                name,
                event_schema.signature,
                schema_json
            )
            .execute(&self.pool)
            .await
            .map_err(|e| Error::Storage(format!("Failed to register event schema: {}", e)))?;
        }
        
        // Store function schemas
        for (name, function_schema) in &schema.function_schemas {
            let schema_json = serde_json::to_string(function_schema)
                .map_err(|e| Error::Serialization(format!("Failed to serialize function schema: {}", e)))?;
            
            sqlx::query!(
                r#"
                INSERT INTO contract_function_schemas (
                    version, 
                    contract_address, 
                    chain_id, 
                    function_name, 
                    function_signature, 
                    schema_json
                ) VALUES ($1, $2, $3, $4, $5, $6)
                ON CONFLICT (version, contract_address, chain_id, function_name) DO UPDATE
                SET 
                    function_signature = $5,
                    schema_json = $6
                "#,
                schema.version,
                schema.contract_address,
                schema.chain_id,
                name,
                function_schema.signature,
                schema_json
            )
            .execute(&self.pool)
            .await
            .map_err(|e| Error::Storage(format!("Failed to register function schema: {}", e)))?;
        }
        
        // Store in memory
        let key = format!("{}:{}:{}", schema.version, schema.contract_address, schema.chain_id);
        self.schemas.insert(key, schema);
        
        Ok(())
    }
    
    /// Get schema version
    pub fn get_schema(
        &self, 
        version: &str, 
        contract_address: &str, 
        chain_id: &str
    ) -> Option<&ContractSchemaVersion> {
        let key = format!("{}:{}:{}", version, contract_address, chain_id);
        self.schemas.get(&key)
    }
    
    /// Get latest schema version for a contract
    pub fn get_latest_schema(
        &self,
        contract_address: &str,
        chain_id: &str
    ) -> Option<&ContractSchemaVersion> {
        let prefix = format!("{}:{}", contract_address, chain_id);
        
        self.schemas
            .values()
            .filter(|s| format!("{}:{}", s.contract_address, s.chain_id) == prefix)
            .max_by_key(|s| s.created_at)
    }
    
    /// Create a migration to update contract schemas
    pub fn create_schema_migration(&self, id: &str, description: &str) -> Arc<dyn Migration> {
        Arc::new(SqlMigration::new(
            id,
            description,
            r#"
            -- Add new fields or tables for contract schema updates
            ALTER TABLE contract_schema_versions
            ADD COLUMN IF NOT EXISTS description TEXT;
            
            ALTER TABLE contract_event_schemas
            ADD COLUMN IF NOT EXISTS is_anonymous BOOLEAN DEFAULT FALSE;
            "#,
            r#"
            -- Rollback schema changes
            ALTER TABLE contract_event_schemas
            DROP COLUMN IF EXISTS is_anonymous;
            
            ALTER TABLE contract_schema_versions
            DROP COLUMN IF EXISTS description;
            "#,
            self.pool.clone(),
        ))
    }
} 