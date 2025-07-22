//! Account subscription system with Merkle tree verification
//!
//! This module provides a chain-agnostic account subscription system that:
//! - Maintains a bloom filter for efficient account filtering
//! - Uses Sparse Merkle Trees for account state verification
//! - Supports adding/removing accounts from subscription lists
//! - Provides cryptographic proofs of account state changes

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use thiserror::Error;

/// Account address type - can represent addresses from any chain
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AccountAddress {
    pub chain_id: String,
    pub address: Vec<u8>,
}

impl AccountAddress {
    pub fn new(chain_id: impl Into<String>, address: impl Into<Vec<u8>>) -> Self {
        Self {
            chain_id: chain_id.into(),
            address: address.into(),
        }
    }

    /// Create from hex string (for Ethereum-like chains)
    pub fn from_hex(
        chain_id: impl Into<String>,
        hex: &str,
    ) -> Result<Self, AccountSubscriptionError> {
        let address = hex::decode(hex.strip_prefix("0x").unwrap_or(hex))
            .map_err(|e| AccountSubscriptionError::InvalidAddress(format!("Invalid hex: {}", e)))?;
        Ok(Self::new(chain_id, address))
    }

    /// Create from base58 string (for Solana-like chains)
    pub fn from_base58(
        chain_id: impl Into<String>,
        base58: &str,
    ) -> Result<Self, AccountSubscriptionError> {
        let address = bs58::decode(base58).into_vec().map_err(|e| {
            AccountSubscriptionError::InvalidAddress(format!("Invalid base58: {}", e))
        })?;
        Ok(Self::new(chain_id, address))
    }

    /// Get address as hex string
    pub fn to_hex(&self) -> String {
        format!("0x{}", hex::encode(&self.address))
    }

    /// Get a hash of this address for use in Merkle tree
    pub fn hash(&self) -> [u8; 32] {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(self.chain_id.as_bytes());
        hasher.update(&self.address);
        hasher.finalize().into()
    }
}

/// Account update information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountUpdate {
    pub account: AccountAddress,
    pub data_hash: [u8; 32],
    pub block_height: u64,
    pub timestamp: u64,
    pub chain_specific_data: Option<Vec<u8>>,
}

/// Merkle tree inclusion proof
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InclusionProof {
    pub account: AccountAddress,
    pub data_hash: [u8; 32],
    pub root_hash: [u8; 32],
    pub proof_path: Vec<[u8; 32]>,
    pub tree_height: u8,
}

/// Subscription filter configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscriptionFilter {
    /// Accounts to subscribe to
    pub accounts: HashSet<AccountAddress>,
    /// Chain IDs to filter by
    pub chain_ids: HashSet<String>,
    /// Block height range (optional)
    pub block_range: Option<(u64, Option<u64>)>,
    /// Maximum number of accounts to track
    pub max_accounts: Option<usize>,
}

impl Default for SubscriptionFilter {
    fn default() -> Self {
        Self {
            accounts: HashSet::new(),
            chain_ids: HashSet::new(),
            block_range: None,
            max_accounts: Some(10_000), // Default limit
        }
    }
}

#[derive(Debug, Error)]
pub enum AccountSubscriptionError {
    #[error("Invalid account address: {0}")]
    InvalidAddress(String),

    #[error("Subscription limit exceeded: {0}")]
    SubscriptionLimitExceeded(usize),

    #[error("Account not found: {0:?}")]
    AccountNotFound(AccountAddress),

    #[error("Merkle tree error: {0}")]
    MerkleTreeError(String),

    #[error("Bloom filter error: {0}")]
    BloomFilterError(String),

    #[error("Chain not supported: {0}")]
    ChainNotSupported(String),

    #[error("Configuration error: {0}")]
    Configuration(String),

    #[error("Connection error: {0}")]
    ConnectionError(String),

    #[error("Invalid request: {0}")]
    InvalidRequest(String),

    #[error("Stream error: {0}")]
    StreamError(String),

    #[error("Not supported: {0}")]
    NotSupported(String),

    #[error("Other error: {0}")]
    Other(String),
}

/// Trait for account subscription managers
#[async_trait]
pub trait AccountSubscriptionManager: Send + Sync {
    /// Add accounts to the subscription list
    async fn add_accounts(
        &self,
        accounts: Vec<AccountAddress>,
    ) -> Result<(), AccountSubscriptionError>;

    /// Remove accounts from the subscription list
    async fn remove_accounts(
        &self,
        accounts: Vec<AccountAddress>,
    ) -> Result<(), AccountSubscriptionError>;

    /// Check if an account is subscribed
    async fn is_subscribed(
        &self,
        account: &AccountAddress,
    ) -> Result<bool, AccountSubscriptionError>;

    /// Get all subscribed accounts for a chain
    async fn get_subscribed_accounts(
        &self,
        chain_id: &str,
    ) -> Result<Vec<AccountAddress>, AccountSubscriptionError>;

    /// Process account updates and update Merkle tree
    async fn process_account_updates(
        &self,
        updates: Vec<AccountUpdate>,
    ) -> Result<[u8; 32], AccountSubscriptionError>;

