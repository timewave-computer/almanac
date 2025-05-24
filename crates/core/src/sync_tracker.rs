/// Chain synchronization status tracking and monitoring
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{SystemTime, Duration};
use serde::{Deserialize, Serialize};
use async_trait::async_trait;
use tokio::sync::RwLock;

use crate::{Result, Error};

/// Synchronization status for a blockchain
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SyncStatus {
    /// Chain is not being synchronized
    NotSyncing,
    
    /// Chain is being synchronized
    Syncing,
    
    /// Chain is synchronized and up to date
    Synced,
    
    /// Chain synchronization has failed
    Failed,
    
    /// Chain is paused (intentionally stopped)
    Paused,
    
    /// Chain is in catchup mode (behind but making progress)
    CatchingUp,
    
    /// Chain has stalled (no progress for extended period)
    Stalled,
}

/// Synchronization configuration for a chain
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainSyncConfig {
    /// Chain name/identifier
    pub chain_name: String,
    
    /// Chain ID
    pub chain_id: u64,
    
    /// RPC endpoint URL
    pub rpc_endpoint: String,
    
    /// Whether synchronization is enabled
    pub enabled: bool,
    
    /// Starting block number for sync
    pub start_block: Option<u64>,
    
    /// Target block number (None for continuous sync)
    pub target_block: Option<u64>,
    
    /// Maximum blocks to sync in a single batch
    pub batch_size: u64,
    
    /// Delay between sync batches (milliseconds)
    pub batch_delay: u64,
    
    /// Maximum time to wait for block (seconds)
    pub block_timeout: u64,
    
    /// Number of confirmations required
    pub confirmations: u64,
    
    /// Whether to track mempool events
    pub track_mempool: bool,
    
    /// Sync priority (higher number = higher priority)
    pub priority: u32,
    
    /// Maximum allowed lag behind head (blocks)
    pub max_lag_blocks: u64,
    
    /// Time threshold for considering sync stalled (seconds)
    pub stall_threshold: u64,
}

impl Default for ChainSyncConfig {
    fn default() -> Self {
        Self {
            chain_name: "unknown".to_string(),
            chain_id: 0,
            rpc_endpoint: "".to_string(),
            enabled: true,
            start_block: None,
            target_block: None,
            batch_size: 100,
            batch_delay: 1000, // 1 second
            block_timeout: 30,
            confirmations: 12,
            track_mempool: false,
            priority: 1,
            max_lag_blocks: 100,
            stall_threshold: 300, // 5 minutes
        }
    }
}

/// Detailed synchronization state information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncState {
    /// Chain identifier
    pub chain: String,
    
    /// Current sync status
    pub status: SyncStatus,
    
    /// Current block number being processed
    pub current_block: u64,
    
    /// Latest block number on the chain
    pub head_block: u64,
    
    /// Starting block for this sync session
    pub start_block: u64,
    
    /// Target block (if specified)
    pub target_block: Option<u64>,
    
    /// Number of blocks processed in this session
    pub blocks_processed: u64,
    
    /// Number of events extracted
    pub events_extracted: u64,
    
    /// Last successful sync timestamp
    pub last_sync_time: SystemTime,
    
    /// Time when sync started
    pub sync_start_time: SystemTime,
    
    /// Estimated time to completion
    pub estimated_completion: Option<SystemTime>,
    
    /// Current sync speed (blocks per second)
    pub sync_speed: f64,
    
    /// Average sync speed over the session
    pub avg_sync_speed: f64,
    
    /// Last error message (if any)
    pub last_error: Option<String>,
    
    /// Number of consecutive errors
    pub error_count: u32,
    
    /// Synchronization metrics
    pub metrics: SyncMetrics,
    
    /// Additional metadata
    pub metadata: HashMap<String, String>,
}

/// Synchronization performance metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncMetrics {
    /// Total time spent syncing
    pub total_sync_duration: Duration,
    
    /// Total blocks processed across all sessions
    pub total_blocks_processed: u64,
    
    /// Total events extracted across all sessions
    pub total_events_extracted: u64,
    
    /// Number of sync sessions
    pub sync_sessions: u32,
    
    /// Number of failed sync attempts
    pub failed_attempts: u32,
    
    /// Average blocks per batch
    pub avg_blocks_per_batch: f64,
    
    /// Peak sync speed achieved
    pub peak_sync_speed: f64,
    
    /// Network latency (milliseconds)
    pub network_latency: f64,
    
    /// RPC success rate (0.0 to 1.0)
    pub rpc_success_rate: f64,
    
    /// Memory usage (bytes)
    pub memory_usage: u64,
    
    /// CPU usage percentage
    pub cpu_usage: f64,
}

