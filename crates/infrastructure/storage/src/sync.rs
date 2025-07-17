// sync.rs - Synchronization utilities for storage operations
//
// Purpose: Provides utilities for coordinating storage operations across
// different storage backends and handling concurrent access patterns.

use almanac_infrastructure_core::{Error, Result, types::Event};
use crate::Storage;

// Need to re-add the other imports that were removed
use almanac_infrastructure_core::{BlockStatus, types::ChainId};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn, error};
use std::time::Duration;
use std::any::Any;
use crate::{BoxedStorage, EventFilter};

#[cfg(all(feature = "rocks", feature = "postgres"))]
use crate::postgres::PostgresStorage;

#[cfg(all(feature = "rocks", feature = "postgres"))]
use crate::rocks::RocksStorage;

// Import Valence data structures
use crate::{
    ValenceAccountInfo, ValenceAccountLibrary, ValenceAccountExecution,
    ValenceProcessorInfo, ValenceProcessorConfig, ValenceProcessorMessage, ValenceMessageStatus,
    ValenceAuthorizationInfo, ValenceAuthorizationGrant,
    ValenceLibraryInfo, ValenceLibraryVersion, ValenceLibraryUsage,
    ValenceLibraryApprovalRequest, ValenceLibraryRemovalRequest,
    ValenceOwnershipUpdateRequest, ValenceProcessorConfigUpdateRequest,
};

/// Helper struct for ownership update parsing
#[derive(Debug)]
struct ValenceOwnershipUpdate {
    account_id: String,
    new_owner: Option<String>,
    new_pending_owner: Option<String>,
    new_pending_expiry: Option<u64>,
}

use tokio::task::JoinHandle;
use tokio::time;
use futures::future::join_all;

/// Configuration for storage synchronization
#[derive(Debug, Clone)]
pub struct SyncConfig {
    /// Synchronization interval in milliseconds
    pub sync_interval_ms: u64,
    
    /// Maximum number of events to synchronize in a single batch
    pub batch_size: usize,
    
    /// Chains to synchronize
    pub chains: Vec<String>,
}

impl Default for SyncConfig {
    fn default() -> Self {
        Self {
            sync_interval_ms: 1000, // 1 second
            batch_size: 100,
            chains: vec!["ethereum".to_string(), "cosmos".to_string()],
        }
    }
}

/// Storage synchronizer for multi-store consistency
pub struct StorageSynchronizer {
    /// Primary storage
    primary: BoxedStorage,
    
    /// Secondary storage
    secondary: BoxedStorage,
    
    /// Synchronization configuration
    config: SyncConfig,
    
    /// Synchronization task handle
    task_handle: RwLock<Option<JoinHandle<()>>>,
    
    /// Is the synchronizer running
    running: RwLock<bool>,
}

impl StorageSynchronizer {
    /// Create a new storage synchronizer with any two storage implementations
    pub async fn new_generic<P, S>(
        primary: Arc<P>, 
        secondary: Arc<S>,
        config: SyncConfig
    ) -> Self 
    where 
        P: Storage + Send + Sync + 'static,
        S: Storage + Send + Sync + 'static
    {
        Self {
            primary: primary as BoxedStorage,
            secondary: secondary as BoxedStorage,
            config,
            task_handle: RwLock::new(None),
            running: RwLock::new(false),
        }
    }
    
    /// Create a new storage synchronizer with RocksDB as primary and PostgreSQL as secondary
    #[cfg(all(feature = "rocks", feature = "postgres"))]
    pub async fn new_rocks_postgres(
        rocks: Arc<RocksStorage>,
        postgres: Arc<PostgresStorage>,
        config: SyncConfig
    ) -> Self {
        Self::new_generic(rocks, postgres, config).await
    }
    
    /// Create a new storage synchronizer with PostgreSQL as primary and RocksDB as secondary
    #[cfg(all(feature = "rocks", feature = "postgres"))]
    pub async fn new_postgres_rocks(
        postgres: Arc<PostgresStorage>,
        rocks: Arc<RocksStorage>,
        config: SyncConfig
    ) -> Self {
        Self::new_generic(postgres, rocks, config).await
    }
    
    /// Start synchronization
    pub async fn start(&self) -> Result<()> {
        let mut running = self.running.write().await;
        
        if *running {
            warn!("Synchronization is already running");
            return Ok(());
        }
        
        *running = true;
        
        let primary = self.primary.clone();
        let secondary = self.secondary.clone();
        let config = self.config.clone();
        
        let handle = tokio::spawn(async move {
            let sync_interval = Duration::from_millis(config.sync_interval_ms);
            
            loop {
                // Synchronize each chain
                for chain in &config.chains {
                    if let Err(e) = Self::sync_chain(&primary, &secondary, chain, config.batch_size).await {
                        error!("Failed to synchronize chain {}: {}", chain, e);
                    }
                }
                
                // Wait for the next sync interval
                time::sleep(sync_interval).await;
            }
        });
        
        let mut task_handle = self.task_handle.write().await;
        *task_handle = Some(handle);
        
        info!("Storage synchronization started");
        Ok(())
    }
    
