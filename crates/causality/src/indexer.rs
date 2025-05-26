/// Main causality indexer that integrates SMT and causality tracking
use std::collections::HashMap;
use std::sync::Arc;
use std::time::SystemTime;

use tokio::sync::RwLock;
use tracing::{info, error};

use indexer_core::event::Event;
use indexer_core::types::ChainId;

use crate::error::{CausalityError, Result};
use crate::types::{CausalityEvent, CausalityIndex, SmtRoot, SmtHasher};
use crate::smt::{SparseMerkleTree, SmtBackend, Blake3SmtHasher};
use crate::storage::{CausalityStorage, CausalityStorageBackend};
use crate::causality::{CausalityTracker, CausalityGraph};

/// Configuration for the causality indexer
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CausalityIndexerConfig {
    /// Enable SMT indexing
    pub enable_smt: bool,
    /// Enable causality tracking
    pub enable_causality_tracking: bool,
    /// Maximum SMT depth
    pub max_smt_depth: usize,
    /// Batch size for processing events
    pub batch_size: usize,
    /// Enable cross-chain causality tracking
    pub enable_cross_chain: bool,
    /// Chains to index
    pub indexed_chains: Vec<ChainId>,
    /// SMT hasher type
    pub hasher_type: HasherType,
}

/// Types of hashers available
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum HasherType {
    /// Blake3 hasher
    Blake3,
    /// SHA256 hasher
    Sha256,
}

impl Default for CausalityIndexerConfig {
    fn default() -> Self {
        Self {
            enable_smt: true,
            enable_causality_tracking: true,
            max_smt_depth: crate::DEFAULT_SMT_DEPTH,
            batch_size: 100,
            enable_cross_chain: true,
            indexed_chains: Vec::new(),
            hasher_type: HasherType::Sha256,
        }
    }
}

/// Main causality indexer
pub struct CausalityIndexer<B: SmtBackend> {
    /// Configuration
    config: CausalityIndexerConfig,
    /// SMT for indexing events and resources
    smt: SparseMerkleTree<B>,
    /// Causality tracker
    causality_tracker: Arc<RwLock<CausalityTracker>>,
    /// Storage backend
    storage: CausalityStorage<B>,
    /// Current SMT root
    current_root: Arc<RwLock<SmtRoot>>,
    /// Causality index
    causality_index: Arc<RwLock<CausalityIndex>>,
    /// Per-chain event counters
    chain_counters: Arc<RwLock<HashMap<ChainId, u64>>>,
}

