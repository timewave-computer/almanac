//! Mock implementations for testing

use almanac_chains::adapters::types::ProofType;
use almanac_chains::{
    BlockMetadata, ChainDataIR, ConsensusMetadata, FinalityStatus, StateChange, StorageProof,
};
use almanac_chains::{ChainAdapter, ChainAdapterError};
use almanac_core::Result as CoreResult;
use almanac_core::{
    BlockStatus, Error, Event, EventFilter, StorageReader, StorageWriter, UnifiedEvent,
};
use async_trait::async_trait;
use futures::Stream;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Mock chain adapter for testing
#[derive(Debug, Clone)]
pub struct MockChainAdapter {
    chain_id: String,
    latest_block: u64,
    blocks: Arc<Mutex<HashMap<u64, ChainDataIR>>>,
    confirmations: u32,
    should_fail: bool,
}

impl MockChainAdapter {
    /// Create a new mock chain adapter
    pub fn new() -> Self {
        Self {
            chain_id: "mock-chain".to_string(),
            latest_block: 1000,
            blocks: Arc::new(Mutex::new(HashMap::new())),
            confirmations: 6,
            should_fail: false,
        }
    }

    /// Set the chain ID
    pub fn with_chain_id(mut self, chain_id: &str) -> Self {
        self.chain_id = chain_id.to_string();
        self
    }

    /// Set the latest block number
    pub fn with_latest_block(mut self, block_number: u64) -> Self {
        self.latest_block = block_number;
        self
    }

    /// Add block data
    pub fn with_block_data(self, block_number: u64, data: ChainDataIR) -> Self {
        self.blocks.lock().unwrap().insert(block_number, data);
        self
    }

    /// Set finality confirmations
    pub fn with_finality_confirmations(mut self, confirmations: u32) -> Self {
        self.confirmations = confirmations;
        self
    }

    /// Make the adapter fail on operations
    pub fn with_failure(mut self, should_fail: bool) -> Self {
        self.should_fail = should_fail;
        self
    }
}

impl Default for MockChainAdapter {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ChainAdapter for MockChainAdapter {
    fn chain_id(&self) -> &str {
        &self.chain_id
    }

    async fn fetch_block(
        &self,
        block_number: u64,
    ) -> std::result::Result<ChainDataIR, ChainAdapterError> {
        if self.should_fail {
            return Err(ChainAdapterError::Network("Mock failure".to_string()));
        }

        if let Some(block_data) = self.blocks.lock().unwrap().get(&block_number) {
            Ok(block_data.clone())
        } else {
            // Generate default block data
            let block_metadata = BlockMetadata {
                block_height: block_number,
                block_hash: format!("mock_block_{}", block_number),
                parent_hash: format!("mock_block_{}", block_number.saturating_sub(1)),
                timestamp: 1640995200 + block_number * 12,
                state_root: format!("mock_state_root_{}", block_number),
                receipts_root: Some(format!("mock_receipts_root_{}", block_number)),
                transactions_root: Some(format!("mock_transactions_root_{}", block_number)),
            };

            let consensus_metadata = ConsensusMetadata {
                validator_set_hash: None,
                proposer: Some(format!("mock_proposer_{}", block_number)),
                round: None,
                signatures: None,
                extra: HashMap::new(),
            };

            Ok(ChainDataIR {
                block_metadata,
                state_changes: vec![],
                consensus_metadata,
                finality_status: FinalityStatus::Confirmed,
            })
        }
    }

    async fn get_latest_block(&self) -> std::result::Result<u64, ChainAdapterError> {
        if self.should_fail {
            return Err(ChainAdapterError::Network("Mock failure".to_string()));
        }
        Ok(self.latest_block)
    }

    async fn is_block_finalized(
        &self,
        block_number: u64,
    ) -> std::result::Result<bool, ChainAdapterError> {
        if self.should_fail {
            return Err(ChainAdapterError::Network("Mock failure".to_string()));
        }
        let confirmations = self.latest_block.saturating_sub(block_number);
        Ok(confirmations >= self.confirmations as u64)
    }

    async fn generate_storage_proofs(
        &self,
        _contract_address: &str,
        storage_keys: &[[u8; 32]],
        _block_number: u64,
    ) -> Result<Vec<StorageProof>, ChainAdapterError> {
        if self.should_fail {
            return Err(ChainAdapterError::Network("Mock failure".to_string()));
        }

        let proofs = storage_keys
            .iter()
            .map(|key| StorageProof {
                key: *key,
                value: [42u8; 32],                               // Mock value
                proof_nodes: vec![vec![1, 2, 3], vec![4, 5, 6]], // Mock proof nodes
                proof_type: ProofType::MerklePatricia,
            })
            .collect();

        Ok(proofs)
    }

