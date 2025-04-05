/// Cosmos blockchain event implementation
use std::time::{SystemTime, UNIX_EPOCH};
use std::any::Any;

use indexer_core::event::Event;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use indexer_common::Result;
use std::collections::HashMap;
use std::fmt;

use cosmrs::tendermint::abci::Event as AbciEvent;
use indexer_storage::{BoxedStorage, ValenceAccountInfo, ValenceAccountLibrary, ValenceAccountExecution};
use tracing::{debug, error, info, trace, warn};

/// Cosmos event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CosmosEvent {
    /// Unique identifier
    id: String,
    
    /// Chain identifier
    chain_id: String,
    
    /// Block number
    block_number: u64,
    
    /// Block hash
    block_hash: String,
    
    /// Transaction hash
    tx_hash: String,
    
    /// Timestamp of the block as unix timestamp
    timestamp: SystemTime,
    
    /// Event type
    event_type: String,
    
    /// Event data attributes
    data: HashMap<String, String>,
    
    /// Raw event data
    raw_data: Vec<u8>,
}

/// Cosmos event type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum CosmosEventType {
    /// Transaction event
    Tx,
    /// Block begin event
    BeginBlock,
    /// Block end event
    EndBlock,
    /// Custom event type
    Custom(String),
}

impl fmt::Display for CosmosEventType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CosmosEventType::Tx => write!(f, "tx"),
            CosmosEventType::BeginBlock => write!(f, "begin_block"),
            CosmosEventType::EndBlock => write!(f, "end_block"),
            CosmosEventType::Custom(s) => write!(f, "{}", s),
        }
    }
}

impl CosmosEvent {
    /// Create a new Cosmos event
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: String,
        chain_id: String,
        block_number: u64,
        block_hash: String,
        tx_hash: String,
        timestamp: u64,
        event_type: String,
        data: HashMap<String, String>,
    ) -> Self {
        // Convert timestamp to SystemTime
        let system_time = if timestamp == 0 {
            SystemTime::now()
        } else {
            UNIX_EPOCH + std::time::Duration::from_secs(timestamp)
        };

        // Serialize data to JSON for raw_data
        let raw_data = serde_json::to_vec(&data).unwrap_or_default();
        
        Self {
            id,
            chain_id,
            block_number,
            block_hash,
            tx_hash,
            timestamp: system_time,
            event_type,
            data,
            raw_data,
        }
    }
    
    /// Create a new event from ABCI event
    pub fn from_abci_event(
        chain_id: String,
        block_number: u64,
        block_hash: String,
        tx_hash: String,
        timestamp: u64,
        abci_event: &AbciEvent,
        event_source: CosmosEventType,
    ) -> Result<Self> {
        // Parse attributes
        let mut data = HashMap::new();
        for attr in &abci_event.attributes {
            // Convert Vec<u8> to strings
            let key = String::from_utf8_lossy(attr.key.as_ref()).to_string();
            let value = String::from_utf8_lossy(attr.value.as_ref()).to_string();
            data.insert(key, value);
        }
        
        // Format event type - include source
        let event_type = format!("{}_{}", event_source, abci_event.kind);
        
        // Create a raw data representation
        let raw_data = serde_json::to_vec(&data).unwrap_or_default();
        
        // Convert timestamp to SystemTime
        let system_time = if timestamp == 0 {
            SystemTime::now()
        } else {
            UNIX_EPOCH + std::time::Duration::from_secs(timestamp)
        };
        
        Ok(Self {
            id: Uuid::new_v4().to_string(),
            chain_id,
            block_number,
            block_hash,
            tx_hash,
            timestamp: system_time,
            event_type,
            data,
            raw_data,
        })
    }
    
    /// Create a mock event for testing
    pub fn new_mock(
        chain_id: String,
        block_number: u64,
        event_type: String,
    ) -> Self {
        // Current timestamp as SystemTime
        let timestamp = SystemTime::now();
        let data = HashMap::new();
        let raw_data = Vec::new();
            
        Self {
            id: Uuid::new_v4().to_string(),
            chain_id,
            block_number,
            block_hash: format!("block_hash_{}", block_number),
            tx_hash: format!("tx_hash_{}", block_number),
            timestamp,
            event_type,
            data,
            raw_data,
        }
    }
    
    /// Get attribute value by key
    pub fn get_attribute(&self, key: &str) -> Option<&String> {
        self.data.get(key)
    }
    
    /// Check if this event matches a specific event type
    pub fn is_event_type(&self, event_type: &str) -> bool {
        self.event_type == event_type
    }
    
    /// Check if this event has a specific attribute
    pub fn has_attribute(&self, key: &str, value: &str) -> bool {
        match self.data.get(key) {
            Some(attr_value) => attr_value == value,
            None => false,
        }
    }
    
    /// Get event data
    pub fn data(&self) -> &HashMap<String, String> {
        &self.data
    }
}