    /// Generate inclusion proof for an account
    async fn generate_inclusion_proof(
        &self,
        account: &AccountAddress,
    ) -> Result<Option<InclusionProof>, AccountSubscriptionError>;

    /// Get the current Merkle tree root
    async fn get_tree_root(&self) -> Result<[u8; 32], AccountSubscriptionError>;

    /// Get accounts that were touched since a specific block
    async fn get_touched_accounts(
        &self,
        since_block: u64,
    ) -> Result<Vec<AccountAddress>, AccountSubscriptionError>;

    /// Apply subscription filter to a list of account updates
    async fn filter_updates(
        &self,
        updates: Vec<AccountUpdate>,
    ) -> Result<Vec<AccountUpdate>, AccountSubscriptionError>;
}

/// Configuration for the account subscription system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountSubscriptionConfig {
    /// Maximum number of accounts that can be subscribed
    pub max_subscribed_accounts: usize,

    /// Bloom filter size (number of bits)
    pub bloom_filter_size: usize,

    /// Number of hash functions for bloom filter
    pub bloom_filter_hash_count: u32,

    /// Merkle tree depth (256 for full address space)
    pub merkle_tree_depth: u8,

    /// Maximum batch size for processing updates
    pub max_batch_size: usize,

    /// Cleanup interval for old data (in blocks)
    pub cleanup_interval_blocks: u64,
}

impl Default for AccountSubscriptionConfig {
    fn default() -> Self {
        Self {
            max_subscribed_accounts: 100_000,
            bloom_filter_size: 1_000_000, // ~1MB
            bloom_filter_hash_count: 7,
            merkle_tree_depth: 255,
            max_batch_size: 1000,
            cleanup_interval_blocks: 1000,
        }
    }
}

/// Events emitted by the subscription system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SubscriptionEvent {
    /// Account was added to subscription
    AccountAdded {
        account: AccountAddress,
        timestamp: u64,
    },

    /// Account was removed from subscription
    AccountRemoved {
        account: AccountAddress,
        timestamp: u64,
    },

    /// Subscribed account was updated
    AccountUpdated {
        account: AccountAddress,
        data_hash: [u8; 32],
        block_height: u64,
        timestamp: u64,
    },

    /// Merkle tree root was updated
    TreeRootUpdated {
        old_root: [u8; 32],
        new_root: [u8; 32],
        timestamp: u64,
    },

    /// Subscription limit was reached
    SubscriptionLimitReached {
        current_count: usize,
        max_count: usize,
        timestamp: u64,
    },
}

/// Trait for receiving subscription events
#[async_trait]
pub trait SubscriptionEventHandler: Send + Sync {
    async fn handle_event(&self, event: SubscriptionEvent) -> Result<(), AccountSubscriptionError>;
}

/// Statistics about the subscription system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscriptionStats {
    pub total_subscribed_accounts: usize,
    pub accounts_by_chain: HashMap<String, usize>,
    pub current_tree_root: [u8; 32],
    pub last_update_block: u64,
    pub total_updates_processed: u64,
    pub bloom_filter_utilization: f64,
    pub average_proof_size: usize,
}

/// Chain-specific account subscription trait
///
/// Implementations should provide chain-specific logic for:
/// - Parsing account addresses
/// - Extracting account data
/// - Handling chain-specific update formats
#[async_trait]
pub trait ChainAccountSubscription: Send + Sync {
    /// Get the chain ID this subscription handles
    fn chain_id(&self) -> &str;

    /// Parse an account address from chain-specific format
    async fn parse_account_address(
        &self,
        address: &str,
    ) -> Result<AccountAddress, AccountSubscriptionError>;

    /// Extract account updates from chain-specific block data
    async fn extract_account_updates(
        &self,
        block_data: &[u8],
        subscribed_accounts: &HashSet<AccountAddress>,
    ) -> Result<Vec<AccountUpdate>, AccountSubscriptionError>;

    /// Validate that an account address is valid for this chain
    async fn validate_account_address(
        &self,
        account: &AccountAddress,
    ) -> Result<bool, AccountSubscriptionError>;

    /// Get account data hash for inclusion in Merkle tree
    async fn compute_account_data_hash(
        &self,
        account_data: &[u8],
    ) -> Result<[u8; 32], AccountSubscriptionError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_account_address_creation() {
        let eth_addr =
            AccountAddress::from_hex("ethereum", "0x742d35Cc6634C0532925a3b8D80bE5e4eb16F8d6ef")
                .unwrap();
        assert_eq!(eth_addr.chain_id, "ethereum");

        let sol_addr =
            AccountAddress::from_base58("solana", "11111111111111111111111111111112").unwrap();
        assert_eq!(sol_addr.chain_id, "solana");
    }

    #[test]
    fn test_account_address_hash() {
        let addr = AccountAddress::new("test", vec![1, 2, 3, 4]);
        let hash = addr.hash();
        assert_eq!(hash.len(), 32);

        // Same input should produce same hash
        let addr2 = AccountAddress::new("test", vec![1, 2, 3, 4]);
        assert_eq!(hash, addr2.hash());
    }

    #[test]
    fn test_subscription_filter_default() {
        let filter = SubscriptionFilter::default();
        assert_eq!(filter.max_accounts, Some(10_000));
        assert!(filter.accounts.is_empty());
    }
}
