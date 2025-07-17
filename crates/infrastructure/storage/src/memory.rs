use crate::Storage;
use almanac_infrastructure_core::{BlockStatus, Error, Result};
use almanac_infrastructure_core::types::{Event, EventFilter};
use crate::{
    ValenceAccountInfo, ValenceAccountLibrary, ValenceAccountExecution, ValenceAccountState,
    ValenceAuthorizationGrant, ValenceAuthorizationInfo, ValenceAuthorizationPolicy, 
    ValenceAuthorizationRequest, ValenceLibraryApproval, ValenceLibraryInfo, 
    ValenceLibraryUsage, ValenceLibraryVersion, ValenceProcessorInfo,
    ValenceProcessorMessage, ValenceProcessorState,
    ValenceLibraryApprovalRequest, ValenceLibraryRemovalRequest, ValenceOwnershipUpdateRequest,
    ValenceProcessorConfigUpdateRequest, ValenceProcessorMessageStatusRequest,
    ValenceAuthorizationPolicyUpdateRequest, ValenceAuthorizationGrantRevocationRequest,
    ValenceAuthorizationRequestDecisionRequest, ValenceLibraryVersionUpdateRequest,
    ValenceLibraryApprovalRevocationRequest,
};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use std::time::SystemTime;

/// Simple event implementation for memory storage
#[derive(Debug, Clone)]
struct SimpleEvent {
    id: String,
    chain: String,
    block_number: u64,
    block_hash: String,
    tx_hash: String,
    timestamp: SystemTime,
    event_type: String,
    raw_data: Vec<u8>,
}

impl Event for SimpleEvent {
    fn id(&self) -> &str { &self.id }
    fn chain(&self) -> &str { &self.chain }
    fn block_number(&self) -> u64 { self.block_number }
    fn block_hash(&self) -> &str { &self.block_hash }
    fn tx_hash(&self) -> &str { &self.tx_hash }
    fn timestamp(&self) -> SystemTime { self.timestamp }
    fn event_type(&self) -> &str { &self.event_type }
    fn raw_data(&self) -> &[u8] { &self.raw_data }
    fn as_any(&self) -> &dyn std::any::Any { self }
}

/// Simple block information for memory storage
#[derive(Debug, Clone)]
pub struct Block {
    pub chain_id: String,
    pub number: u64,
    pub hash: Option<String>,
    pub parent_hash: Option<String>,
    pub timestamp: Option<std::time::SystemTime>,
    pub status: BlockStatus,
}

/// In-memory storage implementation for testing
pub struct MemoryStorage {
    events: Arc<RwLock<Vec<Box<dyn Event>>>>,
    blocks: Arc<RwLock<HashMap<String, HashMap<u64, Block>>>>,
    _latest_block: Arc<RwLock<u64>>,
    processor_states: Arc<RwLock<HashMap<String, serde_json::Value>>>,
}

impl Default for MemoryStorage {
    fn default() -> Self {
        Self::new()
    }
}