    /// Stop synchronization
    pub async fn stop(&self) -> Result<()> {
        let mut running = self.running.write().await;
        
        if !*running {
            warn!("Synchronization is not running");
            return Ok(());
        }
        
        let mut task_handle = self.task_handle.write().await;
        
        if let Some(handle) = task_handle.take() {
            handle.abort();
        }
        
        *running = false;
        
        info!("Storage synchronization stopped");
        Ok(())
    }
    
    /// Synchronize a specific chain
    async fn sync_chain(
        primary: &BoxedStorage, 
        secondary: &BoxedStorage, 
        chain: &str, 
        batch_size: usize
    ) -> Result<()> {
        debug!("Synchronizing chain {} with batch size {}", chain, batch_size);
        
        // Get latest block from primary and secondary storage
        let primary_latest = primary.get_latest_block(chain).await?;
        let secondary_latest = secondary.get_latest_block(chain).await?;
        
        debug!("Latest blocks - Primary: {}, Secondary: {}", primary_latest, secondary_latest);
        
        if secondary_latest >= primary_latest {
            debug!("Secondary storage is up-to-date for chain {}", chain);
            return Ok(());
        }
        
        // Define the range of blocks to synchronize
        let start_block = secondary_latest + 1;
        let end_block = std::cmp::min(
            primary_latest,
            secondary_latest + batch_size as u64
        );
        
        info!("Synchronizing chain {} from block {} to {}", chain, start_block, end_block);
        
        // Create filter to get events from the primary storage
        let mut filter = EventFilter::new();
        filter.chain_ids = Some(vec![ChainId::from(chain)]);
        filter.chain = Some(chain.to_string());
        filter.block_range = Some((start_block, end_block));
        filter.time_range = None;
        filter.event_types = None;
        filter.limit = None;
        filter.offset = None;
        
        // Get events from primary
        let filters = vec![EventFilter {
            chain: Some(chain.to_string()),
            block_range: Some((start_block, end_block)),
            ..Default::default()
        }];
        let events = primary.get_events(filters).await?;
        
        if events.is_empty() {
            debug!("No events to synchronize for chain {}", chain);
            
            // Update the secondary storage's latest block even if no events
            // This prevents getting stuck on empty blocks
            Self::update_secondary_latest_block(secondary, chain, end_block).await?;
            
            return Ok(());
        }
        
        debug!("Found {} events to synchronize for chain {}", events.len(), chain);
        
        // Store events in secondary storage
        let mut futures = Vec::new();
        
        for event in events {
            let secondary_clone = secondary.clone();
            let event_clone = event.clone();
            
            futures.push(tokio::spawn(async move {
                secondary_clone.store_event(event_clone).await
            }));
        }
        
        // Wait for all events to be stored
        let results = join_all(futures).await;
        
        // Check for errors
        for result in results.into_iter().flatten() {
            if let Err(err) = result {
                error!("Failed to store event in secondary storage: {:?}", err);
                return Err(err);
            }
        }
        
        // Update block status for synchronized blocks
        for block_number in start_block..=end_block {
            // Get the status from primary storage
            let status = Self::get_block_status(primary, chain, block_number).await?;
            
            // Update the status in secondary storage
            secondary.update_block_status(chain, block_number, status).await?;
        }
        
        // Update secondary's latest block
        Self::update_secondary_latest_block(secondary, chain, end_block).await?;
        
        info!("Successfully synchronized chain {} from block {} to {}", chain, start_block, end_block);
        
        Ok(())
    }
    
    /// Get block status from storage
    #[allow(unused_variables)]
    async fn get_block_status(
        primary: &BoxedStorage,
        chain: &str,
        block_number: u64
    ) -> Result<BlockStatus> {
        // This is a simplified implementation
        // In a real system, we'd query the actual block status
        // For now, treat all blocks as confirmed
        Ok(BlockStatus::Confirmed)
    }
    
    /// Update the secondary storage's latest block tracking
    async fn update_secondary_latest_block(
        secondary: &BoxedStorage,
        chain: &str,
        latest_block: u64
    ) -> Result<()> {
        // Create a special event to record the latest block
        // This is a workaround since there's no direct way to set the latest block
        let event_id = format!("sync_latest_block_{}_{}", chain, latest_block);
        let event = Box::new(SyncEvent {
            id: event_id,
            chain: chain.to_string(),
            block_number: latest_block,
        });
        
        secondary.store_event(event).await
    }

