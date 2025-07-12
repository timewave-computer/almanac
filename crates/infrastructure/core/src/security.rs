//! Security and authentication utilities for multi-chain indexing

use std::collections::HashMap;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// Rate limiter for RPC requests
pub struct RateLimiter {
    /// Maximum requests per window
    max_requests: usize,
    /// Time window for rate limiting
    window: Duration,
    /// Request counts per endpoint
    request_counts: RwLock<HashMap<String, Vec<Instant>>>,
}

impl RateLimiter {
    /// Create a new rate limiter
    pub fn new(max_requests: usize, window: Duration) -> Self {
        Self {
            max_requests,
            window,
            request_counts: RwLock::new(HashMap::new()),
        }
    }
    
    /// Check if a request is allowed for the given endpoint
    pub async fn is_allowed(&self, endpoint: &str) -> bool {
        let mut counts = self.request_counts.write().await;
        let now = Instant::now();
        
        // Get or create request history for this endpoint
        let requests = counts.entry(endpoint.to_string()).or_insert_with(Vec::new);
        
        // Remove old requests outside the window
        requests.retain(|&time| now.duration_since(time) < self.window);
        
        // Check if we can make another request
        if requests.len() < self.max_requests {
            requests.push(now);
            true
        } else {
            false
        }
    }
    
    /// Get the time until the next request is allowed
    pub async fn time_until_allowed(&self, endpoint: &str) -> Option<Duration> {
        let counts = self.request_counts.read().await;
        if let Some(requests) = counts.get(endpoint) {
            if requests.len() >= self.max_requests {
                if let Some(&oldest) = requests.first() {
                    let elapsed = Instant::now().duration_since(oldest);
                    if elapsed < self.window {
                        return Some(self.window - elapsed);
                    }
                }
            }
        }
        None
    }
}

/// Health checker for monitoring chain connectivity
pub struct HealthChecker {
    /// Health status for each chain
    chain_health: RwLock<HashMap<String, ChainHealth>>,
}

#[derive(Debug, Clone)]
pub struct ChainHealth {
    /// Whether the chain is currently healthy
    pub is_healthy: bool,
    /// Last successful connection time
    pub last_success: Option<Instant>,
    /// Last failed connection time
    pub last_failure: Option<Instant>,
    /// Number of consecutive failures
    pub consecutive_failures: u32,
    /// Current latency in milliseconds
    pub latency_ms: Option<u64>,
}

impl HealthChecker {
    /// Create a new health checker
    pub fn new() -> Self {
        Self {
            chain_health: RwLock::new(HashMap::new()),
        }
    }
    
    /// Record a successful connection to a chain
    pub async fn record_success(&self, chain_id: &str, latency_ms: u64) {
        let mut health = self.chain_health.write().await;
        let chain_health = health.entry(chain_id.to_string()).or_insert_with(|| ChainHealth {
            is_healthy: true,
            last_success: None,
            last_failure: None,
            consecutive_failures: 0,
            latency_ms: None,
        });
        
        chain_health.is_healthy = true;
        chain_health.last_success = Some(Instant::now());
        chain_health.consecutive_failures = 0;
        chain_health.latency_ms = Some(latency_ms);
    }
    
    /// Record a failed connection to a chain
    pub async fn record_failure(&self, chain_id: &str) {
        let mut health = self.chain_health.write().await;
        let chain_health = health.entry(chain_id.to_string()).or_insert_with(|| ChainHealth {
            is_healthy: true,
            last_success: None,
            last_failure: None,
            consecutive_failures: 0,
            latency_ms: None,
        });
        
        chain_health.last_failure = Some(Instant::now());
        chain_health.consecutive_failures += 1;
        
        // Mark as unhealthy after 3 consecutive failures
        if chain_health.consecutive_failures >= 3 {
            chain_health.is_healthy = false;
        }
    }
    
    /// Get the health status of a chain
    pub async fn get_health(&self, chain_id: &str) -> Option<ChainHealth> {
        let health = self.chain_health.read().await;
        health.get(chain_id).cloned()
    }
    
    /// Get health status for all chains
    pub async fn get_all_health(&self) -> HashMap<String, ChainHealth> {
        let health = self.chain_health.read().await;
        health.clone()
    }
    
    /// Check if a chain is healthy
    pub async fn is_healthy(&self, chain_id: &str) -> bool {
        if let Some(health) = self.get_health(chain_id).await {
            health.is_healthy
        } else {
            true // Assume healthy if we haven't checked yet
        }
    }
}

impl Default for HealthChecker {
    fn default() -> Self {
        Self::new()
    }
}

/// Connection pool manager for handling multiple RPC endpoints
pub struct ConnectionManager {
    /// Rate limiter for requests
    rate_limiter: RateLimiter,
    /// Health checker for monitoring
    health_checker: HealthChecker,
}

impl ConnectionManager {
    /// Create a new connection manager
    pub fn new(max_requests_per_minute: usize) -> Self {
        let rate_limiter = RateLimiter::new(max_requests_per_minute, Duration::from_secs(60));
        let health_checker = HealthChecker::new();
        
        Self {
            rate_limiter,
            health_checker,
        }
    }
    
    /// Check if a request can be made to an endpoint
    pub async fn can_make_request(&self, endpoint: &str) -> bool {
        self.rate_limiter.is_allowed(endpoint).await
    }
    
    /// Wait until a request can be made to an endpoint
    pub async fn wait_for_request(&self, endpoint: &str) {
        while !self.rate_limiter.is_allowed(endpoint).await {
            if let Some(wait_time) = self.rate_limiter.time_until_allowed(endpoint).await {
                tokio::time::sleep(wait_time).await;
            } else {
                break;
            }
        }
    }
    
    /// Record a successful request
    pub async fn record_success(&self, chain_id: &str, latency_ms: u64) {
        self.health_checker.record_success(chain_id, latency_ms).await;
    }
    
    /// Record a failed request
    pub async fn record_failure(&self, chain_id: &str) {
        self.health_checker.record_failure(chain_id).await;
    }
    
    /// Check if a chain is healthy
    pub async fn is_chain_healthy(&self, chain_id: &str) -> bool {
        self.health_checker.is_healthy(chain_id).await
    }
    
    /// Get health status for all chains
    pub async fn get_health_report(&self) -> HashMap<String, ChainHealth> {
        self.health_checker.get_all_health().await
    }
}

/// Secure credential storage
pub struct CredentialStore {
    /// Encrypted credentials storage
    credentials: RwLock<HashMap<String, String>>,
}

impl CredentialStore {
    /// Create a new credential store
    pub fn new() -> Self {
        Self {
            credentials: RwLock::new(HashMap::new()),
        }
    }
    
    /// Store a credential (in production, this should be encrypted)
    pub async fn store_credential(&self, key: &str, value: &str) {
        let mut creds = self.credentials.write().await;
        creds.insert(key.to_string(), value.to_string());
    }
    
    /// Retrieve a credential
    pub async fn get_credential(&self, key: &str) -> Option<String> {
        let creds = self.credentials.read().await;
        creds.get(key).cloned()
    }
    
    /// Remove a credential
    pub async fn remove_credential(&self, key: &str) -> bool {
        let mut creds = self.credentials.write().await;
        creds.remove(key).is_some()
    }
    
    /// List all credential keys (not values)
    pub async fn list_keys(&self) -> Vec<String> {
        let creds = self.credentials.read().await;
        creds.keys().cloned().collect()
    }
}

impl Default for CredentialStore {
    fn default() -> Self {
        Self::new()
    }
} 