impl Default for SyncMetrics {
    fn default() -> Self {
        Self {
            total_sync_duration: Duration::from_secs(0),
            total_blocks_processed: 0,
            total_events_extracted: 0,
            sync_sessions: 0,
            failed_attempts: 0,
            avg_blocks_per_batch: 0.0,
            peak_sync_speed: 0.0,
            network_latency: 0.0,
            rpc_success_rate: 1.0,
            memory_usage: 0,
            cpu_usage: 0.0,
        }
    }
}

/// Synchronization event for notifications
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncEvent {
    /// Chain identifier
    pub chain: String,
    
    /// Event type
    pub event_type: SyncEventType,
    
    /// Event timestamp
    pub timestamp: SystemTime,
    
    /// Current sync state
    pub state: SyncState,
    
    /// Event message
    pub message: String,
    
    /// Event metadata
    pub metadata: HashMap<String, String>,
}

/// Types of synchronization events
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SyncEventType {
    /// Sync started
    SyncStarted,
    
    /// Sync completed successfully
    SyncCompleted,
    
    /// Sync failed
    SyncFailed,
    
    /// Sync paused
    SyncPaused,
    
    /// Sync resumed
    SyncResumed,
    
    /// Progress update
    ProgressUpdate,
    
    /// Block processed
    BlockProcessed,
    
    /// Batch completed
    BatchCompleted,
    
    /// Error occurred
    ErrorOccurred,
    
    /// Sync stalled
    SyncStalled,
    
    /// Catching up to head
    CatchingUp,
    
    /// Reached target block
    TargetReached,
}

/// Chain synchronization tracker trait
#[async_trait]
pub trait SyncTracker: Send + Sync {
    /// Start synchronization for a chain
    async fn start_sync(&self, chain: &str, config: ChainSyncConfig) -> Result<()>;
    
    /// Stop synchronization for a chain
    async fn stop_sync(&self, chain: &str) -> Result<()>;
    
    /// Pause synchronization for a chain
    async fn pause_sync(&self, chain: &str) -> Result<()>;
    
    /// Resume synchronization for a chain
    async fn resume_sync(&self, chain: &str) -> Result<()>;
    
    /// Get current sync state for a chain
    async fn get_sync_state(&self, chain: &str) -> Result<Option<SyncState>>;
    
    /// Get sync states for all chains
    async fn get_all_sync_states(&self) -> Result<HashMap<String, SyncState>>;
    
    /// Update sync progress
    async fn update_progress(
        &self,
        chain: &str,
        current_block: u64,
        head_block: u64,
        events_count: u64,
    ) -> Result<()>;
    
    /// Record sync error
    async fn record_error(&self, chain: &str, error: &str) -> Result<()>;
    
    /// Check if chain is healthy (not stalled/failed)
    async fn is_chain_healthy(&self, chain: &str) -> Result<bool>;
    
    /// Get synchronization metrics
    async fn get_metrics(&self, chain: &str) -> Result<Option<SyncMetrics>>;
    
    /// Reset sync state for a chain
    async fn reset_sync(&self, chain: &str) -> Result<()>;
}

/// Default synchronization tracker implementation
pub struct DefaultSyncTracker {
    /// Sync states for each chain
    states: Arc<RwLock<HashMap<String, SyncState>>>,
    
    /// Configuration for each chain
    configs: Arc<RwLock<HashMap<String, ChainSyncConfig>>>,
    
    /// Event listeners
    event_listeners: Arc<RwLock<Vec<Box<dyn SyncEventListener>>>>,
}

/// Synchronization event listener trait
#[async_trait]
pub trait SyncEventListener: Send + Sync {
    /// Handle sync event
    async fn on_sync_event(&self, event: SyncEvent) -> Result<()>;
}

impl DefaultSyncTracker {
    /// Create a new sync tracker
    pub fn new() -> Self {
        Self {
            states: Arc::new(RwLock::new(HashMap::new())),
            configs: Arc::new(RwLock::new(HashMap::new())),
            event_listeners: Arc::new(RwLock::new(Vec::new())),
        }
    }
    