    /// Manually process an event
    pub async fn process_event(
        storage: &BoxedStorage,
        chain: &str,
        block_number: u64
    ) -> Result<()> {
        debug!("Processing events for chain {} at block {}", chain, block_number);
        
        // Get all events for this specific block
        let filters = vec![EventFilter {
            chain: Some(chain.to_string()),
            block_range: Some((block_number, block_number)),
            ..Default::default()
        }];
        let events = storage.get_events(filters).await?;
        
        if events.is_empty() {
            debug!("No events found for chain {} at block {}", chain, block_number);
            return Ok(());
        }
        
        info!("Processing {} events for chain {} at block {}", events.len(), chain, block_number);
        
        // Process each event with validation and type-specific handling
        for event in events {
            // Validate event data
            Self::validate_event(event.as_ref())?;
            
            // Clone event data for processing to avoid ownership issues
            let event_id = event.id().to_string();
            let event_type = event.event_type().to_string();
            
            // Process based on event type
            match event_type.as_str() {
                // Valence Account events
                "valence_account_instantiated" => {
                    debug!("Processing Valence account instantiation for event {}", event.id());
                    
                    // Parse event data to extract account information
                    let account_info = Self::parse_valence_account_instantiation(&event)?;
                    let initial_libraries = Self::parse_valence_account_libraries(&event)?;
                    
                    // Store the account instantiation
                    storage.store_valence_account_instantiation(account_info, initial_libraries).await?;
                    
                    info!("Successfully processed Valence account instantiation for event {}", event.id());
                },
                "valence_library_approved" => {
                    debug!("Processing Valence library approval for event {}", event.id());
                    
                    // Parse event data to extract library approval information
                    let (account_id, library_info) = Self::parse_valence_library_approval(&event)?;
                    
                    // Store the library approval
                    let request = ValenceLibraryApprovalRequest {
                        account_id,
                        library_info,
                        update_block: event.block_number(),
                        update_tx: event.tx_hash().to_string(),
                    };
                    storage.store_valence_library_approval(request).await?;
                    
                    info!("Successfully processed Valence library approval for event {}", event.id());
                },
                "valence_library_removed" => {
                    debug!("Processing Valence library removal for event {}", event.id());
                    
                    // Parse event data to extract library removal information
                    let (account_id, library_address) = Self::parse_valence_library_removal(&event)?;
                    
                    // Store the library removal
                    let request = ValenceLibraryRemovalRequest {
                        account_id,
                        library_address,
                        update_block: event.block_number(),
                        update_tx: event.tx_hash().to_string(),
                    };
                    storage.store_valence_library_removal(request).await?;
                    
                    info!("Successfully processed Valence library removal for event {}", event.id());
                },
                "valence_ownership_updated" => {
                    debug!("Processing Valence ownership update for event {}", event.id());
                    
                    // Parse event data to extract ownership update information
                    let ownership_update = Self::parse_valence_ownership_update(&event)?;
                    
                    // Store the ownership update
                    let request = ValenceOwnershipUpdateRequest {
                        account_id: ownership_update.account_id,
                        new_owner: ownership_update.new_owner,
                        new_pending_owner: ownership_update.new_pending_owner,
                        new_pending_expiry: ownership_update.new_pending_expiry,
                        update_block: event.block_number(),
                        update_tx: event.tx_hash().to_string(),
                    };
                    storage.store_valence_ownership_update(request).await?;
                    
                    info!("Successfully processed Valence ownership update for event {}", event.id());
                },
                "valence_execution" => {
                    debug!("Processing Valence execution for event {}", event.id());
                    
                    // Parse event data to extract execution information
                    let execution_info = Self::parse_valence_execution(&event)?;
                    
                    // Store the execution
                    storage.store_valence_execution(execution_info).await?;
                    
                    info!("Successfully processed Valence execution for event {}", event.id());
                },
                
                // Valence Processor events
                "valence_processor_instantiated" => {
                    debug!("Processing Valence processor instantiation for event {}", event.id());
                    
                    // Parse event data to extract processor information
                    let processor_info = Self::parse_valence_processor_instantiation(&event)?;
                    
                    // Store the processor instantiation
                    storage.store_valence_processor_instantiation(processor_info).await?;
                    
                    info!("Successfully processed Valence processor instantiation for event {}", event.id());
                },
                "valence_processor_config_updated" => {
                    debug!("Processing Valence processor config update for event {}", event.id());
                    
                    // Parse event data to extract processor config update information
                    let (processor_id, config) = Self::parse_valence_processor_config_update(&event)?;
                    
                    // Store the processor config update
                    let request = ValenceProcessorConfigUpdateRequest {
                        processor_id,
                        config,
                        update_block: event.block_number(),
                        update_tx: event.tx_hash().to_string(),
                    };
                    storage.store_valence_processor_config_update(request).await?;
                    
                    info!("Successfully processed Valence processor config update for event {}", event.id());
                },
                "valence_processor_message" => {
                    debug!("Processing Valence processor message for event {}", event.id());
                    
                    // Parse event data to extract processor message information
                    let message = Self::parse_valence_processor_message(&event)?;
                    
                    // Store the processor message
                    storage.store_valence_processor_message(message).await?;
                    
                    info!("Successfully processed Valence processor message for event {}", event.id());
                },
                
                // Valence Authorization events
                "valence_authorization_instantiated" => {
                    debug!("Processing Valence authorization instantiation for event {}", event.id());
                    
                    // Parse event data to extract authorization information
                    let auth_info = Self::parse_valence_authorization_instantiation(&event)?;
                    
                    // Store the authorization instantiation
                    storage.store_valence_authorization_instantiation(auth_info, None).await?;
                    
                    info!("Successfully processed Valence authorization instantiation for event {}", event.id());
                },
                "valence_authorization_grant" => {
                    debug!("Processing Valence authorization grant for event {}", event.id());
                    
                    // Parse event data to extract authorization grant information
                    let grant = Self::parse_valence_authorization_grant(&event)?;
                    
                    // Store the authorization grant
                    storage.store_valence_authorization_grant(grant).await?;
                    
                    info!("Successfully processed Valence authorization grant for event {}", event.id());
                },
                
                // Valence Library events
                "valence_library_instantiated" => {
                    debug!("Processing Valence library instantiation for event {}", event.id());
                    
                    // Parse event data to extract library information
                    let (library_info, initial_version) = Self::parse_valence_library_instantiation(&event)?;
                    
                    // Store the library instantiation
                    storage.store_valence_library_instantiation(library_info, initial_version).await?;
                    
                    info!("Successfully processed Valence library instantiation for event {}", event.id());
                },
                "valence_library_version_added" => {
                    debug!("Processing Valence library version for event {}", event.id());
                    
                    // Parse event data to extract library version information
                    let version = Self::parse_valence_library_version(&event)?;
                    
                    // Store the library version
                    storage.store_valence_library_version(version).await?;
                    
                    info!("Successfully processed Valence library version for event {}", event.id());
                },
                "valence_library_usage" => {
                    debug!("Processing Valence library usage for event {}", event.id());
                    
                    // Parse event data to extract library usage information
                    let usage = Self::parse_valence_library_usage(&event)?;
                    
                    // Store the library usage
                    storage.store_valence_library_usage(usage).await?;
                    
                    info!("Successfully processed Valence library usage for event {}", event.id());
                },
                
                // Generic events
                _ => {
                    debug!("Processing generic event type: {}", event.event_type());
                    // For unknown event types, just ensure they're stored
                    storage.store_event(event).await?;
                    debug!("Successfully processed generic event {}", event_id);
                }
            }
        }
        
        // Mark the block as processed
        storage.update_block_status(chain, block_number, BlockStatus::Confirmed).await?;
        
        info!("Successfully processed all events for chain {} at block {}", chain, block_number);
        Ok(())
    }
    
