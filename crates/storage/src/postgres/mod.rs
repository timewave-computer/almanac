/// PostgreSQL module and storage implementation
#[cfg(feature = "postgres")]
use crate::{
    ValenceProcessorInfo, ValenceProcessorConfig, ValenceProcessorMessage, ValenceMessageStatus,
    ValenceProcessorState, ValenceAuthorizationInfo, ValenceAuthorizationPolicy, ValenceAuthorizationGrant,
    ValenceAuthorizationRequest, ValenceAuthorizationDecision, ValenceLibraryInfo, ValenceLibraryVersion,
    ValenceLibraryUsage, ValenceLibraryState, ValenceLibraryApproval
};
#[cfg(feature = "postgres")]
use tracing::{debug, info, warn};
#[cfg(feature = "postgres")]
use std::sync::Arc;
#[cfg(feature = "postgres")]
use std::collections::HashMap;

#[cfg(feature = "postgres")]
use async_trait::async_trait;
#[cfg(feature = "postgres")]
use indexer_pipeline::{Error, Result, BlockStatus};
#[cfg(feature = "postgres")]
use indexer_core::event::Event;
#[cfg(feature = "postgres")]
use sqlx::{Pool, Postgres, types::Json, Transaction};
#[cfg(feature = "postgres")]
use tracing::instrument;

use crate::EventFilter;
use crate::Storage;
use crate::migrations::initialize_database;
use crate::{ValenceAccountInfo, ValenceAccountLibrary, ValenceAccountExecution, ValenceAccountState};

#[cfg(feature = "postgres")]
pub mod repositories;
#[cfg(feature = "postgres")]
pub mod migrations;

