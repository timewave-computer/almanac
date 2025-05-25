/// Database connection pooling for optimized performance
use std::sync::Arc;
use std::time::Duration;
use std::collections::HashMap;
use tokio::sync::RwLock;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::{Result, Error};

/// Connection pool configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolConfig {
    /// Minimum number of connections to maintain
    pub min_connections: u32,
    
    /// Maximum number of connections in the pool
    pub max_connections: u32,
    
    /// Maximum time to wait for a connection
    pub acquire_timeout: Duration,
    
    /// Maximum lifetime of a connection
    pub max_lifetime: Duration,
    
    /// How long an idle connection is kept
    pub idle_timeout: Duration,
    
    /// Test query to validate connections
    pub test_query: Option<String>,
    
    /// Connection validation timeout
    pub validation_timeout: Duration,
    
    /// How often to run pool maintenance
    pub maintenance_interval: Duration,
    
    /// Maximum number of retries for failed connections
    pub max_retries: u32,
    
    /// Backoff delay between retries
    pub retry_backoff: Duration,
}

impl Default for PoolConfig {
    fn default() -> Self {
        Self {
            min_connections: 5,
            max_connections: 20,
            acquire_timeout: Duration::from_secs(30),
            max_lifetime: Duration::from_secs(1800), // 30 minutes
            idle_timeout: Duration::from_secs(600),   // 10 minutes
            test_query: Some("SELECT 1".to_string()),
            validation_timeout: Duration::from_secs(5),
            maintenance_interval: Duration::from_secs(60), // 1 minute
            max_retries: 3,
            retry_backoff: Duration::from_millis(100),
        }
    }
}

/// Connection pool statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolStats {
    /// Total number of connections in pool
    pub total_connections: u32,
    
    /// Number of active connections
    pub active_connections: u32,
    
    /// Number of idle connections
    pub idle_connections: u32,
    
    /// Number of connections waiting to be acquired
    pub waiting_count: u32,
    
    /// Total connections created since startup
    pub total_created: u64,
    
    /// Total connections closed since startup
    pub total_closed: u64,
    
    /// Total failed connection attempts
    pub total_failed: u64,
    
    /// Average time to acquire a connection
    pub avg_acquire_time: Duration,
    
    /// Average connection lifetime
    pub avg_connection_lifetime: Duration,
    
    /// Pool health percentage (0-100)
    pub health_percentage: f64,
}

/// Database connection trait
#[async_trait]
pub trait DatabaseConnection: Send + Sync {
    /// Execute a query on this connection
    async fn execute(&mut self, query: &str, params: &[&str]) -> Result<u64>;
    
    /// Query and return results
    async fn query(&mut self, query: &str, params: &[&str]) -> Result<Vec<serde_json::Value>>;
    
    /// Test if the connection is still valid
    async fn is_valid(&mut self) -> bool;
    
    /// Close the connection
    async fn close(self) -> Result<()>;
    
    /// Get connection metadata
    fn metadata(&self) -> ConnectionMetadata;
}

/// Connection metadata
#[derive(Debug, Clone)]
pub struct ConnectionMetadata {
    /// Unique connection ID
    pub id: String,
    
    /// When the connection was created
    pub created_at: std::time::SystemTime,
    
    /// Last time the connection was used
    pub last_used: std::time::SystemTime,
    
    /// Number of times this connection has been used
    pub usage_count: u64,
    
    /// Database backend type
    pub backend_type: String,
    
    /// Connection string (sanitized)
    pub connection_info: String,
}

/// Connection pool trait
#[async_trait]
pub trait ConnectionPool<T: DatabaseConnection>: Send + Sync {
    /// Acquire a connection from the pool
    async fn acquire(&self) -> Result<PooledConnection<T>>;
    
    /// Try to acquire a connection without blocking
    async fn try_acquire(&self) -> Result<Option<PooledConnection<T>>>;
    
    /// Get current pool statistics
    async fn stats(&self) -> Result<PoolStats>;
    
    /// Perform pool maintenance (cleanup idle connections, etc.)
    async fn maintain(&self) -> Result<()>;
    
    /// Check pool health
    async fn health_check(&self) -> Result<bool>;
    
