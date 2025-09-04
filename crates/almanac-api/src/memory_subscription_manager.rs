use almanac_chains::subscription::{
    AccountAddress, AccountSubscriptionError, AccountSubscriptionManager, AccountUpdate,
    InclusionProof,
};
use async_trait::async_trait;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

/// Simple in-memory subscription manager for development/testing
pub struct MemorySubscriptionManager {
    subscribed_accounts: Arc<Mutex<HashSet<String>>>,
    updates: Arc<Mutex<Vec<AccountUpdate>>>,
    tree_root: Arc<Mutex<[u8; 32]>>,
}

impl MemorySubscriptionManager {
    pub fn new() -> Self {
        Self {
            subscribed_accounts: Arc::new(Mutex::new(HashSet::new())),
            updates: Arc::new(Mutex::new(Vec::new())),
            tree_root: Arc::new(Mutex::new([0u8; 32])),
        }
    }
}

#[async_trait]
impl AccountSubscriptionManager for MemorySubscriptionManager {
    async fn add_accounts(&self, accounts: Vec<AccountAddress>) -> Result<(), AccountSubscriptionError> {
        let mut subscribed = self.subscribed_accounts.lock().unwrap();
        for account in accounts {
            let key = format!("{}:{}", account.chain_id, account.to_hex());
            tracing::info!("Adding account to subscription: {}", key);
            subscribed.insert(key);
        }
        Ok(())
    }

    async fn remove_accounts(&self, accounts: Vec<AccountAddress>) -> Result<(), AccountSubscriptionError> {
        let mut subscribed = self.subscribed_accounts.lock().unwrap();
        for account in accounts {
            let key = format!("{}:{}", account.chain_id, account.to_hex());
            tracing::info!("Removing account from subscription: {}", key);
            subscribed.remove(&key);
        }
        Ok(())
    }

    async fn is_subscribed(&self, account: &AccountAddress) -> Result<bool, AccountSubscriptionError> {
        let subscribed = self.subscribed_accounts.lock().unwrap();
        let key = format!("{}:{}", account.chain_id, account.to_hex());
        let result = subscribed.contains(&key);
        tracing::debug!("Checking subscription for {}: {}", key, result);
        Ok(result)
    }

    async fn get_subscribed_accounts(&self, chain_id: &str) -> Result<Vec<AccountAddress>, AccountSubscriptionError> {
        let subscribed = self.subscribed_accounts.lock().unwrap();
        let mut accounts = Vec::new();
        
        for key in subscribed.iter() {
            if let Some((cid, hex)) = key.split_once(':') {
                if cid == chain_id {
                    if let Ok(addr) = AccountAddress::from_hex(chain_id, hex) {
                        accounts.push(addr);
                    }
                }
            }
        }
        
        Ok(accounts)
    }

    async fn process_account_updates(
        &self,
        updates: Vec<AccountUpdate>,
    ) -> Result<[u8; 32], AccountSubscriptionError> {
        let mut stored_updates = self.updates.lock().unwrap();
        let mut tree_root = self.tree_root.lock().unwrap();
        
        for update in updates {
            tracing::info!("Processing account update for {}", update.account.to_hex());
            stored_updates.push(update);
        }
        
        // Update the tree root (dummy implementation)
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(&tree_root[..]);
        hasher.update(b"update");
        *tree_root = hasher.finalize().into();
        
        Ok(*tree_root)
    }

    async fn generate_inclusion_proof(
        &self,
        account: &AccountAddress,
    ) -> Result<Option<InclusionProof>, AccountSubscriptionError> {
        let subscribed = self.subscribed_accounts.lock().unwrap();
        let key = format!("{}:{}", account.chain_id, account.to_hex());
        
        if !subscribed.contains(&key) {
            return Ok(None);
        }
        
        // Return a dummy proof for memory implementation
        Ok(Some(InclusionProof {
            account: account.clone(),
            data_hash: [0u8; 32],
            root_hash: *self.tree_root.lock().unwrap(),
            proof_path: vec![[0u8; 32]],
            tree_height: 1,
        }))
    }

    async fn get_tree_root(&self) -> Result<[u8; 32], AccountSubscriptionError> {
        Ok(*self.tree_root.lock().unwrap())
    }

    async fn get_touched_accounts(
        &self,
        since_block: u64,
    ) -> Result<Vec<AccountAddress>, AccountSubscriptionError> {
        let updates = self.updates.lock().unwrap();
        let mut touched = HashSet::new();
        
        for update in updates.iter() {
            if update.block_height >= since_block {
                touched.insert(update.account.clone());
            }
        }
        
        Ok(touched.into_iter().collect())
    }

    async fn filter_updates(
        &self,
        updates: Vec<AccountUpdate>,
    ) -> Result<Vec<AccountUpdate>, AccountSubscriptionError> {
        let subscribed = self.subscribed_accounts.lock().unwrap();
        let mut filtered = Vec::new();
        
        for update in updates {
            let key = format!("{}:{}", update.account.chain_id, update.account.to_hex());
            if subscribed.contains(&key) {
                filtered.push(update);
            }
        }
        
        Ok(filtered)
    }
}

// Extension methods for the API handlers
impl MemorySubscriptionManager {
    pub async fn get_subscription_stats(&self) -> Result<almanac_chains::subscription::SubscriptionStats, AccountSubscriptionError> {
        let subscribed = self.subscribed_accounts.lock().unwrap();
        let updates = self.updates.lock().unwrap();
        
        // Count accounts by chain
        let mut accounts_by_chain = HashMap::new();
        for key in subscribed.iter() {
            if let Some((chain_id, _)) = key.split_once(':') {
                *accounts_by_chain.entry(chain_id.to_string()).or_insert(0) += 1;
            }
        }
        
        Ok(almanac_chains::subscription::SubscriptionStats {
            total_subscribed_accounts: subscribed.len(),
            accounts_by_chain,
            current_tree_root: *self.tree_root.lock().unwrap(),
            last_update_block: updates.last().map(|u| u.block_height).unwrap_or(0),
            total_updates_processed: updates.len() as u64,
            bloom_filter_utilization: 0.0,
            average_proof_size: 32,
        })
    }
    
    pub async fn get_accounts_touched_since_for_chain(
        &self,
        chain_id: Option<String>,
        since_block: u64,
    ) -> Result<Vec<AccountAddress>, AccountSubscriptionError> {
        let updates = self.updates.lock().unwrap();
        let mut touched = HashSet::new();
        
        for update in updates.iter() {
            // Filter by chain if specified
            if let Some(ref cid) = chain_id {
                if &update.account.chain_id != cid {
                    continue;
                }
            }
            
            // Filter by block number
            if update.block_height >= since_block {
                touched.insert(update.account.clone());
            }
        }
        
        Ok(touched.into_iter().collect())
    }
}