// Use the repositories directly instead of using paths
#[cfg(feature = "postgres")]
use repositories::event_repository::{EventRepository, PostgresEventRepository};
#[cfg(feature = "postgres")]
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
        let mut transaction = self.pool.begin().await?;
        
        // Store the event using the repository
        self.event_repository.store_event(event).await?;
        
        // Commit the transaction
        transaction.commit().await?;
        
        Ok(())
    }
    
    async fn get_events(&self, chain: &str, from_block: u64, to_block: u64) -> Result<Vec<Box<dyn Event>>> {
        // Construct filters based on input
        let filter = EventFilter {
            chain: Some(chain.to_string()),
            block_range: Some((from_block, to_block)),
            time_range: None,
            event_types: None,
            limit: None,
            offset: None,
        };
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
         sqlx::query!(
             r#"
             INSERT INTO blocks (chain, block_number, block_hash, timestamp, status)
             VALUES ($1, $2, $3, $4, $5)
             ON CONFLICT (chain, block_number) DO UPDATE SET
                 status = EXCLUDED.status,
                 block_hash = EXCLUDED.block_hash, -- Keep hash updated
                 timestamp = EXCLUDED.timestamp; -- Keep timestamp updated
             "#,
             chain,
             block_number as i64,
             tx_hash, // Assuming tx_hash can stand in for block_hash here, needs clarification
             0i64, // Placeholder for timestamp
             status_str
         )
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
        let status_str = status.as_str();
        
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
    
    async fn get_events_with_status(&self, chain: &str, from_block: u64, to_block: u64, status: BlockStatus) -> Result<Vec<Box<dyn Event>>> {
        let status_str = status.as_str();
        
        // Get the set of block numbers with the given status in the range
        let blocks = sqlx::query!(
            r#"
            SELECT block_number
            FROM blocks
            WHERE chain = $1 AND status = $2 AND block_number >= $3 AND block_number <= $4
            ORDER BY block_number ASC
            "#,
            chain,
            status_str,
            from_block as i64,
            to_block as i64
        )
        .fetch_all(&self.pool)
        .await?;
        
        let block_numbers: Vec<u64> = blocks.into_iter().map(|r| r.block_number as u64).collect();
        
        if block_numbers.is_empty() {
            return Ok(Vec::new());
        }

        // Create filters for each matching block
        let filters: Vec<EventFilter> = block_numbers.iter().map(|&b| EventFilter {
             chain: Some(chain.to_string()),
             block_range: Some((b, b)), // Filter for this specific block
             time_range: None,
             event_types: None,
             limit: None,
             offset: None,
         }).collect();

        // Get events for the matching blocks
        // Note: This might be inefficient if there are many matching blocks.
        // A single query joining events and blocks might be better.
        self.event_repository.get_events(filters).await
    }

    async fn reorg_chain(&self, chain: &str, from_block: u64) -> Result<()> {
        info!(chain, from_block, "Handling reorg in PostgreSQL");
        let mut tx = self.pool.begin().await?;

        // 1. Delete events >= from_block
        sqlx::query!(
            "DELETE FROM events WHERE chain = $1 AND block_number >= $2",
            chain,
            from_block as i64
        )
        .execute(&mut *tx)
        .await?;

        // 2. Delete blocks >= from_block
        sqlx::query!(
            "DELETE FROM blocks WHERE chain = $1 AND block_number >= $2",
            chain,
            from_block as i64
        )
        .execute(&mut *tx)
        .await?;
        
        // 3. Delete valence account executions >= from_block
        sqlx::query!(
             "DELETE FROM valence_account_executions WHERE chain_id = $1 AND block_number >= $2",
             chain,
             from_block as i64
         )
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

    #[instrument(skip(self, account_info, initial_libraries), fields(account_id = %account_info.id))]
    async fn store_valence_account_instantiation(
        &self,
        account_info: ValenceAccountInfo,
        initial_libraries: Vec<ValenceAccountLibrary>,
    ) -> Result<()> {
        let mut tx = self.pool.begin().await?;

        // Insert into valence_accounts
        sqlx::query!(
            r#"
            INSERT INTO valence_accounts (id, chain_id, contract_address, created_at_block, created_at_tx, current_owner, pending_owner, pending_owner_expiry, last_updated_block, last_updated_tx)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            ON CONFLICT (id) DO UPDATE SET
                current_owner = EXCLUDED.current_owner, -- Allow re-instantiation/update if needed?
                pending_owner = EXCLUDED.pending_owner,
                pending_owner_expiry = EXCLUDED.pending_owner_expiry,
                last_updated_block = EXCLUDED.last_updated_block,
                last_updated_tx = EXCLUDED.last_updated_tx;
            "#,
            account_info.id,
            account_info.chain_id,
            account_info.contract_address,
            account_info.created_at_block as i64,
            account_info.created_at_tx,
            account_info.current_owner,
            account_info.pending_owner,
            account_info.pending_owner_expiry.map(|v| v as i64),
            account_info.last_updated_block as i64,
            account_info.last_updated_tx
        )
        .execute(&mut *tx)
        .await?;

        // Insert initial libraries
        for lib in initial_libraries {
            sqlx::query!(
                r#"
                INSERT INTO valence_account_libraries (account_id, library_address, approved_at_block, approved_at_tx)
                VALUES ($1, $2, $3, $4)
                ON CONFLICT (account_id, library_address) DO NOTHING; -- Ignore if already approved
                "#,
                lib.account_id,
                lib.library_address,
                lib.approved_at_block as i64,
                lib.approved_at_tx
            )
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        debug!(account_id = %account_info.id, "Stored Valence account instantiation");
        Ok(())
    }

    #[instrument(skip(self, library_info), fields(account_id = %account_id, library = %library_info.library_address))]
    async fn store_valence_library_approval(
        &self,
        account_id: &str,
        library_info: ValenceAccountLibrary,
        update_block: u64,
        update_tx: &str,
    ) -> Result<()> {
        let mut tx = self.pool.begin().await?;

        // Insert library approval
        sqlx::query!(
            r#"
            INSERT INTO valence_account_libraries (account_id, library_address, approved_at_block, approved_at_tx)
            VALUES ($1, $2, $3, $4)
            ON CONFLICT (account_id, library_address) DO NOTHING; -- Ignore if already approved
            "#,
            account_id,
            library_info.library_address,
            library_info.approved_at_block as i64,
            library_info.approved_at_tx
        )
        .execute(&mut *tx)
        .await?;

        // Update account's last_updated timestamp
        sqlx::query!(
            r#"
            UPDATE valence_accounts
            SET last_updated_block = $1, last_updated_tx = $2
            WHERE id = $3;
            "#,
            update_block as i64,
            update_tx,
            account_id
        )
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        debug!(account_id = %account_id, library = %library_info.library_address, "Stored Valence library approval");
        Ok(())
    }

    #[instrument(skip(self), fields(account_id = %account_id, library = %library_address))]
    async fn store_valence_library_removal(
        &self,
        account_id: &str,
        library_address: &str,
        update_block: u64,
        update_tx: &str,
    ) -> Result<()> {
        let mut tx = self.pool.begin().await?;

        // Remove library
        sqlx::query!(
            r#"
            DELETE FROM valence_account_libraries
            WHERE account_id = $1 AND library_address = $2;
            "#,
            account_id,
            library_address
        )
        .execute(&mut *tx)
        .await?;

        // Update account's last_updated timestamp
         sqlx::query!(
            r#"
            UPDATE valence_accounts
            SET last_updated_block = $1, last_updated_tx = $2
            WHERE id = $3;
            "#,
            update_block as i64,
            update_tx,
            account_id
        )
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        debug!(account_id = %account_id, library = %library_address, "Stored Valence library removal");
        Ok(())
    }

    #[instrument(skip(self, new_owner, new_pending_owner), fields(account_id = %account_id))]
    async fn store_valence_ownership_update(
        &self,
        account_id: &str,
        new_owner: Option<String>,
        new_pending_owner: Option<String>,
        new_pending_expiry: Option<u64>,
        update_block: u64,
        update_tx: &str,
    ) -> Result<()> {
        sqlx::query!(
            r#"
            UPDATE valence_accounts
            SET current_owner = $1, pending_owner = $2, pending_owner_expiry = $3, last_updated_block = $4, last_updated_tx = $5
            WHERE id = $6;
            "#,
            new_owner,
            new_pending_owner,
            new_pending_expiry.map(|v| v as i64),
            update_block as i64,
            update_tx,
            account_id
        )
        .execute(&self.pool) // Can run outside tx if simple update
        .await?;
        debug!(account_id = %account_id, "Stored Valence ownership update");
        Ok(())
    }

    #[instrument(skip(self, execution_info), fields(account_id = %execution_info.account_id, tx_hash = %execution_info.tx_hash))]
    async fn store_valence_execution(
        &self,
        execution_info: ValenceAccountExecution,
    ) -> Result<()> {
        // Convert SystemTime to chrono::DateTime<Utc> for sqlx
        let executed_at_chrono = chrono::DateTime::from(execution_info.executed_at);

        sqlx::query!(
            r#"
            INSERT INTO valence_account_executions (account_id, chain_id, block_number, tx_hash, executor_address, message_index, correlated_event_ids, raw_msgs, payload, executed_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10);
            "#,
            execution_info.account_id,
            execution_info.chain_id,
            execution_info.block_number as i64,
            execution_info.tx_hash,
            execution_info.executor_address,
            execution_info.message_index,
            execution_info.correlated_event_ids.as_deref(),
            execution_info.raw_msgs, // Use Option<Value> directly, sqlx handles JSONB
            execution_info.payload,
            executed_at_chrono
        )
        .execute(&self.pool)
        .await?;
        debug!(account_id = %execution_info.account_id, tx_hash=%execution_info.tx_hash, "Stored Valence execution");
        Ok(())
    }

    #[instrument(skip(self), fields(account_id = %account_id))]
    async fn get_valence_account_state(&self, account_id: &str) -> Result<Option<ValenceAccountState>> {
        // Fetch account details
        let account_row = sqlx::query!(
            r#"
            SELECT chain_id, contract_address, current_owner, pending_owner, pending_owner_expiry, last_updated_block, last_updated_tx 
            FROM valence_accounts WHERE id = $1
            "#,
            account_id
        )
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = account_row {
            // Fetch associated libraries
            let libraries_result = sqlx::query!(
                "SELECT library_address FROM valence_account_libraries WHERE account_id = $1",
                account_id
            )
            .fetch_all(&self.pool)
            .await?;

            let libraries = libraries_result.into_iter().map(|r| r.library_address).collect();

            Ok(Some(ValenceAccountState {
                account_id: account_id.to_string(),
                chain_id: row.chain_id,
                address: row.contract_address,
                current_owner: row.current_owner,
                pending_owner: row.pending_owner,
                pending_owner_expiry: row.pending_owner_expiry.map(|v| v as u64),
                libraries,
                last_update_block: row.last_updated_block as u64,
                last_update_tx: row.last_updated_tx,
            }))
        } else {
            Ok(None)
        }
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
