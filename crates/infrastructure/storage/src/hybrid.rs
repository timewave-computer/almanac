// Hybrid storage implementation combining RocksDB and PostgreSQL
//
// This module provides a storage backend that uses RocksDB for hot data
// (recent blocks) and PostgreSQL for cold data (historical blocks).

use crate::{
    Storage, BoxedStorage, Result, Event, EventFilter, BlockStatus,
    backend::{HybridConfig, QueryRouting},
    ValenceAccountInfo, ValenceAccountLibrary, ValenceAccountExecution,
    ValenceLibraryApprovalRequest, ValenceLibraryRemovalRequest,
    ValenceOwnershipUpdateRequest, ValenceAccountState,
    ValenceProcessorInfo, ValenceProcessorMessage,
    ValenceProcessorConfigUpdateRequest, ValenceProcessorMessageStatusRequest,
    ValenceProcessorState, ValenceAuthorizationInfo, ValenceAuthorizationPolicy,
    ValenceAuthorizationPolicyUpdateRequest, ValenceAuthorizationGrant,
    ValenceAuthorizationGrantRevocationRequest, ValenceAuthorizationRequest,
    ValenceAuthorizationRequestDecisionRequest, ValenceLibraryInfo,
    ValenceLibraryVersion, ValenceLibraryVersionUpdateRequest,
    ValenceLibraryUsage, ValenceLibraryApprovalRevocationRequest,
    ValenceLibraryState, ValenceLibraryApproval,
};
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::RwLock;
use std::time::{SystemTime, Duration};
use std::collections::HashMap;
use tracing::{info, warn, debug};

/// Hybrid storage combining hot (RocksDB) and cold (PostgreSQL) storage
pub struct HybridStorage {
    /// Hot storage for recent data
    hot_storage: BoxedStorage,
    /// Cold storage for historical data
    cold_storage: BoxedStorage,
    /// Hybrid configuration
    config: HybridConfig,
    /// Migration state tracking
    migration_state: Arc<RwLock<MigrationState>>,
}

/// State tracking for data migration
#[derive(Debug, Default)]
struct MigrationState {
    /// Last migration timestamp
    last_migration: Option<SystemTime>,
    /// Blocks currently being migrated
    migrating_blocks: HashMap<String, Vec<u64>>,
    /// Migration statistics
    stats: MigrationStats,
}

/// Migration statistics
#[derive(Debug, Default)]
struct MigrationStats {
    /// Total blocks migrated
    blocks_migrated: u64,
    /// Total events migrated
    _events_migrated: u64,
    /// Failed migration attempts
    failed_attempts: u64,
}

impl HybridStorage {
    /// Create a new hybrid storage instance
    pub fn new(
        hot_storage: BoxedStorage,
        cold_storage: BoxedStorage,
        config: HybridConfig,
    ) -> Result<Self> {
        let storage = Self {
            hot_storage,
            cold_storage,
            config,
            migration_state: Arc::new(RwLock::new(MigrationState::default())),
        };
        
        // Start automatic migration if enabled
        if storage.config.auto_migrate {
            let storage_clone = storage.clone();
            tokio::spawn(async move {
                storage_clone.run_migration_loop().await;
            });
        }
        
        Ok(storage)
    }
    
    /// Run the automatic migration loop
    async fn run_migration_loop(&self) {
        let mut interval = tokio::time::interval(Duration::from_secs(self.config.migration_interval));
        
        loop {
            interval.tick().await;
            
            if let Err(e) = self.migrate_cold_data().await {
                warn!("Migration error: {}", e);
                let mut state = self.migration_state.write().await;
                state.stats.failed_attempts += 1;
            }
        }
    }
    