    /// Resize the pool
    async fn resize(&self, min_size: u32, max_size: u32) -> Result<()>;
    
    /// Close all connections and shutdown the pool
    async fn close(&self) -> Result<()>;
}

/// A connection wrapper that returns to the pool when dropped
pub struct PooledConnection<T: DatabaseConnection> {
    connection: Option<T>,
    #[allow(dead_code)]
    pool: Arc<dyn ConnectionPool<T>>,
    acquired_at: std::time::SystemTime,
}

impl<T: DatabaseConnection> PooledConnection<T> {
    /// Create a new pooled connection
    pub fn new(connection: T, pool: Arc<dyn ConnectionPool<T>>) -> Self {
        Self {
            connection: Some(connection),
            pool,
            acquired_at: std::time::SystemTime::now(),
        }
    }
    
    /// Get a reference to the underlying connection
    pub fn as_ref(&self) -> Option<&T> {
        self.connection.as_ref()
    }
    
    /// Get a mutable reference to the underlying connection
    pub fn as_mut(&mut self) -> Option<&mut T> {
        self.connection.as_mut()
    }
    
    /// Get the time this connection was acquired
    pub fn acquired_at(&self) -> std::time::SystemTime {
        self.acquired_at
    }
    
    /// Manually return the connection to the pool
    pub async fn return_to_pool(mut self) -> Result<()> {
        if let Some(conn) = self.connection.take() {
            // Connection will be returned by the pool implementation
            drop(conn);
        }
        Ok(())
    }
}

impl<T: DatabaseConnection> Drop for PooledConnection<T> {
    fn drop(&mut self) {
        if let Some(_conn) = self.connection.take() {
            // In a real implementation, this would return the connection to the pool
            // For now, the connection will be dropped
        }
    }
}

/// Generic connection pool implementation
pub struct GenericConnectionPool<T: DatabaseConnection> {
    config: PoolConfig,
    connections: Arc<RwLock<Vec<T>>>,
    stats: Arc<RwLock<PoolStats>>,
    maintenance_handle: Option<tokio::task::JoinHandle<()>>,
}

impl<T: DatabaseConnection + 'static> GenericConnectionPool<T> {
    /// Create a new connection pool
    pub fn new(config: PoolConfig) -> Self {
        let pool = Self {
            config: config.clone(),
            connections: Arc::new(RwLock::new(Vec::new())),
            stats: Arc::new(RwLock::new(PoolStats {
                total_connections: 0,
                active_connections: 0,
                idle_connections: 0,
                waiting_count: 0,
                total_created: 0,
                total_closed: 0,
                total_failed: 0,
                avg_acquire_time: Duration::from_millis(0),
                avg_connection_lifetime: Duration::from_millis(0),
                health_percentage: 100.0,
            })),
            maintenance_handle: None,
        };
        
        pool
    }
    
    /// Start the maintenance task
    pub fn start_maintenance(&mut self) {
        let connections = Arc::clone(&self.connections);
        let stats = Arc::clone(&self.stats);
        let config = self.config.clone();
        
        let handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(config.maintenance_interval);
            
            loop {
                interval.tick().await;
                
                // Perform maintenance tasks
                if let Err(e) = Self::perform_maintenance(&connections, &stats, &config).await {
                    tracing::warn!("Pool maintenance failed: {}", e);
                }
            }
        });
        
        self.maintenance_handle = Some(handle);
    }
    
    /// Perform pool maintenance
    async fn perform_maintenance(
        connections: &Arc<RwLock<Vec<T>>>,
        stats: &Arc<RwLock<PoolStats>>,
        config: &PoolConfig,
    ) -> Result<()> {
        let mut conns = connections.write().await;
        let mut stats_guard = stats.write().await;
        
        let now = std::time::SystemTime::now();
        let mut to_remove = Vec::new();
        
        // Check for expired connections
        for (i, conn) in conns.iter_mut().enumerate() {
            let metadata = conn.metadata();
            
            // Check if connection is too old
            if let Ok(elapsed) = now.duration_since(metadata.created_at) {
                if elapsed > config.max_lifetime {
                    to_remove.push(i);
                    continue;
                }
            }
            
            // Check if connection has been idle too long
            if let Ok(idle_time) = now.duration_since(metadata.last_used) {
                if idle_time > config.idle_timeout {
                    to_remove.push(i);
                    continue;
                }
            }
            
            // Validate connection if test query is configured
            if config.test_query.is_some() && !conn.is_valid().await {
                to_remove.push(i);
            }
        }
        
        // Remove expired/invalid connections
        for &index in to_remove.iter().rev() {
            conns.remove(index);
            stats_guard.total_closed += 1;
        }
        
        // Update statistics
        stats_guard.total_connections = conns.len() as u32;
        stats_guard.idle_connections = conns.len() as u32; // Simplified for this example
        stats_guard.health_percentage = if stats_guard.total_connections > 0 {
            (stats_guard.total_connections - to_remove.len() as u32) as f64 
                / stats_guard.total_connections as f64 * 100.0
        } else {
            100.0
        };
        
        tracing::debug!("Pool maintenance completed. Removed {} connections", to_remove.len());
        
        Ok(())
    }
    
    /// Create a new connection (to be implemented by specific backends)
    async fn create_connection(&self) -> Result<T> {
        // This would be implemented by specific database backends
        Err(Error::Generic("Connection creation not implemented".to_string()))
    }
}

