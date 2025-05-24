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
#[cfg(feature = "postgres")]
use indexer_pipeline::{Error, Result, BlockStatus};
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
        _processor_info: ValenceProcessorInfo,
    ) -> Result<()> {
        // Simplified implementation for compilation
        Ok(())
    }
    
    async fn store_valence_processor_config_update(
        &self,
        _processor_id: &str,
        _config: ValenceProcessorConfig,
        _update_block: u64,
        _update_tx: &str,
    ) -> Result<()> {
        // Simplified implementation for compilation
        Ok(())
    }
    
    async fn store_valence_processor_message(
        &self,
        _message: ValenceProcessorMessage,
    ) -> Result<()> {
        // Simplified implementation for compilation
        Ok(())
    }
    
    async fn update_valence_processor_message_status(
        &self,
        _message_id: &str,
        _new_status: ValenceMessageStatus,
        _processed_block: Option<u64>,
        _processed_tx: Option<&str>,
        _retry_count: Option<u32>,
        _next_retry_block: Option<u64>,
        _gas_used: Option<u64>,
        _error: Option<String>,
    ) -> Result<()> {
        // Simplified implementation for compilation
        Ok(())
    }
    
    async fn get_valence_processor_state(&self, _processor_id: &str) -> Result<Option<ValenceProcessorState>> {
        // Simplified implementation for compilation
        Ok(None)
    }
    
    async fn set_valence_processor_state(&self, _processor_id: &str, _state: &ValenceProcessorState) -> Result<()> {
        // Simplified implementation for compilation
        Ok(())
    }
    
    async fn set_historical_valence_processor_state(
        &self,
        _processor_id: &str,
        _block_number: u64,
        _state: &ValenceProcessorState,
    ) -> Result<()> {
        // Simplified implementation for compilation
        Ok(())
    }
    
    async fn get_historical_valence_processor_state(
        &self,
        _processor_id: &str,
        _block_number: u64,
    ) -> Result<Option<ValenceProcessorState>> {
        // Simplified implementation for compilation
        Ok(None)
    }
    
    // --- Valence Authorization Methods ---
    
    async fn store_valence_authorization_instantiation(
        &self,
        _auth_info: ValenceAuthorizationInfo,
        _initial_policy: Option<ValenceAuthorizationPolicy>,
    ) -> Result<()> {
        // Simplified implementation for compilation
        Ok(())
    }
    
    async fn store_valence_authorization_policy(
        &self,
        _policy: ValenceAuthorizationPolicy,
    ) -> Result<()> {
        // Simplified implementation for compilation
        Ok(())
    }
    
    async fn update_active_authorization_policy(
        &self,
        _auth_id: &str,
        _policy_id: &str,
        _update_block: u64,
        _update_tx: &str,
    ) -> Result<()> {
        // Simplified implementation for compilation
        Ok(())
    }
    
    async fn store_valence_authorization_grant(
        &self,
        _grant: ValenceAuthorizationGrant,
    ) -> Result<()> {
        // Simplified implementation for compilation
        Ok(())
    }
    
    async fn revoke_valence_authorization_grant(
        &self,
        _auth_id: &str,
        _grantee: &str,
        _resource: &str,
        _revoked_at_block: u64,
        _revoked_at_tx: &str,
    ) -> Result<()> {
        // Simplified implementation for compilation
        Ok(())
    }
    
    async fn store_valence_authorization_request(
        &self,
        _request: ValenceAuthorizationRequest,
    ) -> Result<()> {
        // Simplified implementation for compilation
        Ok(())
    }
    
    async fn update_valence_authorization_request_decision(
        &self,
        _request_id: &str,
        _decision: ValenceAuthorizationDecision,
        _processed_block: Option<u64>,
        _processed_tx: Option<&str>,
        _reason: Option<String>,
    ) -> Result<()> {
        // Simplified implementation for compilation
        Ok(())
    }

    // --- Valence Library Methods ---
    
    async fn store_valence_library_instantiation(
        &self,
        _library_info: ValenceLibraryInfo,
        _initial_version: Option<ValenceLibraryVersion>,
    ) -> Result<()> {
        // Simplified implementation for compilation
        Ok(())
    }
    
    async fn store_valence_library_version(
        &self,
        _version: ValenceLibraryVersion,
    ) -> Result<()> {
        // Simplified implementation for compilation
        Ok(())
    }
    
    async fn update_active_library_version(
        &self,
        _library_id: &str,
        _version: u32,
        _update_block: u64,
        _update_tx: &str,
    ) -> Result<()> {
        // Simplified implementation for compilation
        Ok(())
    }
    
    async fn store_valence_library_usage(
        &self,
        _usage: ValenceLibraryUsage,
    ) -> Result<()> {
        // Simplified implementation for compilation
        Ok(())
    }
    
    async fn revoke_valence_library_approval(
        &self,
        _library_id: &str,
        _account_id: &str,
        _revoked_at_block: u64,
        _revoked_at_tx: &str,
    ) -> Result<()> {
        // Simplified implementation for compilation
        Ok(())
    }
    
    async fn get_valence_library_state(&self, _library_id: &str) -> Result<Option<ValenceLibraryState>> {
        // Simplified implementation for compilation
        Ok(None)
    }
    
    async fn set_valence_library_state(&self, _library_id: &str, _state: &ValenceLibraryState) -> Result<()> {
        // Simplified implementation for compilation
        Ok(())
    }
    
    async fn get_valence_library_versions(&self, _library_id: &str) -> Result<Vec<ValenceLibraryVersion>> {
        // Simplified implementation for compilation
        Ok(Vec::new())
    }
    
    async fn get_valence_library_approvals(&self, _library_id: &str) -> Result<Vec<ValenceLibraryApproval>> {
        // Simplified implementation for compilation
        Ok(Vec::new())
    }
    
    async fn get_valence_libraries_for_account(&self, _account_id: &str) -> Result<Vec<ValenceLibraryApproval>> {
        // Simplified implementation for compilation
        Ok(Vec::new())
    }
    
    async fn get_valence_library_usage_history(
        &self,
        _library_id: &str,
        _limit: Option<usize>,
        _offset: Option<usize>,
    ) -> Result<Vec<ValenceLibraryUsage>> {
        // Simplified implementation for compilation
        Ok(Vec::new())
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