    /// Add an event listener
    pub async fn add_listener(&self, listener: Box<dyn SyncEventListener>) {
        let mut listeners = self.event_listeners.write().await;
        listeners.push(listener);
    }
    
    /// Emit sync event to all listeners
    async fn emit_event(&self, event: SyncEvent) -> Result<()> {
        let listeners = self.event_listeners.read().await;
        for listener in listeners.iter() {
            if let Err(e) = listener.on_sync_event(event.clone()).await {
                tracing::warn!("Sync event listener error: {}", e);
            }
        }
        Ok(())
    }
    
    /// Create initial sync state
    fn create_initial_state(chain: &str, config: &ChainSyncConfig) -> SyncState {
        let now = SystemTime::now();
        let start_block = config.start_block.unwrap_or(0);
        
        SyncState {
            chain: chain.to_string(),
            status: SyncStatus::NotSyncing,
            current_block: start_block,
            head_block: 0,
            start_block,
            target_block: config.target_block,
            blocks_processed: 0,
            events_extracted: 0,
            last_sync_time: now,
            sync_start_time: now,
            estimated_completion: None,
            sync_speed: 0.0,
            avg_sync_speed: 0.0,
            last_error: None,
            error_count: 0,
            metrics: SyncMetrics::default(),
            metadata: HashMap::new(),
        }
    }
    
    /// Update sync state status
    async fn update_status(&self, chain: &str, status: SyncStatus) -> Result<()> {
        let mut states = self.states.write().await;
        if let Some(state) = states.get_mut(chain) {
            let old_status = state.status.clone();
            state.status = status.clone();
            
            // Emit event if status changed
            if old_status != status {
                let event_type = match status {
                    SyncStatus::Syncing => SyncEventType::SyncStarted,
                    SyncStatus::Synced => SyncEventType::SyncCompleted,
                    SyncStatus::Failed => SyncEventType::SyncFailed,
                    SyncStatus::Paused => SyncEventType::SyncPaused,
                    SyncStatus::CatchingUp => SyncEventType::CatchingUp,
                    SyncStatus::Stalled => SyncEventType::SyncStalled,
                    _ => SyncEventType::ProgressUpdate,
                };
                
                let event = SyncEvent {
                    chain: chain.to_string(),
                    event_type,
                    timestamp: SystemTime::now(),
                    state: state.clone(),
                    message: format!("Status changed from {:?} to {:?}", old_status, status),
                    metadata: HashMap::new(),
                };
                
                drop(states); // Release lock before emitting event
                self.emit_event(event).await?;
            }
        }
        
        Ok(())
    }
    
    /// Calculate estimated completion time
    fn calculate_estimated_completion(&self, state: &SyncState) -> Option<SystemTime> {
        if state.sync_speed > 0.0 {
            let remaining_blocks = if let Some(target) = state.target_block {
                target.saturating_sub(state.current_block)
            } else {
                state.head_block.saturating_sub(state.current_block)
            };
            
            if remaining_blocks > 0 {
                let estimated_seconds = remaining_blocks as f64 / state.sync_speed;
                return SystemTime::now().checked_add(Duration::from_secs_f64(estimated_seconds));
            }
        }
        None
    }
    
    /// Determine sync status based on current state
    fn determine_status(&self, state: &SyncState, config: &ChainSyncConfig) -> SyncStatus {
        let now = SystemTime::now();
        
        // Check if stalled
        if let Ok(duration) = now.duration_since(state.last_sync_time) {
            if duration.as_secs() > config.stall_threshold {
                return SyncStatus::Stalled;
            }
        }
        
        // Check if failed due to too many errors
        if state.error_count > 10 {
            return SyncStatus::Failed;
        }
        
        // Check if reached target
        if let Some(target) = state.target_block {
            if state.current_block >= target {
                return SyncStatus::Synced;
            }
        }
        
        // Check if caught up to head
        let lag = state.head_block.saturating_sub(state.current_block);
        if lag <= config.confirmations {
            return SyncStatus::Synced;
        }
        
        // Check if catching up
        if lag > config.max_lag_blocks {
            return SyncStatus::CatchingUp;
        }
        
        // Default to syncing if making progress
        if state.blocks_processed > 0 {
            SyncStatus::Syncing
        } else {
            SyncStatus::NotSyncing
        }
    }
}