    /// Migrate old data from hot to cold storage
    async fn migrate_cold_data(&self) -> Result<()> {
        info!("Starting cold data migration");
        
        let chains = self.get_all_chains().await?;
        let mut total_migrated = 0;
        
        for chain in chains {
            let latest_block = self.hot_storage.get_latest_block(&chain).await?;
            let cold_threshold = latest_block.saturating_sub(self.config.hot_blocks);
            
            // Find blocks to migrate
            let blocks_to_migrate = self.find_blocks_to_migrate(&chain, cold_threshold).await?;
            
            if blocks_to_migrate.is_empty() {
                continue;
            }
            
            debug!("Migrating {} blocks for chain {}", blocks_to_migrate.len(), chain);
            
            // Mark blocks as migrating
            {
                let mut state = self.migration_state.write().await;
                state.migrating_blocks.insert(chain.clone(), blocks_to_migrate.clone());
            }
            
            // Migrate blocks
            for block in &blocks_to_migrate {
                if let Err(e) = self.migrate_block(&chain, *block).await {
                    warn!("Failed to migrate block {} on chain {}: {}", block, chain, e);
                } else {
                    total_migrated += 1;
                }
            }
            
            // Clear migrating state
            {
                let mut state = self.migration_state.write().await;
                state.migrating_blocks.remove(&chain);
                state.stats.blocks_migrated += total_migrated;
                state.last_migration = Some(SystemTime::now());
            }
        }
        
        info!("Migration completed: {} blocks migrated", total_migrated);
        Ok(())
    }
    
    /// Find blocks that need to be migrated
    async fn find_blocks_to_migrate(&self, _chain: &str, _threshold: u64) -> Result<Vec<u64>> {
        // This is a simplified implementation
        // In practice, you'd query hot storage for old blocks
        Ok(vec![])
    }
    
    /// Migrate a single block from hot to cold storage
    async fn migrate_block(&self, chain: &str, block: u64) -> Result<()> {
        // Get events from hot storage
        let filters = vec![EventFilter {
            chain: Some(chain.to_string()),
            block_range: Some((block, block)),
            ..Default::default()
        }];
        
        let events = self.hot_storage.get_events(filters).await?;
        
        // Store in cold storage
        for event in events {
            self.cold_storage.store_event(event).await?;
        }
        
        // Remove from hot storage (if supported)
        // Note: This would require extending the Storage trait
        
        Ok(())
    }
    
    /// Get all chains in the system
    async fn get_all_chains(&self) -> Result<Vec<String>> {
        // This would need to be implemented based on your data model
        Ok(vec!["ethereum".to_string(), "cosmos".to_string()])
    }
    
    /// Route query based on configuration
    async fn _route_query<F, R>(&self, query_fn: F) -> Result<R>
    where
        F: Fn(&dyn Storage) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<R>> + Send + '_>>,
        R: Send,
    {
        match self.config.query_routing {
            QueryRouting::HotFirst => {
                match query_fn(self.hot_storage.as_ref()).await {
                    Ok(result) => Ok(result),
                    Err(_) => query_fn(self.cold_storage.as_ref()).await,
                }
            }
            QueryRouting::ColdFirst => {
                match query_fn(self.cold_storage.as_ref()).await {
                    Ok(result) => Ok(result),
                    Err(_) => query_fn(self.hot_storage.as_ref()).await,
                }
            }
            QueryRouting::Smart => {
                // Smart routing would analyze the query to determine best backend
                // For now, default to hot first
                self._route_query_hot_first(query_fn).await
            }
            QueryRouting::Parallel => {
                // Parallel would query both and merge results
                // For now, default to hot first
                self._route_query_hot_first(query_fn).await
            }
        }
    }
    
    /// Route query with hot-first strategy
    async fn _route_query_hot_first<F, R>(&self, query_fn: F) -> Result<R>
    where
        F: Fn(&dyn Storage) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<R>> + Send + '_>>,
        R: Send,
    {
        match query_fn(self.hot_storage.as_ref()).await {
            Ok(result) => Ok(result),
            Err(_) => query_fn(self.cold_storage.as_ref()).await,
        }
    }
}

impl Clone for HybridStorage {
    fn clone(&self) -> Self {
        Self {
            hot_storage: self.hot_storage.clone(),
            cold_storage: self.cold_storage.clone(),
            config: self.config.clone(),
            migration_state: self.migration_state.clone(),
        }
    }
}

#[async_trait]
impl Storage for HybridStorage {
    async fn store_event(&self, event: Box<dyn Event>) -> Result<()> {
        // Always store in hot storage first
        self.hot_storage.store_event(event).await
    }
    