    async fn check_block_finality(
        &self,
        block_number: u64,
        current_height: u64,
    ) -> std::result::Result<FinalityStatus, ChainAdapterError> {
        if self.should_fail {
            return Err(ChainAdapterError::Network("Mock failure".to_string()));
        }

        let confirmations = current_height.saturating_sub(block_number);
        Ok(match confirmations {
            0 => FinalityStatus::Pending,
            1..=5 => FinalityStatus::Confirmed,
            6..=31 => FinalityStatus::Safe,
            _ => FinalityStatus::Finalized,
        })
    }

    async fn get_block_metadata(
        &self,
        block_number: u64,
    ) -> std::result::Result<BlockMetadata, ChainAdapterError> {
        if self.should_fail {
            return Err(ChainAdapterError::Network("Mock failure".to_string()));
        }

        Ok(BlockMetadata {
            block_height: block_number,
            block_hash: format!("mock_block_{}", block_number),
            parent_hash: format!("mock_block_{}", block_number.saturating_sub(1)),
            timestamp: 1640995200 + block_number * 12,
            state_root: format!("mock_state_root_{}", block_number),
            receipts_root: Some(format!("mock_receipts_root_{}", block_number)),
            transactions_root: Some(format!("mock_transactions_root_{}", block_number)),
        })
    }

    async fn extract_state_changes(
        &self,
        _block_number: u64,
    ) -> std::result::Result<Vec<StateChange>, ChainAdapterError> {
        if self.should_fail {
            return Err(ChainAdapterError::Network("Mock failure".to_string()));
        }
        Ok(vec![])
    }

    async fn subscribe_blocks(
        &self,
    ) -> std::result::Result<Box<dyn Stream<Item = u64> + Send>, ChainAdapterError> {
        Err(ChainAdapterError::Unsupported(
            "Mock adapter doesn't support subscriptions".to_string(),
        ))
    }
}

/// Mock storage implementation for testing
#[derive(Debug, Clone)]
pub struct MockStorage {
    events: Arc<Mutex<Vec<UnifiedEvent>>>,
    latest_blocks: Arc<Mutex<HashMap<String, u64>>>,
    block_statuses: Arc<Mutex<HashMap<(String, u64), BlockStatus>>>,
    should_fail: bool,
}

impl MockStorage {
    /// Create a new mock storage
    pub fn new() -> Self {
        Self {
            events: Arc::new(Mutex::new(Vec::new())),
            latest_blocks: Arc::new(Mutex::new(HashMap::new())),
            block_statuses: Arc::new(Mutex::new(HashMap::new())),
            should_fail: false,
        }
    }

    /// Add events to the mock storage
    pub fn with_events(self, events: Vec<UnifiedEvent>) -> Self {
        *self.events.lock().unwrap() = events;
        self
    }

    /// Set latest block for a chain
    pub fn with_latest_block(self, chain_id: &str, block_number: u64) -> Self {
        self.latest_blocks
            .lock()
            .unwrap()
            .insert(chain_id.to_string(), block_number);
        self
    }

    /// Set block status
    pub fn with_block_status(self, chain_id: &str, block_number: u64, status: BlockStatus) -> Self {
        self.block_statuses
            .lock()
            .unwrap()
            .insert((chain_id.to_string(), block_number), status);
        self
    }

    /// Make the storage fail on operations
    pub fn with_failure(mut self, should_fail: bool) -> Self {
        self.should_fail = should_fail;
        self
    }
}

impl Default for MockStorage {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl StorageWriter for MockStorage {
    async fn store_event(&self, event: Box<dyn Event>) -> CoreResult<()> {
        if self.should_fail {
            return Err(Error::Storage("Mock storage failure".to_string()));
        }

        // Convert Event trait object to UnifiedEvent
        let unified_event = UnifiedEvent {
            id: event.id().to_string(),
            chain: event.chain().to_string(),
            block_number: event.block_number(),
            block_hash: event.block_hash().to_string(),
            tx_hash: event.tx_hash().to_string(),
            timestamp: event.timestamp(),
            event_type: event.event_type().to_string(),
            contract_address: if event.contract_address().is_empty() {
                None
            } else {
                Some(event.contract_address().to_string())
            },
            raw_data: event.raw_data().to_vec(),
            attributes: event.attributes(),
            source: almanac_core::EventSource::Chain,
        };

        self.events.lock().unwrap().push(unified_event);
        Ok(())
    }

    async fn store_events_batch(&self, events: Vec<Box<dyn Event>>) -> CoreResult<()> {
        if self.should_fail {
            return Err(Error::Storage("Mock storage failure".to_string()));
        }

        for event in events {
            self.store_event(event).await?;
        }
        Ok(())
    }

    async fn update_block_status(
        &self,
        chain: &str,
        block_number: u64,
        status: BlockStatus,
    ) -> CoreResult<()> {
        if self.should_fail {
            return Err(Error::Storage("Mock storage failure".to_string()));
        }
        self.block_statuses
            .lock()
            .unwrap()
            .insert((chain.to_string(), block_number), status);
        Ok(())
    }

    async fn reorg_chain(&self, _chain: &str, _from_block: u64) -> CoreResult<()> {
        if self.should_fail {
            return Err(Error::Storage("Mock storage failure".to_string()));
        }
        Ok(())
    }

