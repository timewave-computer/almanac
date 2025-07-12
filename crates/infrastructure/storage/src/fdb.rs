use async_trait::async_trait;
use foundationdb::api::{FdbApiBuilder, NetworkStop};
use foundationdb::{Database, FdbResult, KeySelector, RangeOption, Transaction};
use futures::TryStreamExt;
use indexer_core::{
    Block, BlockStatus, ChainId, Event, EventFilter, EventType, Storage, UnifiedEvent,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};
use uuid::Uuid;


const EVENTS_PREFIX: &[u8] = b"/events/";
const BLOCKS_PREFIX: &[u8] = b"/blocks/";
const STATE_PREFIX: &[u8] = b"/state/";
const PROCESSOR_PREFIX: &[u8] = b"/processors/";

pub struct FdbConfig {
    pub cluster_file: Option<String>,
    pub namespace: String,
}

impl Default for FdbConfig {
    fn default() -> Self {
        Self {
            cluster_file: None,
            namespace: "almanac".to_string(),
        }
    }
}

pub struct FdbStorage {
    db: Database,
    namespace: String,
    _network: Arc<RwLock<Option<NetworkStop>>>,
}

impl FdbStorage {
    pub async fn new(config: FdbConfig) -> Result<Self, Box<dyn std::error::Error>> {
        let network_builder = FdbApiBuilder::default()
            .build()
            .map_err(|e| format!("Failed to initialize FDB API: {}", e))?;

        let network = Arc::new(RwLock::new(Some(unsafe { network_builder.boot() })));

        let db = if let Some(cluster_file) = &config.cluster_file {
            Database::from_cluster_file(cluster_file)
                .await
                .map_err(|e| format!("Failed to connect to FDB cluster: {}", e))?
        } else {
            Database::default()
                .map_err(|e| format!("Failed to connect to default FDB cluster: {}", e))?
        };

        info!("Connected to FoundationDB cluster");

        Ok(Self {
            db,
            namespace: config.namespace,
            _network: network,
        })
    }


    fn namespace_key(&self, prefix: &[u8], key: &[u8]) -> Vec<u8> {
        let mut full_key = Vec::with_capacity(self.namespace.len() + prefix.len() + key.len() + 2);
        full_key.push(b'/');
        full_key.extend_from_slice(self.namespace.as_bytes());
        full_key.extend_from_slice(prefix);
        full_key.extend_from_slice(key);
        full_key
    }

    fn event_key(&self, chain_id: &str, block_number: u64, index: u32) -> Vec<u8> {
        let key = format!("{}/{:016x}/{:08x}", chain_id, block_number, index);
        self.namespace_key(EVENTS_PREFIX, key.as_bytes())
    }

    fn block_key(&self, chain_id: &str, block_number: u64) -> Vec<u8> {
        let key = format!("{}/{:016x}", chain_id, block_number);
        self.namespace_key(BLOCKS_PREFIX, key.as_bytes())
    }

    fn latest_block_key(&self, chain_id: &str) -> Vec<u8> {
        let key = format!("{}/latest", chain_id);
        self.namespace_key(BLOCKS_PREFIX, key.as_bytes())
    }

    fn processor_state_key(&self, processor_id: &str) -> Vec<u8> {
        self.namespace_key(PROCESSOR_PREFIX, processor_id.as_bytes())
    }

    async fn transact<F, R>(&self, f: F) -> FdbResult<R>
    where
        F: Fn(Transaction) -> FdbResult<R>,
    {
        self.db.run(f).await
    }
}

#[async_trait]
impl Storage for FdbStorage {
    async fn store_event(&self, event: Box<dyn Event>) -> Result<(), Box<dyn std::error::Error>> {
        let chain_id = event.chain_id();
        let block_number = event.block_number();
        let index = event.log_index().unwrap_or(0);

        let key = self.event_key(&chain_id, block_number, index);
        let value = serde_json::to_vec(&UnifiedEvent::from(event))?;

        self.transact(|mut trx| {
            trx.set(&key, &value);
            Ok(())
        })
        .await?;

        debug!(
            "Stored event: chain={}, block={}, index={}",
            chain_id, block_number, index
        );

        Ok(())
    }

