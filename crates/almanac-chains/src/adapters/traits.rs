//! Chain adapter trait definitions

use super::types::{BlockMetadata, ChainDataIR, FinalityStatus, StateChange, StorageProof};
use async_trait::async_trait;
use thiserror::Error;

/// Chain adapter error types
#[derive(Debug, Error)]
pub enum ChainAdapterError {
    #[error("Network error: {0}")]
    Network(String),

    #[error("RPC error: {0}")]
    Rpc(String),

    #[error("Block not found: {0}")]
    BlockNotFound(u64),

    #[error("Invalid block data: {0}")]
    InvalidBlock(String),

    #[error("Layout compilation failed: {0}")]
    LayoutCompilation(String),

    #[error("Proof generation failed: {0}")]
    ProofGeneration(String),

    #[error("Configuration error: {0}")]
    Configuration(String),

    #[error("Unsupported feature: {0}")]
    Unsupported(String),

    #[error("Not implemented: {0}")]
    NotImplemented(String),

    #[error("Other error: {0}")]
    Other(String),
}

/// Trait for chain-specific adapters
///
/// Each blockchain implementation must provide:
/// - Block and state extraction
/// - Storage proof generation
/// - Normalized data transformation
/// - Consensus and finality rules
#[async_trait]
pub trait ChainAdapter: Send + Sync {
    /// Get the chain identifier
    fn chain_id(&self) -> &str;

    /// Fetch and normalize block data
    ///
    /// This method should:
    /// 1. Fetch the block from the chain
    /// 2. Extract all state changes
    /// 3. Generate storage proofs where applicable
    /// 4. Normalize to ChainDataIR format
    async fn fetch_block(&self, block_number: u64) -> Result<ChainDataIR, ChainAdapterError>;

    /// Fetch the latest block number
    async fn get_latest_block(&self) -> Result<u64, ChainAdapterError>;

    /// Check if a block is finalized based on chain-specific rules
    async fn is_block_finalized(&self, block_number: u64) -> Result<bool, ChainAdapterError>;

    /// Check block finality status based on chain-specific rules
    async fn check_block_finality(
        &self,
        block_number: u64,
        current_height: u64,
    ) -> Result<FinalityStatus, ChainAdapterError>;

    /// Generate storage proofs for specific keys
    async fn generate_storage_proofs(
        &self,
        contract_address: &str,
        storage_keys: &[[u8; 32]],
        block_number: u64,
    ) -> Result<Vec<StorageProof>, ChainAdapterError>;

    /// Get chain-specific metadata for a block
    async fn get_block_metadata(
        &self,
        block_number: u64,
    ) -> Result<BlockMetadata, ChainAdapterError>;

    /// Extract state changes from a block
    ///
    /// This should identify all storage slot modifications
    async fn extract_state_changes(
        &self,
        block_number: u64,
    ) -> Result<Vec<StateChange>, ChainAdapterError>;

    /// Subscribe to new blocks (optional)
    async fn subscribe_blocks(
        &self,
    ) -> Result<Box<dyn futures::Stream<Item = u64> + Send>, ChainAdapterError> {
        Err(ChainAdapterError::Unsupported(
            "Block subscription not supported".into(),
        ))
    }
}