#[async_trait]
impl<T: DatabaseConnection + 'static> ConnectionPool<T> for GenericConnectionPool<T> {
    async fn acquire(&self) -> Result<PooledConnection<T>> {
        let start_time = std::time::SystemTime::now();
        
        // Try to get an existing connection
        {
            let mut conns = self.connections.write().await;
            if let Some(conn) = conns.pop() {
                let mut stats = self.stats.write().await;
                stats.active_connections += 1;
                stats.idle_connections = stats.idle_connections.saturating_sub(1);
                
                if let Ok(elapsed) = start_time.elapsed() {
                    stats.avg_acquire_time = elapsed;
                }
                
                return Ok(PooledConnection::new(conn, Arc::new(self.clone()) as Arc<dyn ConnectionPool<T>>));
            }
        }
        
        // Check if we can create a new connection
        let stats = self.stats.read().await;
        if stats.total_connections >= self.config.max_connections {
            return Err(Error::Generic("Connection pool exhausted".to_string()));
        }
        drop(stats);
        
        // Create a new connection
        match self.create_connection().await {
            Ok(conn) => {
                let mut stats = self.stats.write().await;
                stats.total_created += 1;
                stats.total_connections += 1;
                stats.active_connections += 1;
                
                if let Ok(elapsed) = start_time.elapsed() {
                    stats.avg_acquire_time = elapsed;
                }
                
                Ok(PooledConnection::new(conn, Arc::new(self.clone()) as Arc<dyn ConnectionPool<T>>))
            }
            Err(e) => {
                let mut stats = self.stats.write().await;
                stats.total_failed += 1;
                Err(e)
            }
        }
    }
    
    async fn try_acquire(&self) -> Result<Option<PooledConnection<T>>> {
        let mut conns = self.connections.write().await;
        if let Some(conn) = conns.pop() {
            let mut stats = self.stats.write().await;
            stats.active_connections += 1;
            stats.idle_connections = stats.idle_connections.saturating_sub(1);
            
            Ok(Some(PooledConnection::new(conn, Arc::new(self.clone()) as Arc<dyn ConnectionPool<T>>)))
        } else {
            Ok(None)
        }
    }
    
    async fn stats(&self) -> Result<PoolStats> {
        Ok(self.stats.read().await.clone())
    }
    
    async fn maintain(&self) -> Result<()> {
        Self::perform_maintenance(&self.connections, &self.stats, &self.config).await
    }
    
    async fn health_check(&self) -> Result<bool> {
        let stats = self.stats.read().await;
        
        // Pool is healthy if:
        // 1. We have at least min_connections available
        // 2. Health percentage is above 80%
        // 3. We're not at max capacity with waiting requests
        
        let healthy = stats.total_connections >= self.config.min_connections
            && stats.health_percentage >= 80.0
            && !(stats.total_connections >= self.config.max_connections && stats.waiting_count > 0);
        
        Ok(healthy)
    }
    
    async fn resize(&self, min_size: u32, max_size: u32) -> Result<()> {
        if min_size > max_size {
            return Err(Error::InvalidData("min_size cannot be greater than max_size".to_string()));
        }
        
        // Update configuration
        let mut config = self.config.clone();
        config.min_connections = min_size;
        config.max_connections = max_size;
        
        // Adjust current pool size if needed
        let mut conns = self.connections.write().await;
        let current_size = conns.len() as u32;
        
        if current_size > max_size {
            // Remove excess connections
            let to_remove = (current_size - max_size) as usize;
            let new_len = conns.len() - to_remove;
            conns.truncate(new_len);
            
            let mut stats = self.stats.write().await;
            stats.total_closed += to_remove as u64;
            stats.total_connections = conns.len() as u32;
        }
        
        tracing::info!("Pool resized: min={}, max={}, current={}", min_size, max_size, conns.len());
        
        Ok(())
    }
    
    async fn close(&self) -> Result<()> {
        // Stop maintenance task
        if let Some(handle) = &self.maintenance_handle {
            handle.abort();
        }
        
        // Close all connections
        let mut conns = self.connections.write().await;
        let connection_count = conns.len();
        
        // In a real implementation, we would properly close each connection
        conns.clear();
        
        let mut stats = self.stats.write().await;
        stats.total_closed += connection_count as u64;
        stats.total_connections = 0;
        stats.active_connections = 0;
        stats.idle_connections = 0;
        
        tracing::info!("Connection pool closed. {} connections were closed", connection_count);
        
        Ok(())
    }
}