    /// Validate event data integrity
    fn validate_event(event: &dyn Event) -> Result<()> {
        // Basic validation
        if event.id().is_empty() {
            return Err(Error::generic("Event ID cannot be empty"));
        }
        
        if event.chain().is_empty() {
            return Err(Error::generic("Event chain cannot be empty"));
        }
        
        if event.block_number() == 0 {
            return Err(Error::generic("Event block number cannot be zero"));
        }
        
        if event.block_hash().is_empty() {
            return Err(Error::generic("Event block hash cannot be empty"));
        }
        
        if event.tx_hash().is_empty() {
            return Err(Error::generic("Event transaction hash cannot be empty"));
        }
        
        if event.event_type().is_empty() {
            return Err(Error::generic("Event type cannot be empty"));
        }
        
        debug!("Event {} passed validation", event.id());
        Ok(())
    }
    
    // --- Valence Event Parsing Functions ---
    
    /// Parse Valence account instantiation event
    fn parse_valence_account_instantiation(event: &Box<dyn Event>) -> Result<ValenceAccountInfo> {
        // Parse the raw event data to extract account information
        let _raw_data = event.raw_data();
        
        // For minimal implementation, create a basic account info from event metadata
        // In a real implementation, this would parse the actual event data structure
        let account_info = ValenceAccountInfo {
            id: format!("{}:{}", event.chain(), event.tx_hash()),
            chain_id: event.chain().to_string(),
            contract_address: Self::extract_contract_address_from_event(event)?,
            created_at_block: event.block_number(),
            created_at_tx: event.tx_hash().to_string(),
            current_owner: Self::extract_owner_from_event(event)?,
            pending_owner: None,
            pending_owner_expiry: None,
            last_updated_block: event.block_number(),
            last_updated_tx: event.tx_hash().to_string(),
        };
        
        Ok(account_info)
    }
    
