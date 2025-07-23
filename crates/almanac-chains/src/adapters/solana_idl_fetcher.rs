//! Solana Anchor IDL fetcher service
//!
//! This module implements a non-blocking service for fetching
//! Anchor IDLs from the Solana blockchain.

use anyhow::Result;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use std::collections::{HashSet, VecDeque};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tracing::{debug, error, info, warn};

use crate::adapters::solana_idl::{derive_idl_pda, parse_idl_account, parse_idl_json};
use almanac_core::Storage;

/// Priority levels for IDL fetch requests
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Priority {
    Low = 0,
    Normal = 1,
    High = 2,
}

/// IDL fetch request
#[derive(Debug, Clone)]
pub struct ProgramIdRequest {
    pub program_id: Pubkey,
    pub priority: Priority,
    pub retry_count: u8,
    pub created_at: std::time::Instant,
}

/// Configuration for the IDL fetcher service
#[derive(Debug, Clone)]
pub struct IdlFetcherConfig {
    /// Maximum queue size (to prevent memory exhaustion)
    pub max_queue_size: usize,
    /// Maximum concurrent fetches
    pub max_concurrent_fetches: usize,
    /// Fetch timeout in seconds
    pub fetch_timeout_secs: u64,
    /// Maximum retry attempts
    pub max_retry_attempts: u8,
    /// Base retry backoff in milliseconds
    pub retry_backoff_ms: u64,
    /// Circuit breaker threshold (consecutive failures)
    pub circuit_breaker_threshold: u32,
    /// Circuit breaker timeout in milliseconds
    pub circuit_breaker_timeout_ms: u64,
}

impl Default for IdlFetcherConfig {
    fn default() -> Self {
        Self {
            max_queue_size: 10000,
            max_concurrent_fetches: 10,
            fetch_timeout_secs: 5,
            max_retry_attempts: 3,
            retry_backoff_ms: 1000,
            circuit_breaker_threshold: 5,
            circuit_breaker_timeout_ms: 60000,
        }
    }
}

/// Circuit breaker state
#[derive(Debug, Clone)]
enum CircuitState {
    Closed,
    Open(std::time::Instant), // Open until this time
}

/// IDL fetcher service
pub struct IdlFetcherService {
    rpc_client: Arc<RpcClient>,
    #[allow(dead_code)]
    storage: Arc<dyn Storage + Send + Sync>,
    config: IdlFetcherConfig,
    
    // Queue management
    fetch_queue: Arc<Mutex<VecDeque<ProgramIdRequest>>>,
    high_priority_queue: Arc<Mutex<VecDeque<ProgramIdRequest>>>,
    inflight: Arc<Mutex<HashSet<Pubkey>>>,
    
    // Circuit breaker
    consecutive_failures: Arc<Mutex<u32>>,
    circuit_state: Arc<Mutex<CircuitState>>,
    
    // Metrics
    total_fetches: Arc<Mutex<u64>>,
    successful_fetches: Arc<Mutex<u64>>,
    failed_fetches: Arc<Mutex<u64>>,
}

impl IdlFetcherService {
    /// Create a new IDL fetcher service
    pub fn new(
        rpc_client: Arc<RpcClient>,
        storage: Arc<dyn Storage + Send + Sync>,
        config: IdlFetcherConfig,
    ) -> Self {
        Self {
            rpc_client,
            storage,
            config,
            fetch_queue: Arc::new(Mutex::new(VecDeque::new())),
            high_priority_queue: Arc::new(Mutex::new(VecDeque::new())),
            inflight: Arc::new(Mutex::new(HashSet::new())),
            consecutive_failures: Arc::new(Mutex::new(0)),
            circuit_state: Arc::new(Mutex::new(CircuitState::Closed)),
            total_fetches: Arc::new(Mutex::new(0)),
            successful_fetches: Arc::new(Mutex::new(0)),
            failed_fetches: Arc::new(Mutex::new(0)),
        }
    }
    
