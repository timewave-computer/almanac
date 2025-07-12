use crate::{Error, Result};
use async_trait::async_trait;
use bytes::Bytes;
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{RwLock, Semaphore};
use tokio::time::interval;
use tracing::{debug, info, warn};

/// Distributed cache for query results
pub struct DistributedCache {
    /// Local cache storage
    local_cache: Arc<LocalCache>,
    /// Remote cache client for distributed caching
    remote_cache: Option<Arc<dyn RemoteCache>>,
    /// Cache statistics
    stats: Arc<RwLock<CacheStats>>,
    /// Configuration
    config: CacheConfig,
}

impl DistributedCache {
    pub fn new(config: CacheConfig) -> Self {
        let local_cache = Arc::new(LocalCache::new(config.local_cache_size));
        
        // Start background tasks
        let cache_clone = Arc::clone(&local_cache);
        tokio::spawn(async move {
            cache_clone.run_eviction_task().await;
        });

        Self {
            local_cache,
            remote_cache: None,
            stats: Arc::new(RwLock::new(CacheStats::default())),
            config,
        }
    }

    /// Set a remote cache backend
    pub fn with_remote_cache(mut self, remote: Arc<dyn RemoteCache>) -> Self {
        self.remote_cache = Some(remote);
        self
    }

    /// Get a value from the cache
    pub async fn get<T>(&self, key: &str) -> Result<Option<T>>
    where
        T: for<'de> Deserialize<'de> + Send + Sync,
    {
        let mut stats = self.stats.write().await;
        stats.total_requests += 1;

        // Try local cache first
        if let Some(entry) = self.local_cache.get(key).await {
            stats.local_hits += 1;
            let value: T = serde_json::from_slice(&entry.data)
                .map_err(|e| Error::Deserialization(e.to_string()))?;
            return Ok(Some(value));
        }

        // Try remote cache if available
        if let Some(remote) = &self.remote_cache {
            if let Some(data) = remote.get(key).await? {
                stats.remote_hits += 1;
                
                // Populate local cache
                self.local_cache.put(key.to_string(), data.clone(), self.config.default_ttl).await;
                
                let value: T = serde_json::from_slice(&data)
                    .map_err(|e| Error::Deserialization(e.to_string()))?;
                return Ok(Some(value));
            }
        }

        stats.misses += 1;
        Ok(None)
    }

    /// Set a value in the cache
    pub async fn set<T>(&self, key: String, value: &T, ttl: Option<Duration>) -> Result<()>
    where
        T: Serialize + Send + Sync,
    {
        let data = serde_json::to_vec(value)
            .map_err(|e| Error::Serialization(e.to_string()))?;
        let data_bytes = Bytes::from(data);
        
        let ttl = ttl.unwrap_or(self.config.default_ttl);

        // Set in local cache
        self.local_cache.put(key.clone(), data_bytes.clone(), ttl).await;

        // Set in remote cache if available
        if let Some(remote) = &self.remote_cache {
            remote.set(key, data_bytes, ttl).await?;
        }

        let mut stats = self.stats.write().await;
        stats.total_sets += 1;

        Ok(())
    }

    /// Invalidate a cache entry
    pub async fn invalidate(&self, key: &str) -> Result<()> {
        self.local_cache.remove(key).await;

        if let Some(remote) = &self.remote_cache {
            remote.delete(key).await?;
        }

        let mut stats = self.stats.write().await;
        stats.invalidations += 1;

        Ok(())
    }

    /// Invalidate entries matching a pattern
    pub async fn invalidate_pattern(&self, pattern: &str) -> Result<usize> {
        let count = self.local_cache.remove_pattern(pattern).await;

        if let Some(remote) = &self.remote_cache {
            remote.delete_pattern(pattern).await?;
        }

        let mut stats = self.stats.write().await;
        stats.invalidations += count;

        Ok(count)
    }

    /// Get cache statistics
    pub async fn stats(&self) -> CacheStats {
        self.stats.read().await.clone()
    }

    /// Clear all cache entries
    pub async fn clear(&self) -> Result<()> {
        self.local_cache.clear().await;

        if let Some(remote) = &self.remote_cache {
            remote.flush().await?;
        }

        self.stats.write().await.reset();
        Ok(())
    }
}

/// Local in-memory cache implementation
struct LocalCache {
    /// Cache entries
    entries: Arc<DashMap<String, CacheEntry>>,
    /// Maximum cache size in bytes
    max_size: usize,
    /// Current cache size in bytes
    current_size: Arc<RwLock<usize>>,
    /// Eviction semaphore to prevent concurrent evictions
    eviction_lock: Arc<Semaphore>,
}