    /// Parse Valence account libraries from instantiation event
    fn parse_valence_account_libraries(_event: &Box<dyn Event>) -> Result<Vec<ValenceAccountLibrary>> {
        // For minimal implementation, return empty vector
        // In a real implementation, this would parse the initial libraries from event data
        Ok(Vec::new())
    }
    
    /// Parse Valence library approval event
    fn parse_valence_library_approval(event: &Box<dyn Event>) -> Result<(String, ValenceAccountLibrary)> {
        let account_id = format!("{}:{}", event.chain(), Self::extract_account_address_from_event(event)?);
        let library_address = Self::extract_library_address_from_event(event)?;
        
        let library_info = ValenceAccountLibrary {
            account_id: account_id.clone(),
            library_address,
            approved_at_block: event.block_number(),
            approved_at_tx: event.tx_hash().to_string(),
        };
        
        Ok((account_id, library_info))
    }
    
    /// Parse Valence library removal event
    fn parse_valence_library_removal(event: &Box<dyn Event>) -> Result<(String, String)> {
        let account_id = format!("{}:{}", event.chain(), Self::extract_account_address_from_event(event)?);
        let library_address = Self::extract_library_address_from_event(event)?;
        
        Ok((account_id, library_address))
    }
    
    /// Parse Valence ownership update event
    fn parse_valence_ownership_update(event: &Box<dyn Event>) -> Result<ValenceOwnershipUpdate> {
        let account_id = format!("{}:{}", event.chain(), Self::extract_account_address_from_event(event)?);
        
        Ok(ValenceOwnershipUpdate {
            account_id,
            new_owner: Self::extract_new_owner_from_event(event)?,
            new_pending_owner: Self::extract_pending_owner_from_event(event)?,
            new_pending_expiry: Self::extract_pending_expiry_from_event(event)?,
        })
    }
    
    /// Parse Valence execution event
    fn parse_valence_execution(event: &Box<dyn Event>) -> Result<ValenceAccountExecution> {
        let account_id = format!("{}:{}", event.chain(), Self::extract_account_address_from_event(event)?);
        
        Ok(ValenceAccountExecution {
            account_id,
            chain_id: event.chain().to_string(),
            block_number: event.block_number(),
            tx_hash: event.tx_hash().to_string(),
            executor_address: Self::extract_executor_address_from_event(event)?,
            message_index: Self::extract_message_index_from_event(event)?,
            correlated_event_ids: None,
            raw_msgs: Self::extract_raw_msgs_from_event(event)?,
            payload: Self::extract_payload_from_event(event)?,
            executed_at: event.timestamp(),
        })
    }
    
    /// Parse Valence processor instantiation event
    fn parse_valence_processor_instantiation(event: &Box<dyn Event>) -> Result<ValenceProcessorInfo> {
        Ok(ValenceProcessorInfo {
            id: format!("{}:{}", event.chain(), event.tx_hash()),
            chain_id: event.chain().to_string(),
            contract_address: Self::extract_contract_address_from_event(event)?,
            created_at_block: event.block_number(),
            created_at_tx: event.tx_hash().to_string(),
            current_owner: Self::extract_owner_from_event(event)?,
            config: None,
            last_updated_block: event.block_number(),
            last_updated_tx: event.tx_hash().to_string(),
        })
    }
    
    /// Parse Valence processor config update event
    fn parse_valence_processor_config_update(event: &Box<dyn Event>) -> Result<(String, ValenceProcessorConfig)> {
        let processor_id = format!("{}:{}", event.chain(), Self::extract_processor_address_from_event(event)?);
        
        let config = ValenceProcessorConfig {
            max_gas_per_message: Self::extract_max_gas_from_event(event)?,
            message_timeout_blocks: Self::extract_timeout_blocks_from_event(event)?,
            retry_interval_blocks: Self::extract_retry_interval_from_event(event)?,
            max_retry_count: Self::extract_max_retry_count_from_event(event)?,
            paused: Self::extract_paused_status_from_event(event)?,
        };
        
        Ok((processor_id, config))
    }
    
    /// Parse Valence processor message event
    fn parse_valence_processor_message(event: &Box<dyn Event>) -> Result<ValenceProcessorMessage> {
        Ok(ValenceProcessorMessage {
            id: format!("{}:{}:{}", event.chain(), event.tx_hash(), event.id()),
            processor_id: format!("{}:{}", event.chain(), Self::extract_processor_address_from_event(event)?),
            source_chain_id: event.chain().to_string(),
            target_chain_id: Self::extract_target_chain_from_event(event)?,
            sender_address: Self::extract_sender_address_from_event(event)?,
            payload: Self::extract_message_payload_from_event(event)?,
            status: ValenceMessageStatus::Pending,
            created_at_block: event.block_number(),
            created_at_tx: event.tx_hash().to_string(),
            last_updated_block: event.block_number(),
            processed_at_block: None,
            processed_at_tx: None,
            retry_count: 0,
            next_retry_block: None,
            gas_used: None,
            error: None,
        })
    }
    