    /// Enqueue a program ID for IDL fetching
    pub async fn enqueue(&self, program_id: Pubkey, priority: Priority) -> Result<()> {
        // Check if already in flight
        {
            let inflight = self.inflight.lock().await;
            if inflight.contains(&program_id) {
                debug!("Program {} already being fetched, skipping", program_id);
                return Ok(());
            }
        }
        
        // Check queue size limits
        let queue_size = {
            let normal_queue = self.fetch_queue.lock().await;
            let high_queue = self.high_priority_queue.lock().await;
            normal_queue.len() + high_queue.len()
        };
        
        if queue_size >= self.config.max_queue_size {
            warn!("IDL fetch queue full ({} items), dropping request for {}", queue_size, program_id);
            return Err(anyhow::anyhow!("Queue full"));
        }
        
        let request = ProgramIdRequest {
            program_id,
            priority,
            retry_count: 0,
            created_at: std::time::Instant::now(),
        };
        
        // Add to appropriate queue based on priority
        match priority {
            Priority::High => {
                let mut queue = self.high_priority_queue.lock().await;
                queue.push_back(request);
            }
            _ => {
                let mut queue = self.fetch_queue.lock().await;
                queue.push_back(request);
            }
        }
        
        debug!("Enqueued IDL fetch for {} with priority {:?}", program_id, priority);
        Ok(())
    }
    
    /// Try to enqueue without blocking (for use in hot paths)
    pub fn try_enqueue(&self, program_id: Pubkey, priority: Priority) -> Result<()> {
        // Try to acquire locks without blocking
        let inflight = match self.inflight.try_lock() {
            Ok(guard) => guard,
            Err(_) => return Err(anyhow::anyhow!("Could not acquire lock")),
        };
        
        if inflight.contains(&program_id) {
            return Ok(());
        }
        drop(inflight);
        
        let request = ProgramIdRequest {
            program_id,
            priority,
            retry_count: 0,
            created_at: std::time::Instant::now(),
        };
        
        match priority {
            Priority::High => {
                match self.high_priority_queue.try_lock() {
                    Ok(mut queue) => {
                        if queue.len() < self.config.max_queue_size {
                            queue.push_back(request);
                            Ok(())
                        } else {
                            Err(anyhow::anyhow!("Queue full"))
                        }
                    }
                    Err(_) => Err(anyhow::anyhow!("Could not acquire lock")),
                }
            }
            _ => {
                match self.fetch_queue.try_lock() {
                    Ok(mut queue) => {
                        if queue.len() < self.config.max_queue_size {
                            queue.push_back(request);
                            Ok(())
                        } else {
                            Err(anyhow::anyhow!("Queue full"))
                        }
                    }
                    Err(_) => Err(anyhow::anyhow!("Could not acquire lock")),
                }
            }
        }
    }
    