#[async_trait]
impl SyncTracker for DefaultSyncTracker {
    async fn start_sync(&self, chain: &str, config: ChainSyncConfig) -> Result<()> {
        let mut states = self.states.write().await;
        let mut configs = self.configs.write().await;
        
        // Store configuration
        configs.insert(chain.to_string(), config.clone());
        
        // Create or update sync state
        let state = if let Some(existing_state) = states.get_mut(chain) {
            existing_state.status = SyncStatus::Syncing;
            existing_state.sync_start_time = SystemTime::now();
            existing_state.error_count = 0;
            existing_state.last_error = None;
            existing_state.clone()
        } else {
            let mut new_state = Self::create_initial_state(chain, &config);
            new_state.status = SyncStatus::Syncing;
            states.insert(chain.to_string(), new_state.clone());
            new_state
        };
        
        drop(states);
        drop(configs);
        
        // Emit start event
        let event = SyncEvent {
            chain: chain.to_string(),
            event_type: SyncEventType::SyncStarted,
            timestamp: SystemTime::now(),
            state,
            message: format!("Started synchronization for chain {}", chain),
            metadata: HashMap::new(),
        };
        
        self.emit_event(event).await?;
        
        tracing::info!("Started sync for chain: {}", chain);
        Ok(())
    }
    
    async fn stop_sync(&self, chain: &str) -> Result<()> {
        self.update_status(chain, SyncStatus::NotSyncing).await?;
        tracing::info!("Stopped sync for chain: {}", chain);
        Ok(())
    }
    
    async fn pause_sync(&self, chain: &str) -> Result<()> {
        self.update_status(chain, SyncStatus::Paused).await?;
        tracing::info!("Paused sync for chain: {}", chain);
        Ok(())
    }
    
    async fn resume_sync(&self, chain: &str) -> Result<()> {
        let mut states = self.states.write().await;
        if let Some(state) = states.get_mut(chain) {
            if state.status == SyncStatus::Paused {
                state.status = SyncStatus::Syncing;
                
                let event = SyncEvent {
                    chain: chain.to_string(),
                    event_type: SyncEventType::SyncResumed,
                    timestamp: SystemTime::now(),
                    state: state.clone(),
                    message: format!("Resumed synchronization for chain {}", chain),
                    metadata: HashMap::new(),
                };
                
                drop(states);
                self.emit_event(event).await?;
            }
        }
        
        tracing::info!("Resumed sync for chain: {}", chain);
        Ok(())
    }
    
    async fn get_sync_state(&self, chain: &str) -> Result<Option<SyncState>> {
        let states = self.states.read().await;
        Ok(states.get(chain).cloned())
    }
    
    async fn get_all_sync_states(&self) -> Result<HashMap<String, SyncState>> {
        let states = self.states.read().await;
        Ok(states.clone())
    }
    
    async fn update_progress(
        &self,
        chain: &str,
        current_block: u64,
        head_block: u64,
        events_count: u64,
    ) -> Result<()> {
        let configs = self.configs.read().await;
        let config = configs.get(chain).cloned();
        drop(configs);
        
        let mut states = self.states.write().await;
        
        if let Some(state) = states.get_mut(chain) {
            let now = SystemTime::now();
            let previous_block = state.current_block;
            
            // Update basic stats
            state.current_block = current_block;
            state.head_block = head_block;
            state.events_extracted += events_count;
            
            // Calculate blocks processed in this update
            if current_block > previous_block {
                let blocks_processed = current_block - previous_block;
                state.blocks_processed += blocks_processed;
                
                // Calculate sync speed
                if let Ok(duration) = now.duration_since(state.last_sync_time) {
                    if duration.as_secs_f64() > 0.0 {
                        state.sync_speed = blocks_processed as f64 / duration.as_secs_f64();
                        
                        // Update average sync speed
                        if let Ok(total_duration) = now.duration_since(state.sync_start_time) {
                            if total_duration.as_secs_f64() > 0.0 {
                                state.avg_sync_speed = state.blocks_processed as f64 / total_duration.as_secs_f64();
                            }
                        }
                        
                        // Update peak sync speed
                        if state.sync_speed > state.metrics.peak_sync_speed {
                            state.metrics.peak_sync_speed = state.sync_speed;
                        }
                    }
                }
            }
            
            state.last_sync_time = now;
            
            // Calculate estimated completion
            if let Some(config) = &config {
                state.estimated_completion = self.calculate_estimated_completion(state);
                
                // Update status based on progress
                let new_status = self.determine_status(state, config);
                if new_status != state.status {
                    state.status = new_status;
                }
            }
            
            // Update metrics
            if let Ok(total_duration) = now.duration_since(state.sync_start_time) {
                state.metrics.total_sync_duration = total_duration;
            }
            state.metrics.total_blocks_processed = state.blocks_processed;
            state.metrics.total_events_extracted = state.events_extracted;
            
            // Clear error count on successful progress
            if current_block > previous_block {
                state.error_count = 0;
                state.last_error = None;
            }
            
            let state_clone = state.clone();
            drop(states);
            
            // Emit progress event
            let event = SyncEvent {
                chain: chain.to_string(),
                event_type: SyncEventType::ProgressUpdate,
                timestamp: now,
                state: state_clone,
                message: format!(
                    "Progress: block {}/{} ({} events)",
                    current_block, head_block, events_count
                ),
                metadata: HashMap::new(),
            };
            
            self.emit_event(event).await?;
        }
        
        Ok(())
    }
    