// We need Clone for the GenericConnectionPool to work with Arc
impl<T: DatabaseConnection> Clone for GenericConnectionPool<T> {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            connections: Arc::clone(&self.connections),
            stats: Arc::clone(&self.stats),
            maintenance_handle: None, // Don't clone the maintenance handle
        }
    }
}

/// Pool manager for managing multiple connection pools
pub struct PoolManager {
    pools: RwLock<HashMap<String, String>>, // Simplified to just track pool names
}

impl PoolManager {
    /// Create a new pool manager
    pub fn new() -> Self {
        Self {
            pools: RwLock::new(HashMap::new()),
        }
    }
    
    /// Register a connection pool
    pub async fn register_pool<T: DatabaseConnection + 'static>(
        &self,
        name: String,
        _pool: Arc<dyn ConnectionPool<T>>,
    ) -> Result<()> {
        let mut pools = self.pools.write().await;
        // Just track the pool name for simplicity
        pools.insert(name.clone(), "registered".to_string());
        tracing::info!("Registered connection pool: {}", name);
        Ok(())
    }
    
    /// Get a connection pool by name
    pub async fn get_pool<T: DatabaseConnection + 'static>(
        &self,
        name: &str,
    ) -> Result<Option<Arc<dyn ConnectionPool<T>>>> {
        let pools = self.pools.read().await;
        if pools.contains_key(name) {
            // In a real implementation, you'd return the actual pool
            // For now, return None to indicate the pool exists but we can't retrieve it
            Ok(None)
        } else {
            Ok(None)
        }
    }
    
    /// Get statistics for all pools
    pub async fn get_all_stats(&self) -> Result<HashMap<String, serde_json::Value>> {
        let pools = self.pools.read().await;
        let mut all_stats = HashMap::new();
        
        for (name, _status) in pools.iter() {
            all_stats.insert(name.clone(), serde_json::json!({
                "name": name,
                "status": "active",
                "type": "registered"
            }));
        }
        
        Ok(all_stats)
    }
    
    /// Close all pools
    pub async fn close_all(&self) -> Result<()> {
        let mut pools = self.pools.write().await;
        
        let pool_count = pools.len();
        pools.clear();
        
        tracing::info!("All {} connection pools closed", pool_count);
        Ok(())
    }
}

