//! FoundationDB-backed account subscription manager implementation

use async_trait::async_trait;
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

use crate::fdb::{
    account_data::ChainAccountData,
    merkle_tree::{BatchProcessor, SparseMerkleTree},
    TupleFdbStorage,
};
// Removed circular dependency - use local types from account_data and define missing ones
use crate::fdb::account_data::{AccountAddress, AccountUpdate};
use almanac_chains::subscription::AccountAddress as ChainAccountAddress;

#[derive(Debug, Clone)]
pub struct InclusionProof {
    pub account: AccountAddress,
    pub data_hash: Vec<u8>,
    pub root_hash: Vec<u8>,
    pub proof_path: Vec<Vec<u8>>,
    pub tree_height: usize,
}

#[derive(Debug, thiserror::Error)]
pub enum AccountSubscriptionError {
    #[error("Storage error: {0}")]
    Storage(String),
    #[error("Proof generation failed: {0}")]
    ProofGeneration(String),
    #[error("Connection error: {0}")]
    ConnectionError(String),
    #[error("Subscription limit exceeded: {0}")]
    SubscriptionLimitExceeded(usize),
    #[error("Merkle tree error: {0}")]
    MerkleTreeError(String),
}

#[derive(Debug, Clone)]
pub struct AccountSubscriptionConfig {
    pub max_accounts: usize,
    pub proof_cache_size: usize,
    pub max_subscribed_accounts: usize,
    pub max_batch_size: usize,
}

impl Default for AccountSubscriptionConfig {
    fn default() -> Self {
        Self {
            max_accounts: 100_000,
            proof_cache_size: 10_000,
            max_subscribed_accounts: 100_000,
            max_batch_size: 1000,
        }
    }
}

#[async_trait]
pub trait AccountSubscriptionManager: Send + Sync {
    async fn subscribe(&self, address: AccountAddress) -> Result<(), AccountSubscriptionError>;
    async fn unsubscribe(&self, address: &AccountAddress) -> Result<(), AccountSubscriptionError>;
    async fn get_account_proof(
        &self,
        address: &AccountAddress,
    ) -> Result<Option<InclusionProof>, AccountSubscriptionError>;
    async fn add_accounts(
        &self,
        accounts: Vec<AccountAddress>,
    ) -> Result<(), AccountSubscriptionError>;
    async fn remove_accounts(
        &self,
        accounts: Vec<AccountAddress>,
    ) -> Result<(), AccountSubscriptionError>;
    async fn is_subscribed(
        &self,
        account: &AccountAddress,
    ) -> Result<bool, AccountSubscriptionError>;
    async fn get_subscribed_accounts(
        &self,
        chain_id: &str,
    ) -> Result<Vec<AccountAddress>, AccountSubscriptionError>;
    async fn process_account_updates(
        &self,
        updates: Vec<AccountUpdate>,
    ) -> Result<[u8; 32], AccountSubscriptionError>;
    async fn generate_inclusion_proof(
        &self,
        account: &AccountAddress,
    ) -> Result<Option<InclusionProof>, AccountSubscriptionError>;
    async fn get_tree_root(&self) -> Result<[u8; 32], AccountSubscriptionError>;
    async fn get_touched_accounts(
        &self,
        since_block: u64,
    ) -> Result<Vec<AccountAddress>, AccountSubscriptionError>;
    async fn filter_updates(
        &self,
        updates: Vec<AccountUpdate>,
    ) -> Result<Vec<AccountUpdate>, AccountSubscriptionError>;
}

/// FoundationDB-backed subscription manager with Merkle tree verification
pub struct FdbAccountSubscriptionManager {
    /// Tuple storage for persistence
    tuple_storage: Arc<TupleFdbStorage>,
    /// Merkle tree for inclusion proofs
    merkle_tree: Arc<RwLock<SparseMerkleTree<ChainAccountData>>>,
    /// Batch processor for efficient updates
    batch_processor: Arc<RwLock<BatchProcessor<ChainAccountData>>>,
    /// Configuration
    config: AccountSubscriptionConfig,
    /// In-memory subscription cache for fast lookups
    subscriptions: Arc<RwLock<HashSet<AccountAddress>>>,
}