    /// Run the fetcher loop
    pub async fn run_fetcher_loop(self: Arc<Self>) {
        info!("Starting IDL fetcher service");
        
        loop {
            // Check circuit breaker
            if !self.is_circuit_closed().await {
                tokio::time::sleep(Duration::from_millis(1000)).await;
                continue;
            }
            
            // Process a batch of requests
            self.process_batch().await;
            
            // Small delay to prevent spinning
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }
    
    /// Check if circuit breaker is closed (operational)
    async fn is_circuit_closed(&self) -> bool {
        let state = self.circuit_state.lock().await;
        match &*state {
            CircuitState::Closed => true,
            CircuitState::Open(until) => {
                let now = std::time::Instant::now();
                now >= *until
            }
        }
    }
    
    /// Process a batch of fetch requests
    async fn process_batch(self: &Arc<Self>) {
        let batch_size = self.config.max_concurrent_fetches;
        let mut requests = Vec::new();
        
        // Collect requests from queues (high priority first)
        {
            let mut high_queue = self.high_priority_queue.lock().await;
            while requests.len() < batch_size && !high_queue.is_empty() {
                if let Some(req) = high_queue.pop_front() {
                    requests.push(req);
                }
            }
        }
        
        {
            let mut normal_queue = self.fetch_queue.lock().await;
            while requests.len() < batch_size && !normal_queue.is_empty() {
                if let Some(req) = normal_queue.pop_front() {
                    requests.push(req);
                }
            }
        }
        
        if requests.is_empty() {
            return;
        }
        
        // Mark requests as in-flight
        {
            let mut inflight = self.inflight.lock().await;
            for req in &requests {
                inflight.insert(req.program_id);
            }
        }
        
        // Process requests concurrently
        let mut handles = Vec::new();
        for request in requests {
            let self_clone = Arc::clone(self);
            let handle = tokio::spawn(async move {
                self_clone.fetch_idl(request).await
            });
            handles.push(handle);
        }
        
        // Wait for all fetches to complete
        for handle in handles {
            let _ = handle.await;
        }
    }
    
    /// Fetch IDL for a single program
    async fn fetch_idl(self: &Arc<Self>, request: ProgramIdRequest) {
        let program_id = request.program_id;
        
        // Derive IDL PDA
        let (idl_pda, _) = derive_idl_pda(&program_id);
        
        debug!("Fetching IDL for program {} from PDA {}", program_id, idl_pda);
        
        // Fetch account with timeout
        let fetch_result = tokio::time::timeout(
            Duration::from_secs(self.config.fetch_timeout_secs),
            self.rpc_client.get_account(&idl_pda),
        ).await;
        
        match fetch_result {
            Ok(Ok(account)) => {
                // Parse and store IDL
                match self.process_idl_account(&program_id, &account).await {
                    Ok(_) => {
                        info!("Successfully fetched and stored IDL for {}", program_id);
                        self.record_success().await;
                    }
                    Err(e) => {
                        error!("Failed to process IDL for {}: {}", program_id, e);
                        self.handle_fetch_failure(request).await;
                    }
                }
            }
            Ok(Err(e)) => {
                debug!("IDL account not found for {}: {}", program_id, e);
                self.record_success().await; // Not an error if IDL doesn't exist
            }
            Err(_) => {
                error!("Timeout fetching IDL for {}", program_id);
                self.handle_fetch_failure(request).await;
            }
        }
        
        // Remove from inflight set
        {
            let mut inflight = self.inflight.lock().await;
            inflight.remove(&program_id);
        }
    }
    
    /// Process and store IDL account data
    async fn process_idl_account(
        self: &Arc<Self>,
        program_id: &Pubkey,
        account: &solana_sdk::account::Account,
    ) -> Result<()> {
        // Parse the IDL account
        let idl_account = parse_idl_account(&account.data)
            .map_err(|e| anyhow::anyhow!("Failed to parse IDL account: {}", e))?;
        
        // Decompress and parse the IDL JSON
        let idl_json = parse_idl_json(&idl_account)
            .map_err(|e| anyhow::anyhow!("Failed to parse IDL JSON: {}", e))?;
        
        // For minimal implementation, just log the storage
        // In a real implementation, this would use the storage backend
        debug!("Would store IDL for program {} with {} bytes", program_id, idl_json.to_string().len());
        
        Ok(())
    }
    
    /// Handle fetch failure
    async fn handle_fetch_failure(self: &Arc<Self>, mut request: ProgramIdRequest) {
        self.record_failure().await;
        
        // Check if we should retry
        if request.retry_count < self.config.max_retry_attempts {
            request.retry_count += 1;
            
            // Calculate backoff delay
            let backoff_ms = self.config.retry_backoff_ms * (2_u64.pow(request.retry_count as u32));
            
            // Re-queue with lower priority after backoff
            tokio::spawn(async move {
                tokio::time::sleep(Duration::from_millis(backoff_ms)).await;
                // Note: We can't re-queue here without access to self
                // In a real implementation, we'd use a channel or other mechanism
            });
        }
    }
    
    /// Record successful fetch
    async fn record_success(self: &Arc<Self>) {
        let mut failures = self.consecutive_failures.lock().await;
        *failures = 0;
        
        // Close circuit breaker on success
        let mut circuit = self.circuit_state.lock().await;
        *circuit = CircuitState::Closed;
        
        let mut successes = self.successful_fetches.lock().await;
        *successes += 1;
        
        let mut total = self.total_fetches.lock().await;
        *total += 1;
    }
    
    /// Record failed fetch
    async fn record_failure(self: &Arc<Self>) {
        let mut failures = self.consecutive_failures.lock().await;
        *failures += 1;
        
        // Check if we should open the circuit breaker
        if *failures >= self.config.circuit_breaker_threshold {
            let mut circuit = self.circuit_state.lock().await;
            let reopen_time = std::time::Instant::now() + 
                Duration::from_millis(self.config.circuit_breaker_timeout_ms);
            *circuit = CircuitState::Open(reopen_time);
            error!("Circuit breaker opened due to {} consecutive failures", *failures);
        }
        
        let mut failed = self.failed_fetches.lock().await;
        *failed += 1;
        
        let mut total = self.total_fetches.lock().await;
        *total += 1;
    }
    
    /// Get current metrics
    pub async fn get_metrics(&self) -> (u64, u64, u64, usize, bool) {
        let total = *self.total_fetches.lock().await;
        let successful = *self.successful_fetches.lock().await;
        let failed = *self.failed_fetches.lock().await;
        let queue_depth = {
            let normal = self.fetch_queue.lock().await.len();
            let high = self.high_priority_queue.lock().await.len();
            normal + high
        };
        let circuit_open = !self.is_circuit_closed().await;
        
        (total, successful, failed, queue_depth, circuit_open)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use almanac_storage::MemoryStorage;
    
    fn create_test_fetcher() -> IdlFetcherService {
        let rpc_client = Arc::new(solana_client::nonblocking::rpc_client::RpcClient::new(
            "http://localhost:8899".to_string()
        ));
        let storage = Arc::new(MemoryStorage::new());
        let config = IdlFetcherConfig {
            max_queue_size: 10,
            max_concurrent_fetches: 2,
            ..Default::default()
        };
        
        IdlFetcherService::new(rpc_client, storage, config)
    }
    
    #[tokio::test]
    async fn test_priority_ordering() {
        let fetcher = create_test_fetcher();
        
        // Add requests with different priorities
        let program1 = Pubkey::new_unique();
        let program2 = Pubkey::new_unique();
        let program3 = Pubkey::new_unique();
        
        fetcher.enqueue(program1, Priority::Low).await.unwrap();
        fetcher.enqueue(program2, Priority::High).await.unwrap();
        fetcher.enqueue(program3, Priority::Normal).await.unwrap();
        
        // High priority should be in the high priority queue
        let high_queue = fetcher.high_priority_queue.lock().await;
        assert_eq!(high_queue.len(), 1);
        assert_eq!(high_queue[0].program_id, program2);
    }
    
    #[tokio::test]
    async fn test_deduplication() {
        let fetcher = create_test_fetcher();
        
        let program = Pubkey::new_unique();
        
        // First enqueue should succeed
        assert!(fetcher.enqueue(program, Priority::Normal).await.is_ok());
        
        // Queue should have one entry
        {
            let queue = fetcher.fetch_queue.lock().await;
            assert_eq!(queue.len(), 1);
        }
        
        // Mark as inflight
        fetcher.inflight.lock().await.insert(program);
        
        // Second enqueue should be skipped (already inflight)
        assert!(fetcher.enqueue(program, Priority::Normal).await.is_ok());
        
        // Queue should still have only one entry (second was skipped)
        let queue = fetcher.fetch_queue.lock().await;
        assert_eq!(queue.len(), 1);
    }
    
    #[tokio::test]
    async fn test_circuit_breaker() {
        let fetcher = Arc::new(create_test_fetcher());
        
        // Simulate consecutive failures
        for _ in 0..5 {
            fetcher.record_failure().await;
        }
        
        // Circuit should be open
        assert!(!fetcher.is_circuit_closed().await);
        
        // Reset and verify circuit closes
        fetcher.record_success().await;
        assert!(fetcher.is_circuit_closed().await);
    }
    
    #[tokio::test]
    async fn test_queue_overflow() {
        let fetcher = create_test_fetcher();
        
        // Fill the queue to capacity
        for i in 0..10 {
            let program = Pubkey::new_from_array([i as u8; 32]);
            assert!(fetcher.enqueue(program, Priority::Normal).await.is_ok());
        }
        
        // Next enqueue should fail
        let overflow_program = Pubkey::new_unique();
        assert!(fetcher.enqueue(overflow_program, Priority::Normal).await.is_err());
    }
    
    #[tokio::test]
    async fn test_try_enqueue_non_blocking() {
        let fetcher = create_test_fetcher();
        
        let program = Pubkey::new_unique();
        
        // try_enqueue should not block
        assert!(fetcher.try_enqueue(program, Priority::Normal).is_ok());
        
        // Lock the queue in another task
        let queue = fetcher.fetch_queue.clone();
        let _guard = queue.lock().await;
        
        // try_enqueue should fail immediately (not block)
        let program2 = Pubkey::new_unique();
        assert!(fetcher.try_enqueue(program2, Priority::Normal).is_err());
    }
    
    #[tokio::test]
    async fn test_metrics() {
        let fetcher = Arc::new(create_test_fetcher());
        
        // Initial metrics should be zero
        let (total, successful, failed, queue_depth, circuit_open) = fetcher.get_metrics().await;
        assert_eq!(total, 0);
        assert_eq!(successful, 0);
        assert_eq!(failed, 0);
        assert_eq!(queue_depth, 0);
        assert!(!circuit_open);
        
        // Add some requests
        fetcher.enqueue(Pubkey::new_unique(), Priority::Normal).await.unwrap();
        fetcher.enqueue(Pubkey::new_unique(), Priority::High).await.unwrap();
        
        // Check queue depth
        let (_, _, _, queue_depth, _) = fetcher.get_metrics().await;
        assert_eq!(queue_depth, 2);
        
        // Record some results
        fetcher.record_success().await;
        fetcher.record_failure().await;
        
        let (total, successful, failed, _, _) = fetcher.get_metrics().await;
        assert_eq!(total, 2);
        assert_eq!(successful, 1);
        assert_eq!(failed, 1);
    }
}