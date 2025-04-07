// Main storage interface for Almanac indexers
//
// This crate provides storage implementations for various backends
// including PostgreSQL and RocksDB

// Re-export from core
pub use indexer_core::{Error, Result, BlockStatus};

// Common module imports
use std::sync::Arc;
use async_trait::async_trait;
use indexer_core::event::Event;
use serde::{Serialize, Deserialize};

// Conditional modules based on features
#[cfg(feature = "rocks")]
pub mod rocks;

#[cfg(feature = "postgres")]
pub mod postgres;

// Common modules
pub mod sync;
pub mod migrations;
pub mod tests;

// Type aliases
pub type BoxedStorage = Arc<dyn Storage + Send + Sync>;

/// Storage interface for Almanac indexer data
#[async_trait]
pub trait Storage {
    /// Store an event
    async fn store_event(&self, chain: &str, event: Box<dyn Event>) -> Result<()>;
    
    /// Get events by chain and block range
    async fn get_events(&self, chain: &str, from_block: u64, to_block: u64) -> Result<Vec<Box<dyn Event>>>;
    
    /// Get the latest block height for a chain
    async fn get_latest_block(&self, chain: &str) -> Result<u64>;
    
    /// Get the latest block with a specific status for a chain
    async fn get_latest_block_with_status(&self, chain: &str, status: BlockStatus) -> Result<u64>;
    
    /// Mark a block as processed with status
    async fn mark_block_processed(&self, chain: &str, block_number: u64, tx_hash: &str, status: BlockStatus) -> Result<()>;

    /// Update block status
    async fn update_block_status(&self, chain: &str, block_number: u64, status: BlockStatus) -> Result<()>;
    
    /// Get events with specific block status
    async fn get_events_with_status(&self, chain: &str, from_block: u64, to_block: u64, status: BlockStatus) -> Result<Vec<Box<dyn Event>>>;
    
    /// Handle chain reorganization from a specific block
    async fn reorg_chain(&self, chain: &str, from_block: u64) -> Result<()>;
    
    // Valence Account methods
    
    /// Stores information about a new Valence Account contract instantiation.
    async fn store_valence_account_instantiation(
        &self,
        account_info: ValenceAccountInfo,
        initial_libraries: Vec<ValenceAccountLibrary>,
    ) -> Result<()>;
    
    /// Adds a library to an existing Valence account's approved list.
    async fn store_valence_library_approval(
        &self,
        account_id: &str,
        library_info: ValenceAccountLibrary,
        update_block: u64,
        update_tx: &str,
    ) -> Result<()>;
    
    /// Removes a library from an existing Valence account's approved list.
    async fn store_valence_library_removal(
        &self,
        account_id: &str,
        library_address: &str,
        update_block: u64,
        update_tx: &str,
    ) -> Result<()>;
    
    /// Updates the ownership details of a Valence account.
    async fn store_valence_ownership_update(
        &self,
        account_id: &str,
        new_owner: Option<String>,
        new_pending_owner: Option<String>,
        new_pending_expiry: Option<u64>,
        update_block: u64,
        update_tx: &str,
    ) -> Result<()>;
    
    /// Stores a record of an execution triggered by a Valence account.
    async fn store_valence_execution(
        &self,
        execution_info: ValenceAccountExecution,
    ) -> Result<()>;
    
    /// Retrieves the current state of a Valence account.
    async fn get_valence_account_state(&self, account_id: &str) -> Result<Option<ValenceAccountState>>;
    
    /// Sets the current state of a Valence account.
    async fn set_valence_account_state(&self, account_id: &str, state: &ValenceAccountState) -> Result<()>;
    
    /// Deletes the current state of a Valence account.
    async fn delete_valence_account_state(&self, account_id: &str) -> Result<()>;
    
    /// Stores the historical state of a Valence account.
    async fn set_historical_valence_account_state(
        &self,
        account_id: &str,
        block_number: u64,
        state: &ValenceAccountState,
    ) -> Result<()>;
    
    /// Retrieves the historical state of a Valence account.
    async fn get_historical_valence_account_state(
        &self,
        account_id: &str,
        block_number: u64,
    ) -> Result<Option<ValenceAccountState>>;
    
    /// Deletes the historical state of a Valence account.
    async fn delete_historical_valence_account_state(
        &self,
        account_id: &str,
        block_number: u64,
    ) -> Result<()>;
    
    /// Sets the latest block number for which historical state is stored for an account.
    async fn set_latest_historical_valence_block(
        &self,
        account_id: &str,
        block_number: u64,
    ) -> Result<()>;