    async fn get_events(
        &self,
        filters: Vec<EventFilter>,
    ) -> Result<Vec<Box<dyn Event>>, Box<dyn std::error::Error>> {
        let mut events = Vec::new();

        for filter in filters {
            let chain_id = filter.chain_id.as_deref().unwrap_or("");
            let from_block = filter.from_block.unwrap_or(0);
            let to_block = filter.to_block.unwrap_or(u64::MAX);

            let start_key = self.event_key(chain_id, from_block, 0);
            let end_key = self.event_key(chain_id, to_block + 1, 0);

            let range = RangeOption {
                begin: KeySelector::first_greater_or_equal(&start_key),
                end: KeySelector::first_greater_or_equal(&end_key),
                mode: foundationdb::options::StreamingMode::Iterator,
                limit: None,
                reverse: false,
            };

            let results = self
                .transact(|trx| {
                    let mut results = Vec::new();
                    let iter = trx.get_range(&range, 1, false);
                    
                    futures::executor::block_on(async {
                        let mut stream = iter;
                        while let Ok(Some(kv)) = stream.try_next().await {
                            results.push(kv);
                        }
                    });
                    
                    Ok(results)
                })
                .await?;

            for kv in results {
                let event: UnifiedEvent = serde_json::from_slice(kv.value())?;
                
                if filter.matches(&event) {
                    events.push(Box::new(event) as Box<dyn Event>);
                }
            }
        }

        Ok(events)
    }

    async fn get_latest_block(
        &self,
        chain: &str,
    ) -> Result<Option<u64>, Box<dyn std::error::Error>> {
        let key = self.latest_block_key(chain);

        let result = self
            .transact(|trx| Ok(trx.get(&key, false)))
            .await?
            .await?;

        match result {
            Some(value) => {
                let block_number = u64::from_be_bytes(
                    value
                        .as_ref()
                        .try_into()
                        .map_err(|_| "Invalid block number format")?,
                );
                Ok(Some(block_number))
            }
            None => Ok(None),
        }
    }

    async fn update_block_status(
        &self,
        chain: &str,
        block_number: u64,
        status: BlockStatus,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let block_key = self.block_key(chain, block_number);
        let latest_key = self.latest_block_key(chain);

        let block_data = Block {
            chain_id: chain.to_string(),
            number: block_number,
            hash: None,
            parent_hash: None,
            timestamp: None,
            status,
        };

        let block_value = serde_json::to_vec(&block_data)?;
        let latest_value = block_number.to_be_bytes();

        self.transact(|mut trx| {
            trx.set(&block_key, &block_value);
            
            let current_latest = trx.get(&latest_key, false);
            if let Ok(Some(current)) = futures::executor::block_on(current_latest) {
                let current_number = u64::from_be_bytes(
                    current.as_ref().try_into().unwrap_or([0; 8])
                );
                if block_number > current_number {
                    trx.set(&latest_key, &latest_value);
                }
            } else {
                trx.set(&latest_key, &latest_value);
            }
            
            Ok(())
        })
        .await?;

        info!(
            "Updated block status: chain={}, block={}, status={:?}",
            chain, block_number, status
        );

        Ok(())
    }

    async fn get_block_status(
        &self,
        chain: &str,
        block_number: u64,
    ) -> Result<Option<BlockStatus>, Box<dyn std::error::Error>> {
        let key = self.block_key(chain, block_number);

        let result = self
            .transact(|trx| Ok(trx.get(&key, false)))
            .await?
            .await?;

        match result {
            Some(value) => {
                let block: Block = serde_json::from_slice(value.as_ref())?;
                Ok(Some(block.status))
            }
            None => Ok(None),
        }
    }