impl Event for CosmosEvent {
    fn id(&self) -> &str {
        &self.id
    }
    
    fn chain(&self) -> &str {
        &self.chain_id
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
    
    fn timestamp(&self) -> SystemTime {
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

/// Process Valence Account-related events from a CosmWasm contract
pub async fn process_valence_account_event(
    storage: &BoxedStorage,
    chain_id: &str,
    event: &CosmosEvent,
    tx_hash: &str
) -> Result<()> {
    debug!(
        chain_id = %chain_id,
        tx_hash = %tx_hash,
        event_type = %event.event_type,
        "Processing Valence Account event"
    );
    
    // Extract contract address from event
    let contract_address = match event.get_attribute("_contract_address") {
        Some(addr) => addr,
        None => {
            warn!(event_type = %event.event_type, "Missing contract address in Valence Account event");
            return Err(Error::generic("Missing contract address in Valence Account event"));
        }
    };
    
    // Create a unique ID for this account
    let account_id = format!("{}:{}", chain_id, contract_address);
    
    // Based on event type, process accordingly
    match event.event_type.as_str() {
        "instantiate" => {
            debug!(account_id = %account_id, "Processing Valence Account instantiation");
            
            // Extract owner from attributes
            let owner = event.get_attribute("owner").or_else(|| event.get_attribute("current_owner"));
            
            let account_info = ValenceAccountInfo {
                id: account_id.clone(),
                chain_id: chain_id.to_string(),
                contract_address: contract_address.to_string(),
                created_at_block: event.block_number,
                created_at_tx: tx_hash.to_string(),
                current_owner: owner.map(ToString::to_string),
                pending_owner: None,  // No pending owner at instantiation
                pending_owner_expiry: None,
                last_updated_block: event.block_number, 
                last_updated_tx: tx_hash.to_string(),
            };
            
            // Store the account instantiation with no initial libraries
            storage.store_valence_account_instantiation(account_info, Vec::new()).await?;
            
            info!(
                account_id = %account_id,
                owner = ?owner,
                "Indexed Valence Account instantiation"
            );
        },
        
        "execute" => {
            // Check if this is a library approval or ownership change event
            if let Some(action) = event.get_attribute("action") {
                match action {
                    "approve_library" | "add_library" => {
                        if let Some(library_address) = event.get_attribute("library") {
                            debug!(
                                account_id = %account_id,
                                library = %library_address,
                                "Processing library approval"
                            );
                            
                            let library_info = ValenceAccountLibrary {
                                account_id: account_id.clone(),
                                library_address: library_address.to_string(),
                                approved_at_block: event.block_number,
                                approved_at_tx: tx_hash.to_string(),
                            };
                            
                            storage.store_valence_library_approval(
                                &account_id,
                                library_info,
                                event.block_number,
                                tx_hash
                            ).await?;
                            
                            info!(
                                account_id = %account_id,
                                library = %library_address,
                                "Indexed library approval"
                            );
                        } else {
                            warn!(account_id = %account_id, "Library approval event missing library address");
                        }
                    },
                    
                    "transfer_ownership" | "update_owner" => {
                        debug!(account_id = %account_id, "Processing ownership update");
                        
                        // Get new owner values
                        let new_owner = event.get_attribute("new_owner").map(ToString::to_string);
                        let pending_owner = event.get_attribute("pending_owner").map(ToString::to_string);
                        let expiry = event.get_attribute("expiry").and_then(|e| e.parse::<u64>().ok());
                        
                        storage.store_valence_ownership_update(
                            &account_id,
                            new_owner.clone(),
                            pending_owner.clone(),
                            expiry,
                            event.block_number,
                            tx_hash
                        ).await?;
                        
                        info!(
                            account_id = %account_id,
                            new_owner = ?new_owner,
                            pending_owner = ?pending_owner,
                            "Indexed ownership update"
                        );
                    },
                    
                    "execute_msgs" | "execute_submsgs" => {
                        debug!(account_id = %account_id, "Processing account execution");
                        
                        // Extract execution details
                        let executor = event.get_attribute("sender")
                            .or_else(|| event.get_attribute("executor"))
                            .unwrap_or("unknown");
                        
                        let execution_info = ValenceAccountExecution {
                            account_id: account_id.clone(),
                            chain_id: chain_id.to_string(),
                            block_number: event.block_number,
                            tx_hash: tx_hash.to_string(),
                            executor_address: executor.to_string(),
                            message_index: event.get_attribute("msg_index")
                                .and_then(|idx| idx.parse::<i32>().ok())
                                .unwrap_or(0),
                            correlated_event_ids: None, // Would need separate correlation logic
                            raw_msgs: event.get_attribute("msgs")
                                .map(|m| serde_json::from_str(m).unwrap_or(serde_json::Value::Null)),
                            payload: event.get_attribute("payload").map(ToString::to_string),
                            executed_at: SystemTime::now(),
                        };
                        
                        storage.store_valence_execution(execution_info).await?;
                        
                        info!(
                            account_id = %account_id,
                            executor = %executor,
                            "Indexed account execution"
                        );
                    },
                    
                    _ => {
                        debug!(account_id = %account_id, action = %action, "Unknown account action, skipping");
                    }
                }
            }
        },
        
        "wasm" => {
            // Some chains might emit generic "wasm" events
            // Check for signature attributes that indicate account events
            if event.has_attribute("action", "approve_library") || 
               event.has_attribute("action", "add_library") {
                // Process as library approval
                if let Some(library_address) = event.get_attribute("library") {
                    let library_info = ValenceAccountLibrary {
                        account_id: account_id.clone(),
                        library_address: library_address.to_string(),
                        approved_at_block: event.block_number,
                        approved_at_tx: tx_hash.to_string(),
                    };
                    
                    storage.store_valence_library_approval(
                        &account_id,
                        library_info,
                        event.block_number,
                        tx_hash
                    ).await?;
                    
                    info!(
                        account_id = %account_id,
                        library = %library_address,
                        "Indexed library approval from wasm event"
                    );
                }
            }
        },
        
        _ => {
            debug!(
                account_id = %account_id,
                event_type = %event.event_type,
                "Unrecognized Valence Account event type, skipping"
            );
        }
    }
    
    Ok(())
}

/// Process Valence Processor-related events from a CosmWasm contract
pub async fn process_valence_processor_event(
    storage: &BoxedStorage,
    chain_id: &str,
    event: &CosmosEvent,
    tx_hash: &str
) -> Result<()> {
    debug!(
        chain_id = %chain_id,
        tx_hash = %tx_hash,
        event_type = %event.event_type,
        "Processing Valence Processor event"
    );
    
    // Extract contract address from event
    let contract_address = match event.get_attribute("_contract_address") {
        Some(addr) => addr,
        None => {
            warn!(event_type = %event.event_type, "Missing contract address in Valence Processor event");
            return Err(Error::generic("Missing contract address in Valence Processor event"));
        }
    };
    
    // Create a unique ID for this processor
    let processor_id = format!("{}:{}", chain_id, contract_address);
    
    // Based on event type, process accordingly
    match event.event_type.as_str() {
        "instantiate" => {
            debug!(processor_id = %processor_id, "Processing Valence Processor instantiation");
            
            // Extract owner from attributes
            let owner = event.get_attribute("owner").or_else(|| event.get_attribute("current_owner"));
            
            // Extract configuration values
            let max_gas = event.get_attribute("max_gas_per_message").and_then(|v| v.parse::<u64>().ok());
            let timeout = event.get_attribute("message_timeout_blocks").and_then(|v| v.parse::<u64>().ok());
            let retry_interval = event.get_attribute("retry_interval_blocks").and_then(|v| v.parse::<u64>().ok());
            let max_retries = event.get_attribute("max_retry_count").and_then(|v| v.parse::<u32>().ok());
            let paused = event.get_attribute("paused").map_or(false, |v| v == "true");
            
            let config = indexer_storage::ValenceProcessorConfig {
                max_gas_per_message: max_gas,
                message_timeout_blocks: timeout,
                retry_interval_blocks: retry_interval,
                max_retry_count: max_retries,
                paused,
            };
            
            let processor_info = indexer_storage::ValenceProcessorInfo {
                id: processor_id.clone(),
                chain_id: chain_id.to_string(),
                contract_address: contract_address.to_string(),
                created_at_block: event.block_number,
                created_at_tx: tx_hash.to_string(),
                current_owner: owner.map(ToString::to_string),
                config: Some(config),
                last_updated_block: event.block_number, 
                last_updated_tx: tx_hash.to_string(),
            };
            
            // Store the processor instantiation
            storage.store_valence_processor_instantiation(processor_info).await?;
            
            info!(
                processor_id = %processor_id,
                owner = ?owner,
                "Indexed Valence Processor instantiation"
            );
        },
        
        "execute" => {
            // Check action type for processor operations
            if let Some(action) = event.get_attribute("action") {
                match action {
                    "update_config" | "set_config" => {
                        debug!(processor_id = %processor_id, "Processing processor config update");
                        
                        // Extract updated configuration values
                        let max_gas = event.get_attribute("max_gas_per_message").and_then(|v| v.parse::<u64>().ok());
                        let timeout = event.get_attribute("message_timeout_blocks").and_then(|v| v.parse::<u64>().ok());
                        let retry_interval = event.get_attribute("retry_interval_blocks").and_then(|v| v.parse::<u64>().ok());
                        let max_retries = event.get_attribute("max_retry_count").and_then(|v| v.parse::<u32>().ok());
                        let paused = event.get_attribute("paused").map_or(false, |v| v == "true");
                        
                        let config = indexer_storage::ValenceProcessorConfig {
                            max_gas_per_message: max_gas,
                            message_timeout_blocks: timeout,
                            retry_interval_blocks: retry_interval,
                            max_retry_count: max_retries,
                            paused,
                        };
                        
                        storage.store_valence_processor_config_update(
                            &processor_id,
                            config,
                            event.block_number,
                            tx_hash
                        ).await?;
                        
                        info!(
                            processor_id = %processor_id,
                            "Indexed processor config update"
                        );
                    },
                    
                    "submit_message" | "enqueue_message" => {
                        debug!(processor_id = %processor_id, "Processing message submission");
                        
                        // Extract message details
                        let source_chain = event.get_attribute("source_chain").unwrap_or(chain_id);
                        let target_chain = event.get_attribute("target_chain").unwrap_or("unknown");
                        let sender = event.get_attribute("sender").unwrap_or("unknown");
                        let message_id = event.get_attribute("message_id").unwrap_or_else(|| tx_hash);
                        let payload = event.get_attribute("payload").unwrap_or("");
                        
                        let message = indexer_storage::ValenceProcessorMessage {
                            id: message_id.to_string(),
                            processor_id: processor_id.clone(),
                            source_chain_id: source_chain.to_string(),
                            target_chain_id: target_chain.to_string(),
                            sender_address: sender.to_string(),
                            payload: payload.to_string(),
                            status: indexer_storage::ValenceMessageStatus::Pending,
                            created_at_block: event.block_number,
                            created_at_tx: tx_hash.to_string(),
                            last_updated_block: event.block_number,
                            processed_at_block: None,
                            processed_at_tx: None,
                            retry_count: 0,
                            next_retry_block: None,
                            gas_used: None,
                            error: None,
                        };
                        
                        storage.store_valence_processor_message(message).await?;
                        
                        info!(
                            processor_id = %processor_id,
                            message_id = %message_id,
                            target_chain = %target_chain,
                            "Indexed cross-chain message submission"
                        );
                    },
                    
                    "process_message" | "execute_message" => {
                        debug!(processor_id = %processor_id, "Processing message execution");
                        
                        // Extract message execution details
                        let message_id = event.get_attribute("message_id").unwrap_or("");
                        let success = event.get_attribute("success").map_or(true, |v| v == "true");
                        let gas_used = event.get_attribute("gas_used").and_then(|v| v.parse::<u64>().ok());
                        let error_msg = event.get_attribute("error");
                        
                        let status = if success {
                            indexer_storage::ValenceMessageStatus::Completed
                        } else {
                            indexer_storage::ValenceMessageStatus::Failed
                        };
                        
                        storage.update_valence_processor_message_status(
                            message_id,
                            status,
                            Some(event.block_number),
                            Some(tx_hash),
                            None, // Retry count wouldn't change here
                            None, // Next retry block set separately
                            gas_used,
                            error_msg.map(ToString::to_string),
                        ).await?;
                        
                        info!(
                            processor_id = %processor_id,
                            message_id = %message_id,
                            success = %success,
                            "Indexed cross-chain message execution"
                        );
                    },
                    
                    "retry_message" => {
                        debug!(processor_id = %processor_id, "Processing message retry scheduling");
                        
                        // Extract retry details
                        let message_id = event.get_attribute("message_id").unwrap_or("");
                        let retry_count = event.get_attribute("retry_count").and_then(|v| v.parse::<u32>().ok());
                        let next_retry = event.get_attribute("next_retry_block").and_then(|v| v.parse::<u64>().ok());
                        
                        storage.update_valence_processor_message_status(
                            message_id,
                            indexer_storage::ValenceMessageStatus::Failed, // Still failed until retried
                            None, // Block number doesn't change
                            None, // Tx hash doesn't change
                            retry_count,
                            next_retry,
                            None, // Gas used doesn't change
                            None, // Error message doesn't change
                        ).await?;
                        
                        info!(
                            processor_id = %processor_id,
                            message_id = %message_id,
                            retry_count = ?retry_count,
                            next_retry = ?next_retry,
                            "Indexed cross-chain message retry scheduling"
                        );
                    },
                    
                    "timeout_message" => {
                        debug!(processor_id = %processor_id, "Processing message timeout");
                        
                        // Extract message details
                        let message_id = event.get_attribute("message_id").unwrap_or("");
                        
                        storage.update_valence_processor_message_status(
                            message_id,
                            indexer_storage::ValenceMessageStatus::TimedOut,
                            Some(event.block_number),
                            Some(tx_hash),
                            None, // Retry count doesn't change
                            None, // Next retry block doesn't matter
                            None, // Gas used doesn't change
                            Some("Message timed out".to_string()),
                        ).await?;
                        
                        info!(
                            processor_id = %processor_id,
                            message_id = %message_id,
                            "Indexed cross-chain message timeout"
                        );
                    },
                    
                    _ => {
                        debug!(processor_id = %processor_id, action = %action, "Unknown processor action, skipping");
                    }
                }
            }
        },
        
        _ => {
            debug!(
                processor_id = %processor_id,
                event_type = %event.event_type,
                "Unrecognized Valence Processor event type, skipping"
            );
        }
    }
    
    Ok(())
}

/// Process Valence Authorization-related events from a CosmWasm contract
pub async fn process_valence_authorization_event(
    storage: &BoxedStorage,
    chain_id: &str,
    event: &CosmosEvent,
    tx_hash: &str
) -> Result<()> {
    debug!(
        chain_id = %chain_id,
        tx_hash = %tx_hash,
        event_type = %event.event_type,
        "Processing Valence Authorization event"
    );
    
    // Extract contract address from event
    let contract_address = match event.get_attribute("_contract_address") {
        Some(addr) => addr,
        None => {
            warn!(event_type = %event.event_type, "Missing contract address in Valence Authorization event");
            return Err(Error::generic("Missing contract address in Valence Authorization event"));
        }
    };
    
    // Create a unique ID for this authorization contract
    let auth_id = format!("{}:{}", chain_id, contract_address);
    
    // Based on event type, process accordingly
    match event.event_type.as_str() {
        "instantiate" => {
            debug!(auth_id = %auth_id, "Processing Valence Authorization instantiation");
            
            // Extract owner from attributes
            let owner = event.get_attribute("owner").or_else(|| event.get_attribute("current_owner"));
            
            let auth_info = indexer_storage::ValenceAuthorizationInfo {
                id: auth_id.clone(),
                chain_id: chain_id.to_string(),
                contract_address: contract_address.to_string(),
                created_at_block: event.block_number(),
                created_at_tx: tx_hash.to_string(),
                current_owner: owner.map(ToString::to_string),
                active_policy_id: None,  // No active policy at instantiation
                last_updated_block: event.block_number(),
                last_updated_tx: tx_hash.to_string(),
            };
            
            // Look for initial policy in the event attributes
            let initial_policy = if let (Some(policy_id), Some(policy_version), Some(content_hash)) = (
                event.get_attribute("policy_id"), 
                event.get_attribute("policy_version").and_then(|v| v.parse::<u32>().ok()),
                event.get_attribute("policy_hash")
            ) {
                let policy = indexer_storage::ValenceAuthorizationPolicy {
                    id: policy_id.to_string(),
                    auth_id: auth_id.clone(),
                    version: policy_version,
                    content_hash: content_hash.to_string(),
                    created_at_block: event.block_number(),
                    created_at_tx: tx_hash.to_string(),
                    is_active: true, // Initial policy is active
                    metadata: None,
                };
                Some(policy)
            } else {
                None
            };
            
            // Store the authorization instantiation
            storage.store_valence_authorization_instantiation(auth_info, initial_policy).await?;
            
            info!(
                auth_id = %auth_id,
                owner = ?owner,
                has_initial_policy = %initial_policy.is_some(),
                "Indexed Valence Authorization instantiation"
            );
        },
        
        "execute" => {
            // Check if this is a policy update, grant, or other authorization event
            if let Some(action) = event.get_attribute("action") {
                match action {
                    "create_policy" | "update_policy" => {
                        debug!(auth_id = %auth_id, "Processing policy creation/update");
                        
                        if let (Some(policy_id), Some(version_str), Some(content_hash)) = (
                            event.get_attribute("policy_id"),
                            event.get_attribute("policy_version"),
                            event.get_attribute("content_hash"),
                        ) {
                            let version = version_str.parse::<u32>().unwrap_or(1);
                            
                            let policy = indexer_storage::ValenceAuthorizationPolicy {
                                id: policy_id.to_string(),
                                auth_id: auth_id.clone(),
                                version,
                                content_hash: content_hash.to_string(),
                                created_at_block: event.block_number(),
                                created_at_tx: tx_hash.to_string(),
                                is_active: false, // New policies aren't active by default
                                metadata: event.get_attribute("metadata")
                                    .and_then(|m| serde_json::from_str(m).ok()),
                            };
                            
                            storage.store_valence_authorization_policy(policy).await?;
                            
                            let is_active = event.get_attribute("is_active").map_or(false, |v| v == "true");
                            if is_active {
                                storage.update_active_authorization_policy(
                                    &auth_id,
                                    policy_id,
                                    event.block_number(),
                                    tx_hash
                                ).await?;
                            }
                            
                            info!(
                                auth_id = %auth_id,
                                policy_id = %policy_id,
                                version = %version,
                                "Indexed authorization policy creation/update"
                            );
                        } else {
                            warn!(auth_id = %auth_id, "Policy creation/update event missing required attributes");
                        }
                    },
                    
                    "activate_policy" => {
                        debug!(auth_id = %auth_id, "Processing policy activation");
                        
                        if let Some(policy_id) = event.get_attribute("policy_id") {
                            storage.update_active_authorization_policy(
                                &auth_id,
                                policy_id,
                                event.block_number(),
                                tx_hash
                            ).await?;
                            
                            info!(
                                auth_id = %auth_id,
                                policy_id = %policy_id,
                                "Indexed policy activation"
                            );
                        } else {
                            warn!(auth_id = %auth_id, "Policy activation event missing policy_id");
                        }
                    },
                    
                    "grant" | "grant_permission" => {
                        debug!(auth_id = %auth_id, "Processing permission grant");
                        
                        if let (Some(grantee), Some(permissions_str), Some(resources_str)) = (
                            event.get_attribute("grantee"),
                            event.get_attribute("permissions"),
                            event.get_attribute("resources"),
                        ) {
                            // Parse comma-separated permissions and resources
                            let permissions: Vec<String> = permissions_str
                                .split(',')
                                .map(|s| s.trim().to_string())
                                .collect();
                            
                            let resources: Vec<String> = resources_str
                                .split(',')
                                .map(|s| s.trim().to_string())
                                .collect();
                            
                            // Generate grant ID
                            let grant_id = format!("{}:{}:{}", auth_id, grantee, permissions_str);
                            let expiry = event.get_attribute("expiry").and_then(|e| e.parse::<u64>().ok());
                            
                            let grant = indexer_storage::ValenceAuthorizationGrant {
                                id: grant_id,
                                auth_id: auth_id.clone(),
                                grantee: grantee.to_string(),
                                permissions,
                                resources,
                                granted_at_block: event.block_number(),
                                granted_at_tx: tx_hash.to_string(),
                                expiry,
                                is_active: true,
                                revoked_at_block: None,
                                revoked_at_tx: None,
                            };
                            
                            storage.store_valence_authorization_grant(grant).await?;
                            
                            info!(
                                auth_id = %auth_id,
                                grantee = %grantee,
                                permissions = %permissions_str,
                                "Indexed permission grant"
                            );
                        } else {
                            warn!(auth_id = %auth_id, "Grant event missing required attributes");
                        }
                    },
                    
                    "revoke" | "revoke_permission" => {
                        debug!(auth_id = %auth_id, "Processing permission revocation");
                        
                        if let (Some(grantee), Some(resource)) = (
                            event.get_attribute("grantee"),
                            event.get_attribute("resource"),
                        ) {
                            storage.revoke_valence_authorization_grant(
                                &auth_id,
                                grantee,
                                resource,
                                event.block_number(),
                                tx_hash
                            ).await?;
                            
                            info!(
                                auth_id = %auth_id,
                                grantee = %grantee,
                                resource = %resource,
                                "Indexed permission revocation"
                            );
                        } else {
                            warn!(auth_id = %auth_id, "Revocation event missing required attributes");
                        }
                    },
                    
                    "request" | "authorization_request" => {
                        debug!(auth_id = %auth_id, "Processing authorization request");
                        
                        if let (Some(requester), Some(action_type), Some(resource)) = (
                            event.get_attribute("requester"),
                            event.get_attribute("action"),
                            event.get_attribute("resource"),
                        ) {
                            // Generate request ID
                            let request_id = format!("{}:{}:{}:{}", auth_id, requester, action_type, resource);
                            
                            // Check if there's a decision in this event
                            let decision_str = event.get_attribute("decision").unwrap_or("pending");
                            let decision = match decision_str {
                                "approved" => indexer_storage::ValenceAuthorizationDecision::Approved,
                                "denied" => indexer_storage::ValenceAuthorizationDecision::Denied,
                                "error" => indexer_storage::ValenceAuthorizationDecision::Error,
                                _ => indexer_storage::ValenceAuthorizationDecision::Pending,
                            };
                            
                            let request = indexer_storage::ValenceAuthorizationRequest {
                                id: request_id,
                                auth_id: auth_id.clone(),
                                requester: requester.to_string(),
                                action: action_type.to_string(),
                                resource: resource.to_string(),
                                request_data: event.get_attribute("request_data").map(ToString::to_string),
                                decision,
                                requested_at_block: event.block_number(),
                                requested_at_tx: tx_hash.to_string(),
                                processed_at_block: if decision != indexer_storage::ValenceAuthorizationDecision::Pending {
                                    Some(event.block_number())
                                } else {
                                    None
                                },
                                processed_at_tx: if decision != indexer_storage::ValenceAuthorizationDecision::Pending {
                                    Some(tx_hash.to_string())
                                } else {
                                    None
                                },
                                reason: event.get_attribute("reason").map(ToString::to_string),
                            };
                            
                            storage.store_valence_authorization_request(request).await?;
                            
                            info!(
                                auth_id = %auth_id,
                                requester = %requester,
                                action = %action_type,
                                resource = %resource,
                                decision = ?decision,
                                "Indexed authorization request"
                            );
                        } else {
                            warn!(auth_id = %auth_id, "Authorization request event missing required attributes");
                        }
                    },
                    
                    "decide" | "process_request" => {
                        debug!(auth_id = %auth_id, "Processing authorization decision");
                        
                        if let (Some(request_id), Some(decision_str)) = (
                            event.get_attribute("request_id"),
                            event.get_attribute("decision"),
                        ) {
                            let decision = match decision_str {
                                "approved" => indexer_storage::ValenceAuthorizationDecision::Approved,
                                "denied" => indexer_storage::ValenceAuthorizationDecision::Denied,
                                "error" => indexer_storage::ValenceAuthorizationDecision::Error,
                                _ => indexer_storage::ValenceAuthorizationDecision::Pending,
                            };
                            
                            storage.update_valence_authorization_request_decision(
                                request_id,
                                decision,
                                Some(event.block_number()),
                                Some(tx_hash),
                                event.get_attribute("reason").map(ToString::to_string),
                            ).await?;
                            
                            info!(
                                auth_id = %auth_id,
                                request_id = %request_id,
                                decision = ?decision,
                                "Indexed authorization decision"
                            );
                        } else {
                            warn!(auth_id = %auth_id, "Decision event missing required attributes");
                        }
                    },
                    
                    _ => {
                        debug!(auth_id = %auth_id, action = %action, "Unknown authorization action, skipping");
                    }
                }
            }
        },
        
        _ => {
            debug!(
                auth_id = %auth_id,
                event_type = %event.event_type,
                "Unrecognized Valence Authorization event type, skipping"
            );
        }
    }
    
    Ok(())
}

/// Process Valence Library-related events from a CosmWasm contract
pub async fn process_valence_library_event(
    storage: &BoxedStorage,
    chain_id: &str,
    event: &CosmosEvent,
    tx_hash: &str
) -> Result<()> {
    debug!(
        chain_id = %chain_id,
        tx_hash = %tx_hash,
        event_type = %event.event_type,
        "Processing Valence Library event"
    );
    
    // Extract contract address from event
    let contract_address = match event.get_attribute("_contract_address") {
        Some(addr) => addr,
        None => {
            warn!(event_type = %event.event_type, "Missing contract address in Valence Library event");
            return Err(Error::generic("Missing contract address in Valence Library event"));
        }
    };
    
    // Create a unique ID for this library contract
    let library_id = format!("{}:{}", chain_id, contract_address);
    
    // Based on event type, process accordingly
    match event.event_type.as_str() {
        "instantiate" => {
            debug!(library_id = %library_id, "Processing Valence Library instantiation");
            
            // Extract owner from attributes
            let owner = event.get_attribute("owner").or_else(|| event.get_attribute("current_owner"));
            let library_type = event.get_attribute("library_type").unwrap_or("unknown");
            
            let library_info = indexer_storage::ValenceLibraryInfo {
                id: library_id.clone(),
                chain_id: chain_id.to_string(),
                contract_address: contract_address.to_string(),
                library_type: library_type.to_string(),
                created_at_block: event.block_number(),
                created_at_tx: tx_hash.to_string(),
                current_owner: owner.map(ToString::to_string),
                current_version: None,  // No version at instantiation
                last_updated_block: event.block_number(),
                last_updated_tx: tx_hash.to_string(),
            };
            
            // Look for initial version in the event attributes
            let initial_version = if let (Some(version_str), Some(code_hash)) = (
                event.get_attribute("version").and_then(|v| v.parse::<u32>().ok()),
                event.get_attribute("code_hash")
            ) {
                let version_id = format!("{}:v{}", library_id, version_str);
                
                // Extract features if present
                let features = event.get_attribute("features")
                    .map(|f| f.split(',').map(|s| s.trim().to_string()).collect())
                    .unwrap_or_else(Vec::new);
                
                let version = indexer_storage::ValenceLibraryVersion {
                    id: version_id,
                    library_id: library_id.clone(),
                    version: version_str,
                    code_hash: code_hash.to_string(),
                    created_at_block: event.block_number(),
                    created_at_tx: tx_hash.to_string(),
                    is_active: true, // Initial version is active
                    features,
                    metadata: event.get_attribute("metadata")
                        .and_then(|m| serde_json::from_str(m).ok()),
                };
                Some(version)
            } else {
                None
            };
            
            // Store the library instantiation
            storage.store_valence_library_instantiation(library_info, initial_version).await?;
            
            info!(
                library_id = %library_id,
                owner = ?owner,
                library_type = %library_type,
                has_initial_version = %initial_version.is_some(),
                "Indexed Valence Library instantiation"
            );
        },
        
        "execute" => {
            // Check if this is a version update, usage event, or approval event
            if let Some(action) = event.get_attribute("action") {
                match action {
                    "add_version" | "update_version" => {
                        debug!(library_id = %library_id, "Processing library version addition/update");
                        
                        if let (Some(version_str), Some(code_hash)) = (
                            event.get_attribute("version").and_then(|v| v.parse::<u32>().ok()),
                            event.get_attribute("code_hash"),
                        ) {
                            let version_id = format!("{}:v{}", library_id, version_str);
                            
                            // Extract features if present
                            let features = event.get_attribute("features")
                                .map(|f| f.split(',').map(|s| s.trim().to_string()).collect())
                                .unwrap_or_else(Vec::new);
                            
                            let is_active = event.get_attribute("is_active").map_or(false, |v| v == "true");
                            
                            let version = indexer_storage::ValenceLibraryVersion {
                                id: version_id,
                                library_id: library_id.clone(),
                                version: version_str,
                                code_hash: code_hash.to_string(),
                                created_at_block: event.block_number(),
                                created_at_tx: tx_hash.to_string(),
                                is_active,
                                features,
                                metadata: event.get_attribute("metadata")
                                    .and_then(|m| serde_json::from_str(m).ok()),
                            };
                            
                            storage.store_valence_library_version(version).await?;
                            
                            // If this version is active, update the library's current version
                            if is_active {
                                storage.update_active_library_version(
                                    &library_id,
                                    version_str,
                                    event.block_number(),
                                    tx_hash
                                ).await?;
                            }
                            
                            info!(
                                library_id = %library_id,
                                version = %version_str,
                                is_active = %is_active,
                                "Indexed library version"
                            );
                        } else {
                            warn!(library_id = %library_id, "Version addition/update event missing required attributes");
                        }
                    },
                    
                    "activate_version" => {
                        debug!(library_id = %library_id, "Processing version activation");
                        
                        if let Some(version_str) = event.get_attribute("version").and_then(|v| v.parse::<u32>().ok()) {
                            storage.update_active_library_version(
                                &library_id,
                                version_str,
                                event.block_number(),
                                tx_hash
                            ).await?;
                            
                            info!(
                                library_id = %library_id,
                                version = %version_str,
                                "Indexed version activation"
                            );
                        } else {
                            warn!(library_id = %library_id, "Version activation event missing version");
                        }
                    },
                    
                    "approve" | "authorize" => {
                        debug!(library_id = %library_id, "Processing library approval");
                        
                        if let Some(account_id) = event.get_attribute("account_id") {
                            let approval_id = format!("{}:{}", library_id, account_id);
                            
                            let approval = indexer_storage::ValenceLibraryApproval {
                                id: approval_id,
                                library_id: library_id.clone(),
                                account_id: account_id.to_string(),
                                approved_at_block: event.block_number(),
                                approved_at_tx: tx_hash.to_string(),
                                is_active: true,
                                revoked_at_block: None,
                                revoked_at_tx: None,
                            };
                            
                            storage.store_valence_library_approval(approval).await?;
                            
                            info!(
                                library_id = %library_id,
                                account_id = %account_id,
                                "Indexed library approval"
                            );
                        } else {
                            warn!(library_id = %library_id, "Library approval event missing account_id");
                        }
                    },
                    
                    "revoke" | "deauthorize" => {
                        debug!(library_id = %library_id, "Processing library approval revocation");
                        
                        if let Some(account_id) = event.get_attribute("account_id") {
                            storage.revoke_valence_library_approval(
                                &library_id,
                                account_id,
                                event.block_number(),
                                tx_hash
                            ).await?;
                            
                            info!(
                                library_id = %library_id,
                                account_id = %account_id,
                                "Indexed library approval revocation"
                            );
                        } else {
                            warn!(library_id = %library_id, "Library revocation event missing account_id");
                        }
                    },
                    
                    "use" | "execute" => {
                        debug!(library_id = %library_id, "Processing library usage");
                        
                        let user_address = event.get_attribute("user")
                            .or_else(|| event.get_attribute("executor"))
                            .or_else(|| event.get_attribute("sender"))
                            .unwrap_or("unknown");
                        
                        let usage_id = format!("{}:{}:{}", library_id, user_address, tx_hash);
                        
                        let usage = indexer_storage::ValenceLibraryUsage {
                            id: usage_id,
                            library_id: library_id.clone(),
                            user_address: user_address.to_string(),
                            account_id: event.get_attribute("account_id").map(ToString::to_string),
                            function_name: event.get_attribute("function").map(ToString::to_string),
                            usage_at_block: event.block_number(),
                            usage_at_tx: tx_hash.to_string(),
                            gas_used: event.get_attribute("gas_used").and_then(|g| g.parse::<u64>().ok()),
                            success: event.get_attribute("success").map_or(true, |s| s == "true"),
                            error: event.get_attribute("error").map(ToString::to_string),
                        };
                        
                        storage.store_valence_library_usage(usage).await?;
                        
                        info!(
                            library_id = %library_id,
                            user = %user_address,
                            function = ?event.get_attribute("function"),
                            "Indexed library usage"
                        );
                    },
                    
                    _ => {
                        debug!(library_id = %library_id, action = %action, "Unknown library action, skipping");
                    }
                }
            }
        },
        
        _ => {
            debug!(
                library_id = %library_id,
                event_type = %event.event_type,
                "Unrecognized Valence Library event type, skipping"
            );
        }
    }
    
    Ok(())
} 