    async fn get_events(&self, filters: Vec<EventFilter>) -> Result<Vec<Box<dyn Event>>> {
        // Determine which backend to query based on filters
        let use_hot = filters.iter().any(|f| {
            if let Some((_from, _to)) = f.block_range {
                // Check if querying recent blocks
                true // Simplified logic
            } else {
                true
            }
        });
        
        if use_hot {
            match self.hot_storage.get_events(filters.clone()).await {
                Ok(events) if !events.is_empty() => Ok(events),
                _ => self.cold_storage.get_events(filters).await,
            }
        } else {
            self.cold_storage.get_events(filters).await
        }
    }
    
    async fn get_latest_block(&self, chain: &str) -> Result<u64> {
        // Latest block should be in hot storage
        self.hot_storage.get_latest_block(chain).await
    }
    
    async fn get_latest_block_with_status(&self, chain: &str, status: BlockStatus) -> Result<u64> {
        // Try hot first, then cold
        match self.hot_storage.get_latest_block_with_status(chain, status).await {
            Ok(block) => Ok(block),
            Err(_) => self.cold_storage.get_latest_block_with_status(chain, status).await,
        }
    }
    
    async fn update_block_status(&self, chain: &str, block_number: u64, status: BlockStatus) -> Result<()> {
        // Update in both storages
        let hot_result = self.hot_storage.update_block_status(chain, block_number, status).await;
        let cold_result = self.cold_storage.update_block_status(chain, block_number, status).await;
        
        // Return error if both failed
        hot_result.or(cold_result)
    }
    
    async fn get_events_with_status(&self, filters: Vec<EventFilter>, status: BlockStatus) -> Result<Vec<Box<dyn Event>>> {
        // Try hot first, then cold
        match self.hot_storage.get_events_with_status(filters.clone(), status).await {
            Ok(events) if !events.is_empty() => Ok(events),
            _ => self.cold_storage.get_events_with_status(filters, status).await,
        }
    }
    
    async fn reorg_chain(&self, chain: &str, from_block: u64) -> Result<()> {
        // Reorg affects both storages
        self.hot_storage.reorg_chain(chain, from_block).await?;
        self.cold_storage.reorg_chain(chain, from_block).await
    }
    
    async fn get_latest_block_before(&self, chain: &str, before_block: u64) -> Result<u64> {
        // Try hot first, then cold
        match self.hot_storage.get_latest_block_before(chain, before_block).await {
            Ok(block) => Ok(block),
            Err(_) => self.cold_storage.get_latest_block_before(chain, before_block).await,
        }
    }
    
    // Valence Account methods - delegate to cold storage for consistency
    
    async fn store_valence_account_instantiation(
        &self,
        account_info: ValenceAccountInfo,
        initial_libraries: Vec<ValenceAccountLibrary>,
    ) -> Result<()> {
        self.cold_storage.store_valence_account_instantiation(account_info, initial_libraries).await
    }
    
    async fn store_valence_library_approval(&self, request: ValenceLibraryApprovalRequest) -> Result<()> {
        self.cold_storage.store_valence_library_approval(request).await
    }
    
    async fn store_valence_library_removal(&self, request: ValenceLibraryRemovalRequest) -> Result<()> {
        self.cold_storage.store_valence_library_removal(request).await
    }
    
    async fn store_valence_ownership_update(&self, request: ValenceOwnershipUpdateRequest) -> Result<()> {
        self.cold_storage.store_valence_ownership_update(request).await
    }
    
    async fn store_valence_execution(&self, execution_info: ValenceAccountExecution) -> Result<()> {
        self.cold_storage.store_valence_execution(execution_info).await
    }
    
    async fn get_valence_account_state(&self, account_id: &str) -> Result<Option<ValenceAccountState>> {
        self.cold_storage.get_valence_account_state(account_id).await
    }
    
    async fn set_valence_account_state(&self, account_id: &str, state: &ValenceAccountState) -> Result<()> {
        self.cold_storage.set_valence_account_state(account_id, state).await
    }
    
    async fn delete_valence_account_state(&self, account_id: &str) -> Result<()> {
        self.cold_storage.delete_valence_account_state(account_id).await
    }
    
