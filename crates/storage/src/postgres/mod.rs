/// PostgreSQL module and storage implementation
#[cfg(feature = "postgres")]
use crate::{
    ValenceProcessorInfo, ValenceProcessorConfig, ValenceProcessorMessage, ValenceMessageStatus,
    ValenceProcessorState, ValenceAuthorizationInfo, ValenceAuthorizationPolicy, ValenceAuthorizationGrant,
    ValenceAuthorizationRequest, ValenceAuthorizationDecision, ValenceLibraryInfo, ValenceLibraryVersion,
    ValenceLibraryUsage, ValenceLibraryState, ValenceLibraryApproval
};
#[cfg(feature = "postgres")]
use tracing::{debug, info, warn, instrument};
#[cfg(feature = "postgres")]
use std::sync::Arc;
#[cfg(feature = "postgres")]
use std::time::SystemTime;
#[cfg(feature = "postgres")]
use std::collections::HashMap;

#[cfg(feature = "postgres")]
use async_trait::async_trait;
use indexer_core::{Error, Result, BlockStatus};
#[cfg(feature = "postgres")]
use indexer_core::event::Event;
#[cfg(feature = "postgres")]
use indexer_core::types::{ChainId, EventFilter};
#[cfg(feature = "postgres")]
use sqlx::{Pool, Postgres};
#[cfg(feature = "postgres")]
use chrono::{DateTime, Utc};

use crate::Storage;
use crate::{ValenceAccountInfo, ValenceAccountLibrary, ValenceAccountExecution, ValenceAccountState};

#[cfg(feature = "postgres")]
pub mod repositories;
#[cfg(feature = "postgres")]
pub mod migrations;

#[cfg(feature = "postgres")]
use repositories::event_repository::{EventRepository, PostgresEventRepository};
#[cfg(feature = "postgres")]
use repositories::contract_schema_repository::{
    ContractSchemaRepository, PostgresContractSchemaRepository
};
#[cfg(feature = "postgres")]
use self::migrations::initialize_database;

#[cfg(feature = "postgres")]
fn timestamp_to_datetime(time: SystemTime) -> DateTime<Utc> {
    DateTime::<Utc>::from(time)
}

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
            url: "postgres://localhost/indexer".to_string(),
            max_connections: 5,
            connection_timeout: 30,
        }
    }
}

/// PostgreSQL storage
#[cfg(feature = "postgres")]
pub struct PostgresStorage {
    /// Database connection pool
    pool: Pool<Postgres>,
    
    /// Event repository
    event_repository: Arc<dyn EventRepository>,
    
    /// Contract schema repository
    contract_schema_repository: Arc<dyn ContractSchemaRepository>,
}

#[cfg(feature = "postgres")]
#[async_trait]
impl Storage for PostgresStorage {
    async fn store_event(&self, _chain: &str, event: Box<dyn Event>) -> Result<()> {
        let transaction = self.pool.begin().await?;
        
        // Store the event using the repository
        self.event_repository.store_event(event).await?;
        
        // Commit the transaction
        transaction.commit().await?;
        
        Ok(())
    }
    
    async fn get_events(&self, chain: &str, from_block: u64, to_block: u64) -> Result<Vec<Box<dyn Event>>> {
        let mut filter = EventFilter::new();
        filter.chain_ids = Some(vec![ChainId::from(chain)]);
        filter.chain = Some(chain.to_string());
        filter.block_range = Some((from_block, to_block));
        filter.limit = Some(1);
        filter.offset = Some(0);
        // Get events using the repository
        self.event_repository.get_events(vec![filter]).await
    }
    
    async fn get_latest_block(&self, chain: &str) -> Result<u64> {
        // Get the latest block using the repository
        self.event_repository.get_latest_block(chain).await
    }
    
    async fn mark_block_processed(&self, chain: &str, block_number: u64, tx_hash: &str, status: BlockStatus) -> Result<()> {
        // TODO: Implement properly - currently relies on update_block_status
        // Potentially store tx_hash in the blocks table as well?
         let status_str = status.as_str();
         sqlx::query(
             r#"
             INSERT INTO blocks (chain, number, hash, timestamp, status)
             VALUES ($1, $2, $3, $4, $5)
             ON CONFLICT (chain, number) DO UPDATE SET
                 status = EXCLUDED.status,
                 hash = EXCLUDED.hash,
                 timestamp = EXCLUDED.timestamp
             "#
         )
         .bind(chain)
         .bind(block_number as i64)
         .bind(tx_hash) // Assuming tx_hash can stand in for block_hash here, needs clarification
         .bind(0i64) // Placeholder for timestamp
         .bind(status_str)
         .execute(&self.pool)
         .await?;
         Ok(())
    }

    async fn update_block_status(&self, chain: &str, block_number: u64, status: BlockStatus) -> Result<()> {
        // Update block status in the database
        let mut transaction = self.pool.begin().await?;
        
        // Convert enum to string
        let status_str = status.as_str();
        
        // Update the status in the blocks table
        sqlx::query(
            r#"
            UPDATE blocks
            SET status = $1
            WHERE chain = $2 AND number = $3
            "#
        )
        .bind(status_str)
        .bind(chain)
        .bind(block_number as i64)
        .execute(&mut *transaction)
        .await?;
        
        // Commit the transaction
        transaction.commit().await?;
        
        Ok(())
    }
    
    async fn get_latest_block_with_status(&self, chain: &str, status: BlockStatus) -> Result<u64> {
        // Convert enum to string
        let status_str = status.as_str();
        
        // Query the latest block with the given status
        let result: Option<(Option<i64>,)> = sqlx::query_as(
            r#"
            SELECT MAX(number) as max_block
            FROM blocks
            WHERE chain = $1 AND status = $2
            "#
        )
        .bind(chain)
        .bind(status_str)
        .fetch_optional(&self.pool)
        .await?;
        
        // Return the max block or 0 if no blocks found
        let max_block = result
            .and_then(|(max_block,)| max_block)
            .unwrap_or(0) as u64;
        
        Ok(max_block)
    }
    
    async fn get_events_with_status(&self, chain: &str, from_block: u64, to_block: u64, status: BlockStatus) -> Result<Vec<Box<dyn Event>>> {
        let status_str = status.as_str();
        
        // Get the set of block numbers with the given status in the range
        let blocks: Vec<(i64,)> = sqlx::query_as(
            r#"
            SELECT number
            FROM blocks
            WHERE chain = $1 AND status = $2 AND number >= $3 AND number <= $4
            ORDER BY number ASC
            "#
        )
        .bind(chain)
        .bind(status_str)
        .bind(from_block as i64)
        .bind(to_block as i64)
        .fetch_all(&self.pool)
        .await?;
        
        let block_numbers: Vec<u64> = blocks.into_iter().map(|(number,)| number as u64).collect();
        
        if block_numbers.is_empty() {
            return Ok(Vec::new());
        }

        // Create filters for each matching block
        let filters: Vec<EventFilter> = block_numbers.iter().map(|&b| {
            let mut filter = EventFilter::new();
            filter.chain_ids = Some(vec![ChainId::from(chain)]);
            filter.chain = Some(chain.to_string());
            filter.block_range = Some((b, b)); // Filter for this specific block
            filter
        }).collect();

        // Get events for the matching blocks
        // Note: This might be inefficient if there are many matching blocks.
        // A single query joining events and blocks might be better.
        self.event_repository.get_events(filters).await
    }