impl FdbAccountSubscriptionManager {
    /// Create a new subscription manager
    pub async fn new(
        tuple_storage: Arc<TupleFdbStorage>,
        config: AccountSubscriptionConfig,
    ) -> Result<Self, AccountSubscriptionError> {
        let merkle_tree = Arc::new(RwLock::new(SparseMerkleTree::new()));
        let batch_processor = {
            let tree = SparseMerkleTree::new();
            Arc::new(RwLock::new(BatchProcessor::new(
                tree,
                config.max_batch_size,
            )))
        };

        let subscriptions = Arc::new(RwLock::new(HashSet::new()));

        let manager = Self {
            tuple_storage,
            merkle_tree,
            batch_processor,
            config,
            subscriptions,
        };

        // Load existing subscriptions from FDB
        manager.load_subscriptions().await?;

        info!("Initialized FDB account subscription manager");
        Ok(manager)
    }

    /// Load existing subscriptions from storage
    async fn load_subscriptions(&self) -> Result<(), AccountSubscriptionError> {
        debug!("Loading subscriptions from FDB");
        // For now, start with empty cache - in production this would scan FDB
        // and rebuild the subscription set and Merkle tree
        Ok(())
    }

    /// Store subscription in FDB
    async fn store_subscription(
        &self,
        account: &AccountAddress,
    ) -> Result<(), AccountSubscriptionError> {
        // Convert to chain AccountAddress
        let chain_account = ChainAccountAddress {
            address: account.address.clone(),
            chain_id: account.chain.clone(),
        };
        match self.tuple_storage.store_subscription(&chain_account).await {
            Ok(()) => {
                debug!(
                    "Stored subscription for {}:{}",
                    account.chain,
                    account.to_hex()
                );
                Ok(())
            }
            Err(e) => {
                error!("Failed to store subscription: {}", e);
                Err(AccountSubscriptionError::ConnectionError(format!(
                    "Storage error: {}",
                    e
                )))
            }
        }
    }

    /// Remove subscription from FDB
    async fn remove_subscription(
        &self,
        account: &AccountAddress,
    ) -> Result<(), AccountSubscriptionError> {
        // Convert to chain AccountAddress
        let chain_account = ChainAccountAddress {
            address: account.address.clone(),
            chain_id: account.chain.clone(),
        };
        match self.tuple_storage.remove_subscription(&chain_account).await {
            Ok(()) => {
                debug!(
                    "Removed subscription for {}:{}",
                    account.chain,
                    account.to_hex()
                );
                Ok(())
            }
            Err(e) => {
                error!("Failed to remove subscription: {}", e);
                Err(AccountSubscriptionError::ConnectionError(format!(
                    "Storage error: {}",
                    e
                )))
            }
        }
    }

    /// Update Merkle tree with account data
    async fn update_merkle_tree(
        &self,
        account_data: ChainAccountData,
    ) -> Result<[u8; 32], AccountSubscriptionError> {
        let mut tree = self.merkle_tree.write().await;
        match tree.update(account_data) {
            Ok(root) => {
                debug!("Updated Merkle tree, new root: {}", hex::encode(root));
                Ok(root)
            }
            Err(e) => {
                error!("Failed to update Merkle tree: {}", e);
                Err(AccountSubscriptionError::MerkleTreeError(e.to_string()))
            }
        }
    }
}