    async fn set_historical_valence_account_state(
        &self,
        account_id: &str,
        block_number: u64,
        state: &ValenceAccountState,
    ) -> Result<()> {
        self.cold_storage.set_historical_valence_account_state(account_id, block_number, state).await
    }
    
    async fn get_historical_valence_account_state(
        &self,
        account_id: &str,
        block_number: u64,
    ) -> Result<Option<ValenceAccountState>> {
        self.cold_storage.get_historical_valence_account_state(account_id, block_number).await
    }
    
    async fn delete_historical_valence_account_state(
        &self,
        account_id: &str,
        block_number: u64,
    ) -> Result<()> {
        self.cold_storage.delete_historical_valence_account_state(account_id, block_number).await
    }
    
    async fn set_latest_historical_valence_block(
        &self,
        account_id: &str,
        block_number: u64,
    ) -> Result<()> {
        self.cold_storage.set_latest_historical_valence_block(account_id, block_number).await
    }
    
    async fn get_latest_historical_valence_block(&self, account_id: &str) -> Result<Option<u64>> {
        self.cold_storage.get_latest_historical_valence_block(account_id).await
    }
    
    async fn delete_latest_historical_valence_block(&self, account_id: &str) -> Result<()> {
        self.cold_storage.delete_latest_historical_valence_block(account_id).await
    }
    
    // Valence Processor methods
    
    async fn store_valence_processor_instantiation(&self, processor_info: ValenceProcessorInfo) -> Result<()> {
        self.cold_storage.store_valence_processor_instantiation(processor_info).await
    }
    
    async fn store_valence_processor_config_update(&self, request: ValenceProcessorConfigUpdateRequest) -> Result<()> {
        self.cold_storage.store_valence_processor_config_update(request).await
    }
    
    async fn store_valence_processor_message(&self, message: ValenceProcessorMessage) -> Result<()> {
        self.cold_storage.store_valence_processor_message(message).await
    }
    
    async fn update_valence_processor_message_status(&self, request: ValenceProcessorMessageStatusRequest) -> Result<()> {
        self.cold_storage.update_valence_processor_message_status(request).await
    }
    
    async fn get_valence_processor_state(&self, processor_id: &str) -> Result<Option<ValenceProcessorState>> {
        self.cold_storage.get_valence_processor_state(processor_id).await
    }
    
    async fn set_valence_processor_state(&self, processor_id: &str, state: &ValenceProcessorState) -> Result<()> {
        self.cold_storage.set_valence_processor_state(processor_id, state).await
    }
    
    async fn set_historical_valence_processor_state(
        &self,
        processor_id: &str,
        block_number: u64,
        state: &ValenceProcessorState,
    ) -> Result<()> {
        self.cold_storage.set_historical_valence_processor_state(processor_id, block_number, state).await
    }
    
    async fn get_historical_valence_processor_state(
        &self,
        processor_id: &str,
        block_number: u64,
    ) -> Result<Option<ValenceProcessorState>> {
        self.cold_storage.get_historical_valence_processor_state(processor_id, block_number).await
    }
    
    // Valence Authorization methods
    
    async fn store_valence_authorization_instantiation(
        &self,
        auth_info: ValenceAuthorizationInfo,
        initial_policy: Option<ValenceAuthorizationPolicy>,
    ) -> Result<()> {
        self.cold_storage.store_valence_authorization_instantiation(auth_info, initial_policy).await
    }
    
    async fn store_valence_authorization_policy(&self, policy: ValenceAuthorizationPolicy) -> Result<()> {
        self.cold_storage.store_valence_authorization_policy(policy).await
    }
    
    async fn update_active_authorization_policy(&self, request: ValenceAuthorizationPolicyUpdateRequest) -> Result<()> {
        self.cold_storage.update_active_authorization_policy(request).await
    }
    
    async fn store_valence_authorization_grant(&self, grant: ValenceAuthorizationGrant) -> Result<()> {
        self.cold_storage.store_valence_authorization_grant(grant).await
    }
    
    async fn revoke_valence_authorization_grant(&self, request: ValenceAuthorizationGrantRevocationRequest) -> Result<()> {
        self.cold_storage.revoke_valence_authorization_grant(request).await
    }
    