    /// Parse Valence authorization instantiation event
    fn parse_valence_authorization_instantiation(event: &Box<dyn Event>) -> Result<ValenceAuthorizationInfo> {
        Ok(ValenceAuthorizationInfo {
            id: format!("{}:{}", event.chain(), event.tx_hash()),
            chain_id: event.chain().to_string(),
            contract_address: Self::extract_contract_address_from_event(event)?,
            created_at_block: event.block_number(),
            created_at_tx: event.tx_hash().to_string(),
            current_owner: Self::extract_owner_from_event(event)?,
            active_policy_id: None,
            last_updated_block: event.block_number(),
            last_updated_tx: event.tx_hash().to_string(),
        })
    }
    
    /// Parse Valence authorization grant event
    fn parse_valence_authorization_grant(event: &Box<dyn Event>) -> Result<ValenceAuthorizationGrant> {
        let grant = ValenceAuthorizationGrant {
            id: format!("grant_{}_{}", event.id(), chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)),
            auth_id: Self::extract_auth_address_from_event(event)?,
            grantee: Self::extract_grantee_from_event(event)?,
            permissions: Self::extract_permissions_from_event(event)?,
            resources: Self::extract_resources_from_event(event)?,
            conditions: Some(serde_json::json!({})), // Default empty conditions
            expires_at_block: Self::extract_expiry_from_event(event)?,
            created_at_block: event.block_number(),
            updated_at_block: event.block_number(),
            revoked_at_block: None,
            is_active: true,
        };
        