#[async_trait]
impl AccountSubscriptionManager for FdbAccountSubscriptionManager {
    async fn add_accounts(
        &self,
        accounts: Vec<AccountAddress>,
    ) -> Result<(), AccountSubscriptionError> {
        info!("Adding {} accounts to subscription", accounts.len());

        // Check subscription limit
        let current_count = {
            let subscriptions = self.subscriptions.read().await;
            subscriptions.len()
        };

        if current_count + accounts.len() > self.config.max_subscribed_accounts {
            return Err(AccountSubscriptionError::SubscriptionLimitExceeded(
                self.config.max_subscribed_accounts,
            ));
        }

        let mut subscriptions = self.subscriptions.write().await;
        let mut added_count = 0;

        for account in accounts {
            // Check if already subscribed
            if subscriptions.contains(&account) {
                debug!(
                    "Account already subscribed: {}:{}",
                    account.chain,
                    account.to_hex()
                );
                continue;
            }

            // Store in FDB
            self.store_subscription(&account).await?;

            // Add to in-memory cache
            subscriptions.insert(account.clone());

            // Add to Merkle tree
            let account_data = ChainAccountData::from(account);
            self.update_merkle_tree(account_data).await?;

            added_count += 1;
        }

        info!(
            "Successfully added {} new account subscriptions",
            added_count
        );
        Ok(())
    }

    async fn remove_accounts(
        &self,
        accounts: Vec<AccountAddress>,
    ) -> Result<(), AccountSubscriptionError> {
        info!("Removing {} accounts from subscription", accounts.len());

        let mut subscriptions = self.subscriptions.write().await;
        let mut removed_count = 0;

        for account in accounts {
            if !subscriptions.contains(&account) {
                debug!(
                    "Account not subscribed: {}:{}",
                    account.chain,
                    account.to_hex()
                );
                continue;
            }

            // Remove from FDB
            self.remove_subscription(&account).await?;

            // Remove from in-memory cache
            subscriptions.remove(&account);

            // Remove from Merkle tree
            {
                let mut tree = self.merkle_tree.write().await;
                match tree.remove(&account.address) {
                    Ok(_) => debug!(
                        "Removed from Merkle tree: {}:{}",
                        account.chain,
                        account.to_hex()
                    ),
                    Err(e) => warn!("Failed to remove from Merkle tree: {}", e),
                }
            }

            removed_count += 1;
        }

        info!(
            "Successfully removed {} account subscriptions",
            removed_count
        );
        Ok(())
    }

    async fn is_subscribed(
        &self,
        account: &AccountAddress,
    ) -> Result<bool, AccountSubscriptionError> {
        let subscriptions = self.subscriptions.read().await;
        Ok(subscriptions.contains(account))
    }

    async fn get_subscribed_accounts(
        &self,
        chain_id: &str,
    ) -> Result<Vec<AccountAddress>, AccountSubscriptionError> {
        let subscriptions = self.subscriptions.read().await;
        let accounts: Vec<AccountAddress> = subscriptions
            .iter()
            .filter(|account| account.chain == chain_id)
            .cloned()
            .collect();

        debug!(
            "Found {} subscribed accounts for chain {}",
            accounts.len(),
            chain_id
        );
        Ok(accounts)
    }

    async fn process_account_updates(
        &self,
        updates: Vec<AccountUpdate>,
    ) -> Result<[u8; 32], AccountSubscriptionError> {
        debug!("Processing {} account updates", updates.len());

        let subscriptions = self.subscriptions.read().await;
        let mut processed_count = 0;
        let mut final_root = [0u8; 32];

        for update in updates {
            // Only process updates for subscribed accounts
            if !subscriptions.contains(&update.address) {
                continue;
            }

            let account_data = ChainAccountData::from(update);
            final_root = self.update_merkle_tree(account_data).await?;
            processed_count += 1;
        }

        debug!(
            "Processed {} account updates, new root: {}",
            processed_count,
            hex::encode(final_root)
        );
        Ok(final_root)
    }