#[derive(Clone)]
struct CacheEntry {
    data: Bytes,
    expires_at: Instant,
    last_access: Instant,
    access_count: u64,
    size: usize,
}

impl LocalCache {
    fn new(max_size: usize) -> Self {
        Self {
            entries: Arc::new(DashMap::new()),
            max_size,
            current_size: Arc::new(RwLock::new(0)),
            eviction_lock: Arc::new(Semaphore::new(1)),
        }
    }

    async fn get(&self, key: &str) -> Option<CacheEntry> {
        let mut entry = self.entries.get_mut(key)?;
        
        // Check if expired
        if entry.expires_at < Instant::now() {
            drop(entry);
            self.remove(key).await;
            return None;
        }

        // Update access info
        entry.last_access = Instant::now();
        entry.access_count += 1;
        
        Some(entry.clone())
    }

    async fn put(&self, key: String, data: Bytes, ttl: Duration) {
        let size = data.len();
        let entry = CacheEntry {
            data,
            expires_at: Instant::now() + ttl,
            last_access: Instant::now(),
            access_count: 1,
            size,
        };

        // Check if we need to evict before inserting
        let mut current_size = self.current_size.write().await;
        if *current_size + size > self.max_size {
            drop(current_size);
            self.evict_lru(size).await;
            current_size = self.current_size.write().await;
        }

        // Insert or update
        if let Some(old_entry) = self.entries.insert(key, entry) {
            *current_size = current_size.saturating_sub(old_entry.size);
        }
        *current_size += size;
    }

    async fn remove(&self, key: &str) {
        if let Some((_, entry)) = self.entries.remove(key) {
            let mut current_size = self.current_size.write().await;
            *current_size = current_size.saturating_sub(entry.size);
        }
    }

    async fn remove_pattern(&self, pattern: &str) -> usize {
        let regex = regex::Regex::new(pattern).unwrap_or_else(|_| regex::Regex::new(".*").unwrap());
        let mut count = 0;
        let mut total_size = 0;

        self.entries.retain(|key, entry| {
            if regex.is_match(key) {
                count += 1;
                total_size += entry.size;
                false
            } else {
                true
            }
        });

        if total_size > 0 {
            let mut current_size = self.current_size.write().await;
            *current_size = current_size.saturating_sub(total_size);
        }

        count
    }

    async fn clear(&self) {
        self.entries.clear();
        *self.current_size.write().await = 0;
    }

    async fn evict_lru(&self, needed_space: usize) {
        let _permit = self.eviction_lock.acquire().await.unwrap();
        
        // Collect entries with their keys and sort by LRU
        let mut entries: Vec<(String, Instant, usize)> = self.entries.iter()
            .map(|entry| (entry.key().clone(), entry.value().last_access, entry.value().size))
            .collect();
        
        entries.sort_by_key(|(_, last_access, _)| *last_access);

        let mut freed_space = 0;
        let mut keys_to_remove = Vec::new();

        for (key, _, size) in entries {
            keys_to_remove.push(key);
            freed_space += size;
            
            if freed_space >= needed_space {
                break;
            }
        }

        // Remove selected entries
        for key in keys_to_remove {
            self.entries.remove(&key);
        }

        let mut current_size = self.current_size.write().await;
        *current_size = current_size.saturating_sub(freed_space);
    }

    async fn run_eviction_task(self: Arc<Self>) {
        let mut ticker = interval(Duration::from_secs(60));
        
        loop {
            ticker.tick().await;
            
            // Remove expired entries
            let now = Instant::now();
            let mut expired_size = 0;
            
            self.entries.retain(|_, entry| {
                if entry.expires_at < now {
                    expired_size += entry.size;
                    false
                } else {
                    true
                }
            });

            if expired_size > 0 {
                let mut current_size = self.current_size.write().await;
                *current_size = current_size.saturating_sub(expired_size);
                debug!("Evicted {} bytes of expired entries", expired_size);
            }
        }
    }
}

/// Trait for remote cache implementations
#[async_trait]
pub trait RemoteCache: Send + Sync {
    /// Get a value from the remote cache
    async fn get(&self, key: &str) -> Result<Option<Bytes>>;
    
    /// Set a value in the remote cache
    async fn set(&self, key: String, value: Bytes, ttl: Duration) -> Result<()>;
    
    /// Delete a key from the remote cache
    async fn delete(&self, key: &str) -> Result<()>;
    
    /// Delete keys matching a pattern
    async fn delete_pattern(&self, pattern: &str) -> Result<()>;
    
    /// Flush all cache entries
    async fn flush(&self) -> Result<()>;
}