    async fn record_error(&self, chain: &str, error: &str) -> Result<()> {
        let mut states = self.states.write().await;
        
        if let Some(state) = states.get_mut(chain) {
            state.error_count += 1;
            state.last_error = Some(error.to_string());
            state.metrics.failed_attempts += 1;
            
            // Update status if too many errors
            if state.error_count > 10 {
                state.status = SyncStatus::Failed;
            }
            
            let state_clone = state.clone();
            drop(states);
            
            // Emit error event
            let event = SyncEvent {
                chain: chain.to_string(),
                event_type: SyncEventType::ErrorOccurred,
                timestamp: SystemTime::now(),
                state: state_clone,
                message: format!("Error occurred: {}", error),
                metadata: HashMap::new(),
            };
            
            self.emit_event(event).await?;
        }
        
        tracing::warn!("Sync error for chain {}: {}", chain, error);
        Ok(())
    }
    
    async fn is_chain_healthy(&self, chain: &str) -> Result<bool> {
        let states = self.states.read().await;
        if let Some(state) = states.get(chain) {
            Ok(matches!(state.status, SyncStatus::Syncing | SyncStatus::Synced | SyncStatus::CatchingUp))
        } else {
            Ok(false)
        }
    }
    
    async fn get_metrics(&self, chain: &str) -> Result<Option<SyncMetrics>> {
        let states = self.states.read().await;
        Ok(states.get(chain).map(|state| state.metrics.clone()))
    }
    
    async fn reset_sync(&self, chain: &str) -> Result<()> {
        let configs = self.configs.read().await;
        let config = configs.get(chain).cloned();
        drop(configs);
        
        if let Some(config) = config {
            let mut states = self.states.write().await;
            let new_state = Self::create_initial_state(chain, &config);
            states.insert(chain.to_string(), new_state);
        }
        
        tracing::info!("Reset sync state for chain: {}", chain);
        Ok(())
    }
}

/// Sync tracker manager for handling multiple chains
pub struct SyncTrackerManager {
    tracker: Arc<dyn SyncTracker>,
    enabled_chains: Arc<RwLock<Vec<String>>>,
}

impl SyncTrackerManager {
    /// Create a new sync tracker manager
    pub fn new(tracker: Arc<dyn SyncTracker>) -> Self {
        Self {
            tracker,
            enabled_chains: Arc::new(RwLock::new(Vec::new())),
        }
    }
    
    /// Add a chain to track
    pub async fn add_chain(&self, chain: String, config: ChainSyncConfig) -> Result<()> {
        let mut chains = self.enabled_chains.write().await;
        if !chains.contains(&chain) {
            chains.push(chain.clone());
        }
        drop(chains);
        
        self.tracker.start_sync(&chain, config).await
    }
    
    /// Remove a chain from tracking
    pub async fn remove_chain(&self, chain: &str) -> Result<()> {
        let mut chains = self.enabled_chains.write().await;
        chains.retain(|c| c != chain);
        drop(chains);
        
        self.tracker.stop_sync(chain).await
    }
    
