//! Simple FoundationDB-backed account subscription manager
//!
//! This manager handles subscription tracking.
//! (Merkle trees are managed by individual chain adapters)

use async_trait::async_trait;
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

use crate::fdb::TupleFdbStorage;
use crate::fdb::account_data::{AccountAddress, AccountUpdate};

#[derive(Debug, thiserror::Error)]
pub enum AccountSubscriptionError {
    #[error("Storage error: {0}")]
    Storage(String),
    #[error("Connection error: {0}")]
    ConnectionError(String),
    #[error("Subscription limit exceeded: {0}")]
    SubscriptionLimitExceeded(usize),
}

#[derive(Debug, Clone)]
pub struct AccountSubscriptionConfig {
    pub max_subscribed_accounts: usize,
}

impl Default for AccountSubscriptionConfig {
    fn default() -> Self {
        Self {
            max_subscribed_accounts: 100_000,
        }
    }
}

#[async_trait]
pub trait AccountSubscriptionManager: Send + Sync {
    async fn subscribe(&self, address: AccountAddress) -> Result<(), AccountSubscriptionError>;
    async fn unsubscribe(&self, address: &AccountAddress) -> Result<(), AccountSubscriptionError>;
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
    async fn filter_updates(
        &self,
        updates: Vec<AccountUpdate>,
    ) -> Result<Vec<AccountUpdate>, AccountSubscriptionError>;
}

/// Simple FoundationDB-backed subscription manager
pub struct FdbAccountSubscriptionManager {
    /// Tuple storage for persistence
    tuple_storage: Arc<TupleFdbStorage>,
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
        let subscriptions = Arc::new(RwLock::new(HashSet::new()));

        let manager = Self {
            tuple_storage,
            config,
            subscriptions,
        };

        // Load existing subscriptions from FDB
        manager.load_subscriptions().await?;

        info!("Initialized simple FDB account subscription manager");
        Ok(manager)
    }

    /// Load existing subscriptions from storage
    async fn load_subscriptions(&self) -> Result<(), AccountSubscriptionError> {
        // For each chain, load subscribed accounts
        let chains = vec!["ethereum", "cosmos", "solana"]; // TODO: Make configurable
        
        for chain in chains {
            match self.tuple_storage.get_subscribed_accounts(chain).await {
                Ok(accounts) => {
                    let mut subs = self.subscriptions.write().await;
                    for address in accounts {
                        let account_addr = AccountAddress::new(address, chain.to_string());
                        subs.insert(account_addr);
                    }
                    debug!("Loaded {} subscriptions for chain {}", subs.len(), chain);
                }
                Err(e) => {
                    warn!("Failed to load subscriptions for chain {}: {}", chain, e);
                }
            }
        }

        Ok(())
    }
}

#[async_trait]
impl AccountSubscriptionManager for FdbAccountSubscriptionManager {
    async fn subscribe(&self, address: AccountAddress) -> Result<(), AccountSubscriptionError> {
        self.add_accounts(vec![address]).await
    }

    async fn unsubscribe(&self, address: &AccountAddress) -> Result<(), AccountSubscriptionError> {
        self.remove_accounts(vec![address.clone()]).await
    }

    async fn add_accounts(
        &self,
        accounts: Vec<AccountAddress>,
    ) -> Result<(), AccountSubscriptionError> {
        if accounts.is_empty() {
            return Ok(());
        }

        let mut subs = self.subscriptions.write().await;
        
        // Check subscription limit
        let new_total = subs.len() + accounts.len();
        if new_total > self.config.max_subscribed_accounts {
            return Err(AccountSubscriptionError::SubscriptionLimitExceeded(new_total));
        }

        let mut added_count = 0;
        for account in accounts {
            // Store in FDB
            match self
                .tuple_storage
                .add_account_subscription(&account.chain, &account.address, chrono::Utc::now().timestamp() as u64)
                .await
            {
                Ok(_) => {
                    subs.insert(account.clone());
                    added_count += 1;
                    debug!("Added subscription for account: {}", account.to_hex());
                }
                Err(e) => {
                    error!("Failed to add subscription for account {}: {}", account.to_hex(), e);
                }
            }
        }

        info!("Added {} account subscriptions", added_count);
        Ok(())
    }

    async fn remove_accounts(
        &self,
        accounts: Vec<AccountAddress>,
    ) -> Result<(), AccountSubscriptionError> {
        if accounts.is_empty() {
            return Ok(());
        }

        let mut subs = self.subscriptions.write().await;
        let mut removed_count = 0;

        for account in accounts {
            // Remove from FDB
            match self
                .tuple_storage
                .remove_account_subscription(&account.chain, &account.address)
                .await
            {
                Ok(_) => {
                    subs.remove(&account);
                    removed_count += 1;
                    debug!("Removed subscription for account: {}", account.to_hex());
                }
                Err(e) => {
                    error!("Failed to remove subscription for account {}: {}", account.to_hex(), e);
                }
            }
        }

        info!("Removed {} account subscriptions", removed_count);
        Ok(())
    }

    async fn is_subscribed(
        &self,
        account: &AccountAddress,
    ) -> Result<bool, AccountSubscriptionError> {
        let subs = self.subscriptions.read().await;
        Ok(subs.contains(account))
    }

    async fn get_subscribed_accounts(
        &self,
        chain_id: &str,
    ) -> Result<Vec<AccountAddress>, AccountSubscriptionError> {
        let subs = self.subscriptions.read().await;
        Ok(subs
            .iter()
            .filter(|acc| acc.chain == chain_id)
            .cloned()
            .collect())
    }

    async fn filter_updates(
        &self,
        updates: Vec<AccountUpdate>,
    ) -> Result<Vec<AccountUpdate>, AccountSubscriptionError> {
        let subs = self.subscriptions.read().await;
        
        Ok(updates
            .into_iter()
            .filter(|update| subs.contains(&update.address))
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use almanac_core::storage::memory::MemoryStorage;

    #[tokio::test]
    async fn test_subscription_management() {
        // This would need a test implementation of TupleFdbStorage
        // For now, we'll skip the actual test implementation
    }
}