    /// Retrieves the latest block number for which historical state is stored for an account.
    async fn get_latest_historical_valence_block(&self, account_id: &str) -> Result<Option<u64>>;

    /// Deletes the record of the latest historical block for an account.
    async fn delete_latest_historical_valence_block(&self, account_id: &str) -> Result<()>;

    // Valence Processor data models

    /// Stores information about a new Valence Processor contract instantiation.
    async fn store_valence_processor_instantiation(
        &self,
        processor_info: ValenceProcessorInfo,
    ) -> Result<()>;

    /// Updates the configuration of a Valence Processor.
    async fn store_valence_processor_config_update(
        &self,
        processor_id: &str,
        config: ValenceProcessorConfig,
        update_block: u64,
        update_tx: &str,
    ) -> Result<()>;

    /// Stores a new cross-chain message submitted to the processor.
    async fn store_valence_processor_message(
        &self,
        message: ValenceProcessorMessage,
    ) -> Result<()>;

    /// Updates the status of an existing processor message.
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
    ) -> Result<()>;

    /// Retrieves the current state of a Valence Processor.
    async fn get_valence_processor_state(&self, processor_id: &str) -> Result<Option<ValenceProcessorState>>;

    /// Sets the current state of a Valence Processor.
    async fn set_valence_processor_state(&self, processor_id: &str, state: &ValenceProcessorState) -> Result<()>;

    /// Stores a historical snapshot of a processor's state at a specific block.
    async fn set_historical_valence_processor_state(
        &self,
        processor_id: &str,
        block_number: u64,
        state: &ValenceProcessorState,
    ) -> Result<()>;

    /// Retrieves a historical snapshot of a processor's state.
    async fn get_historical_valence_processor_state(
        &self,
        processor_id: &str,
        block_number: u64,
    ) -> Result<Option<ValenceProcessorState>>;

    // Valence Authorization data models

    /// Stores information about a new Valence Authorization contract instantiation.
    async fn store_valence_authorization_instantiation(
        &self,
        auth_info: ValenceAuthorizationInfo,
        initial_policy: Option<ValenceAuthorizationPolicy>,
    ) -> Result<()>;

    /// Creates or updates an authorization policy.
    async fn store_valence_authorization_policy(
        &self,
        policy: ValenceAuthorizationPolicy,
    ) -> Result<()>;

    /// Updates the active policy for an authorization contract.
    async fn update_active_authorization_policy(
        &self,
        auth_id: &str,
        policy_id: &str,
        update_block: u64,
        update_tx: &str,
    ) -> Result<()>;

    /// Records a new authorization grant.
    async fn store_valence_authorization_grant(
        &self,
        grant: ValenceAuthorizationGrant,
    ) -> Result<()>;

    /// Revokes an existing authorization grant.
    async fn revoke_valence_authorization_grant(
        &self,
        auth_id: &str,
        grantee: &str,
        resource: &str,
        revoked_at_block: u64,
        revoked_at_tx: &str,
    ) -> Result<()>;

    /// Records an authorization request and its decision.
    async fn store_valence_authorization_request(
        &self,
        request: ValenceAuthorizationRequest,
    ) -> Result<()>;

    /// Updates an existing authorization request's decision.
    async fn update_valence_authorization_request_decision(
        &self,
        request_id: &str,
        decision: ValenceAuthorizationDecision,
        processed_block: Option<u64>,
        processed_tx: Option<&str>,
        reason: Option<String>,
    ) -> Result<()>;

    // Valence Library data models

    /// Stores information about a new Valence Library contract instantiation.
    async fn store_valence_library_instantiation(
        &self,
        library_info: ValenceLibraryInfo,
        initial_version: Option<ValenceLibraryVersion>,
    ) -> Result<()>;

    /// Records a new version of a library.
    async fn store_valence_library_version(
        &self,
        version: ValenceLibraryVersion,
    ) -> Result<()>;

    /// Updates the active version for a library.
    async fn update_active_library_version(
        &self,
        library_id: &str,
        version: u32,
        update_block: u64,
        update_tx: &str,
    ) -> Result<()>;

    /// Records usage of a library.
    async fn store_valence_library_usage(
        &self,
        usage: ValenceLibraryUsage,
    ) -> Result<()>;

    /// Revokes a library approval.
    async fn revoke_valence_library_approval(
        &self,
        library_id: &str,
        account_id: &str,
        revoked_at_block: u64,
        revoked_at_tx: &str,
    ) -> Result<()>;

    /// Get the current state of a Valence library.
    async fn get_valence_library_state(&self, library_id: &str) -> Result<Option<ValenceLibraryState>>;

    /// Set the current state of a Valence library.
    async fn set_valence_library_state(&self, library_id: &str, state: &ValenceLibraryState) -> Result<()>;

    /// Get library versions for a specific library.
    async fn get_valence_library_versions(&self, library_id: &str) -> Result<Vec<ValenceLibraryVersion>>;

    /// Get library approvals for a specific library.
    async fn get_valence_library_approvals(&self, library_id: &str) -> Result<Vec<ValenceLibraryApproval>>;

    /// Get libraries approved for a specific account.
    async fn get_valence_libraries_for_account(&self, account_id: &str) -> Result<Vec<ValenceLibraryApproval>>;

    /// Get usage history for a specific library.
    async fn get_valence_library_usage_history(
        &self,
        library_id: &str,
        limit: Option<usize>,
        offset: Option<usize>,
    ) -> Result<Vec<ValenceLibraryUsage>>;

    // Additional methods as needed
}