    /// Get health status of all chains
    pub async fn get_health_status(&self) -> Result<HashMap<String, bool>> {
        let chains = self.enabled_chains.read().await;
        let mut health_status = HashMap::new();
        
        for chain in chains.iter() {
            let is_healthy = self.tracker.is_chain_healthy(chain).await?;
            health_status.insert(chain.clone(), is_healthy);
        }
        
        Ok(health_status)
    }
    
    /// Get summary of all chain states
    pub async fn get_summary(&self) -> Result<HashMap<String, SyncStatus>> {
        let states = self.tracker.get_all_sync_states().await?;
        Ok(states.into_iter().map(|(chain, state)| (chain, state.status)).collect())
    }
}

/// Predefined configurations for common chains
pub struct PredefinedSyncConfigs;

impl PredefinedSyncConfigs {
    /// Get Ethereum sync configuration
    pub fn ethereum() -> ChainSyncConfig {
        ChainSyncConfig {
            chain_name: "ethereum".to_string(),
            chain_id: 1,
            rpc_endpoint: "https://eth.llamarpc.com".to_string(),
            enabled: true,
            start_block: Some(18000000), // Relatively recent block
            target_block: None,
            batch_size: 100,
            batch_delay: 1000,
            block_timeout: 30,
            confirmations: 12,
            track_mempool: false,
            priority: 1,
            max_lag_blocks: 50,
            stall_threshold: 300,
        }
    }
    
    /// Get Polygon sync configuration
    pub fn polygon() -> ChainSyncConfig {
        ChainSyncConfig {
            chain_name: "polygon".to_string(),
            chain_id: 137,
            rpc_endpoint: "https://polygon.llamarpc.com".to_string(),
            enabled: true,
            start_block: Some(45000000),
            target_block: None,
            batch_size: 500, // Faster blocks, larger batches
            batch_delay: 500,
            block_timeout: 15,
            confirmations: 128,
            track_mempool: false,
            priority: 2,
            max_lag_blocks: 200,
            stall_threshold: 180,
        }
    }
    