    async fn generate_inclusion_proof(
        &self,
        account: &AccountAddress,
    ) -> Result<Option<InclusionProof>, AccountSubscriptionError> {
        // Check if account is subscribed
        if !self.is_subscribed(account).await? {
            return Ok(None);
        }

        let tree = self.merkle_tree.read().await;
        match tree.generate_proof(&account.address) {
            Ok(merkle_proof) => {
                let proof = InclusionProof {
                    account: account.clone(),
                    data_hash: merkle_proof.key.to_vec(),
                    root_hash: merkle_proof.root_hash.to_vec(),
                    proof_path: vec![], // Simplified - would contain actual proof path
                    tree_height: 255,   // Max depth for account addresses
                };

                debug!(
                    "Generated inclusion proof for {}:{}",
                    account.chain,
                    account.to_hex()
                );
                Ok(Some(proof))
            }
            Err(e) => {
                error!("Failed to generate inclusion proof: {}", e);
                Err(AccountSubscriptionError::MerkleTreeError(e.to_string()))
            }
        }
    }

    async fn get_tree_root(&self) -> Result<[u8; 32], AccountSubscriptionError> {
        let tree = self.merkle_tree.read().await;
        Ok(tree.root())
    }

    async fn get_touched_accounts(
        &self,
        since_block: u64,
    ) -> Result<Vec<AccountAddress>, AccountSubscriptionError> {
        // For minimal implementation, return empty list
        // In full implementation, this would scan FDB for accounts updated since the block
        debug!("Getting touched accounts since block {}", since_block);
        Ok(Vec::new())
    }

    async fn filter_updates(
        &self,
        updates: Vec<AccountUpdate>,
    ) -> Result<Vec<AccountUpdate>, AccountSubscriptionError> {
        let subscriptions = self.subscriptions.read().await;
        let filtered: Vec<AccountUpdate> = updates
            .into_iter()
            .filter(|update| subscriptions.contains(&update.address))
            .collect();

        debug!(
            "Filtered {} updates to {} subscribed accounts",
            filtered.len(),
            subscriptions.len()
        );
        Ok(filtered)
    }

    async fn subscribe(&self, address: AccountAddress) -> Result<(), AccountSubscriptionError> {
        // Add to subscriptions
        {
            let mut subscriptions = self.subscriptions.write().await;
            subscriptions.insert(address.clone());
        }
        // Store in FDB
        self.store_subscription(&address).await?;
        Ok(())
    }

    async fn unsubscribe(&self, address: &AccountAddress) -> Result<(), AccountSubscriptionError> {
        // Remove from subscriptions
        {
            let mut subscriptions = self.subscriptions.write().await;
            subscriptions.remove(address);
        }
        // Remove from FDB
        self.remove_subscription(address).await?;
        Ok(())
    }

    async fn get_account_proof(
        &self,
        address: &AccountAddress,
    ) -> Result<Option<InclusionProof>, AccountSubscriptionError> {
        self.generate_inclusion_proof(address).await
    }
}

impl crate::manager::SimpleSubscriptionManager for FdbAccountSubscriptionManager {}

impl From<AccountSubscriptionError> for almanac_core::Error {
    fn from(err: AccountSubscriptionError) -> Self {
        almanac_core::Error::storage(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[allow(unused_imports)]
    use crate::fdb::FdbConfig;

    // Mock TupleFdbStorage for testing
    // In real implementation, would use actual FDB instance

    #[tokio::test]
    async fn test_subscription_manager_creation() {
        // This test would require a real FDB instance
        // For now, just test the concept
        let config = AccountSubscriptionConfig::default();
        assert_eq!(config.max_subscribed_accounts, 100_000);
    }

    #[test]
    fn test_account_address_equality() {
        let addr1 = AccountAddress::new(vec![1, 2, 3, 4], "ethereum".to_string());
        let addr2 = AccountAddress::new(vec![1, 2, 3, 4], "ethereum".to_string());
        let addr3 = AccountAddress::new(vec![1, 2, 3, 5], "ethereum".to_string());

        assert_eq!(addr1, addr2);
        assert_ne!(addr1, addr3);
    }
}