impl MemoryStorage {
    pub fn new() -> Self {
        Self {
            events: Arc::new(RwLock::new(Vec::new())),
            blocks: Arc::new(RwLock::new(HashMap::new())),
            _latest_block: Arc::new(RwLock::new(0)),
            processor_states: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            events: Arc::new(RwLock::new(Vec::with_capacity(capacity))),
            blocks: Arc::new(RwLock::new(HashMap::new())),
            _latest_block: Arc::new(RwLock::new(0)),
            processor_states: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

#[async_trait::async_trait]
impl Storage for MemoryStorage {
    async fn store_event(&self, event: Box<dyn Event>) -> Result<()> {
        let mut events = self.events.write().await;
        events.push(event);
        Ok(())
    }

    async fn get_events(&self, _filters: Vec<EventFilter>) -> Result<Vec<Box<dyn Event>>> {
        let events = self.events.read().await;
        // TODO: Apply filtering based on the filters parameter
        // For now, return all events
        let results: Vec<Box<dyn Event>> = events.iter()
            .map(|e| {
                Box::new(SimpleEvent {
                    id: e.id().to_string(),
                    chain: e.chain().to_string(),
                    block_number: e.block_number(),
                    block_hash: e.block_hash().to_string(),
                    tx_hash: e.tx_hash().to_string(),
                    timestamp: e.timestamp(),
                    event_type: e.event_type().to_string(),
                    raw_data: e.raw_data().to_vec(),
                }) as Box<dyn Event>
            })
            .collect();
        Ok(results)
    }

    async fn get_latest_block(&self, chain: &str) -> Result<u64> {
        let blocks = self.blocks.read().await;
        match blocks.get(chain).and_then(|chain_blocks| chain_blocks.keys().max().copied()) {
            Some(block_num) => Ok(block_num),
            None => Ok(0), // Return 0 if no blocks found
        }
    }

    async fn update_block_status(
        &self,
        chain: &str,
        block_number: u64,
        status: BlockStatus,
    ) -> Result<()> {
        let mut blocks = self.blocks.write().await;
        let chain_blocks = blocks.entry(chain.to_string()).or_insert_with(HashMap::new);
        
        let block = Block {
            chain_id: chain.to_string(),
            number: block_number,
            hash: None,
            parent_hash: None,
            timestamp: None,
            status,
        };
        
        chain_blocks.insert(block_number, block);
        Ok(())
    }

    async fn get_latest_block_with_status(&self, chain: &str, _status: BlockStatus) -> Result<u64> {
        // For memory storage, we don't differentiate by status
        self.get_latest_block(chain).await
    }

    async fn get_events_with_status(&self, _filters: Vec<EventFilter>, _status: BlockStatus) -> Result<Vec<Box<dyn Event>>> {
        // For memory storage, we don't differentiate by status
        // Just return all events that match any of the filters
        let events = self.events.read().await;
        // Clone the events manually since Box<dyn Event> doesn't implement Clone
        let cloned_events: Vec<Box<dyn Event>> = events.iter()
            .map(|e| {
                // This is a workaround - in production you'd implement proper cloning
                Box::new(SimpleEvent {
                    id: e.id().to_string(),
                    chain: e.chain().to_string(),
                    block_number: e.block_number(),
                    block_hash: e.block_hash().to_string(),
                    tx_hash: e.tx_hash().to_string(),
                    timestamp: e.timestamp(),
                    event_type: e.event_type().to_string(),
                    raw_data: e.raw_data().to_vec(),
                }) as Box<dyn Event>
            })
            .collect();
        Ok(cloned_events)
    }

    async fn reorg_chain(&self, chain: &str, from_block: u64) -> Result<()> {
        let mut blocks = self.blocks.write().await;
        let mut events = self.events.write().await;
        
        // Remove blocks from the specified block number onwards
        if let Some(chain_blocks) = blocks.get_mut(chain) {
            let blocks_to_remove: Vec<u64> = chain_blocks.keys()
                .filter(|&&block_num| block_num >= from_block)
                .cloned()
                .collect();
            
            for block_num in blocks_to_remove {
                chain_blocks.remove(&block_num);
            }
        }
        
        // Remove events from the specified block number onwards
        events.retain(|event| {
            event.chain() != chain || event.block_number() < from_block
        });
        
        Ok(())
    }

    // Valence Account methods - return errors for memory storage
    async fn store_valence_account_instantiation(
        &self,
        _account_info: ValenceAccountInfo,
        _initial_libraries: Vec<ValenceAccountLibrary>,
    ) -> Result<()> {
        Err(Error::Generic("Valence storage not implemented in memory backend".to_string()))
    }

    async fn store_valence_library_approval(
        &self,
        _request: ValenceLibraryApprovalRequest,
    ) -> Result<()> {
        Err(Error::Generic("Valence storage not implemented in memory backend".to_string()))
    }

    async fn store_valence_library_removal(
        &self,
        _request: ValenceLibraryRemovalRequest,
    ) -> Result<()> {
        Err(Error::Generic("Valence storage not implemented in memory backend".to_string()))
    }

    async fn store_valence_ownership_update(
        &self,
        _request: ValenceOwnershipUpdateRequest,
    ) -> Result<()> {
        Err(Error::Generic("Valence storage not implemented in memory backend".to_string()))
    }

    async fn store_valence_execution(
        &self,
        _execution_info: ValenceAccountExecution,
    ) -> Result<()> {
        Err(Error::Generic("Valence storage not implemented in memory backend".to_string()))
    }

    async fn get_valence_account_state(&self, _account_id: &str) -> Result<Option<ValenceAccountState>> {
        Ok(None)
    }

    async fn set_valence_account_state(&self, _account_id: &str, _state: &ValenceAccountState) -> Result<()> {
        Err(Error::Generic("Valence storage not implemented in memory backend".to_string()))
    }

    async fn delete_valence_account_state(&self, _account_id: &str) -> Result<()> {
        Err(Error::Generic("Valence storage not implemented in memory backend".to_string()))
    }

    async fn set_historical_valence_account_state(
        &self,
        _account_id: &str,
        _block_number: u64,
        _state: &ValenceAccountState,
    ) -> Result<()> {
        Err(Error::Generic("Valence storage not implemented in memory backend".to_string()))
    }

    async fn get_historical_valence_account_state(
        &self,
        _account_id: &str,
        _block_number: u64,
    ) -> Result<Option<ValenceAccountState>> {
        Ok(None)
    }

    async fn delete_historical_valence_account_state(
        &self,
        _account_id: &str,
        _block_number: u64,
    ) -> Result<()> {
        Err(Error::Generic("Valence storage not implemented in memory backend".to_string()))
    }

    async fn set_latest_historical_valence_block(
        &self,
        _account_id: &str,
        _block_number: u64,
    ) -> Result<()> {
        Err(Error::Generic("Valence storage not implemented in memory backend".to_string()))
    }

    async fn get_latest_historical_valence_block(&self, _account_id: &str) -> Result<Option<u64>> {
        Ok(None)
    }

    async fn delete_latest_historical_valence_block(&self, _account_id: &str) -> Result<()> {
        Err(Error::Generic("Valence storage not implemented in memory backend".to_string()))
    }

    async fn store_valence_processor_instantiation(
        &self,
        _processor_info: ValenceProcessorInfo,
    ) -> Result<()> {
        Err(Error::Generic("Valence storage not implemented in memory backend".to_string()))
    }

    async fn store_valence_processor_config_update(
        &self,
        _request: ValenceProcessorConfigUpdateRequest,
    ) -> Result<()> {
        Err(Error::Generic("Valence storage not implemented in memory backend".to_string()))
    }

    async fn store_valence_processor_message(
        &self,
        _message: ValenceProcessorMessage,
    ) -> Result<()> {
        Err(Error::Generic("Valence storage not implemented in memory backend".to_string()))
    }

    async fn update_valence_processor_message_status(
        &self,
        _request: ValenceProcessorMessageStatusRequest,
    ) -> Result<()> {
        Err(Error::Generic("Valence storage not implemented in memory backend".to_string()))
    }

    async fn get_valence_processor_state(&self, _processor_id: &str) -> Result<Option<ValenceProcessorState>> {
        Ok(None)
    }

    async fn set_valence_processor_state(&self, _processor_id: &str, _state: &ValenceProcessorState) -> Result<()> {
        Err(Error::Generic("Valence storage not implemented in memory backend".to_string()))
    }

    async fn set_historical_valence_processor_state(
        &self,
        _processor_id: &str,
        _block_number: u64,
        _state: &ValenceProcessorState,
    ) -> Result<()> {
        Err(Error::Generic("Valence storage not implemented in memory backend".to_string()))
    }

    async fn get_historical_valence_processor_state(
        &self,
        _processor_id: &str,
        _block_number: u64,
    ) -> Result<Option<ValenceProcessorState>> {
        Ok(None)
    }

    async fn store_valence_authorization_instantiation(
        &self,
        _auth_info: ValenceAuthorizationInfo,
        _initial_policy: Option<ValenceAuthorizationPolicy>,
    ) -> Result<()> {
        Err(Error::Generic("Valence storage not implemented in memory backend".to_string()))
    }

    async fn store_valence_authorization_policy(
        &self,
        _policy: ValenceAuthorizationPolicy,
    ) -> Result<()> {
        Err(Error::Generic("Valence storage not implemented in memory backend".to_string()))
    }

    async fn update_active_authorization_policy(
        &self,
        _request: ValenceAuthorizationPolicyUpdateRequest,
    ) -> Result<()> {
        Err(Error::Generic("Valence storage not implemented in memory backend".to_string()))
    }

    async fn store_valence_authorization_grant(
        &self,
        _grant: ValenceAuthorizationGrant,
    ) -> Result<()> {
        Err(Error::Generic("Valence storage not implemented in memory backend".to_string()))
    }

    async fn revoke_valence_authorization_grant(
        &self,
        _request: ValenceAuthorizationGrantRevocationRequest,
    ) -> Result<()> {
        Err(Error::Generic("Valence storage not implemented in memory backend".to_string()))
    }

    async fn store_valence_authorization_request(
        &self,
        _request: ValenceAuthorizationRequest,
    ) -> Result<()> {
        Err(Error::Generic("Valence storage not implemented in memory backend".to_string()))
    }

    async fn update_valence_authorization_request_decision(
        &self,
        _request: ValenceAuthorizationRequestDecisionRequest,
    ) -> Result<()> {
        Err(Error::Generic("Valence storage not implemented in memory backend".to_string()))
    }

    async fn store_valence_library_instantiation(
        &self,
        _library_info: ValenceLibraryInfo,
        _initial_version: Option<ValenceLibraryVersion>,
    ) -> Result<()> {
        Err(Error::Generic("Valence storage not implemented in memory backend".to_string()))
    }

    async fn store_valence_library_version(
        &self,
        _version: ValenceLibraryVersion,
    ) -> Result<()> {
        Err(Error::Generic("Valence storage not implemented in memory backend".to_string()))
    }

    async fn update_active_library_version(
        &self,
        _request: ValenceLibraryVersionUpdateRequest,
    ) -> Result<()> {
        Err(Error::Generic("Valence storage not implemented in memory backend".to_string()))
    }

    async fn store_valence_library_usage(
        &self,
        _usage: ValenceLibraryUsage,
    ) -> Result<()> {
        Err(Error::Generic("Valence storage not implemented in memory backend".to_string()))
    }

    async fn revoke_valence_library_approval(
        &self,
        _request: ValenceLibraryApprovalRevocationRequest,
    ) -> Result<()> {
        Err(Error::Generic("Valence storage not implemented in memory backend".to_string()))
    }

    async fn get_valence_library_state(&self, _library_id: &str) -> Result<Option<crate::ValenceLibraryState>> {
        Ok(None)
    }

    async fn set_valence_library_state(&self, _library_id: &str, _state: &crate::ValenceLibraryState) -> Result<()> {
        Err(Error::Generic("Valence storage not implemented in memory backend".to_string()))
    }

    async fn get_valence_library_versions(&self, _library_id: &str) -> Result<Vec<ValenceLibraryVersion>> {
        Ok(Vec::new())
    }

    async fn get_valence_library_approvals(&self, _library_id: &str) -> Result<Vec<ValenceLibraryApproval>> {
        Ok(Vec::new())
    }

    async fn get_valence_libraries_for_account(&self, _account_id: &str) -> Result<Vec<ValenceLibraryApproval>> {
        Ok(Vec::new())
    }

    async fn get_valence_library_usage_history(
        &self,
        _library_id: &str,
        _limit: Option<usize>,
        _offset: Option<usize>,
    ) -> Result<Vec<ValenceLibraryUsage>> {
        Ok(Vec::new())
    }

    async fn set_processor_state(&self, _chain: &str, _block_number: u64, _state: &str) -> Result<()> {
        let mut states = self.processor_states.write().await;
        let key = format!("{}:{}", _chain, _block_number);
        states.insert(key, serde_json::Value::String(_state.to_string()));
        Ok(())
    }
    
    async fn get_processor_state(&self, _chain: &str, _block_number: u64) -> Result<Option<String>> {
        let states = self.processor_states.read().await;
        let key = format!("{}:{}", _chain, _block_number);
        Ok(states.get(&key).and_then(|v| v.as_str()).map(|s| s.to_string()))
    }
    
    async fn set_historical_processor_state(&self, _chain: &str, _block_number: u64, _state: &str) -> Result<()> {
        // For memory storage, historical and current state are the same
        self.set_processor_state(_chain, _block_number, _state).await
    }
    
    async fn get_historical_processor_state(&self, _chain: &str, _block_number: u64) -> Result<Option<String>> {
        // For memory storage, historical and current state are the same
        self.get_processor_state(_chain, _block_number).await
    }
}