// Storage factory function
#[cfg(feature = "rocks")]
pub fn create_rocks_storage(path: &str) -> Result<BoxedStorage> {
    use rocks::{RocksConfig, RocksStorage};
    
    let config = RocksConfig {
        path: path.to_string(),
        create_if_missing: true,
        cache_size_mb: 128, // Default cache size
    };
    let storage = RocksStorage::new(config)?;
    Ok(Arc::new(storage))
}

#[cfg(feature = "postgres")]
pub async fn create_postgres_storage(connection_string: &str) -> Result<BoxedStorage> {
    use postgres::{PostgresConfig, PostgresStorage};
    
    let config = PostgresConfig {
        url: connection_string.to_string(),
        max_connections: 5, // Default max connections
        connection_timeout: 30, // Default timeout
    };
    let storage = PostgresStorage::new(config).await?;
    Ok(Arc::new(storage))
}

// Re-export repositories from postgres module
#[cfg(feature = "postgres")]
pub use postgres::repositories;

/// Re-export contract schema types for convenience
#[cfg(feature = "postgres")]
pub use migrations::schema::{
    ContractSchemaVersion, EventSchema, FieldSchema, FunctionSchema,
    ContractSchemaRegistry
};

// Add structs to pass Valence data around
// Could also define these in indexer-cosmos or a new valence-types crate