impl Default for PoolManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::SystemTime;
    
    // Mock database connection for testing
    struct MockConnection {
        id: String,
        created_at: SystemTime,
        last_used: SystemTime,
        usage_count: u64,
        valid: bool,
    }
    
    impl MockConnection {
        fn new(id: String) -> Self {
            let now = SystemTime::now();
            Self {
                id,
                created_at: now,
                last_used: now,
                usage_count: 0,
                valid: true,
            }
        }
    }
    
    #[async_trait]
    impl DatabaseConnection for MockConnection {
        async fn execute(&mut self, _query: &str, _params: &[&str]) -> Result<u64> {
            self.usage_count += 1;
            self.last_used = SystemTime::now();
            Ok(1)
        }
        
        async fn query(&mut self, _query: &str, _params: &[&str]) -> Result<Vec<serde_json::Value>> {
            self.usage_count += 1;
            self.last_used = SystemTime::now();
            Ok(vec![serde_json::json!({"result": "test"})]) 
        }
        
        async fn is_valid(&mut self) -> bool {
            self.valid
        }
        
        async fn close(self) -> Result<()> {
            Ok(())
        }
        
        fn metadata(&self) -> ConnectionMetadata {
            ConnectionMetadata {
                id: self.id.clone(),
                created_at: self.created_at,
                last_used: self.last_used,
                usage_count: self.usage_count,
                backend_type: "mock".to_string(),
                connection_info: "mock://test".to_string(),
            }
        }
    }
    
    #[test]
    fn test_pool_config_default() {
        let config = PoolConfig::default();
        
        assert_eq!(config.min_connections, 5);
        assert_eq!(config.max_connections, 20);
        assert_eq!(config.acquire_timeout, Duration::from_secs(30));
        assert!(config.test_query.is_some());
        assert_eq!(config.max_retries, 3);
    }
    
    #[tokio::test]
    async fn test_pooled_connection() {
        // Create a mock pool (simplified for testing)
        struct MockPool;
        
        #[async_trait]
        impl ConnectionPool<MockConnection> for MockPool {
            async fn acquire(&self) -> Result<PooledConnection<MockConnection>> {
                let conn = MockConnection::new("test-1".to_string());
                Ok(PooledConnection::new(conn, Arc::new(self.clone())))
            }
            
            async fn try_acquire(&self) -> Result<Option<PooledConnection<MockConnection>>> {
                Ok(Some(self.acquire().await?))
            }
            
            async fn stats(&self) -> Result<PoolStats> {
                Ok(PoolStats {
                    total_connections: 1,
                    active_connections: 0,
                    idle_connections: 1,
                    waiting_count: 0,
                    total_created: 1,
                    total_closed: 0,
                    total_failed: 0,
                    avg_acquire_time: Duration::from_millis(1),
                    avg_connection_lifetime: Duration::from_secs(60),
                    health_percentage: 100.0,
                })
            }
            
            async fn maintain(&self) -> Result<()> { Ok(()) }
            async fn health_check(&self) -> Result<bool> { Ok(true) }
            async fn resize(&self, _min: u32, _max: u32) -> Result<()> { Ok(()) }
            async fn close(&self) -> Result<()> { Ok(()) }
        }
        
        impl Clone for MockPool {
            fn clone(&self) -> Self { MockPool }
        }
        
        let pool = MockPool;
        let mut pooled_conn = pool.acquire().await.unwrap();
        
        // Test connection usage
        if let Some(conn) = pooled_conn.as_mut() {
            let result = conn.execute("SELECT 1", &[]).await.unwrap();
            assert_eq!(result, 1);
            
            let metadata = conn.metadata();
            assert_eq!(metadata.id, "test-1");
            assert_eq!(metadata.usage_count, 1);
        }
    }
    
    #[tokio::test]
    async fn test_pool_manager() {
        let manager = PoolManager::new();
        
        // Test empty state
        let stats = manager.get_all_stats().await.unwrap();
        assert!(stats.is_empty());
        
        // Test closing all pools
        manager.close_all().await.unwrap();
    }
    
    #[test]
    fn test_connection_metadata() {
        let conn = MockConnection::new("test-meta".to_string());
        let metadata = conn.metadata();
        
        assert_eq!(metadata.id, "test-meta");
        assert_eq!(metadata.backend_type, "mock");
        assert_eq!(metadata.connection_info, "mock://test");
        assert_eq!(metadata.usage_count, 0);
    }
} 