    async fn set_processor_state(
        &self,
        processor_id: &str,
        state: serde_json::Value,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let key = self.processor_state_key(processor_id);
        let value = serde_json::to_vec(&state)?;

        self.transact(|mut trx| {
            trx.set(&key, &value);
            Ok(())
        })
        .await?;

        debug!("Set processor state: id={}", processor_id);

        Ok(())
    }

    async fn get_processor_state(
        &self,
        processor_id: &str,
    ) -> Result<Option<serde_json::Value>, Box<dyn std::error::Error>> {
        let key = self.processor_state_key(processor_id);

        let result = self
            .transact(|trx| Ok(trx.get(&key, false)))
            .await?
            .await?;

        match result {
            Some(value) => {
                let state = serde_json::from_slice(value.as_ref())?;
                Ok(Some(state))
            }
            None => Ok(None),
        }
    }

    async fn get_event_count(&self) -> Result<u64, Box<dyn std::error::Error>> {
        let start_key = self.namespace_key(EVENTS_PREFIX, b"");
        let end_key = self.namespace_key(EVENTS_PREFIX, b"\xff");

        let range = RangeOption {
            begin: KeySelector::first_greater_or_equal(&start_key),
            end: KeySelector::first_greater_or_equal(&end_key),
            mode: foundationdb::options::StreamingMode::Small,
            limit: None,
            reverse: false,
        };

        let count = self
            .transact(|trx| {
                let mut count = 0u64;
                let iter = trx.get_range(&range, 1, false);
                
                futures::executor::block_on(async {
                    let mut stream = iter;
                    while let Ok(Some(_)) = stream.try_next().await {
                        count += 1;
                    }
                });
                
                Ok(count)
            })
            .await?;

        Ok(count)
    }

    async fn get_supported_chains(&self) -> Result<Vec<ChainId>, Box<dyn std::error::Error>> {
        let start_key = self.namespace_key(BLOCKS_PREFIX, b"");
        let end_key = self.namespace_key(BLOCKS_PREFIX, b"\xff");

        let range = RangeOption {
            begin: KeySelector::first_greater_or_equal(&start_key),
            end: KeySelector::first_greater_or_equal(&end_key),
            mode: foundationdb::options::StreamingMode::Small,
            limit: None,
            reverse: false,
        };

        let chains = self
            .transact(|trx| {
                let mut chains = std::collections::HashSet::new();
                let iter = trx.get_range(&range, 1, false);
                
                futures::executor::block_on(async {
                    let mut stream = iter;
                    while let Ok(Some(kv)) = stream.try_next().await {
                        let key = std::str::from_utf8(kv.key()).unwrap_or("");
                        if let Some(chain_part) = key.split('/').nth(3) {
                            chains.insert(chain_part.to_string());
                        }
                    }
                });
                
                Ok(chains.into_iter().collect::<Vec<_>>())
            })
            .await?;

        Ok(chains)
    }

    async fn clear(&self) -> Result<(), Box<dyn std::error::Error>> {
        let start_key = self.namespace_key(b"/", b"");
        let end_key = self.namespace_key(b"/", b"\xff");

        self.transact(|mut trx| {
            trx.clear_range(&start_key, &end_key);
            Ok(())
        })
        .await?;

        info!("Cleared all data in namespace: {}", self.namespace);

        Ok(())
    }

    // Valence-specific methods - minimal implementation for distributed storage focus
    async fn store_valence_account_instantiation(
        &self,
        _account_info: indexer_core::ValenceAccountInfo,
    ) -> Result<(), Box<dyn std::error::Error>> {
        Err("Valence account storage not implemented in FDB backend".into())
    }

    async fn store_valence_account_config_update(
        &self,
        _config_update: indexer_core::ValenceAccountConfigUpdate,
    ) -> Result<(), Box<dyn std::error::Error>> {
        Err("Valence account config storage not implemented in FDB backend".into())
    }