    async fn store_valence_authorization_request(&self, request: ValenceAuthorizationRequest) -> Result<()> {
        self.cold_storage.store_valence_authorization_request(request).await
    }
    
    async fn update_valence_authorization_request_decision(
        &self,
        request: ValenceAuthorizationRequestDecisionRequest,
    ) -> Result<()> {
        self.cold_storage.update_valence_authorization_request_decision(request).await
    }
    
    // Valence Library methods
    
    async fn store_valence_library_instantiation(
        &self,
        library_info: ValenceLibraryInfo,
        initial_version: Option<ValenceLibraryVersion>,
    ) -> Result<()> {
        self.cold_storage.store_valence_library_instantiation(library_info, initial_version).await
    }
    
    async fn store_valence_library_version(&self, version: ValenceLibraryVersion) -> Result<()> {
        self.cold_storage.store_valence_library_version(version).await
    }
    
    async fn update_active_library_version(&self, request: ValenceLibraryVersionUpdateRequest) -> Result<()> {
        self.cold_storage.update_active_library_version(request).await
    }
    
    async fn store_valence_library_usage(&self, usage: ValenceLibraryUsage) -> Result<()> {
        self.cold_storage.store_valence_library_usage(usage).await
    }
    
    async fn revoke_valence_library_approval(&self, request: ValenceLibraryApprovalRevocationRequest) -> Result<()> {
        self.cold_storage.revoke_valence_library_approval(request).await
    }
    
    async fn get_valence_library_state(&self, library_id: &str) -> Result<Option<ValenceLibraryState>> {
        self.cold_storage.get_valence_library_state(library_id).await
    }
    
    async fn set_valence_library_state(&self, library_id: &str, state: &ValenceLibraryState) -> Result<()> {
        self.cold_storage.set_valence_library_state(library_id, state).await
    }
    
    async fn get_valence_library_versions(&self, library_id: &str) -> Result<Vec<ValenceLibraryVersion>> {
        self.cold_storage.get_valence_library_versions(library_id).await
    }
    
    async fn get_valence_library_approvals(&self, library_id: &str) -> Result<Vec<ValenceLibraryApproval>> {
        self.cold_storage.get_valence_library_approvals(library_id).await
    }
    
    async fn get_valence_libraries_for_account(&self, account_id: &str) -> Result<Vec<ValenceLibraryApproval>> {
        self.cold_storage.get_valence_libraries_for_account(account_id).await
    }
    
    async fn get_valence_library_usage_history(
        &self,
        library_id: &str,
        limit: Option<usize>,
        offset: Option<usize>,
    ) -> Result<Vec<ValenceLibraryUsage>> {
        self.cold_storage.get_valence_library_usage_history(library_id, limit, offset).await
    }
    
    // Processor state methods
    
    async fn set_processor_state(&self, chain: &str, block_number: u64, state: &str) -> Result<()> {
        // Store in hot for recent blocks
        self.hot_storage.set_processor_state(chain, block_number, state).await
    }
    
    async fn get_processor_state(&self, chain: &str, block_number: u64) -> Result<Option<String>> {
        // Try hot first, then cold
        match self.hot_storage.get_processor_state(chain, block_number).await {
            Ok(Some(state)) => Ok(Some(state)),
            _ => self.cold_storage.get_processor_state(chain, block_number).await,
        }
    }
    
    async fn set_historical_processor_state(&self, chain: &str, block_number: u64, state: &str) -> Result<()> {
        // Historical state goes to cold storage
        self.cold_storage.set_historical_processor_state(chain, block_number, state).await
    }
    
    async fn get_historical_processor_state(&self, chain: &str, block_number: u64) -> Result<Option<String>> {
        self.cold_storage.get_historical_processor_state(chain, block_number).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::MemoryStorage;
    
    #[tokio::test]
    async fn test_hybrid_storage_creation() {
        let hot = Arc::new(MemoryStorage::new());
        let cold = Arc::new(MemoryStorage::new());
        let config = HybridConfig::default();
        
        let hybrid = HybridStorage::new(hot, cold, config).unwrap();
        
        // Test basic functionality
        let latest = hybrid.get_latest_block("ethereum").await.unwrap();
        assert_eq!(latest, 0);
    }
}