    async fn health_check(&self) -> CoreResult<()> {
        if self.should_fail {
            return Err(Error::Storage("Mock storage failure".to_string()));
        }
        Ok(())
    }
}

#[async_trait]
impl StorageReader for MockStorage {
    async fn get_events(&self, filters: Vec<EventFilter>) -> CoreResult<Vec<Box<dyn Event>>> {
        if self.should_fail {
            return Err(Error::Storage("Mock storage failure".to_string()));
        }

        let events = self.events.lock().unwrap();
        let mut filtered_events = events.clone();

        // Apply filters
        for filter in filters {
            if let Some(chain) = &filter.chain {
                filtered_events.retain(|e| &e.chain == chain);
            }

            if let Some(addresses) = &filter.addresses {
                filtered_events.retain(|e| {
                    e.contract_address
                        .as_ref()
                        .map(|addr| addresses.iter().any(|a| a == addr))
                        .unwrap_or(false)
                });
            }

            if let Some(event_types) = &filter.event_types {
                filtered_events.retain(|e| event_types.iter().any(|t| t == &e.event_type));
            }

            if let Some((from_block, to_block)) = filter.block_range {
                filtered_events
                    .retain(|e| e.block_number >= from_block && e.block_number <= to_block);
            }

            if let Some(tx_hashes) = &filter.tx_hashes {
                filtered_events.retain(|e| tx_hashes.iter().any(|h| h == &e.tx_hash));
            }

            // Apply pagination
            if let Some(offset) = filter.offset {
                if offset < filtered_events.len() {
                    filtered_events = filtered_events[offset..].to_vec();
                } else {
                    filtered_events.clear();
                }
            }

            if let Some(limit) = filter.limit {
                filtered_events.truncate(limit);
            }
        }

        // Convert UnifiedEvents to trait objects
        let result: Vec<Box<dyn Event>> = filtered_events
            .into_iter()
            .map(|e| Box::new(e) as Box<dyn Event>)
            .collect();

        Ok(result)
    }

    async fn get_latest_block(&self, chain: &str) -> CoreResult<u64> {
        if self.should_fail {
            return Err(Error::Storage("Mock storage failure".to_string()));
        }
        Ok(self
            .latest_blocks
            .lock()
            .unwrap()
            .get(chain)
            .copied()
            .unwrap_or(0))
    }

    async fn get_latest_block_with_status(
        &self,
        chain: &str,
        _status: BlockStatus,
    ) -> CoreResult<u64> {
        if self.should_fail {
            return Err(Error::Storage("Mock storage failure".to_string()));
        }
        // Simplified implementation - just return latest block
        Ok(self
            .latest_blocks
            .lock()
            .unwrap()
            .get(chain)
            .copied()
            .unwrap_or(0))
    }

    async fn get_events_with_status(
        &self,
        filters: Vec<EventFilter>,
        _status: BlockStatus,
    ) -> CoreResult<Vec<Box<dyn Event>>> {
        if self.should_fail {
            return Err(Error::Storage("Mock storage failure".to_string()));
        }
        // Simplified implementation - just return filtered events
        self.get_events(filters).await
    }

    async fn get_latest_block_before(&self, chain: &str, before_block: u64) -> CoreResult<u64> {
        if self.should_fail {
            return Err(Error::Storage("Mock storage failure".to_string()));
        }
        let latest = self
            .latest_blocks
            .lock()
            .unwrap()
            .get(chain)
            .copied()
            .unwrap_or(0);
        Ok(latest.min(before_block.saturating_sub(1)))
    }

    async fn health_check(&self) -> CoreResult<()> {
        if self.should_fail {
            return Err(Error::Storage("Mock storage failure".to_string()));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixtures;
    use almanac_core::StorageReader;

    #[tokio::test]
    async fn test_mock_chain_adapter() {
        let adapter = MockChainAdapter::new()
            .with_chain_id("test-chain")
            .with_latest_block(1000);

        assert_eq!(adapter.chain_id(), "test-chain");
        assert_eq!(adapter.get_latest_block().await.unwrap(), 1000);
        assert!(!adapter.is_block_finalized(995).await.unwrap());
        assert!(adapter.is_block_finalized(990).await.unwrap());
    }

    #[tokio::test]
    async fn test_mock_chain_adapter_failure() {
        let adapter = MockChainAdapter::new().with_failure(true);
        assert!(adapter.get_latest_block().await.is_err());
    }

    #[tokio::test]
    async fn test_mock_storage() {
        let events = fixtures::create_test_events(5);
        let storage = MockStorage::new().with_events(events.clone());

        let filter = EventFilter::new().with_chain("test-chain");
        let retrieved_events = storage.get_events(vec![filter]).await.unwrap();
        assert_eq!(retrieved_events.len(), 5);
    }

    #[tokio::test]
    async fn test_mock_storage_failure() {
        let storage = MockStorage::new().with_failure(true);
        let filter = EventFilter::new();
        assert!(storage.get_events(vec![filter]).await.is_err());
    }
}