/// Redis-based remote cache implementation
pub struct RedisCache {
    client: redis::Client,
    key_prefix: String,
}

impl RedisCache {
    pub fn new(redis_url: &str, key_prefix: String) -> Result<Self> {
        let client = redis::Client::open(redis_url)
            .map_err(|e| Error::Generic(format!("Failed to connect to Redis: {}", e)))?;
        
        Ok(Self {
            client,
            key_prefix,
        })
    }

    fn make_key(&self, key: &str) -> String {
        format!("{}:{}", self.key_prefix, key)
    }
}

#[async_trait]
impl RemoteCache for RedisCache {
    async fn get(&self, key: &str) -> Result<Option<Bytes>> {
        let mut conn = self.client.get_async_connection().await
            .map_err(|e| Error::Generic(format!("Redis connection error: {}", e)))?;
        
        let full_key = self.make_key(key);
        let data: Option<Vec<u8>> = redis::cmd("GET")
            .arg(&full_key)
            .query_async(&mut conn)
            .await
            .map_err(|e| Error::Generic(format!("Redis GET error: {}", e)))?;
        
        Ok(data.map(Bytes::from))
    }

    async fn set(&self, key: String, value: Bytes, ttl: Duration) -> Result<()> {
        let mut conn = self.client.get_async_connection().await
            .map_err(|e| Error::Generic(format!("Redis connection error: {}", e)))?;
        
        let full_key = self.make_key(&key);
        redis::cmd("SETEX")
            .arg(&full_key)
            .arg(ttl.as_secs())
            .arg(value.as_ref())
            .query_async(&mut conn)
            .await
            .map_err(|e| Error::Generic(format!("Redis SET error: {}", e)))?;
        
        Ok(())
    }

    async fn delete(&self, key: &str) -> Result<()> {
        let mut conn = self.client.get_async_connection().await
            .map_err(|e| Error::Generic(format!("Redis connection error: {}", e)))?;
        
        let full_key = self.make_key(key);
        redis::cmd("DEL")
            .arg(&full_key)
            .query_async(&mut conn)
            .await
            .map_err(|e| Error::Generic(format!("Redis DEL error: {}", e)))?;
        
        Ok(())
    }

    async fn delete_pattern(&self, pattern: &str) -> Result<()> {
        let mut conn = self.client.get_async_connection().await
            .map_err(|e| Error::Generic(format!("Redis connection error: {}", e)))?;
        
        let full_pattern = self.make_key(pattern);
        
        // Use SCAN to find matching keys
        let mut cursor = 0;
        loop {
            let (new_cursor, keys): (i64, Vec<String>) = redis::cmd("SCAN")
                .arg(cursor)
                .arg("MATCH")
                .arg(&full_pattern)
                .arg("COUNT")
                .arg(100)
                .query_async(&mut conn)
                .await
                .map_err(|e| Error::Generic(format!("Redis SCAN error: {}", e)))?;
            
            // Delete found keys
            if !keys.is_empty() {
                redis::cmd("DEL")
                    .arg(&keys)
                    .query_async::<_, ()>(&mut conn)
                    .await
                    .map_err(|e| Error::Generic(format!("Redis DEL error: {}", e)))?;
            }
            
            cursor = new_cursor;
            if cursor == 0 {
                break;
            }
        }
        
        Ok(())
    }

    async fn flush(&self) -> Result<()> {
        // Don't flush entire Redis, just keys with our prefix
        self.delete_pattern("*").await
    }
}

/// Cache configuration
#[derive(Debug, Clone)]
pub struct CacheConfig {
    /// Maximum size of local cache in bytes
    pub local_cache_size: usize,
    /// Default TTL for cache entries
    pub default_ttl: Duration,
    /// Whether to enable distributed caching
    pub enable_distributed: bool,
    /// Redis URL for distributed caching
    pub redis_url: Option<String>,
    /// Key prefix for distributed cache
    pub cache_key_prefix: String,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            local_cache_size: 1024 * 1024 * 1024, // 1GB
            default_ttl: Duration::from_secs(3600), // 1 hour
            enable_distributed: false,
            redis_url: None,
            cache_key_prefix: "almanac".to_string(),
        }
    }
}

/// Cache statistics
#[derive(Debug, Clone, Default)]
pub struct CacheStats {
    pub total_requests: u64,
    pub local_hits: u64,
    pub remote_hits: u64,
    pub misses: u64,
    pub total_sets: u64,
    pub invalidations: usize,
}

