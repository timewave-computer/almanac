/// PostgreSQL storage implementation
use std::path::Path;
use std::sync::Arc;

use async_trait::async_trait;
use sqlx::{postgres::PgPoolOptions, Pool, Postgres};

use indexer_core::event::Event;
use indexer_core::Result;

use crate::{Storage, EventFilter};
use crate::migrations::ContractSchemaVersion;

mod repositories;
use repositories::{
    EventRepository, PostgresEventRepository, 
    ContractSchemaRepository, PostgresContractSchemaRepository
};

// Re-export the migration module
mod migrations;
pub use migrations::PostgresMigrationManager;

/// PostgreSQL configuration
pub struct PostgresConfig {
    /// Connection string
    pub connection_string: String,
    
    /// Maximum number of connections
    pub max_connections: u32,
    
    /// Whether to migrate on startup
    pub migrate: bool,
    
    /// Path to migration files
    pub migrations_path: String,
}

impl Default for PostgresConfig {
    fn default() -> Self {
        Self {
            connection_string: "postgres://postgres:postgres@localhost/indexer".to_string(),
            max_connections: 5,
            migrate: true,
            migrations_path: "./crates/storage/migrations".to_string(),
        }
    }
}

/// PostgreSQL storage
pub struct PostgresStorage {
    /// Connection pool
    pool: Pool<Postgres>,
    
    /// Event repository
    event_repository: Arc<dyn EventRepository>,
    
    /// Contract schema repository
    contract_schema_repository: Arc<dyn ContractSchemaRepository>,
}

impl PostgresStorage {
    /// Create a new PostgreSQL storage
    pub async fn new(config: PostgresConfig) -> Result<Self> {
        // Initialize the database if necessary
        if config.migrate {
            let migrations_path = Path::new(&config.migrations_path);
            if migrations_path.exists() {
                let migration_manager = PostgresMigrationManager::new(
                    &config.connection_string,
                    &config.migrations_path
                );
                migration_manager.migrate().await?;
            } else {
                tracing::warn!("Migrations directory not found at {}", config.migrations_path);
                // Fall back to inline schema creation for backwards compatibility
                Self::create_schema_inline(&config.connection_string).await?;
            }
        }
        
        // Create the connection pool
        let pool = PgPoolOptions::new()
            .max_connections(config.max_connections)
            .connect(&config.connection_string)
            .await?;
        
        // Create repositories
        let event_repository = Arc::new(PostgresEventRepository::new(pool.clone()));
        let contract_schema_repository = Arc::new(PostgresContractSchemaRepository::new(pool.clone()));
        
        Ok(Self {
            pool,
            event_repository,
            contract_schema_repository,
        })
    }
    
    /// Create schema inline (backwards compatibility method)
    async fn create_schema_inline(connection_string: &str) -> Result<()> {
        // Connect to the database
        let pool = Pool::<Postgres>::connect(connection_string).await?;
        
        // Create tables with inline SQL
        // Note: This should be replaced with proper migrations in production
        tracing::warn!("Using inline schema creation (for backwards compatibility)");
        
        // Create events table
        sqlx::query!(
            r#"
            CREATE TABLE IF NOT EXISTS events (
                id TEXT PRIMARY KEY,
                chain TEXT NOT NULL,
                block_number BIGINT NOT NULL,
                block_hash TEXT NOT NULL,
                tx_hash TEXT NOT NULL,
                timestamp BIGINT NOT NULL,
                event_type TEXT NOT NULL,
                raw_data BYTEA NOT NULL,
                created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
            );
            
            CREATE INDEX IF NOT EXISTS idx_events_chain ON events (chain);
            CREATE INDEX IF NOT EXISTS idx_events_block_number ON events (block_number);
            CREATE INDEX IF NOT EXISTS idx_events_timestamp ON events (timestamp);
            CREATE INDEX IF NOT EXISTS idx_events_event_type ON events (event_type);
            "#
        )
        .execute(&pool)
        .await?;
        
        // Create blocks table
        sqlx::query!(
            r#"
            CREATE TABLE IF NOT EXISTS blocks (
                chain TEXT NOT NULL,
                block_number BIGINT NOT NULL,
                block_hash TEXT NOT NULL,
                timestamp BIGINT NOT NULL,
                created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
                PRIMARY KEY (chain, block_number)
            );
            "#
        )
        .execute(&pool)
        .await?;
        
        Ok(())
    }
    
    /// Get the contract schema repository
    pub fn contract_schema_repository(&self) -> Arc<dyn ContractSchemaRepository> {
        self.contract_schema_repository.clone()
    }
    
    /// Register a contract schema version
    pub async fn register_schema(&self, schema: ContractSchemaVersion) -> Result<()> {
        self.contract_schema_repository.register_schema(schema).await
    }
    
    /// Get a contract schema version
    pub async fn get_schema(&self, version: &str, contract_address: &str, chain_id: &str) -> Result<Option<ContractSchemaVersion>> {
        self.contract_schema_repository.get_schema(version, contract_address, chain_id).await
    }
    
    /// Get the latest contract schema version
    pub async fn get_latest_schema(&self, contract_address: &str, chain_id: &str) -> Result<Option<ContractSchemaVersion>> {
        self.contract_schema_repository.get_latest_schema(contract_address, chain_id).await
    }
}

#[async_trait]
impl Storage for PostgresStorage {
    async fn store_event(&self, event: Box<dyn Event>) -> Result<()> {
        self.event_repository.store_event(event).await
    }
    
    async fn get_events(&self, filters: Vec<EventFilter>) -> Result<Vec<Box<dyn Event>>> {
        self.event_repository.get_events(filters).await
    }
    
    async fn get_latest_block(&self, chain: &str) -> Result<u64> {
        self.event_repository.get_latest_block(chain).await
    }
} 