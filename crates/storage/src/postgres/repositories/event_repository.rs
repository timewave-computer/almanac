/// Event repository implementation for PostgreSQL using sqlx
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use async_trait::async_trait;
use sqlx::{postgres::PgRow, Pool, Postgres, Row};
use sqlx::FromRow;

use indexer_core::event::{Event, EventContainer, EventMetadata};
use indexer_core::Result;

use crate::EventFilter;

/// Event data as stored in the database
#[derive(Debug, FromRow)]
pub struct EventRecord {
    /// Unique identifier for the event
    pub id: String,
    
    /// Chain from which the event originated
    pub chain: String,
    
    /// Block number or height at which the event occurred
    pub block_number: i64,
    
    /// Hash of the block containing the event
    pub block_hash: String,
    
    /// Hash of the transaction containing the event
    pub tx_hash: String,
    
    /// Timestamp when the event occurred
    pub timestamp: i64,
    
    /// Type of the event
    pub event_type: String,
    
    /// Raw event data
    pub raw_data: Vec<u8>,
    
    /// Created at timestamp
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Repository for event data
#[async_trait]
pub trait EventRepository: Send + Sync + 'static {
    /// Store an event
    async fn store_event(&self, event: Box<dyn Event>) -> Result<()>;
    
    /// Get events by filters
    async fn get_events(&self, filters: Vec<EventFilter>) -> Result<Vec<Box<dyn Event>>>;
    
    /// Get the latest block height for a chain
    async fn get_latest_block(&self, chain: &str) -> Result<u64>;
}

/// PostgreSQL implementation of the event repository
pub struct PostgresEventRepository {
    /// Connection pool
    pool: Pool<Postgres>,
}

impl PostgresEventRepository {
    /// Create a new PostgreSQL event repository
    pub fn new(pool: Pool<Postgres>) -> Self {
        Self { pool }
    }
    
    /// Convert database record to core event
    fn record_to_event(&self, record: EventRecord) -> Box<dyn Event> {
        let metadata = EventMetadata {
            id: record.id,
            chain: record.chain,
            block_number: record.block_number as u64,
            block_hash: record.block_hash,
            tx_hash: record.tx_hash,
            timestamp: record.timestamp as u64,
            event_type: record.event_type,
        };
        
        Box::new(EventWrapper {
            metadata,
            raw_data: record.raw_data,
        })
    }
    
    /// Ensure the block for this event exists in the database
    async fn ensure_block_exists(&self, event: &dyn Event) -> Result<()> {
        sqlx::query!(
            r#"
            INSERT INTO blocks (chain, block_number, block_hash, timestamp)
            VALUES ($1, $2, $3, $4)
            ON CONFLICT (chain, block_number) DO NOTHING
            "#,
            event.chain(),
            event.block_number() as i64,
            event.block_hash(),
            event.timestamp().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs() as i64
        )
        .execute(&self.pool)
        .await?;
        
        Ok(())
    }
}

// Create a new type to implement Event trait
#[derive(Debug)]
pub struct EventWrapper {
    metadata: EventMetadata,
    raw_data: Vec<u8>,
}

// Implementation for our own wrapper
impl Event for EventWrapper {
    fn id(&self) -> &str {
        &self.metadata.id
    }
    
    fn chain(&self) -> &str {
        &self.metadata.chain
    }
    
    fn block_number(&self) -> u64 {
        self.metadata.block_number
    }
    
    fn block_hash(&self) -> &str {
        &self.metadata.block_hash
    }
    
    fn tx_hash(&self) -> &str {
        &self.metadata.tx_hash
    }
    
    fn timestamp(&self) -> SystemTime {
        UNIX_EPOCH + std::time::Duration::from_secs(self.metadata.timestamp)
    }
    
    fn event_type(&self) -> &str {
        &self.metadata.event_type
    }
    
    fn raw_data(&self) -> &[u8] {
        &self.raw_data
    }
}

#[async_trait]
impl EventRepository for PostgresEventRepository {
    /// Store an event in the database
    async fn store_event(&self, event: Box<dyn Event>) -> Result<()> {
        // Insert the event
        sqlx::query!(
            r#"
            INSERT INTO events (id, chain, block_number, block_hash, tx_hash, timestamp, event_type, raw_data)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            ON CONFLICT (id) DO NOTHING
            "#,
            event.id(),
            event.chain(),
            event.block_number() as i64,
            event.block_hash(),
            event.tx_hash(),
            event.timestamp().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs() as i64,
            event.event_type(),
            event.raw_data()
        )
        .execute(&self.pool)
        .await?;
        
        // Check if we need to insert a block
        self.ensure_block_exists(event.as_ref()).await?;
        
        Ok(())
    }
    
    async fn get_events(&self, filters: Vec<EventFilter>) -> Result<Vec<Box<dyn Event>>> {
        // This is a simplified implementation for demonstration purposes
        // A real implementation would build a dynamic query based on the filters
        
        // For now, we'll just query all events if filters is empty, or return an empty vector
        if filters.is_empty() {
            let records = sqlx::query_as::<_, EventRecord>(
                r#"
                SELECT id, chain, block_number, block_hash, tx_hash, timestamp, event_type, raw_data, created_at
                FROM events
                ORDER BY timestamp DESC
                LIMIT 100
                "#
            )
            .fetch_all(&self.pool)
            .await?;
            
            let events = records.into_iter()
                .map(|record| self.record_to_event(record))
                .collect();
            
            return Ok(events);
        }
        
        // If we got here, we have filters but our implementation is simplified
        // In a real implementation, we would build a dynamic query
        Ok(Vec::new())
    }
    
    async fn get_latest_block(&self, chain: &str) -> Result<u64> {
        // Get the latest block from the database
        let result = sqlx::query!(
            r#"
            SELECT MAX(block_number) as max_block
            FROM blocks
            WHERE chain = $1
            "#,
            chain
        )
        .fetch_one(&self.pool)
        .await?;
        
        let max_block = result.max_block.unwrap_or(0) as u64;
        
        Ok(max_block)
    }
} 