    /// Get Cosmos sync configuration
    pub fn cosmos() -> ChainSyncConfig {
        ChainSyncConfig {
            chain_name: "cosmos".to_string(),
            chain_id: 1,
            rpc_endpoint: "https://cosmos-rpc.polkachu.com".to_string(),
            enabled: true,
            start_block: Some(15000000),
            target_block: None,
            batch_size: 200,
            batch_delay: 800,
            block_timeout: 20,
            confirmations: 1,
            track_mempool: false,
            priority: 3,
            max_lag_blocks: 100,
            stall_threshold: 240,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    
    // Mock event listener for testing
    struct TestEventListener {
        event_count: Arc<AtomicUsize>,
    }
    
    impl TestEventListener {
        fn new() -> (Self, Arc<AtomicUsize>) {
            let counter = Arc::new(AtomicUsize::new(0));
            (Self { event_count: counter.clone() }, counter)
        }
    }
    
    #[async_trait]
    impl SyncEventListener for TestEventListener {
        async fn on_sync_event(&self, _event: SyncEvent) -> Result<()> {
            self.event_count.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }
    }
    
    #[tokio::test]
    async fn test_sync_tracker_creation() {
        let tracker = DefaultSyncTracker::new();
        
        // Should start with no states
        let states = tracker.get_all_sync_states().await.unwrap();
        assert!(states.is_empty());
    }
    
    #[tokio::test]
    async fn test_start_and_stop_sync() {
        let tracker = DefaultSyncTracker::new();
        let config = PredefinedSyncConfigs::ethereum();
        
        // Start sync
        tracker.start_sync("ethereum", config).await.unwrap();
        
        // Check state
        let state = tracker.get_sync_state("ethereum").await.unwrap();
        assert!(state.is_some());
        assert_eq!(state.unwrap().status, SyncStatus::Syncing);
        
        // Stop sync
        tracker.stop_sync("ethereum").await.unwrap();
        
        // Check state
        let state = tracker.get_sync_state("ethereum").await.unwrap();
        assert!(state.is_some());
        assert_eq!(state.unwrap().status, SyncStatus::NotSyncing);
    }
    
    #[tokio::test]
    async fn test_progress_tracking() {
        let tracker = DefaultSyncTracker::new();
        let config = PredefinedSyncConfigs::ethereum();
        
        tracker.start_sync("ethereum", config).await.unwrap();
        
        // Update progress starting from a different block
        tracker.update_progress("ethereum", 18000100, 18001000, 50).await.unwrap();
        
        let state = tracker.get_sync_state("ethereum").await.unwrap().unwrap();
        assert_eq!(state.current_block, 18000100);
        assert_eq!(state.head_block, 18001000);
        assert_eq!(state.events_extracted, 50);
        assert!(state.blocks_processed > 0);
    }
    
    #[tokio::test]
    async fn test_error_handling() {
        let tracker = DefaultSyncTracker::new();
        let config = PredefinedSyncConfigs::ethereum();
        
        tracker.start_sync("ethereum", config).await.unwrap();
        
        // Record error
        tracker.record_error("ethereum", "Connection failed").await.unwrap();
        
        let state = tracker.get_sync_state("ethereum").await.unwrap().unwrap();
        assert_eq!(state.error_count, 1);
        assert!(state.last_error.is_some());
        assert!(state.last_error.unwrap().contains("Connection failed"));
    }
    
    #[tokio::test]
    async fn test_pause_and_resume() {
        let tracker = DefaultSyncTracker::new();
        let config = PredefinedSyncConfigs::ethereum();
        
        tracker.start_sync("ethereum", config).await.unwrap();
        
        // Pause sync
        tracker.pause_sync("ethereum").await.unwrap();
        let state = tracker.get_sync_state("ethereum").await.unwrap().unwrap();
        assert_eq!(state.status, SyncStatus::Paused);
        
        // Resume sync
        tracker.resume_sync("ethereum").await.unwrap();
        let state = tracker.get_sync_state("ethereum").await.unwrap().unwrap();
        assert_eq!(state.status, SyncStatus::Syncing);
    }
    
    #[tokio::test]
    async fn test_health_check() {
        let tracker = DefaultSyncTracker::new();
        let config = PredefinedSyncConfigs::ethereum();
        
        // Should be unhealthy before starting
        let healthy = tracker.is_chain_healthy("ethereum").await.unwrap();
        assert!(!healthy);
        
        // Start sync
        tracker.start_sync("ethereum", config).await.unwrap();
        
        // Should be healthy when syncing
        let healthy = tracker.is_chain_healthy("ethereum").await.unwrap();
        assert!(healthy);
        
        // Record many errors to make it unhealthy
        for i in 0..11 {
            tracker.record_error("ethereum", &format!("Error {}", i)).await.unwrap();
        }
        
        let healthy = tracker.is_chain_healthy("ethereum").await.unwrap();
        assert!(!healthy);
    }
    
    #[tokio::test]
    async fn test_event_listeners() {
        let tracker = DefaultSyncTracker::new();
        let (listener, counter) = TestEventListener::new();
        
        tracker.add_listener(Box::new(listener)).await;
        
        let config = PredefinedSyncConfigs::ethereum();
        tracker.start_sync("ethereum", config).await.unwrap();
        
        // Should have received at least one event
        assert!(counter.load(Ordering::SeqCst) > 0);
    }
    
    #[tokio::test]
    async fn test_sync_manager() {
        let tracker = Arc::new(DefaultSyncTracker::new());
        let manager = SyncTrackerManager::new(tracker);
        
        // Add chain
        let config = PredefinedSyncConfigs::ethereum();
        manager.add_chain("ethereum".to_string(), config).await.unwrap();
        
        // Check health
        let health = manager.get_health_status().await.unwrap();
        assert_eq!(health.get("ethereum"), Some(&true));
        
        // Get summary
        let summary = manager.get_summary().await.unwrap();
        assert_eq!(summary.get("ethereum"), Some(&SyncStatus::Syncing));
    }
    
    #[test]
    fn test_predefined_configs() {
        let eth_config = PredefinedSyncConfigs::ethereum();
        assert_eq!(eth_config.chain_name, "ethereum");
        assert_eq!(eth_config.chain_id, 1);
        assert_eq!(eth_config.batch_size, 100);
        assert_eq!(eth_config.confirmations, 12);
        
        let polygon_config = PredefinedSyncConfigs::polygon();
        assert_eq!(polygon_config.chain_name, "polygon");
        assert_eq!(polygon_config.chain_id, 137);
        assert_eq!(polygon_config.batch_size, 500);
        
        let cosmos_config = PredefinedSyncConfigs::cosmos();
        assert_eq!(cosmos_config.chain_name, "cosmos");
        assert_eq!(cosmos_config.confirmations, 1);
    }
} 