impl CacheStats {
    pub fn hit_rate(&self) -> f64 {
        if self.total_requests == 0 {
            0.0
        } else {
            let hits = self.local_hits + self.remote_hits;
            hits as f64 / self.total_requests as f64
        }
    }

    pub fn local_hit_rate(&self) -> f64 {
        if self.total_requests == 0 {
            0.0
        } else {
            self.local_hits as f64 / self.total_requests as f64
        }
    }

    fn reset(&mut self) {
        *self = Self::default();
    }
}

/// Query result cache with automatic invalidation
pub struct QueryResultCache {
    cache: Arc<DistributedCache>,
    invalidation_rules: Arc<RwLock<Vec<InvalidationRule>>>,
}

#[derive(Clone)]
struct InvalidationRule {
    pattern: regex::Regex,
    dependencies: Vec<String>,
    ttl_override: Option<Duration>,
}

impl QueryResultCache {
    pub fn new(cache: Arc<DistributedCache>) -> Self {
        Self {
            cache,
            invalidation_rules: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Add an invalidation rule
    pub async fn add_invalidation_rule(
        &self,
        pattern: &str,
        dependencies: Vec<String>,
        ttl_override: Option<Duration>,
    ) -> Result<()> {
        let regex = regex::Regex::new(pattern)
            .map_err(|e| Error::Generic(format!("Invalid regex pattern: {}", e)))?;
        
        let rule = InvalidationRule {
            pattern: regex,
            dependencies,
            ttl_override,
        };

        self.invalidation_rules.write().await.push(rule);
        Ok(())
    }

    /// Cache a query result with automatic dependency tracking
    pub async fn cache_query_result<T>(
        &self,
        query_key: String,
        result: &T,
        base_ttl: Duration,
    ) -> Result<()>
    where
        T: Serialize + Send + Sync,
    {
        // Check for TTL overrides based on rules
        let rules = self.invalidation_rules.read().await;
        let ttl = rules.iter()
            .find(|rule| rule.pattern.is_match(&query_key))
            .and_then(|rule| rule.ttl_override)
            .unwrap_or(base_ttl);

        self.cache.set(query_key, result, Some(ttl)).await
    }

    /// Invalidate cache entries based on a change event
    pub async fn invalidate_on_change(&self, change_type: &str) -> Result<usize> {
        let rules = self.invalidation_rules.read().await;
        let mut total_invalidated = 0;

        for rule in rules.iter() {
            if rule.dependencies.contains(&change_type.to_string()) {
                let pattern = rule.pattern.as_str();
                let count = self.cache.invalidate_pattern(pattern).await?;
                total_invalidated += count;
                
                info!("Invalidated {} cache entries for pattern {} due to {} change",
                      count, pattern, change_type);
            }
        }

        Ok(total_invalidated)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_local_cache() {
        let cache = LocalCache::new(1024 * 1024); // 1MB
        
        // Test basic operations
        let data = Bytes::from("test data");
        cache.put("key1".to_string(), data.clone(), Duration::from_secs(60)).await;
        
        let entry = cache.get("key1").await.unwrap();
        assert_eq!(entry.data, data);
        
        // Test expiration
        cache.put("key2".to_string(), data.clone(), Duration::from_millis(10)).await;
        tokio::time::sleep(Duration::from_millis(20)).await;
        assert!(cache.get("key2").await.is_none());
    }

    #[tokio::test]
    async fn test_distributed_cache() {
        let config = CacheConfig::default();
        let cache = DistributedCache::new(config);
        
        // Test set and get
        #[derive(Serialize, Deserialize, PartialEq, Debug)]
        struct TestData {
            id: u64,
            name: String,
        }
        
        let data = TestData {
            id: 42,
            name: "test".to_string(),
        };
        
        cache.set("test_key".to_string(), &data, None).await.unwrap();
        let retrieved: TestData = cache.get("test_key").await.unwrap().unwrap();
        assert_eq!(data, retrieved);
        
        // Test cache stats
        let stats = cache.stats().await;
        assert_eq!(stats.total_requests, 1);
        assert_eq!(stats.local_hits, 1);
    }

    #[tokio::test]
    async fn test_cache_eviction() {
        let cache = LocalCache::new(100); // Very small cache
        
        // Fill cache beyond capacity
        for i in 0..20 {
            let data = Bytes::from(vec![0u8; 10]); // 10 bytes each
            cache.put(format!("key{}", i), data, Duration::from_secs(60)).await;
        }
        
        // Check that size is within limits
        let current_size = *cache.current_size.read().await;
        assert!(current_size <= 100);
        
        // Check that some entries were evicted
        let entry_count = cache.entries.len();
        assert!(entry_count < 20);
    }
}