        Ok(grant)
    }
    
    /// Parse Valence library instantiation event
    fn parse_valence_library_instantiation(event: &Box<dyn Event>) -> Result<(ValenceLibraryInfo, Option<ValenceLibraryVersion>)> {
        let library_info = ValenceLibraryInfo {
            id: format!("{}:{}", event.chain(), event.tx_hash()),
            chain_id: event.chain().to_string(),
            contract_address: Self::extract_contract_address_from_event(event)?,
            library_type: Self::extract_library_type_from_event(event)?,
            created_at_block: event.block_number(),
            created_at_tx: event.tx_hash().to_string(),
            current_owner: Self::extract_owner_from_event(event)?,
            current_version: Some(1),
            last_updated_block: event.block_number(),
            last_updated_tx: event.tx_hash().to_string(),
        };
        
        // Create initial version
        let initial_version = Some(ValenceLibraryVersion {
            id: format!("{}:{}:v1", event.chain(), event.tx_hash()),
            library_id: library_info.id.clone(),
            version: 1,
            code_hash: Self::extract_code_hash_from_event(event)?,
            created_at_block: event.block_number(),
            created_at_tx: event.tx_hash().to_string(),
            is_active: true,
            features: Self::extract_features_from_event(event)?,
            metadata: None,
        });
        
        Ok((library_info, initial_version))
    }
    
    /// Parse Valence library version event
    fn parse_valence_library_version(event: &Box<dyn Event>) -> Result<ValenceLibraryVersion> {
        Ok(ValenceLibraryVersion {
            id: format!("{}:{}:v{}", event.chain(), event.tx_hash(), Self::extract_version_number_from_event(event)?),
            library_id: format!("{}:{}", event.chain(), Self::extract_library_address_from_event(event)?),
            version: Self::extract_version_number_from_event(event)?,
            code_hash: Self::extract_code_hash_from_event(event)?,
            created_at_block: event.block_number(),
            created_at_tx: event.tx_hash().to_string(),
            is_active: Self::extract_is_active_from_event(event)?,
            features: Self::extract_features_from_event(event)?,
            metadata: None,
        })
    }
    
    /// Parse Valence library usage event
    fn parse_valence_library_usage(event: &Box<dyn Event>) -> Result<ValenceLibraryUsage> {
        let usage = ValenceLibraryUsage {
            id: format!("usage_{}_{}", event.id(), chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)),
            library_id: Self::extract_library_id_from_event(event)?,
            user_address: Self::extract_user_address_from_event(event)?,
            account_id: Self::extract_account_id_from_event(event)?.unwrap_or_default(),
            function_name: Self::extract_function_name_from_event(event)?.unwrap_or_default(),
            parameters: Self::extract_parameters_from_event(event)?,
            result: Self::extract_result_from_event(event)?,
            created_at_block: event.block_number(),
            transaction_hash: event.tx_hash().to_string(),
            gas_used: Some(Self::extract_gas_used_from_event(event).unwrap_or(0)),
            success: Self::extract_success_from_event(event).unwrap_or(true),
        };
        
        Ok(usage)
    }
    
    // --- Helper Data Extraction Functions ---
    
    /// Extract contract address from event (minimal implementation)
    fn extract_contract_address_from_event(event: &Box<dyn Event>) -> Result<String> {
        // For minimal implementation, derive from tx_hash
        // In a real implementation, this would parse the actual event data
        Ok(format!("contract_{}", &event.tx_hash()[..8]))
    }
    
    /// Extract owner from event
    fn extract_owner_from_event(event: &Box<dyn Event>) -> Result<Option<String>> {
        // For minimal implementation, use a default owner
        Ok(Some(format!("owner_{}", &event.tx_hash()[..8])))
    }
    
    /// Extract account address from event
    fn extract_account_address_from_event(event: &Box<dyn Event>) -> Result<String> {
        Ok(format!("account_{}", &event.tx_hash()[..8]))
    }
    
    /// Extract library address from event
    fn extract_library_address_from_event(event: &Box<dyn Event>) -> Result<String> {
        Ok(format!("library_{}", &event.tx_hash()[..8]))
    }
    
    /// Extract new owner from event
    fn extract_new_owner_from_event(_event: &Box<dyn Event>) -> Result<Option<String>> {
        Ok(Some("new_owner".to_string()))
    }
    
    /// Extract pending owner from event
    fn extract_pending_owner_from_event(_event: &Box<dyn Event>) -> Result<Option<String>> {
        Ok(None)
    }
    
    /// Extract pending expiry from event
    fn extract_pending_expiry_from_event(_event: &Box<dyn Event>) -> Result<Option<u64>> {
        Ok(None)
    }
    
    /// Extract executor address from event
    fn extract_executor_address_from_event(event: &Box<dyn Event>) -> Result<String> {
        Ok(format!("executor_{}", &event.tx_hash()[..8]))
    }
    
    /// Extract message index from event
    fn extract_message_index_from_event(_event: &Box<dyn Event>) -> Result<i32> {
        Ok(0)
    }
    
    /// Extract raw messages from event
    fn extract_raw_msgs_from_event(_event: &Box<dyn Event>) -> Result<Option<serde_json::Value>> {
        Ok(None)
    }
    
    /// Extract payload from event
    fn extract_payload_from_event(_event: &Box<dyn Event>) -> Result<Option<String>> {
        Ok(Some("payload".to_string()))
    }
    
    /// Extract processor address from event
    fn extract_processor_address_from_event(event: &Box<dyn Event>) -> Result<String> {
        Ok(format!("processor_{}", &event.tx_hash()[..8]))
    }
    
    /// Extract max gas from event
    fn extract_max_gas_from_event(_event: &Box<dyn Event>) -> Result<Option<u64>> {
        Ok(Some(1000000))
    }
    
    /// Extract timeout blocks from event
    fn extract_timeout_blocks_from_event(_event: &Box<dyn Event>) -> Result<Option<u64>> {
        Ok(Some(100))
    }
    
    /// Extract retry interval from event
    fn extract_retry_interval_from_event(_event: &Box<dyn Event>) -> Result<Option<u64>> {
        Ok(Some(10))
    }
    
    /// Extract max retry count from event
    fn extract_max_retry_count_from_event(_event: &Box<dyn Event>) -> Result<Option<u32>> {
        Ok(Some(3))
    }
    
    /// Extract paused status from event
    fn extract_paused_status_from_event(_event: &Box<dyn Event>) -> Result<bool> {
        Ok(false)
    }
    
    /// Extract target chain from event
    fn extract_target_chain_from_event(event: &Box<dyn Event>) -> Result<String> {
        Ok(event.chain().to_string())
    }
    
    /// Extract sender address from event
    fn extract_sender_address_from_event(event: &Box<dyn Event>) -> Result<String> {
        Ok(format!("sender_{}", &event.tx_hash()[..8]))
    }
    
    /// Extract message payload from event
    fn extract_message_payload_from_event(_event: &Box<dyn Event>) -> Result<String> {
        Ok("message_payload".to_string())
    }
    
    /// Extract auth address from event
    fn extract_auth_address_from_event(event: &Box<dyn Event>) -> Result<String> {
        Ok(format!("auth_{}", &event.tx_hash()[..8]))
    }
    
    /// Extract grantee from event
    fn extract_grantee_from_event(event: &Box<dyn Event>) -> Result<String> {
        Ok(format!("grantee_{}", &event.tx_hash()[..8]))
    }
    
    /// Extract permissions from event
    fn extract_permissions_from_event(_event: &Box<dyn Event>) -> Result<Vec<String>> {
        Ok(vec!["read".to_string(), "write".to_string()])
    }
    
    /// Extract resources from event
    fn extract_resources_from_event(_event: &Box<dyn Event>) -> Result<Vec<String>> {
        Ok(vec!["resource1".to_string()])
    }
    
    /// Extract expiry from event
    fn extract_expiry_from_event(_event: &Box<dyn Event>) -> Result<Option<u64>> {
        Ok(None)
    }
    
    /// Extract library type from event
    fn extract_library_type_from_event(_event: &Box<dyn Event>) -> Result<String> {
        Ok("generic".to_string())
    }
    
    /// Extract code hash from event
    fn extract_code_hash_from_event(event: &Box<dyn Event>) -> Result<String> {
        Ok(format!("hash_{}", &event.tx_hash()[..8]))
    }
    
    /// Extract features from event
    fn extract_features_from_event(_event: &Box<dyn Event>) -> Result<Vec<String>> {
        Ok(vec!["feature1".to_string()])
    }
    
    /// Extract version number from event
    fn extract_version_number_from_event(_event: &Box<dyn Event>) -> Result<u32> {
        Ok(1)
    }
    
    /// Extract is active from event
    fn extract_is_active_from_event(_event: &Box<dyn Event>) -> Result<bool> {
        Ok(true)
    }
    
    /// Extract user address from event
    fn extract_user_address_from_event(event: &Box<dyn Event>) -> Result<String> {
        Ok(format!("user_{}", &event.tx_hash()[..8]))
    }
    
    /// Extract account ID from event
    fn extract_account_id_from_event(event: &Box<dyn Event>) -> Result<Option<String>> {
        Ok(Some(format!("{}:{}", event.chain(), &event.tx_hash()[..8])))
    }
    
    /// Extract function name from event
    fn extract_function_name_from_event(_event: &Box<dyn Event>) -> Result<Option<String>> {
        Ok(Some("execute".to_string()))
    }
    
    /// Extract gas used from event
    fn extract_gas_used_from_event(_event: &Box<dyn Event>) -> Result<u64> {
        // Extract gas used from event data
        // This is a placeholder - in a real implementation we'd parse the event data
        Ok(0)
    }
    
    /// Extract success status from event
    fn extract_success_from_event(_event: &Box<dyn Event>) -> Result<bool> {
        // Extract success status from event data
        // This is a placeholder - in a real implementation we'd parse the event data
        Ok(true)
    }
    
    /// Extract library ID from event
    fn extract_library_id_from_event(event: &Box<dyn Event>) -> Result<String> {
        Ok(format!("{}:{}", event.chain(), event.tx_hash()))
    }

    /// Extract parameters from event
    fn extract_parameters_from_event(_event: &Box<dyn Event>) -> Result<Option<serde_json::Value>> {
        Ok(Some(serde_json::json!({})))
    }

    /// Extract result from event
    fn extract_result_from_event(_event: &Box<dyn Event>) -> Result<Option<serde_json::Value>> {
        Ok(Some(serde_json::json!({})))
    }
}

