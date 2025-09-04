#[cfg(test)]
pub use crate::routes::create_test_router;

#[cfg(test)]
use almanac_chains::subscription::{
    AccountAddress, AccountSubscriptionError, AccountSubscriptionManager, AccountUpdate,
    InclusionProof,
};
#[cfg(test)]
use async_trait::async_trait;
#[cfg(test)]
use std::collections::HashSet;
#[cfg(test)]
use std::sync::{Arc, Mutex};

#[cfg(test)]
pub struct MockSubscriptionManager {
    subscribed_accounts: Arc<Mutex<HashSet<String>>>,
}

#[cfg(test)]
impl MockSubscriptionManager {
    pub fn new() -> Self {
        Self {
            subscribed_accounts: Arc::new(Mutex::new(HashSet::new())),
        }
    }
}

#[cfg(test)]
#[async_trait]
impl AccountSubscriptionManager for MockSubscriptionManager {
    async fn add_accounts(&self, accounts: Vec<AccountAddress>) -> Result<(), AccountSubscriptionError> {
        let mut subscribed = self.subscribed_accounts.lock().unwrap();
        for account in accounts {
            subscribed.insert(format!("{}:{}", account.chain_id, account.to_hex()));
        }
        Ok(())
    }

    async fn remove_accounts(&self, accounts: Vec<AccountAddress>) -> Result<(), AccountSubscriptionError> {
        let mut subscribed = self.subscribed_accounts.lock().unwrap();
        for account in accounts {
            subscribed.remove(&format!("{}:{}", account.chain_id, account.to_hex()));
        }
        Ok(())
    }

    async fn is_subscribed(&self, account: &AccountAddress) -> Result<bool, AccountSubscriptionError> {
        let subscribed = self.subscribed_accounts.lock().unwrap();
        Ok(subscribed.contains(&format!("{}:{}", account.chain_id, account.to_hex())))
    }

    async fn get_subscribed_accounts(&self, chain_id: &str) -> Result<Vec<AccountAddress>, AccountSubscriptionError> {
        let subscribed = self.subscribed_accounts.lock().unwrap();
        let accounts: Vec<AccountAddress> = subscribed
            .iter()
            .filter_map(|key| {
                let parts: Vec<&str> = key.split(':').collect();
                if parts.len() == 2 && parts[0] == chain_id {
                    AccountAddress::from_hex(parts[0], parts[1]).ok()
                } else {
                    None
                }
            })
            .collect();
        Ok(accounts)
    }

    async fn process_account_updates(&self, _updates: Vec<AccountUpdate>) -> Result<[u8; 32], AccountSubscriptionError> {
        // Return dummy root hash for testing
        Ok([0u8; 32])
    }

    async fn generate_inclusion_proof(&self, account: &AccountAddress) -> Result<Option<InclusionProof>, AccountSubscriptionError> {
        // Return a dummy proof for testing
        Ok(Some(InclusionProof {
            account: account.clone(),
            data_hash: [0u8; 32],
            root_hash: [1u8; 32],
            proof_path: vec![[2u8; 32], [3u8; 32]],
            tree_height: 2,
        }))
    }

    async fn get_tree_root(&self) -> Result<[u8; 32], AccountSubscriptionError> {
        Ok([3u8; 32])
    }

    async fn get_touched_accounts(&self, _since_block: u64) -> Result<Vec<AccountAddress>, AccountSubscriptionError> {
        let subscribed = self.subscribed_accounts.lock().unwrap();
        let accounts: Vec<AccountAddress> = subscribed
            .iter()
            .filter_map(|key| {
                let parts: Vec<&str> = key.split(':').collect();
                if parts.len() == 2 {
                    AccountAddress::from_hex(parts[0], parts[1]).ok()
                } else {
                    None
                }
            })
            .collect();
        Ok(accounts)
    }

    async fn filter_updates(&self, updates: Vec<AccountUpdate>) -> Result<Vec<AccountUpdate>, AccountSubscriptionError> {
        // In test mode, return all updates without filtering
        Ok(updates)
    }
}

#[cfg(test)]
pub async fn create_test_app_state() -> crate::state::AppState {
    use std::sync::Arc;

    let storage = Arc::new(crate::MemoryExtendedStorage::new());
    let subscription_manager = MockSubscriptionManager::new();

    crate::state::AppState::new(storage)
        .with_subscription_manager(subscription_manager)
}