    async fn store_valence_account_library_approval(
        &self,
        _approval: indexer_core::ValenceAccountLibraryApproval,
    ) -> Result<(), Box<dyn std::error::Error>> {
        Err("Valence library approval storage not implemented in FDB backend".into())
    }

    async fn update_valence_account_library_approval_status(
        &self,
        _request: crate::ValenceLibraryApprovalRequest,
    ) -> Result<(), Box<dyn std::error::Error>> {
        Err("Valence library approval update not implemented in FDB backend".into())
    }

    async fn store_valence_account_action(
        &self,
        _action: indexer_core::ValenceAccountAction,
    ) -> Result<(), Box<dyn std::error::Error>> {
        Err("Valence account action storage not implemented in FDB backend".into())
    }

    async fn update_valence_account_action_status(
        &self,
        _request: crate::ValenceAccountActionStatusRequest,
    ) -> Result<(), Box<dyn std::error::Error>> {
        Err("Valence account action status update not implemented in FDB backend".into())
    }

    async fn get_valence_account_info(&self, _account_id: &str) -> Result<Option<indexer_core::ValenceAccountInfo>, Box<dyn std::error::Error>> {
        Ok(None)
    }

    async fn get_valence_account_actions(
        &self,
        _account_id: &str,
        _limit: Option<usize>,
        _offset: Option<usize>,
    ) -> Result<Vec<indexer_core::ValenceAccountAction>, Box<dyn std::error::Error>> {
        Ok(Vec::new())
    }

    async fn store_valence_processor_instantiation(
        &self,
        _processor_info: indexer_core::ValenceProcessorInfo,
        _initial_config: Option<indexer_core::ValenceProcessorConfig>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        Err("Valence processor storage not implemented in FDB backend".into())
    }

    async fn store_valence_processor_config_update(
        &self,
        _request: crate::ValenceProcessorConfigUpdateRequest,
    ) -> Result<(), Box<dyn std::error::Error>> {
        Err("Valence processor config update not implemented in FDB backend".into())
    }

    async fn store_valence_processor_message(
        &self,
        _message: crate::ValenceProcessorMessage,
    ) -> Result<(), Box<dyn std::error::Error>> {
        Err("Valence processor message storage not implemented in FDB backend".into())
    }

    async fn update_valence_processor_message_status(
        &self,
        _request: crate::ValenceProcessorMessageStatusRequest,
    ) -> Result<(), Box<dyn std::error::Error>> {
        Err("Valence processor message status update not implemented in FDB backend".into())
    }

    async fn get_valence_processor_state(&self, _processor_id: &str) -> Result<Option<crate::ValenceProcessorState>, Box<dyn std::error::Error>> {
        Ok(None)
    }

    async fn set_valence_processor_state(&self, _processor_id: &str, _state: &crate::ValenceProcessorState) -> Result<(), Box<dyn std::error::Error>> {
        Err("Valence processor state storage not implemented in FDB backend".into())
    }

    async fn set_historical_valence_processor_state(
        &self,
        _processor_id: &str,
        _block_number: u64,
        _state: &crate::ValenceProcessorState,
    ) -> Result<(), Box<dyn std::error::Error>> {
        Err("Valence historical processor state storage not implemented in FDB backend".into())
    }

    async fn get_historical_valence_processor_state(
        &self,
        _processor_id: &str,
        _block_number: u64,
    ) -> Result<Option<crate::ValenceProcessorState>, Box<dyn std::error::Error>> {
        Ok(None)
    }

    async fn store_valence_authorization_instantiation(
        &self,
        _auth_info: indexer_core::ValenceAuthorizationInfo,
        _initial_policy: Option<indexer_core::ValenceAuthorizationPolicy>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        Err("Valence authorization storage not implemented in FDB backend".into())
    }

    async fn store_valence_authorization_policy(
        &self,
        _policy: indexer_core::ValenceAuthorizationPolicy,
    ) -> Result<(), Box<dyn std::error::Error>> {
        Err("Valence authorization policy storage not implemented in FDB backend".into())
    }