impl<B: SmtBackend> CausalityIndexer<B> {
    /// Create a new causality indexer
    pub fn new(
        config: CausalityIndexerConfig,
        smt_backend: B,
        causality_backend: Box<dyn CausalityStorageBackend>,
    ) -> Result<Self> {
        let hasher: Box<dyn SmtHasher> = match config.hasher_type {
            HasherType::Blake3 => Box::new(Blake3SmtHasher),
            HasherType::Sha256 => Box::new(crate::smt::Sha256SmtHasher),
        };

        let smt = SparseMerkleTree::new(smt_backend.clone(), hasher);
        let causality_tracker = Arc::new(RwLock::new(CausalityTracker::new(
            match config.hasher_type {
                HasherType::Blake3 => Box::new(Blake3SmtHasher),
                HasherType::Sha256 => Box::new(crate::smt::Sha256SmtHasher),
            }
        )));
        let storage = CausalityStorage::new(causality_backend, smt_backend);

        Ok(Self {
            config,
            smt,
            causality_tracker,
            storage,
            current_root: Arc::new(RwLock::new(SparseMerkleTree::<B>::empty_root())),
            causality_index: Arc::new(RwLock::new(CausalityIndex::new())),
            chain_counters: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    /// Initialize the indexer
    pub async fn initialize(&self) -> Result<()> {
        info!("Initializing causality indexer");

        // Load existing index if available
        if let Some(index) = self.storage.causality_backend().get_index().await? {
            let mut current_index = self.causality_index.write().await;
            *current_index = index.clone();
            
            let mut current_root = self.current_root.write().await;
            *current_root = index.root;
            
            info!("Loaded existing causality index with {} events", index.event_count);
        }

        // Initialize chain counters
        let mut counters = self.chain_counters.write().await;
        for chain_id in &self.config.indexed_chains {
            counters.insert(chain_id.clone(), 0);
        }

        info!("Causality indexer initialized successfully");
        Ok(())
    }

    /// Process a single event
    pub async fn process_event(&self, event: &dyn Event) -> Result<()> {
        let chain_id = ChainId(event.chain().to_string());
        
        // Check if we should index this chain
        if !self.config.indexed_chains.is_empty() && !self.config.indexed_chains.contains(&chain_id) {
            return Ok(());
        }

        let causality_event = CausalityEvent::from_event(event);
        
        // Process with SMT if enabled
        if self.config.enable_smt {
            self.process_event_smt(&causality_event).await?;
        }

        // Process with causality tracking if enabled
        if self.config.enable_causality_tracking {
            self.process_event_causality(causality_event.clone()).await?;
        }

        // Store the event
        self.storage.causality_backend().store_event(&causality_event).await?;

        // Update counters and index
        self.update_index(&causality_event).await?;

        Ok(())
    }

    /// Process event with SMT
    async fn process_event_smt(&self, event: &CausalityEvent) -> Result<()> {
        let key = event.smt_key(self.smt.hasher());
        let data = event.to_bytes()?;
        
        let current_root = *self.current_root.read().await;
        let new_root = self.smt.insert(current_root, &key, &data).await?;
        
        let mut root_guard = self.current_root.write().await;
        *root_guard = new_root;
        
        Ok(())
    }

    /// Process event with causality tracking
    async fn process_event_causality(&self, event: CausalityEvent) -> Result<()> {
        let mut tracker = self.causality_tracker.write().await;
        tracker.add_event(event)?;
        Ok(())
    }

    /// Update the causality index
    async fn update_index(&self, event: &CausalityEvent) -> Result<()> {
        let mut index = self.causality_index.write().await;
        let mut counters = self.chain_counters.write().await;
        
        // Update event count
        index.event_count += 1;
        
        // Update chain counter
        let chain_counter = counters.entry(event.chain_id.clone()).or_insert(0);
        *chain_counter += 1;
        
        // Update chain list if new
        if !index.chains.contains(&event.chain_id) {
            index.chains.push(event.chain_id.clone());
        }
        
        // Update root and timestamp
        index.root = *self.current_root.read().await;
        index.last_updated = SystemTime::now();
        
        // Store updated index
        self.storage.causality_backend().store_index(&index).await?;
        
        Ok(())
    }

    /// Get current SMT root
    pub async fn get_current_root(&self) -> SmtRoot {
        *self.current_root.read().await
    }

    /// Get causality index
    pub async fn get_causality_index(&self) -> CausalityIndex {
        self.causality_index.read().await.clone()
    }

    /// Get causality graph
    pub async fn get_causality_graph(&self) -> CausalityGraph {
        self.causality_tracker.read().await.graph().clone()
    }

    /// Generate SMT proof for an event
    pub async fn generate_event_proof(&self, event_id: &str) -> Result<Option<crate::types::SmtProof>> {
        // Get the event to find its SMT key
        if let Some(event) = self.storage.causality_backend().get_event(event_id).await? {
            let key = event.smt_key(self.smt.hasher());
            let root = self.get_current_root().await;
            self.smt.get_proof(root, &key).await
        } else {
            Ok(None)
        }
    }

    /// Verify an SMT proof for an event
    pub async fn verify_event_proof(
        &self,
        event_id: &str,
        proof: &crate::types::SmtProof,
        root: &SmtRoot,
    ) -> Result<bool> {
        if let Some(event) = self.storage.causality_backend().get_event(event_id).await? {
            let key = event.smt_key(self.smt.hasher());
            let data = event.to_bytes()?;
            Ok(self.smt.verify_proof(root, &key, &data, proof))
        } else {
            Ok(false)
        }
    }

    /// Get events for a specific chain
    pub async fn get_chain_events(&self, chain_id: &ChainId) -> Result<Vec<CausalityEvent>> {
        self.storage.causality_backend().get_chain_events(chain_id).await
    }

    /// Get events in a block range
    pub async fn get_events_in_range(
        &self,
        chain_id: &ChainId,
        start_block: u64,
        end_block: u64,
    ) -> Result<Vec<CausalityEvent>> {
        self.storage.causality_backend()
            .get_events_in_range(chain_id, start_block, end_block)
            .await
    }

    /// Get statistics about the indexer
    pub async fn get_statistics(&self) -> CausalityIndexerStatistics {
        let index = self.causality_index.read().await;
        let counters = self.chain_counters.read().await;
        
        CausalityIndexerStatistics {
            total_events: index.event_count,
            total_resources: index.resource_count,
            indexed_chains: index.chains.len(),
            current_root: index.root,
            last_updated: index.last_updated,
            chain_event_counts: counters.clone(),
        }
    }
}

/// Statistics about the causality indexer
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CausalityIndexerStatistics {
    /// Total number of events indexed
    pub total_events: u64,
    /// Total number of resources indexed
    pub total_resources: u64,
    /// Number of indexed chains
    pub indexed_chains: usize,
    /// Current SMT root
    pub current_root: SmtRoot,
    /// Last update timestamp
    pub last_updated: SystemTime,
    /// Per-chain event counts
    pub chain_event_counts: HashMap<ChainId, u64>,
}

/// Event processor implementation for the causality indexer
pub struct CausalityEventProcessor<B: SmtBackend> {
    indexer: Arc<CausalityIndexer<B>>,
}

impl<B: SmtBackend> CausalityEventProcessor<B> {
    /// Create a new causality event processor
    pub fn new(indexer: Arc<CausalityIndexer<B>>) -> Self {
        Self { indexer }
    }

    /// Process a single event
    pub async fn process_event(&self, event: &dyn Event) -> indexer_core::Result<()> {
        match self.indexer.process_event(event).await {
            Ok(()) => Ok(()),
            Err(e) => {
                error!("Failed to process event in causality indexer: {}", e);
                Err(indexer_core::Error::generic(format!("Causality indexer error: {}", e)))
            }
        }
    }

    /// Process multiple events
    pub async fn process_events(&self, events: &[&dyn Event]) -> indexer_core::Result<()> {
        for event in events {
            self.process_event(*event).await?;
        }
        Ok(())
    }

    /// Get the processor name
    pub fn name(&self) -> &str {
        "causality-indexer"
    }
}

/// Builder for creating causality indexers
pub struct CausalityIndexerBuilder<B: SmtBackend> {
    config: CausalityIndexerConfig,
    smt_backend: Option<B>,
    causality_backend: Option<Box<dyn CausalityStorageBackend>>,
}

impl<B: SmtBackend> CausalityIndexerBuilder<B> {
    /// Create a new builder
    pub fn new() -> Self {
        Self {
            config: CausalityIndexerConfig::default(),
            smt_backend: None,
            causality_backend: None,
        }
    }

    /// Set the configuration
    pub fn with_config(mut self, config: CausalityIndexerConfig) -> Self {
        self.config = config;
        self
    }

    /// Set the SMT backend
    pub fn with_smt_backend(mut self, backend: B) -> Self {
        self.smt_backend = Some(backend);
        self
    }

    /// Set the causality storage backend
    pub fn with_causality_backend(mut self, backend: Box<dyn CausalityStorageBackend>) -> Self {
        self.causality_backend = Some(backend);
        self
    }

    /// Add a chain to index
    pub fn add_chain(mut self, chain_id: ChainId) -> Self {
        self.config.indexed_chains.push(chain_id);
        self
    }

    /// Set the hasher type
    pub fn with_hasher(mut self, hasher_type: HasherType) -> Self {
        self.config.hasher_type = hasher_type;
        self
    }

    /// Enable or disable SMT indexing
    pub fn enable_smt(mut self, enable: bool) -> Self {
        self.config.enable_smt = enable;
        self
    }

    /// Enable or disable causality tracking
    pub fn enable_causality_tracking(mut self, enable: bool) -> Self {
        self.config.enable_causality_tracking = enable;
        self
    }

    /// Build the causality indexer
    pub fn build(self) -> Result<CausalityIndexer<B>> {
        let smt_backend = self.smt_backend
            .ok_or_else(|| CausalityError::ConfigError("SMT backend not provided".to_string()))?;
        
        let causality_backend = self.causality_backend
            .ok_or_else(|| CausalityError::ConfigError("Causality backend not provided".to_string()))?;

        CausalityIndexer::new(self.config, smt_backend, causality_backend)
    }
}

impl<B: SmtBackend> Default for CausalityIndexerBuilder<B> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::smt::MemorySmtBackend;
    use crate::storage::MemoryCausalityStorage;
    use std::time::SystemTime;

    #[tokio::test]
    async fn test_causality_indexer_creation() {
        let smt_backend = MemorySmtBackend::new();
        let causality_backend = Box::new(MemoryCausalityStorage::new());
        
        let indexer = CausalityIndexerBuilder::new()
            .with_smt_backend(smt_backend)
            .with_causality_backend(causality_backend)
            .add_chain(ChainId("test-chain".to_string()))
            .build()
            .unwrap();

        indexer.initialize().await.unwrap();
        
        let stats = indexer.get_statistics().await;
        assert_eq!(stats.total_events, 0);
        assert_eq!(stats.indexed_chains, 0);
    }

    #[tokio::test]
    async fn test_causality_indexer_event_processing() {
        let smt_backend = MemorySmtBackend::new();
        let causality_backend = Box::new(MemoryCausalityStorage::new());
        
        let indexer = CausalityIndexerBuilder::new()
            .with_smt_backend(smt_backend)
            .with_causality_backend(causality_backend)
            .add_chain(ChainId("test-chain".to_string()))
            .build()
            .unwrap();

        indexer.initialize().await.unwrap();

        // Create a mock event
        let event = CausalityEvent {
            id: "test-event-1".to_string(),
            chain_id: ChainId("test-chain".to_string()),
            block_number: 100,
            tx_hash: "0x123".to_string(),
            event_type: crate::types::CausalityEventType::CrossDomainMessage,
            timestamp: SystemTime::now(),
            data: crate::types::CausalityEventData::CrossDomainMessage {
                source_domain: crate::types::empty_hash(),
                target_domain: crate::types::empty_hash(),
                message_type: "test".to_string(),
                payload: b"test data".to_vec(),
            },
        };

        // Process the event (we'd need to implement Event trait for CausalityEvent for this to work)
        // For now, just test the internal method
        indexer.process_event_smt(&event).await.unwrap();
        indexer.update_index(&event).await.unwrap();

        let stats = indexer.get_statistics().await;
        assert_eq!(stats.total_events, 1);
        
        let root = indexer.get_current_root().await;
        assert_ne!(root, SparseMerkleTree::<MemorySmtBackend>::empty_root());
    }
} 