    /// Handle chain reorganization
    async fn reorg_chain(&self, chain: &str, from_block: u64) -> Result<()> {
        self.handle_chain_reorg(chain, from_block).await
    }

    /// Stores a record of an execution triggered by a Valence account.
    #[instrument(skip(self, execution_info), fields(account_id = %execution_info.account_id))]
    async fn store_valence_execution(
        &self,
        execution_info: ValenceAccountExecution,
    ) -> Result<()> {
        // Convert SystemTime to DateTime<Utc> for PostgreSQL
        let executed_at: DateTime<Utc> = DateTime::<Utc>::from(execution_info.executed_at);
        
        // Use regular SQLx query to avoid compile-time validation
        sqlx::query(
            r#"
            INSERT INTO valence_account_executions (
                chain_id,
                account_id,
                executor_address,
                payload,
                raw_msgs,
                tx_hash,
                block_number,
                message_index,
                executed_at,
                correlated_event_ids
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            "#
        )
        .bind(&execution_info.chain_id)
        .bind(&execution_info.account_id)
        .bind(&execution_info.executor_address)
        .bind(&execution_info.payload)
        .bind(&execution_info.raw_msgs)
        .bind(&execution_info.tx_hash)
        .bind(execution_info.block_number as i64)
        .bind(execution_info.message_index)
        .bind(executed_at)
        .bind(execution_info.correlated_event_ids.as_deref())
        .execute(&self.pool)
        .await?;

        debug!(
            account_id = %execution_info.account_id, 
            tx_hash = %execution_info.tx_hash, 
            "Stored Valence execution"
        );
        Ok(())
    }

    /// Retrieves the current state of a Valence account.
    #[instrument(skip(self), fields(account_id = %account_id))]
    async fn get_valence_account_state(&self, account_id: &str) -> Result<Option<ValenceAccountState>> {
        use sqlx::Row;
        
        // Fetch account details using regular SQLx query
        let account_row_result = sqlx::query(
            r#"
            SELECT 
                chain_id, 
                contract_address, 
                current_owner, 
                pending_owner, 
                pending_owner_expiry, 
                last_updated_block, 
                last_updated_tx 
            FROM valence_accounts WHERE id = $1
            "#
        )
        .bind(account_id)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(account_row) = account_row_result {
            // Fetch associated libraries using regular SQLx query
            let library_rows = sqlx::query(
                "SELECT library_address FROM valence_account_libraries WHERE account_id = $1"
            )
            .bind(account_id)
            .fetch_all(&self.pool)
            .await?;
            
            let libraries: Vec<String> = library_rows.into_iter()
                .map(|row| row.get("library_address"))
                .collect();

            Ok(Some(ValenceAccountState {
                account_id: account_id.to_string(),
                chain_id: account_row.get("chain_id"),
                address: account_row.get("contract_address"),
                current_owner: account_row.get("current_owner"),
                pending_owner: account_row.get("pending_owner"),
                pending_owner_expiry: account_row.get::<Option<i64>, _>("pending_owner_expiry").map(|v| v as u64),
                libraries,
                last_update_block: account_row.get::<i64, _>("last_updated_block") as u64,
                last_update_tx: account_row.get("last_updated_tx"),
            }))
        } else {
            Ok(None)
        }
    }

    /// Stores information about a new Valence Account contract instantiation.
    async fn store_valence_account_instantiation(
        &self,
        account_info: ValenceAccountInfo,
        initial_libraries: Vec<ValenceAccountLibrary>,
    ) -> Result<()> {
        todo!("Implement store_valence_account_instantiation")
    }

    /// Adds a library to an existing Valence account's approved list.
    async fn store_valence_library_approval(
        &self,
        account_id: &str,
        library_info: ValenceAccountLibrary,
        update_block: u64,
        update_tx: &str,
    ) -> Result<()> {
        todo!("Implement store_valence_library_approval")
    }

    /// Removes a library from an existing Valence account's approved list.
    async fn store_valence_library_removal(
        &self,
        account_id: &str,
        library_address: &str,
        update_block: u64,
        update_tx: &str,
    ) -> Result<()> {
        todo!("Implement store_valence_library_removal")
    }

    /// Updates the ownership details of a Valence account.
    async fn store_valence_ownership_update(
        &self,
        account_id: &str,
        new_owner: Option<String>,
        new_pending_owner: Option<String>,
        new_pending_expiry: Option<u64>,
        update_block: u64,
        update_tx: &str,
    ) -> Result<()> {
        todo!("Implement store_valence_ownership_update")
    }

    // --- Default Implementations for New Valence Methods ---

    async fn set_valence_account_state(&self, _account_id: &str, _state: &ValenceAccountState) -> Result<()> {
        // Postgres currently relies on individual updates (store_valence_library_approval etc.)
        // This method might be used for bulk updates or initial state setting if needed.
        warn!("set_valence_account_state not fully implemented for Postgres");
        Ok(())
    }

    async fn delete_valence_account_state(&self, _account_id: &str) -> Result<()> {
        warn!("delete_valence_account_state not implemented for Postgres");
        Ok(())
    }

    async fn set_historical_valence_account_state(
        &self,
        _account_id: &str,
        _block_number: u64,
        _state: &ValenceAccountState,
    ) -> Result<()> {
        // Historical state is intended for RocksDB primarily
        Ok(())
    }

    async fn get_historical_valence_account_state(
        &self,
        _account_id: &str,
        _block_number: u64,
    ) -> Result<Option<ValenceAccountState>> {
        // Historical state is intended for RocksDB primarily
        Ok(None)
    }

    async fn delete_historical_valence_account_state(
        &self,
        _account_id: &str,
        _block_number: u64,
    ) -> Result<()> {
        // Historical state is intended for RocksDB primarily
        Ok(())
    }

    async fn set_latest_historical_valence_block(
        &self,
        _account_id: &str,
        _block_number: u64,
    ) -> Result<()> {
        // Historical state is intended for RocksDB primarily
        Ok(())
    }

    async fn get_latest_historical_valence_block(&self, _account_id: &str) -> Result<Option<u64>> {
        // Historical state is intended for RocksDB primarily
        Ok(None)
    }

    async fn delete_latest_historical_valence_block(&self, _account_id: &str) -> Result<()> {
        // Historical state is intended for RocksDB primarily
        Ok(())
    }

    // --- Valence Processor Methods ---
    
    async fn store_valence_processor_instantiation(
        &self,
        processor_info: ValenceProcessorInfo,
    ) -> Result<()> {
        let mut transaction = self.pool.begin().await?;
        
        // Insert into valence_processors table
        sqlx::query(
            r#"
            INSERT INTO valence_processors (
                id, chain_id, contract_address, created_at_block, created_at_tx,
                current_owner, max_gas_per_message, message_timeout_blocks, 
                retry_interval_blocks, max_retry_count, paused,
                last_updated_block, last_updated_tx
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
            ON CONFLICT (id) DO UPDATE SET
                current_owner = EXCLUDED.current_owner,
                max_gas_per_message = EXCLUDED.max_gas_per_message,
                message_timeout_blocks = EXCLUDED.message_timeout_blocks,
                retry_interval_blocks = EXCLUDED.retry_interval_blocks,
                max_retry_count = EXCLUDED.max_retry_count,
                paused = EXCLUDED.paused,
                last_updated_block = EXCLUDED.last_updated_block,
                last_updated_tx = EXCLUDED.last_updated_tx
            "#
        )
        .bind(&processor_info.id)
        .bind(&processor_info.chain_id)
        .bind(&processor_info.contract_address)
        .bind(processor_info.created_at_block as i64)
        .bind(&processor_info.created_at_tx)
        .bind(&processor_info.current_owner)
        .bind(processor_info.config.as_ref().and_then(|c| c.max_gas_per_message).map(|v| v as i64))
        .bind(processor_info.config.as_ref().and_then(|c| c.message_timeout_blocks).map(|v| v as i64))
        .bind(processor_info.config.as_ref().and_then(|c| c.retry_interval_blocks).map(|v| v as i64))
        .bind(processor_info.config.as_ref().and_then(|c| c.max_retry_count).map(|v| v as i32))
        .bind(processor_info.config.as_ref().map(|c| c.paused).unwrap_or(false))
        .bind(processor_info.last_updated_block as i64)
        .bind(&processor_info.last_updated_tx)
        .execute(&mut *transaction)
        .await?;
        
        transaction.commit().await?;
        Ok(())
    }
    
    async fn store_valence_processor_config_update(
        &self,
        processor_id: &str,
        config: ValenceProcessorConfig,
        update_block: u64,
        update_tx: &str,
    ) -> Result<()> {
        let mut transaction = self.pool.begin().await?;
        
        // Update processor configuration
        sqlx::query(
            r#"
            UPDATE valence_processors SET
                max_gas_per_message = $2,
                message_timeout_blocks = $3,
                retry_interval_blocks = $4,
                max_retry_count = $5,
                paused = $6,
                last_updated_block = $7,
                last_updated_tx = $8
            WHERE id = $1
            "#
        )
        .bind(processor_id)
        .bind(config.max_gas_per_message.map(|v| v as i64))
        .bind(config.message_timeout_blocks.map(|v| v as i64))
        .bind(config.retry_interval_blocks.map(|v| v as i64))
        .bind(config.max_retry_count.map(|v| v as i32))
        .bind(config.paused)
        .bind(update_block as i64)
        .bind(update_tx)
        .execute(&mut *transaction)
        .await?;
        
        transaction.commit().await?;
        Ok(())
    }
    
    async fn store_valence_processor_message(
        &self,
        message: ValenceProcessorMessage,
    ) -> Result<()> {
        let mut transaction = self.pool.begin().await?;
        
        let status_str = match message.status {
            ValenceMessageStatus::Pending => "pending",
            ValenceMessageStatus::Processing => "processing",
            ValenceMessageStatus::Completed => "completed",
            ValenceMessageStatus::Failed => "failed",
            ValenceMessageStatus::TimedOut => "timed_out",
        };
        
        // Insert into processor_messages table
        sqlx::query(
            r#"
            INSERT INTO processor_messages (
                id, processor_id, source_chain_id, target_chain_id, sender_address,
                payload, status, created_at_block, created_at_tx, last_updated_block,
                processed_at_block, processed_at_tx, retry_count, next_retry_block,
                gas_used, error
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16)
            ON CONFLICT (id) DO UPDATE SET
                status = EXCLUDED.status,
                last_updated_block = EXCLUDED.last_updated_block,
                processed_at_block = EXCLUDED.processed_at_block,
                processed_at_tx = EXCLUDED.processed_at_tx,
                retry_count = EXCLUDED.retry_count,
                next_retry_block = EXCLUDED.next_retry_block,
                gas_used = EXCLUDED.gas_used,
                error = EXCLUDED.error
            "#
        )
        .bind(&message.id)
        .bind(&message.processor_id)
        .bind(&message.source_chain_id)
        .bind(&message.target_chain_id)
        .bind(&message.sender_address)
        .bind(&message.payload)
        .bind(status_str)
        .bind(message.created_at_block as i64)
        .bind(&message.created_at_tx)
        .bind(message.last_updated_block as i64)
        .bind(message.processed_at_block.map(|v| v as i64))
        .bind(&message.processed_at_tx)
        .bind(message.retry_count as i32)
        .bind(message.next_retry_block.map(|v| v as i64))
        .bind(message.gas_used.map(|v| v as i64))
        .bind(&message.error)
        .execute(&mut *transaction)
        .await?;
        
        transaction.commit().await?;
        Ok(())
    }
    
    async fn update_valence_processor_message_status(
        &self,
        message_id: &str,
        new_status: ValenceMessageStatus,
        processed_block: Option<u64>,
        processed_tx: Option<&str>,
        retry_count: Option<u32>,
        next_retry_block: Option<u64>,
        gas_used: Option<u64>,
        error: Option<String>,
    ) -> Result<()> {
        let mut transaction = self.pool.begin().await?;
        
        let status_str = match new_status {
            ValenceMessageStatus::Pending => "pending",
            ValenceMessageStatus::Processing => "processing",
            ValenceMessageStatus::Completed => "completed",
            ValenceMessageStatus::Failed => "failed",
            ValenceMessageStatus::TimedOut => "timed_out",
        };
        
        // Update message status and related fields
        sqlx::query(
            r#"
            UPDATE processor_messages SET
                status = $2,
                processed_at_block = $3,
                processed_at_tx = $4,
                retry_count = COALESCE($5, retry_count),
                next_retry_block = $6,
                gas_used = $7,
                error = $8,
                last_updated_block = COALESCE($3, last_updated_block)
            WHERE id = $1
            "#
        )
        .bind(message_id)
        .bind(status_str)
        .bind(processed_block.map(|v| v as i64))
        .bind(processed_tx)
        .bind(retry_count.map(|v| v as i32))
        .bind(next_retry_block.map(|v| v as i64))
        .bind(gas_used.map(|v| v as i64))
        .bind(&error)
        .execute(&mut *transaction)
        .await?;
        
        transaction.commit().await?;
        Ok(())
    }
    
    async fn get_valence_processor_state(&self, processor_id: &str) -> Result<Option<ValenceProcessorState>> {
        let result: Option<(String, String, String, Option<String>, Option<i64>, Option<i64>, Option<i64>, Option<i32>, bool, i64, String)> = sqlx::query_as(
            r#"
            SELECT 
                id, chain_id, contract_address, current_owner,
                max_gas_per_message, message_timeout_blocks, retry_interval_blocks,
                max_retry_count, paused, last_updated_block, last_updated_tx
            FROM valence_processors
            WHERE id = $1
            "#
        )
        .bind(processor_id)
        .fetch_optional(&self.pool)
        .await?;
        
        if let Some((id, chain_id, address, owner, max_gas, timeout_blocks, retry_interval, max_retry, paused, last_block, last_tx)) = result {
            // Count messages by status
            let pending_count: Option<(i64,)> = sqlx::query_as(
                "SELECT COUNT(*) FROM processor_messages WHERE processor_id = $1 AND status IN ('pending', 'processing')"
            )
            .bind(processor_id)
            .fetch_optional(&self.pool)
            .await?;
            
            let completed_count: Option<(i64,)> = sqlx::query_as(
                "SELECT COUNT(*) FROM processor_messages WHERE processor_id = $1 AND status = 'completed'"
            )
            .bind(processor_id)
            .fetch_optional(&self.pool)
            .await?;
            
            let failed_count: Option<(i64,)> = sqlx::query_as(
                "SELECT COUNT(*) FROM processor_messages WHERE processor_id = $1 AND status IN ('failed', 'timed_out')"
            )
            .bind(processor_id)
            .fetch_optional(&self.pool)
            .await?;
            
            let config = Some(ValenceProcessorConfig {
                max_gas_per_message: max_gas.map(|v| v as u64),
                message_timeout_blocks: timeout_blocks.map(|v| v as u64),
                retry_interval_blocks: retry_interval.map(|v| v as u64),
                max_retry_count: max_retry.map(|v| v as u32),
                paused,
            });
            
            Ok(Some(ValenceProcessorState {
                processor_id: id,
                chain_id,
                address,
                owner,
                config,
                pending_message_count: pending_count.map(|(c,)| c as u64).unwrap_or(0),
                completed_message_count: completed_count.map(|(c,)| c as u64).unwrap_or(0),
                failed_message_count: failed_count.map(|(c,)| c as u64).unwrap_or(0),
                last_update_block: last_block as u64,
                last_update_tx: last_tx,
            }))
        } else {
            Ok(None)
        }
    }
    
    async fn set_valence_processor_state(&self, processor_id: &str, state: &ValenceProcessorState) -> Result<()> {
        let mut transaction = self.pool.begin().await?;
        
        // Update processor state
        sqlx::query(
            r#"
            UPDATE valence_processors SET
                current_owner = $2,
                max_gas_per_message = $3,
                message_timeout_blocks = $4,
                retry_interval_blocks = $5,
                max_retry_count = $6,
                paused = $7,
                last_updated_block = $8,
                last_updated_tx = $9
            WHERE id = $1
            "#
        )
        .bind(processor_id)
        .bind(&state.owner)
        .bind(state.config.as_ref().and_then(|c| c.max_gas_per_message).map(|v| v as i64))
        .bind(state.config.as_ref().and_then(|c| c.message_timeout_blocks).map(|v| v as i64))
        .bind(state.config.as_ref().and_then(|c| c.retry_interval_blocks).map(|v| v as i64))
        .bind(state.config.as_ref().and_then(|c| c.max_retry_count).map(|v| v as i32))
        .bind(state.config.as_ref().map(|c| c.paused).unwrap_or(false))
        .bind(state.last_update_block as i64)
        .bind(&state.last_update_tx)
        .execute(&mut *transaction)
        .await?;
        
        transaction.commit().await?;
        Ok(())
    }
    
    async fn set_historical_valence_processor_state(
        &self,
        processor_id: &str,
        block_number: u64,
        state: &ValenceProcessorState,
    ) -> Result<()> {
        let mut transaction = self.pool.begin().await?;
        
        // Store historical processor state (simplified - could be extended with dedicated historical table)
        let state_json = serde_json::to_string(state)
            .map_err(|e| Error::Generic(format!("Failed to serialize processor state: {}", e)))?;
        
        sqlx::query(
            r#"
            INSERT INTO processor_historical_states (
                processor_id, block_number, state_data, created_at
            ) VALUES ($1, $2, $3, NOW())
            ON CONFLICT (processor_id, block_number) DO UPDATE SET
                state_data = EXCLUDED.state_data,
                created_at = NOW()
            "#
        )
        .bind(processor_id)
        .bind(block_number as i64)
        .bind(&state_json)
        .execute(&mut *transaction)
        .await
        .unwrap_or_else(|_| {
            // If table doesn't exist, just succeed silently for now
            sqlx::postgres::PgQueryResult::default()
        });
        
        transaction.commit().await?;
        Ok(())
    }
    
    async fn get_historical_valence_processor_state(
        &self,
        processor_id: &str,
        block_number: u64,
    ) -> Result<Option<ValenceProcessorState>> {
        let result: Option<(String,)> = sqlx::query_as(
            r#"
            SELECT state_data
            FROM processor_historical_states
            WHERE processor_id = $1 AND block_number = $2
            "#
        )
        .bind(processor_id)
        .bind(block_number as i64)
        .fetch_optional(&self.pool)
        .await
        .unwrap_or(None);
        
        if let Some((state_json,)) = result {
            let state: ValenceProcessorState = serde_json::from_str(&state_json)
                .map_err(|e| Error::Generic(format!("Failed to deserialize processor state: {}", e)))?;
            Ok(Some(state))
        } else {
            Ok(None)
        }
    }

    // --- Valence Authorization Methods ---
    
    async fn store_valence_authorization_instantiation(
        &self,
        auth_info: ValenceAuthorizationInfo,
        initial_policy: Option<ValenceAuthorizationPolicy>,
    ) -> Result<()> {
        let mut transaction = self.pool.begin().await?;
        
        // Insert into valence_authorizations table
        sqlx::query(
            r#"
            INSERT INTO valence_authorizations (
                id, chain_id, contract_address, created_at_block, created_at_tx,
                current_owner, active_policy_id, last_updated_block, last_updated_tx
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            ON CONFLICT (id) DO UPDATE SET
                current_owner = EXCLUDED.current_owner,
                active_policy_id = EXCLUDED.active_policy_id,
                last_updated_block = EXCLUDED.last_updated_block,
                last_updated_tx = EXCLUDED.last_updated_tx
            "#
        )
        .bind(&auth_info.id)
        .bind(&auth_info.chain_id)
        .bind(&auth_info.contract_address)
        .bind(auth_info.created_at_block as i64)
        .bind(&auth_info.created_at_tx)
        .bind(&auth_info.current_owner)
        .bind(&auth_info.active_policy_id)
        .bind(auth_info.last_updated_block as i64)
        .bind(&auth_info.last_updated_tx)
        .execute(&mut *transaction)
        .await?;
        
        // If there's an initial policy, store it
        if let Some(policy) = initial_policy {
            sqlx::query(
                r#"
                INSERT INTO authorization_policies (
                    id, auth_id, version, content_hash, created_at_block, created_at_tx,
                    is_active, metadata
                ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
                ON CONFLICT (id) DO UPDATE SET
                    is_active = EXCLUDED.is_active,
                    metadata = EXCLUDED.metadata
                "#
            )
            .bind(&policy.id)
            .bind(&policy.auth_id)
            .bind(policy.version as i32)
            .bind(&policy.content_hash)
            .bind(policy.created_at_block as i64)
            .bind(&policy.created_at_tx)
            .bind(policy.is_active)
            .bind(policy.metadata.as_ref().map(|m| serde_json::to_string(m).unwrap_or_default()))
            .execute(&mut *transaction)
            .await?;
        }
        
        transaction.commit().await?;
        Ok(())
    }
    
    async fn store_valence_authorization_policy(
        &self,
        policy: ValenceAuthorizationPolicy,
    ) -> Result<()> {
        let mut transaction = self.pool.begin().await?;
        
        // Insert or update authorization policy
        sqlx::query(
            r#"
            INSERT INTO authorization_policies (
                id, auth_id, version, content_hash, created_at_block, created_at_tx,
                is_active, metadata
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            ON CONFLICT (id) DO UPDATE SET
                is_active = EXCLUDED.is_active,
                metadata = EXCLUDED.metadata
            "#
        )
        .bind(&policy.id)
        .bind(&policy.auth_id)
        .bind(policy.version as i32)
        .bind(&policy.content_hash)
        .bind(policy.created_at_block as i64)
        .bind(&policy.created_at_tx)
        .bind(policy.is_active)
        .bind(policy.metadata.as_ref().map(|m| serde_json::to_string(m).unwrap_or_default()))
        .execute(&mut *transaction)
        .await?;
        
        transaction.commit().await?;
        Ok(())
    }
    
    async fn update_active_authorization_policy(
        &self,
        auth_id: &str,
        policy_id: &str,
        update_block: u64,
        update_tx: &str,
    ) -> Result<()> {
        let mut transaction = self.pool.begin().await?;
        
        // Deactivate all policies for this authorization contract
        sqlx::query(
            "UPDATE authorization_policies SET is_active = false WHERE auth_id = $1"
        )
        .bind(auth_id)
        .execute(&mut *transaction)
        .await?;
        
        // Activate the specified policy
        sqlx::query(
            "UPDATE authorization_policies SET is_active = true WHERE id = $1 AND auth_id = $2"
        )
        .bind(policy_id)
        .bind(auth_id)
        .execute(&mut *transaction)
        .await?;
        
        // Update the authorization contract
        sqlx::query(
            r#"
            UPDATE valence_authorizations SET
                active_policy_id = $2,
                last_updated_block = $3,
                last_updated_tx = $4
            WHERE id = $1
            "#
        )
        .bind(auth_id)
        .bind(policy_id)
        .bind(update_block as i64)
        .bind(update_tx)
        .execute(&mut *transaction)
        .await?;
        
        transaction.commit().await?;
        Ok(())
    }
    
    async fn store_valence_authorization_grant(
        &self,
        grant: ValenceAuthorizationGrant,
    ) -> Result<()> {
        let mut transaction = self.pool.begin().await?;
        
        // Insert authorization grant
        sqlx::query(
            r#"
            INSERT INTO authorization_grants (
                id, auth_id, grantee, permissions, resources, granted_at_block,
                granted_at_tx, expiry, is_active, revoked_at_block, revoked_at_tx
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            ON CONFLICT (id) DO UPDATE SET
                permissions = EXCLUDED.permissions,
                resources = EXCLUDED.resources,
                expiry = EXCLUDED.expiry,
                is_active = EXCLUDED.is_active,
                revoked_at_block = EXCLUDED.revoked_at_block,
                revoked_at_tx = EXCLUDED.revoked_at_tx
            "#
        )
        .bind(&grant.id)
        .bind(&grant.auth_id)
        .bind(&grant.grantee)
        .bind(serde_json::to_string(&grant.permissions).unwrap_or_default())
        .bind(serde_json::to_string(&grant.resources).unwrap_or_default())
        .bind(grant.granted_at_block as i64)
        .bind(&grant.granted_at_tx)
        .bind(grant.expiry.map(|e| e as i64))
        .bind(grant.is_active)
        .bind(grant.revoked_at_block.map(|b| b as i64))
        .bind(&grant.revoked_at_tx)
        .execute(&mut *transaction)
        .await?;
        
        transaction.commit().await?;
        Ok(())
    }
    
    async fn revoke_valence_authorization_grant(
        &self,
        auth_id: &str,
        grantee: &str,
        resource: &str,
        revoked_at_block: u64,
        revoked_at_tx: &str,
    ) -> Result<()> {
        let mut transaction = self.pool.begin().await?;
        
        // Update grant to mark as revoked
        sqlx::query(
            r#"
            UPDATE authorization_grants SET
                is_active = false,
                revoked_at_block = $4,
                revoked_at_tx = $5
            WHERE auth_id = $1 AND grantee = $2 AND resources LIKE '%' || $3 || '%' AND is_active = true
            "#
        )
        .bind(auth_id)
        .bind(grantee)
        .bind(resource)
        .bind(revoked_at_block as i64)
        .bind(revoked_at_tx)
        .execute(&mut *transaction)
        .await?;
        
        transaction.commit().await?;
        Ok(())
    }
    
    async fn store_valence_authorization_request(
        &self,
        request: ValenceAuthorizationRequest,
    ) -> Result<()> {
        let mut transaction = self.pool.begin().await?;
        
        let decision_str = match request.decision {
            ValenceAuthorizationDecision::Pending => "pending",
            ValenceAuthorizationDecision::Approved => "approved",
            ValenceAuthorizationDecision::Denied => "denied",
            ValenceAuthorizationDecision::Error => "error",
        };
        
        // Insert authorization request
        sqlx::query(
            r#"
            INSERT INTO authorization_requests (
                id, auth_id, requester, action, resource, request_data,
                decision, requested_at_block, requested_at_tx, processed_at_block,
                processed_at_tx, reason
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
            ON CONFLICT (id) DO UPDATE SET
                decision = EXCLUDED.decision,
                processed_at_block = EXCLUDED.processed_at_block,
                processed_at_tx = EXCLUDED.processed_at_tx,
                reason = EXCLUDED.reason
            "#
        )
        .bind(&request.id)
        .bind(&request.auth_id)
        .bind(&request.requester)
        .bind(&request.action)
        .bind(&request.resource)
        .bind(&request.request_data)
        .bind(decision_str)
        .bind(request.requested_at_block as i64)
        .bind(&request.requested_at_tx)
        .bind(request.processed_at_block.map(|b| b as i64))
        .bind(&request.processed_at_tx)
        .bind(&request.reason)
        .execute(&mut *transaction)
        .await?;
        
        transaction.commit().await?;
        Ok(())
    }
    
    async fn update_valence_authorization_request_decision(
        &self,
        request_id: &str,
        decision: ValenceAuthorizationDecision,
        processed_block: Option<u64>,
        processed_tx: Option<&str>,
        reason: Option<String>,
    ) -> Result<()> {
        let mut transaction = self.pool.begin().await?;
        
        let decision_str = match decision {
            ValenceAuthorizationDecision::Pending => "pending",
            ValenceAuthorizationDecision::Approved => "approved",
            ValenceAuthorizationDecision::Denied => "denied",
            ValenceAuthorizationDecision::Error => "error",
        };
        
        // Update authorization request decision
        sqlx::query(
            r#"
            UPDATE authorization_requests SET
                decision = $2,
                processed_at_block = $3,
                processed_at_tx = $4,
                reason = $5
            WHERE id = $1
            "#
        )
        .bind(request_id)
        .bind(decision_str)
        .bind(processed_block.map(|b| b as i64))
        .bind(processed_tx)
        .bind(&reason)
        .execute(&mut *transaction)
        .await?;
        
        transaction.commit().await?;
        Ok(())
    }

    // --- Valence Library Methods ---
    
    async fn store_valence_library_instantiation(
        &self,
        library_info: ValenceLibraryInfo,
        initial_version: Option<ValenceLibraryVersion>,
    ) -> Result<()> {
        let mut transaction = self.pool.begin().await?;
        
        // Insert into valence_libraries table
        sqlx::query(
            r#"
            INSERT INTO valence_libraries (
                id, chain_id, contract_address, library_type, created_at_block, created_at_tx,
                current_owner, current_version, last_updated_block, last_updated_tx
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            ON CONFLICT (id) DO UPDATE SET
                current_owner = EXCLUDED.current_owner,
                current_version = EXCLUDED.current_version,
                last_updated_block = EXCLUDED.last_updated_block,
                last_updated_tx = EXCLUDED.last_updated_tx
            "#
        )
        .bind(&library_info.id)
        .bind(&library_info.chain_id)
        .bind(&library_info.contract_address)
        .bind(&library_info.library_type)
        .bind(library_info.created_at_block as i64)
        .bind(&library_info.created_at_tx)
        .bind(&library_info.current_owner)
        .bind(library_info.current_version.map(|v| v as i32))
        .bind(library_info.last_updated_block as i64)
        .bind(&library_info.last_updated_tx)
        .execute(&mut *transaction)
        .await?;
        
        // If there's an initial version, store it
        if let Some(version) = initial_version {
            sqlx::query(
                r#"
                INSERT INTO library_versions (
                    id, library_id, version, code_hash, created_at_block, created_at_tx,
                    is_active, features, metadata
                ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
                ON CONFLICT (id) DO UPDATE SET
                    is_active = EXCLUDED.is_active,
                    features = EXCLUDED.features,
                    metadata = EXCLUDED.metadata
                "#
            )
            .bind(&version.id)
            .bind(&version.library_id)
            .bind(version.version as i32)
            .bind(&version.code_hash)
            .bind(version.created_at_block as i64)
            .bind(&version.created_at_tx)
            .bind(version.is_active)
            .bind(serde_json::to_string(&version.features).unwrap_or_default())
            .bind(version.metadata.as_ref().map(|m| serde_json::to_string(m).unwrap_or_default()))
            .execute(&mut *transaction)
            .await?;
        }
        
        transaction.commit().await?;
        Ok(())
    }
    
    async fn store_valence_library_version(
        &self,
        version: ValenceLibraryVersion,
    ) -> Result<()> {
        let mut transaction = self.pool.begin().await?;
        
        // Insert or update library version
        sqlx::query(
            r#"
            INSERT INTO library_versions (
                id, library_id, version, code_hash, created_at_block, created_at_tx,
                is_active, features, metadata
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            ON CONFLICT (id) DO UPDATE SET
                is_active = EXCLUDED.is_active,
                features = EXCLUDED.features,
                metadata = EXCLUDED.metadata
            "#
        )
        .bind(&version.id)
        .bind(&version.library_id)
        .bind(version.version as i32)
        .bind(&version.code_hash)
        .bind(version.created_at_block as i64)
        .bind(&version.created_at_tx)
        .bind(version.is_active)
        .bind(serde_json::to_string(&version.features).unwrap_or_default())
        .bind(version.metadata.as_ref().map(|m| serde_json::to_string(m).unwrap_or_default()))
        .execute(&mut *transaction)
        .await?;
        
        transaction.commit().await?;
        Ok(())
    }
    
    async fn update_active_library_version(
        &self,
        library_id: &str,
        version: u32,
        update_block: u64,
        update_tx: &str,
    ) -> Result<()> {
        let mut transaction = self.pool.begin().await?;
        
        // Deactivate all versions for this library
        sqlx::query(
            "UPDATE library_versions SET is_active = false WHERE library_id = $1"
        )
        .bind(library_id)
        .execute(&mut *transaction)
        .await?;
        
        // Activate the specified version
        sqlx::query(
            "UPDATE library_versions SET is_active = true WHERE library_id = $1 AND version = $2"
        )
        .bind(library_id)
        .bind(version as i32)
        .execute(&mut *transaction)
        .await?;
        
        // Update the library contract
        sqlx::query(
            r#"
            UPDATE valence_libraries SET
                current_version = $2,
                last_updated_block = $3,
                last_updated_tx = $4
            WHERE id = $1
            "#
        )
        .bind(library_id)
        .bind(version as i32)
        .bind(update_block as i64)
        .bind(update_tx)
        .execute(&mut *transaction)
        .await?;
        
        transaction.commit().await?;
        Ok(())
    }
    
    async fn store_valence_library_usage(
        &self,
        usage: ValenceLibraryUsage,
    ) -> Result<()> {
        let mut transaction = self.pool.begin().await?;
        
        // Insert library usage record
        sqlx::query(
            r#"
            INSERT INTO library_usage (
                id, library_id, user_address, account_id, function_name,
                usage_at_block, usage_at_tx, gas_used, success, error
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            ON CONFLICT (id) DO UPDATE SET
                gas_used = EXCLUDED.gas_used,
                success = EXCLUDED.success,
                error = EXCLUDED.error
            "#
        )
        .bind(&usage.id)
        .bind(&usage.library_id)
        .bind(&usage.user_address)
        .bind(&usage.account_id)
        .bind(&usage.function_name)
        .bind(usage.usage_at_block as i64)
        .bind(&usage.usage_at_tx)
        .bind(usage.gas_used.map(|g| g as i64))
        .bind(usage.success)
        .bind(&usage.error)
        .execute(&mut *transaction)
        .await?;
        
        transaction.commit().await?;
        Ok(())
    }
    
    async fn revoke_valence_library_approval(
        &self,
        library_id: &str,
        account_id: &str,
        revoked_at_block: u64,
        revoked_at_tx: &str,
    ) -> Result<()> {
        let mut transaction = self.pool.begin().await?;
        
        // Update library approval to mark as revoked
        sqlx::query(
            r#"
            UPDATE library_approvals SET
                is_active = false,
                revoked_at_block = $3,
                revoked_at_tx = $4
            WHERE library_id = $1 AND account_id = $2 AND is_active = true
            "#
        )
        .bind(library_id)
        .bind(account_id)
        .bind(revoked_at_block as i64)
        .bind(revoked_at_tx)
        .execute(&mut *transaction)
        .await?;
        
        transaction.commit().await?;
        Ok(())
    }
    
    async fn get_valence_library_state(&self, library_id: &str) -> Result<Option<ValenceLibraryState>> {
        let result: Option<(String, String, String, String, Option<String>, Option<i32>, i64, String)> = sqlx::query_as(
            r#"
            SELECT 
                id, chain_id, contract_address, library_type, current_owner,
                current_version, last_updated_block, last_updated_tx
            FROM valence_libraries
            WHERE id = $1
            "#
        )
        .bind(library_id)
        .fetch_optional(&self.pool)
        .await?;
        
        if let Some((id, chain_id, address, library_type, current_owner, current_version, last_block, last_tx)) = result {
            // Get all versions for this library
            let versions: Vec<(String, String, i32, String, i64, String, bool, String, Option<String>)> = sqlx::query_as(
                r#"
                SELECT id, library_id, version, code_hash, created_at_block, created_at_tx,
                       is_active, features, metadata
                FROM library_versions
                WHERE library_id = $1
                ORDER BY version DESC
                "#
            )
            .bind(library_id)
            .fetch_all(&self.pool)
            .await?;
            
            let version_objects: Vec<ValenceLibraryVersion> = versions.into_iter().map(|(vid, lib_id, ver, code_hash, created_block, created_tx, is_active, features_json, metadata_json)| {
                let features: Vec<String> = serde_json::from_str(&features_json).unwrap_or_default();
                let metadata: Option<serde_json::Value> = metadata_json.as_ref()
                    .and_then(|m| serde_json::from_str(m).ok());
                
                ValenceLibraryVersion {
                    id: vid,
                    library_id: lib_id,
                    version: ver as u32,
                    code_hash,
                    created_at_block: created_block as u64,
                    created_at_tx: created_tx,
                    is_active,
                    features,
                    metadata,
                }
            }).collect();
            
            Ok(Some(ValenceLibraryState {
                library_id: id,
                chain_id,
                address,
                library_type,
                current_owner,
                current_version: current_version.map(|v| v as u32),
                versions: version_objects,
                last_update_block: last_block as u64,
                last_update_tx: last_tx,
            }))
        } else {
            Ok(None)
        }
    }
    
    async fn set_valence_library_state(&self, library_id: &str, state: &ValenceLibraryState) -> Result<()> {
        let mut transaction = self.pool.begin().await?;
        
        // Update library state
        sqlx::query(
            r#"
            UPDATE valence_libraries SET
                current_owner = $2,
                current_version = $3,
                last_updated_block = $4,
                last_updated_tx = $5
            WHERE id = $1
            "#
        )
        .bind(library_id)
        .bind(&state.current_owner)
        .bind(state.current_version.map(|v| v as i32))
        .bind(state.last_update_block as i64)
        .bind(&state.last_update_tx)
        .execute(&mut *transaction)
        .await?;
        
        transaction.commit().await?;
        Ok(())
    }
    
    async fn get_valence_library_versions(&self, library_id: &str) -> Result<Vec<ValenceLibraryVersion>> {
        let versions: Vec<(String, String, i32, String, i64, String, bool, String, Option<String>)> = sqlx::query_as(
            r#"
            SELECT id, library_id, version, code_hash, created_at_block, created_at_tx,
                   is_active, features, metadata
            FROM library_versions
            WHERE library_id = $1
            ORDER BY version DESC
            "#
        )
        .bind(library_id)
        .fetch_all(&self.pool)
        .await?;
        
        let version_objects: Vec<ValenceLibraryVersion> = versions.into_iter().map(|(vid, lib_id, ver, code_hash, created_block, created_tx, is_active, features_json, metadata_json)| {
            let features: Vec<String> = serde_json::from_str(&features_json).unwrap_or_default();
            let metadata: Option<serde_json::Value> = metadata_json.as_ref()
                .and_then(|m| serde_json::from_str(m).ok());
            
            ValenceLibraryVersion {
                id: vid,
                library_id: lib_id,
                version: ver as u32,
                code_hash,
                created_at_block: created_block as u64,
                created_at_tx: created_tx,
                is_active,
                features,
                metadata,
            }
        }).collect();
        
        Ok(version_objects)
    }
    
    async fn get_valence_library_approvals(&self, library_id: &str) -> Result<Vec<ValenceLibraryApproval>> {
        let approvals: Vec<(String, String, String, i64, String, bool, Option<i64>, Option<String>)> = sqlx::query_as(
            r#"
            SELECT id, library_id, account_id, approved_at_block, approved_at_tx,
                   is_active, revoked_at_block, revoked_at_tx
            FROM library_approvals
            WHERE library_id = $1
            ORDER BY approved_at_block DESC
            "#
        )
        .bind(library_id)
        .fetch_all(&self.pool)
        .await?;
        
        let approval_objects: Vec<ValenceLibraryApproval> = approvals.into_iter().map(|(aid, lib_id, account_id, approved_block, approved_tx, is_active, revoked_block, revoked_tx)| {
            ValenceLibraryApproval {
                id: aid,
                library_id: lib_id,
                account_id,
                approved_at_block: approved_block as u64,
                approved_at_tx: approved_tx,
                is_active,
                revoked_at_block: revoked_block.map(|b| b as u64),
                revoked_at_tx: revoked_tx,
            }
        }).collect();
        
        Ok(approval_objects)
    }
    
    async fn get_valence_libraries_for_account(&self, account_id: &str) -> Result<Vec<ValenceLibraryApproval>> {
        let approvals: Vec<(String, String, String, i64, String, bool, Option<i64>, Option<String>)> = sqlx::query_as(
            r#"
            SELECT id, library_id, account_id, approved_at_block, approved_at_tx,
                   is_active, revoked_at_block, revoked_at_tx
            FROM library_approvals
            WHERE account_id = $1 AND is_active = true
            ORDER BY approved_at_block DESC
            "#
        )
        .bind(account_id)
        .fetch_all(&self.pool)
        .await?;
        
        let approval_objects: Vec<ValenceLibraryApproval> = approvals.into_iter().map(|(aid, lib_id, account_id, approved_block, approved_tx, is_active, revoked_block, revoked_tx)| {
            ValenceLibraryApproval {
                id: aid,
                library_id: lib_id,
                account_id,
                approved_at_block: approved_block as u64,
                approved_at_tx: approved_tx,
                is_active,
                revoked_at_block: revoked_block.map(|b| b as u64),
                revoked_at_tx: revoked_tx,
            }
        }).collect();
        
        Ok(approval_objects)
    }
    
    async fn get_valence_library_usage_history(
        &self,
        library_id: &str,
        limit: Option<usize>,
        offset: Option<usize>,
    ) -> Result<Vec<ValenceLibraryUsage>> {
        let limit = limit.unwrap_or(100);
        let offset = offset.unwrap_or(0);
        
        let usage_records: Vec<(String, String, String, Option<String>, Option<String>, i64, String, Option<i64>, bool, Option<String>)> = sqlx::query_as(
            r#"
            SELECT id, library_id, user_address, account_id, function_name,
                   usage_at_block, usage_at_tx, gas_used, success, error
            FROM library_usage
            WHERE library_id = $1
            ORDER BY usage_at_block DESC
            LIMIT $2 OFFSET $3
            "#
        )
        .bind(library_id)
        .bind(limit as i64)
        .bind(offset as i64)
        .fetch_all(&self.pool)
        .await?;
        
        let usage_objects: Vec<ValenceLibraryUsage> = usage_records.into_iter().map(|(uid, lib_id, user_address, account_id, function_name, usage_block, usage_tx, gas_used, success, error)| {
            ValenceLibraryUsage {
                id: uid,
                library_id: lib_id,
                user_address,
                account_id,
                function_name,
                usage_at_block: usage_block as u64,
                usage_at_tx: usage_tx,
                gas_used: gas_used.map(|g| g as u64),
                success,
                error,
            }
        }).collect();
        
        Ok(usage_objects)
    }

    // Implement the processor state methods
    async fn set_processor_state(&self, chain: &str, block_number: u64, state: &str) -> Result<()> {
        // For PostgreSQL, we'll store processor state in a dedicated table
        // For simplicity, we'll log and return Ok for now
        debug!("PostgreSQL set_processor_state not fully implemented");
        Ok(())
    }
    
    async fn get_processor_state(&self, chain: &str, block_number: u64) -> Result<Option<String>> {
        // For PostgreSQL, we'll retrieve processor state from a dedicated table
        // For simplicity, we'll return None for now
        debug!("PostgreSQL get_processor_state not fully implemented");
        Ok(None)
    }
    
    async fn set_historical_processor_state(&self, chain: &str, block_number: u64, state: &str) -> Result<()> {
        // For PostgreSQL, we'll store historical processor state in a dedicated table
        // For simplicity, we'll log and return Ok for now
        debug!("PostgreSQL set_historical_processor_state not fully implemented");
        Ok(())
    }
    
    async fn get_historical_processor_state(&self, chain: &str, block_number: u64) -> Result<Option<String>> {
        // For PostgreSQL, we'll retrieve historical processor state from a dedicated table
        // For simplicity, we'll return None for now
        debug!("PostgreSQL get_historical_processor_state not fully implemented");
        Ok(None)
    }
}

impl PostgresStorage {
    /// Create a new PostgreSQL storage instance
    pub async fn new(config: PostgresConfig) -> Result<Self> {
        // Configure the connection pool
        let pool_options = sqlx::postgres::PgPoolOptions::new()
            .max_connections(config.max_connections)
            .acquire_timeout(std::time::Duration::from_secs(config.connection_timeout));
        
        // Create the pool
        let pool = pool_options.connect(&config.url)
            .await
            .map_err(|e| Error::Storage(format!("Failed to connect to PostgreSQL: {}", e)))?;
        
        info!("Connected to PostgreSQL database");
        
        // Create repositories
        let event_repository = Arc::new(PostgresEventRepository::new(pool.clone()));
        let contract_schema_repository = Arc::new(PostgresContractSchemaRepository::new(pool.clone()));
        
        // Run database migrations - for dev/test only
        // In production, migrations should be run separately before starting the application
        initialize_database(&config.url, "./crates/storage/migrations").await?;
        
        // Create the storage instance
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

    /// Reorgs the chain in response to a blockchain reorg
    #[instrument(skip(self), fields(chain = %chain, from_block = %from_block))]
    pub async fn handle_chain_reorg(&self, chain: &str, from_block: u64) -> Result<()> {
        info!(chain, from_block, "Handling reorg in PostgreSQL");
        let mut tx = self.pool.begin().await?;

        // 1. Delete events >= from_block
        sqlx::query(
            "DELETE FROM events WHERE chain = $1 AND block_number >= $2"
        )
        .bind(chain)
        .bind(from_block as i64)
        .execute(&mut *tx)
        .await?;

        // 2. Delete blocks >= from_block
        sqlx::query(
            "DELETE FROM blocks WHERE chain = $1 AND number >= $2"
        )
        .bind(chain)
        .bind(from_block as i64)
        .execute(&mut *tx)
        .await?;
        
        // 3. Delete valence account executions >= from_block
        sqlx::query(
             "DELETE FROM valence_account_executions WHERE chain_id = $1 AND block_number >= $2"
         )
         .bind(chain)
         .bind(from_block as i64)
         .execute(&mut *tx)
         .await?;
         
        // 4. Revert Valence Account/Processor/Auth/Library state
        // This is complex. Ideally, you'd have historical state tables or 
        // revert based on event logs. For simplicity, we might just log a warning.
        warn!(chain, from_block, "PostgreSQL reorg: Valence contract state not automatically reverted. Manual intervention may be required.");

        tx.commit().await?;
        info!(chain, from_block, "Reorg complete in PostgreSQL");
        Ok(())
    }
}