#[derive(Debug, Clone)]
pub struct ValenceAccountInfo {
    pub id: String,                         // Unique ID (e.g., chain_id + contract_address)
    pub chain_id: String,
    pub contract_address: String,
    pub created_at_block: u64,
    pub created_at_tx: String,
    pub current_owner: Option<String>,
    pub pending_owner: Option<String>,
    pub pending_owner_expiry: Option<u64>,
    pub last_updated_block: u64,
    pub last_updated_tx: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValenceAccountLibrary {
    pub account_id: String,
    pub library_address: String,
    pub approved_at_block: u64,
    pub approved_at_tx: String,
}

#[derive(Debug, Clone)]
pub struct ValenceAccountExecution {
    pub account_id: String,
    pub chain_id: String,
    pub block_number: u64,
    pub tx_hash: String,
    pub executor_address: String,
    pub message_index: i32, // Assuming i32 fits message index
    pub correlated_event_ids: Option<Vec<String>>,
    pub raw_msgs: Option<serde_json::Value>,
    pub payload: Option<String>,
    pub executed_at: std::time::SystemTime,
}

// Valence Processor data models

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ValenceProcessorInfo {
    pub id: String,                          // Unique ID (e.g., chain_id + contract_address)
    pub chain_id: String,
    pub contract_address: String,
    pub created_at_block: u64,
    pub created_at_tx: String,
    pub current_owner: Option<String>,
    pub config: Option<ValenceProcessorConfig>,
    pub last_updated_block: u64,
    pub last_updated_tx: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ValenceProcessorConfig {
    pub max_gas_per_message: Option<u64>,
    pub message_timeout_blocks: Option<u64>,
    pub retry_interval_blocks: Option<u64>,
    pub max_retry_count: Option<u32>,
    pub paused: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ValenceProcessorMessage {
    pub id: String,                           // Unique message ID
    pub processor_id: String,                 // Processor contract ID
    pub source_chain_id: String,              // Chain where message originated
    pub target_chain_id: String,              // Chain where message is to be processed
    pub sender_address: String,               // Address that submitted the message
    pub payload: String,                      // Message payload (could be base64/hex encoded)
    pub status: ValenceMessageStatus,
    pub created_at_block: u64,
    pub created_at_tx: String,
    pub last_updated_block: u64,
    pub processed_at_block: Option<u64>,
    pub processed_at_tx: Option<String>,
    pub retry_count: u32,
    pub next_retry_block: Option<u64>,
    pub gas_used: Option<u64>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ValenceMessageStatus {
    Pending,
    Processing,
    Completed,
    Failed,
    TimedOut,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ValenceProcessorState {
    pub processor_id: String,
    pub chain_id: String,
    pub address: String,
    pub owner: Option<String>,
    pub config: Option<ValenceProcessorConfig>,
    pub pending_message_count: u64,
    pub completed_message_count: u64,
    pub failed_message_count: u64,
    pub last_update_block: u64,
    pub last_update_tx: String,
}

// Valence Authorization data models

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ValenceAuthorizationInfo {
    pub id: String,                           // Unique ID (e.g., chain_id + contract_address)
    pub chain_id: String,
    pub contract_address: String,
    pub created_at_block: u64,
    pub created_at_tx: String,
    pub current_owner: Option<String>,        // Owner of the authorization contract
    pub active_policy_id: Option<String>,     // Currently active policy ID
    pub last_updated_block: u64,
    pub last_updated_tx: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ValenceAuthorizationPolicy {
    pub id: String,                          // Unique policy identifier
    pub auth_id: String,                     // ID of the parent authorization contract
    pub version: u32,                        // Policy version number
    pub content_hash: String,                // Hash of the policy content for verification
    pub created_at_block: u64,
    pub created_at_tx: String,
    pub is_active: bool,                     // Whether this policy is currently active
    pub metadata: Option<serde_json::Value>, // Additional policy metadata
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ValenceAuthorizationGrant {
    pub id: String,                          // Unique grant ID
    pub auth_id: String,                     // Parent authorization contract ID
    pub grantee: String,                     // Address of the grantee
    pub permissions: Vec<String>,            // Granted permissions
    pub resources: Vec<String>,              // Resources the permissions apply to
    pub granted_at_block: u64,
    pub granted_at_tx: String,
    pub expiry: Option<u64>,                 // Optional expiration (block height or timestamp)
    pub is_active: bool,                     // Whether this grant is still active
    pub revoked_at_block: Option<u64>,       // When the grant was revoked, if applicable
    pub revoked_at_tx: Option<String>,       // Transaction that revoked the grant
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ValenceAuthorizationRequest {
    pub id: String,                          // Unique request ID
    pub auth_id: String,                     // Parent authorization contract ID
    pub requester: String,                   // Address requesting authorization
    pub action: String,                      // Requested action
    pub resource: String,                    // Resource to act upon
    pub request_data: Option<String>,        // Additional request data
    pub decision: ValenceAuthorizationDecision,
    pub requested_at_block: u64,
    pub requested_at_tx: String,
    pub processed_at_block: Option<u64>,     // When the request was processed
    pub processed_at_tx: Option<String>,
    pub reason: Option<String>,              // Reason for decision
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ValenceAuthorizationDecision {
    Pending,
    Approved,
    Denied,
    Error,
}

// Valence Library data models

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ValenceLibraryInfo {
    pub id: String,                           // Unique ID (e.g., chain_id + contract_address)
    pub chain_id: String,
    pub contract_address: String,
    pub library_type: String,                 // Type of library (e.g., "swap", "bridge", "messaging")
    pub created_at_block: u64,
    pub created_at_tx: String,
    pub current_owner: Option<String>,        // Owner of the library contract
    pub current_version: Option<u32>,         // Current active version
    pub last_updated_block: u64,
    pub last_updated_tx: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ValenceLibraryVersion {
    pub id: String,                          // Unique version ID
    pub library_id: String,                  // ID of the parent library
    pub version: u32,                        // Version number
    pub code_hash: String,                   // Hash of the version's code
    pub created_at_block: u64,
    pub created_at_tx: String,
    pub is_active: bool,                     // Whether this version is current
    pub features: Vec<String>,               // Features supported by this version
    pub metadata: Option<serde_json::Value>, // Additional version metadata
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ValenceLibraryUsage {
    pub id: String,                          // Unique usage ID
    pub library_id: String,                  // ID of the library used
    pub user_address: String,                // Address using the library
    pub account_id: Option<String>,          // If used by a Valence account
    pub function_name: Option<String>,       // Function being used, if available
    pub usage_at_block: u64,
    pub usage_at_tx: String,
    pub gas_used: Option<u64>,               // Gas used by the library call
    pub success: bool,                       // Whether the usage was successful
    pub error: Option<String>,               // Error message if failed
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ValenceLibraryApproval {
    pub id: String,                          // Unique approval ID
    pub library_id: String,                  // Library that was approved
    pub account_id: String,                  // Account approving the library
    pub approved_at_block: u64,
    pub approved_at_tx: String,
    pub is_active: bool,                     // Whether approval is still active
    pub revoked_at_block: Option<u64>,       // When the approval was revoked
    pub revoked_at_tx: Option<String>,       // Transaction that revoked the approval
}

/// Represents the current state of a Valence library contract
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValenceLibraryState {
    /// The unique identifier for the library
    pub library_id: String,
    /// The chain ID the library belongs to
    pub chain_id: String,
    /// The contract address
    pub address: String,
    /// Type of library
    pub library_type: String,
    /// Current owner of the library
    pub current_owner: Option<String>,
    /// Current version number
    pub current_version: Option<u32>,
    /// List of all versions
    #[serde(default)]
    pub versions: Vec<ValenceLibraryVersion>,
    /// The block number at which this state was last updated
    pub last_update_block: u64,
    /// The transaction hash of the last update
    pub last_update_tx: String,
}

/// Represents the latest known state of a Valence account in storage.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValenceAccountState {
    /// The unique identifier for the account (e.g., "<chain_id>:<contract_address>").
    pub account_id: String,
    /// The chain ID the account belongs to.
    pub chain_id: String,
    /// The contract address of the account.
    pub address: String,
    /// The current owner address (if any).
    pub current_owner: Option<String>,
    /// The pending owner address (if any).
    pub pending_owner: Option<String>,
    /// Timestamp when the pending ownership expires (if applicable).
    pub pending_owner_expiry: Option<u64>,
    /// List of currently approved library addresses.
    #[serde(default)] // Ensure empty vec if missing during deserialization
    pub libraries: Vec<String>,
    /// The block number at which this state was last updated.
    pub last_update_block: u64,
    /// The transaction hash of the last update.
    pub last_update_tx: String,
}

/// Represents the current state of a Valence authorization contract
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValenceAuthorizationState {
    /// The unique identifier for the authorization contract
    pub auth_id: String,
    /// The chain ID the authorization contract belongs to
    pub chain_id: String,
    /// The contract address
    pub address: String,
    /// Current owner of the authorization contract
    pub current_owner: Option<String>,
    /// Currently active policy ID
    pub active_policy_id: Option<String>,
    /// Map of grantee addresses to their permissions
    #[serde(default)]
    pub active_grants: Vec<ValenceAuthorizationGrant>,
    /// The block number at which this state was last updated
    pub last_update_block: u64,
    /// The transaction hash of the last update
    pub last_update_tx: String,
}

/// Default implementations for Storage trait methods
pub mod storage_defaults {
    use super::*;
    
    pub async fn get_latest_block_with_status(
        storage: &dyn Storage,
        chain: &str, 
        _status: BlockStatus
    ) -> Result<u64> {
        storage.get_latest_block(chain).await
    }

    pub async fn update_block_status(
        _storage: &dyn Storage,
        _chain: &str, 
        _block_number: u64, 
        _status: BlockStatus
    ) -> Result<()> {
        Ok(()) // Default is often no-op or relies on underlying impl
    }

    pub async fn get_events_with_status(
        storage: &dyn Storage,
        chain: &str,
        from_block: u64, 
        to_block: u64,
        _status: BlockStatus
    ) -> Result<Vec<Box<dyn Event>>> {
        // Default implementation might just fetch all events in range
        // and rely on higher layers for status filtering if needed, 
        // or assume the underlying storage handles it implicitly.
        storage.get_events(chain, from_block, to_block).await
    }
}

/// Event filter for querying events
#[derive(Debug, Clone)]
pub struct EventFilter {
    /// Chain filter
    pub chain: Option<String>,
    
    /// Block range filter (min, max)
    pub block_range: Option<(u64, u64)>,
    
    /// Time range filter (min, max) in unix seconds
    pub time_range: Option<(u64, u64)>,
    
    /// Event type filter
    pub event_types: Option<Vec<String>>,
    
    /// Result limit
    pub limit: Option<usize>,
    
    /// Result offset
    pub offset: Option<usize>,
} 