    async fn update_active_authorization_policy(
        &self,
        _request: crate::ValenceAuthorizationPolicyUpdateRequest,
    ) -> Result<(), Box<dyn std::error::Error>> {
        Err("Valence authorization policy update not implemented in FDB backend".into())
    }

    async fn store_valence_authorization_grant(
        &self,
        _grant: indexer_core::ValenceAuthorizationGrant,
    ) -> Result<(), Box<dyn std::error::Error>> {
        Err("Valence authorization grant storage not implemented in FDB backend".into())
    }

    async fn revoke_valence_authorization_grant(
        &self,
        _request: crate::ValenceAuthorizationGrantRevocationRequest,
    ) -> Result<(), Box<dyn std::error::Error>> {
        Err("Valence authorization grant revocation not implemented in FDB backend".into())
    }

    async fn store_valence_authorization_request(
        &self,
        _request: indexer_core::ValenceAuthorizationRequest,
    ) -> Result<(), Box<dyn std::error::Error>> {
        Err("Valence authorization request storage not implemented in FDB backend".into())
    }

    async fn update_valence_authorization_request_decision(
        &self,
        _request: crate::ValenceAuthorizationRequestDecisionRequest,
    ) -> Result<(), Box<dyn std::error::Error>> {
        Err("Valence authorization request decision update not implemented in FDB backend".into())
    }

    async fn store_valence_library_instantiation(
        &self,
        _library_info: indexer_core::ValenceLibraryInfo,
        _initial_version: Option<indexer_core::ValenceLibraryVersion>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        Err("Valence library storage not implemented in FDB backend".into())
    }

    async fn store_valence_library_version(
        &self,
        _version: indexer_core::ValenceLibraryVersion,
    ) -> Result<(), Box<dyn std::error::Error>> {
        Err("Valence library version storage not implemented in FDB backend".into())
    }

    async fn update_valence_library_current_version(
        &self,
        _request: crate::ValenceLibraryVersionUpdateRequest,
    ) -> Result<(), Box<dyn std::error::Error>> {
        Err("Valence library version update not implemented in FDB backend".into())
    }

    async fn store_valence_library_approval(
        &self,
        _approval: indexer_core::ValenceLibraryApproval,
    ) -> Result<(), Box<dyn std::error::Error>> {
        Err("Valence library approval storage not implemented in FDB backend".into())
    }

    async fn revoke_valence_library_approval(
        &self,
        _request: crate::ValenceLibraryApprovalRevocationRequest,
    ) -> Result<(), Box<dyn std::error::Error>> {
        Err("Valence library approval revocation not implemented in FDB backend".into())
    }

    async fn store_valence_library_usage(
        &self,
        _usage: indexer_core::ValenceLibraryUsage,
    ) -> Result<(), Box<dyn std::error::Error>> {
        Err("Valence library usage storage not implemented in FDB backend".into())
    }

    async fn get_valence_account_policies(&self, _account_id: &str) -> Result<Vec<indexer_core::ValenceAuthorizationPolicy>, Box<dyn std::error::Error>> {
        Ok(Vec::new())
    }

    async fn get_valence_authorization_grants(&self, _subject_account: &str) -> Result<Vec<indexer_core::ValenceAuthorizationGrant>, Box<dyn std::error::Error>> {
        Ok(Vec::new())
    }

    async fn get_valence_libraries_for_account(&self, _account_id: &str) -> Result<Vec<crate::ValenceLibraryApproval>, Box<dyn std::error::Error>> {
        Ok(Vec::new())
    }

    async fn get_valence_library_usage_history(
        &self,
        _library_id: &str,
        _limit: Option<usize>,
        _offset: Option<usize>,
    ) -> Result<Vec<indexer_core::ValenceLibraryUsage>, Box<dyn std::error::Error>> {
        Ok(Vec::new())
    }