/// Special event type for synchronization purposes
#[derive(Debug, Clone)]
struct SyncEvent {
    id: String,
    chain: String,
    block_number: u64,
}

impl Event for SyncEvent {
    fn id(&self) -> &str {
        &self.id
    }
    
    fn chain(&self) -> &str {
        &self.chain
    }
    
    fn block_number(&self) -> u64 {
        self.block_number
    }
    
    fn block_hash(&self) -> &str {
        "sync_block_hash"
    }
    
    fn tx_hash(&self) -> &str {
        "sync_tx_hash"
    }
    
    fn timestamp(&self) -> std::time::SystemTime {
        std::time::SystemTime::now()
    }
    
    fn event_type(&self) -> &str {
        "sync"
    }
    
    fn raw_data(&self) -> &[u8] {
        &[]
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Extension trait for Box<dyn Event> to add cloning capabilities
trait EventExt {
    fn clone(&self) -> Box<dyn Event>;
}

impl EventExt for Box<dyn Event> {
    fn clone(&self) -> Box<dyn Event> {
        Box::new(ClonedEvent {
            id: self.id().to_string(),
            chain: self.chain().to_string(),
            block_number: self.block_number(),
            block_hash: self.block_hash().to_string(),
            tx_hash: self.tx_hash().to_string(),
            timestamp: self.timestamp(),
            event_type: self.event_type().to_string(),
            raw_data: self.raw_data().to_vec(),
        })
    }
}

/// Helper struct for cloning events
#[derive(Debug)]
struct ClonedEvent {
    id: String,
    chain: String,
    block_number: u64,
    block_hash: String,
    tx_hash: String,
    timestamp: std::time::SystemTime,
    event_type: String,
    raw_data: Vec<u8>,
}

impl Event for ClonedEvent {
    fn id(&self) -> &str {
        &self.id
    }
    
    fn chain(&self) -> &str {
        &self.chain
    }
    
    fn block_number(&self) -> u64 {
        self.block_number
    }
    
    fn block_hash(&self) -> &str {
        &self.block_hash
    }
    
    fn tx_hash(&self) -> &str {
        &self.tx_hash
    }
    
    fn timestamp(&self) -> std::time::SystemTime {
        self.timestamp
    }
    
    fn event_type(&self) -> &str {
        &self.event_type
    }
    
    fn raw_data(&self) -> &[u8] {
        &self.raw_data
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
} 