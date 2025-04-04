/// PostgreSQL module and storage implementation
use std::sync::Arc;
use std::collections::HashMap;

use async_trait::async_trait;
use indexer_common::{Error, Result, BlockStatus};
use indexer_core::event::Event;
use sqlx::{Pool, Postgres};
use tracing::info;

use crate::EventFilter;
use crate::Storage;
use crate::migrations::initialize_database;

pub mod repositories;
pub mod migrations;

// Use the repositories directly instead of using paths
use repositories::event_repository::{EventRepository, PostgresEventRepository};
use repositories::contract_schema_repository::{
    ContractSchemaRepository, PostgresContractSchemaRepository
};

/// PostgreSQL storage configuration
#[derive(Debug, Clone)]
pub struct PostgresConfig {
    /// Database connection URL
    pub url: String,
    
    /// Max connections in the pool
    pub max_connections: u32,
    
    /// Connection timeout in seconds
    pub connection_timeout: u64,
}

impl Default for PostgresConfig {
    fn default() -> Self {
        Self {
            url: "postgres://postgres:postgres@localhost:5432/indexer".to_string(),
            max_connections: 5,
            connection_timeout: 30,
        }
    }
}

/// PostgreSQL storage
pub struct PostgresStorage {
    /// Database connection pool
    pool: Pool<Postgres>,
    
    /// Event repository
    event_repository: Arc<dyn EventRepository>,
    
    /// Contract schema repository
    contract_schema_repository: Arc<dyn ContractSchemaRepository>,
}

#[async_trait]
impl Storage for PostgresStorage {
    async fn store_event(&self, event: Box<dyn Event>) -> Result<()> {
        let transaction = self.pool.begin().await?;
        
        // Store the event using the repository
        self.event_repository.store_event(event).await?;
        
        // Commit the transaction
        transaction.commit().await?;
        
        Ok(())
    }
    
    async fn get_events(&self, filters: Vec<EventFilter>) -> Result<Vec<Box<dyn Event>>> {
        // Get events using the repository
        self.event_repository.get_events(filters).await
    }
    
    async fn get_latest_block(&self, chain: &str) -> Result<u64> {
        // Get the latest block using the repository
        self.event_repository.get_latest_block(chain).await
    }
    
    async fn update_block_status(&self, chain: &str, block_number: u64, status: BlockStatus) -> Result<()> {
        // Update block status in the database
        let mut transaction = self.pool.begin().await?;
        
        // Convert enum to string
        let status_str = match status {
            BlockStatus::Confirmed => "confirmed",
            BlockStatus::Safe => "safe",
            BlockStatus::Justified => "justified",
            BlockStatus::Finalized => "finalized",
        };
        
        // Update the status in the blocks table
        sqlx::query!(
            r#"
            UPDATE blocks
            SET status = $1
            WHERE chain = $2 AND block_number = $3
            "#,
            status_str,
            chain,
            block_number as i64
        )
        .execute(&mut *transaction)
        .await?;
        
        // Commit the transaction
        transaction.commit().await?;
        
        Ok(())
    }
    
    async fn get_latest_block_with_status(&self, chain: &str, status: BlockStatus) -> Result<u64> {
        // Convert enum to string
        let status_str = match status {
            BlockStatus::Confirmed => "confirmed",
            BlockStatus::Safe => "safe",
            BlockStatus::Justified => "justified",
            BlockStatus::Finalized => "finalized",
        };
        
        // Query the latest block with the given status
        let result = sqlx::query!(
            r#"
            SELECT MAX(block_number) as max_block
            FROM blocks
            WHERE chain = $1 AND status = $2
            "#,
            chain,
            status_str
        )
        .fetch_one(&self.pool)
        .await?;
        
        // Return the max block or 0 if no blocks found
        let max_block = result.max_block.unwrap_or(0) as u64;
        
        Ok(max_block)
    }
    
    async fn get_events_with_status(&self, filters: Vec<EventFilter>, status: BlockStatus) -> Result<Vec<Box<dyn Event>>> {
        // This is a more complex query that needs to join events with blocks
        // For this implementation, we'll just get all events and then filter by status
        
        // Convert enum to string
        let status_str = match status {
            BlockStatus::Confirmed => "confirmed",
            BlockStatus::Safe => "safe",
            BlockStatus::Justified => "justified",
            BlockStatus::Finalized => "finalized",
        };
        
        // Get the set of blocks with the given status
        let blocks = sqlx::query!(
            r#"
            SELECT chain, block_number
            FROM blocks
            WHERE status = $1
            "#,
            status_str
        )
        .fetch_all(&self.pool)
        .await?;
        
        // Create a map of chain to block numbers
        let mut chain_blocks = HashMap::new();
        for block in blocks {
            let chain_blocks_entry = chain_blocks
                .entry(block.chain.clone())
                .or_insert_with(Vec::new);
            chain_blocks_entry.push(block.block_number as u64);
        }
        
        // Get all events matching the filters
        let events = self.get_events(filters).await?;
        
        // Filter events to only include those from blocks with the given status
        let filtered_events = events
            .into_iter()
            .filter(|event| {
                // Get the set of blocks for this chain
                if let Some(blocks) = chain_blocks.get(event.chain()) {
                    // Check if the event's block is in the set
                    blocks.contains(&event.block_number())
                } else {
                    // No blocks for this chain, so filter out the event
                    false
                }
            })
            .collect();
        
        Ok(filtered_events)
    }
}

impl PostgresStorage {
    /// Create a new PostgreSQL storage
    pub async fn new(config: PostgresConfig) -> Result<Self> {
        // Create a connection pool
        let pool_options = sqlx::postgres::PgPoolOptions::new()
            .max_connections(config.max_connections)
            .acquire_timeout(std::time::Duration::from_secs(config.connection_timeout));
        
        let pool = pool_options.connect(&config.url)
            .await
            .map_err(|e| Error::generic(format!("Failed to connect to PostgreSQL: {}", e)))?;
        
        info!("Connected to PostgreSQL database");
        
        // Initialize database tables
        initialize_database(&pool).await?;
        
        // Create repositories
        let event_repository = Arc::new(PostgresEventRepository::new(pool.clone()));
        let contract_schema_repository = Arc::new(PostgresContractSchemaRepository::new(pool.clone()));
        
        Ok(Self {
            pool,
            event_repository,
            contract_schema_repository,
        })
    }
    
    /// Store a contract schema
    pub async fn store_contract_schema(&self, chain: &str, address: &str, schema_data: &[u8]) -> Result<()> {
        self.contract_schema_repository.store_schema(chain, address, schema_data).await
    }
    
    /// Get a contract schema
    pub async fn get_contract_schema(&self, chain: &str, address: &str) -> Result<Option<Vec<u8>>> {
        self.contract_schema_repository.get_schema(chain, address).await
    }
} 