    async fn set_processor_state(&self, chain: &str, block_number: u64, state: &str) -> Result<(), Box<dyn std::error::Error>> {
        let key = format!("{}/state/{:016x}", chain, block_number);
        let full_key = self.namespace_key(STATE_PREFIX, key.as_bytes());
        
        self.transact(|mut trx| {
            trx.set(&full_key, state.as_bytes());
            Ok(())
        })
        .await?;
        
        Ok(())
    }
    
    async fn get_processor_state(&self, chain: &str, block_number: u64) -> Result<Option<String>, Box<dyn std::error::Error>> {
        let key = format!("{}/state/{:016x}", chain, block_number);
        let full_key = self.namespace_key(STATE_PREFIX, key.as_bytes());
        
        let result = self
            .transact(|trx| Ok(trx.get(&full_key, false)))
            .await?
            .await?;
            
        match result {
            Some(value) => Ok(Some(String::from_utf8(value.to_vec())?)),
            None => Ok(None),
        }
    }
    
    async fn set_historical_processor_state(&self, chain: &str, block_number: u64, state: &str) -> Result<(), Box<dyn std::error::Error>> {
        let key = format!("{}/historical/{:016x}", chain, block_number);
        let full_key = self.namespace_key(STATE_PREFIX, key.as_bytes());
        
        self.transact(|mut trx| {
            trx.set(&full_key, state.as_bytes());
            Ok(())
        })
        .await?;
        
        Ok(())
    }
    
    async fn get_historical_processor_state(&self, chain: &str, block_number: u64) -> Result<Option<String>, Box<dyn std::error::Error>> {
        let key = format!("{}/historical/{:016x}", chain, block_number);
        let full_key = self.namespace_key(STATE_PREFIX, key.as_bytes());
        
        let result = self
            .transact(|trx| Ok(trx.get(&full_key, false)))
            .await?
            .await?;
            
        match result {
            Some(value) => Ok(Some(String::from_utf8(value.to_vec())?)),
            None => Ok(None),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use indexer_core::EventType;

    #[derive(Debug, Clone, Serialize, Deserialize)]
    struct TestEvent {
        chain_id: String,
        block_number: u64,
        log_index: u32,
        event_type: EventType,
    }

    impl Event for TestEvent {
        fn chain_id(&self) -> String {
            self.chain_id.clone()
        }

        fn block_number(&self) -> u64 {
            self.block_number
        }

        fn log_index(&self) -> Option<u32> {
            Some(self.log_index)
        }

        fn transaction_hash(&self) -> Option<String> {
            None
        }

        fn event_type(&self) -> EventType {
            self.event_type.clone()
        }

        fn as_any(&self) -> &dyn std::any::Any {
            self
        }
    }

    #[tokio::test]
    async fn test_fdb_storage_basic_operations() {
        let config = FdbConfig::default();
        let storage = match FdbStorage::new(config).await {
            Ok(s) => s,
            Err(_) => {
                eprintln!("FoundationDB not available, skipping test");
                return;
            }
        };

        storage.clear().await.unwrap();

        let event = TestEvent {
            chain_id: "ethereum".to_string(),
            block_number: 100,
            log_index: 0,
            event_type: EventType::Generic,
        };

        storage
            .store_event(Box::new(event.clone()))
            .await
            .unwrap();

        let filter = EventFilter {
            chain_id: Some("ethereum".to_string()),
            from_block: Some(100),
            to_block: Some(100),
            ..Default::default()
        };

        let events = storage.get_events(vec![filter]).await.unwrap();
        assert_eq!(events.len(), 1);

        storage
            .update_block_status("ethereum", 100, BlockStatus::Finalized)
            .await
            .unwrap();

        let status = storage
            .get_block_status("ethereum", 100)
            .await
            .unwrap();
        assert_eq!(status, Some(BlockStatus::Finalized));

        let latest = storage.get_latest_block("ethereum").await.unwrap();
        assert_eq!(latest, Some(100));
    }
}