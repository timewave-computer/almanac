/// Query result caching functionality using Redis
use std::collections::HashMap;
use std::time::{Duration, SystemTime};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use async_trait::async_trait;

use crate::event::Event;
use crate::types::{EventFilter, AggregationResult};
use crate::{Result, Error};

/// Cache key types for different query patterns
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum CacheKeyType {
    /// Event query results cache
    Events,
    /// Aggregation query results cache
    Aggregation,
    /// Full-text search results cache
    TextSearch,
    /// Correlation query results cache
    Correlation,
    /// Index usage statistics cache
    IndexStats,
}

/// Cache configuration
#[derive(Debug, Clone)]
pub struct CacheConfig {
    /// Redis connection URL
    pub redis_url: String,
    
    /// Default TTL for cache entries
    pub default_ttl: Duration,
    
    /// TTL by cache key type
    pub type_ttls: HashMap<CacheKeyType, Duration>,
    
    /// Maximum cache key length
    pub max_key_length: usize,
    
    /// Cache namespace prefix
    pub namespace: String,
    
    /// Enable cache compression
    pub compression: bool,
    
    /// Maximum cached item size (bytes)
    pub max_item_size: usize,
}

impl Default for CacheConfig {
    fn default() -> Self {
        let mut type_ttls = HashMap::new();
        type_ttls.insert(CacheKeyType::Events, Duration::from_secs(300)); // 5 minutes
        type_ttls.insert(CacheKeyType::Aggregation, Duration::from_secs(600)); // 10 minutes
        type_ttls.insert(CacheKeyType::TextSearch, Duration::from_secs(180)); // 3 minutes
        type_ttls.insert(CacheKeyType::Correlation, Duration::from_secs(900)); // 15 minutes
        type_ttls.insert(CacheKeyType::IndexStats, Duration::from_secs(3600)); // 1 hour
        
        Self {
            redis_url: "redis://localhost:6379".to_string(),
            default_ttl: Duration::from_secs(300),
            type_ttls,
            max_key_length: 250,
            namespace: "almanac".to_string(),
            compression: true,
            max_item_size: 1024 * 1024, // 1MB
        }
    }
}

/// Cache statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStats {
    /// Total cache hits
    pub hits: u64,
    
    /// Total cache misses  
    pub misses: u64,
    
    /// Hit rate percentage
    pub hit_rate: f64,
    
    /// Number of cached items
    pub item_count: u64,
    
    /// Total cache size in bytes
    pub total_size: u64,
    
    /// Memory usage percentage
    pub memory_usage: f64,
    
    /// Cache statistics by type
    pub by_type: HashMap<CacheKeyType, TypeStats>,
}

/// Statistics for a specific cache type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeStats {
    /// Hits for this type
    pub hits: u64,
    
    /// Misses for this type
    pub misses: u64,
    
    /// Number of items of this type
    pub item_count: u64,
    
    /// Total size for this type
    pub size: u64,
    
    /// Average TTL for this type
    pub avg_ttl: Duration,
}

/// Cache entry metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntry<T> {
    /// Cached data
    pub data: T,
    
    /// Entry creation timestamp
    pub created_at: SystemTime,
    
    /// Entry expiration timestamp
    pub expires_at: SystemTime,
    
    /// Cache key type
    pub key_type: CacheKeyType,
    
    /// Entry size in bytes
    pub size: usize,
    
    /// Number of hits for this entry
    pub hits: u64,
}

/// Cache operations trait
#[async_trait]
pub trait Cache: Send + Sync {
    /// Get a cached item by key
    async fn get<T>(&self, key: &str) -> Result<Option<T>>
    where
        T: DeserializeOwned + Send;
    
    /// Store an item in cache with TTL
    async fn set<T>(&self, key: &str, value: &T, ttl: Option<Duration>) -> Result<()>
    where
        T: Serialize + Send + Sync;
    
    /// Delete an item from cache
    async fn delete(&self, key: &str) -> Result<bool>;
    
    /// Check if a key exists in cache
    async fn exists(&self, key: &str) -> Result<bool>;
    
    /// Set TTL for an existing key
    async fn expire(&self, key: &str, ttl: Duration) -> Result<bool>;
    
    /// Get multiple items by keys
    async fn get_many<T>(&self, keys: &[String]) -> Result<Vec<Option<T>>>
    where
        T: DeserializeOwned + Send;
    
    /// Store multiple items with TTL
    async fn set_many<T>(&self, items: &[(String, T)], ttl: Option<Duration>) -> Result<()>
    where
        T: Serialize + Send + Sync;
    
    /// Clear all cache entries
    async fn clear(&self) -> Result<u64>;
    
    /// Clear cache entries by pattern
    async fn clear_pattern(&self, pattern: &str) -> Result<u64>;
    
    /// Get cache statistics
    async fn stats(&self) -> Result<CacheStats>;
    
    /// Get cache health information
    async fn health_check(&self) -> Result<bool>;
}

/// Redis-based cache implementation
pub struct RedisCache {
    client: redis::Client,
    config: CacheConfig,
    stats: tokio::sync::RwLock<CacheStats>,
}

impl RedisCache {
    /// Create a new Redis cache instance
    pub async fn new(config: CacheConfig) -> Result<Self> {
        let client = redis::Client::open(config.redis_url.as_str())
            .map_err(|e| Error::Database(format!("Failed to create Redis client: {}", e)))?;
        
        // Test connection
        let mut conn = client.get_async_connection().await
            .map_err(|e| Error::Database(format!("Failed to connect to Redis: {}", e)))?;
        
        redis::cmd("PING").query_async::<_, String>(&mut conn).await
            .map_err(|e| Error::Database(format!("Redis ping failed: {}", e)))?;
        
        let stats = CacheStats {
            hits: 0,
            misses: 0,
            hit_rate: 0.0,
            item_count: 0,
            total_size: 0,
            memory_usage: 0.0,
            by_type: HashMap::new(),
        };
        
        Ok(Self {
            client,
            config,
            stats: tokio::sync::RwLock::new(stats),
        })
    }
    
    /// Generate a cache key with namespace and hash
    fn make_key(&self, key: &str, key_type: CacheKeyType) -> String {
        let type_prefix = match key_type {
            CacheKeyType::Events => "evt",
            CacheKeyType::Aggregation => "agg",
            CacheKeyType::TextSearch => "txt",
            CacheKeyType::Correlation => "cor",
            CacheKeyType::IndexStats => "idx",
        };
        
        let full_key = format!("{}:{}:{}", self.config.namespace, type_prefix, key);
        
        // Hash key if it's too long
        if full_key.len() > self.config.max_key_length {
            let hash = format!("{:x}", md5::compute(full_key.as_bytes()));
            format!("{}:{}:h:{}", self.config.namespace, type_prefix, hash)
        } else {
            full_key
        }
    }
    
    /// Get TTL for a cache key type
    #[allow(dead_code)]
    fn get_ttl(&self, key_type: CacheKeyType) -> Duration {
        self.config.type_ttls.get(&key_type)
            .copied()
            .unwrap_or(self.config.default_ttl)
    }
    
    /// Serialize and optionally compress data
    fn serialize_data<T>(&self, data: &T) -> Result<Vec<u8>>
    where
        T: Serialize,
    {
        let json_bytes = serde_json::to_vec(data)
            .map_err(Error::Serialization)?;
        
        if self.config.compression && json_bytes.len() > 256 {
            // Use gzip compression for larger items
            use std::io::Write;
            let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
            encoder.write_all(&json_bytes)
                .map_err(Error::IO)?;
            encoder.finish()
                .map_err(Error::IO)
        } else {
            Ok(json_bytes)
        }
    }
    
    /// Deserialize and optionally decompress data
    fn deserialize_data<T>(&self, data: &[u8]) -> Result<T>
    where
        T: DeserializeOwned,
    {
        // Try to detect if data is compressed (gzip magic number)
        let json_bytes = if data.len() >= 2 && data[0] == 0x1f && data[1] == 0x8b {
            // Decompress gzip data
            use std::io::Read;
            let mut decoder = flate2::read::GzDecoder::new(data);
            let mut decompressed = Vec::new();
            decoder.read_to_end(&mut decompressed)
                .map_err(Error::IO)?;
            decompressed
        } else {
            data.to_vec()
        };
        
        serde_json::from_slice(&json_bytes)
            .map_err(Error::Serialization)
    }
    
    /// Update cache statistics
    async fn update_stats(&self, key_type: CacheKeyType, hit: bool, size: Option<usize>) {
        let mut stats = self.stats.write().await;
        
        if hit {
            stats.hits += 1;
        } else {
            stats.misses += 1;
        }
        
        stats.hit_rate = if stats.hits + stats.misses > 0 {
            stats.hits as f64 / (stats.hits + stats.misses) as f64 * 100.0
        } else {
            0.0
        };
        
        let type_stats = stats.by_type.entry(key_type).or_insert(TypeStats {
            hits: 0,
            misses: 0,
            item_count: 0,
            size: 0,
            avg_ttl: Duration::from_secs(0),
        });
        
        if hit {
            type_stats.hits += 1;
        } else {
            type_stats.misses += 1;
        }
        
        if let Some(size) = size {
            if !hit {
                type_stats.item_count += 1;
                type_stats.size += size as u64;
                stats.total_size += size as u64;
                stats.item_count += 1;
            }
        }
    }
}

#[async_trait]
impl Cache for RedisCache {
    async fn get<T>(&self, key: &str) -> Result<Option<T>>
    where
        T: DeserializeOwned + Send,
    {
        let cache_key = self.make_key(key, CacheKeyType::Events);
        let mut conn = self.client.get_async_connection().await
            .map_err(|e| Error::Database(format!("Redis connection failed: {}", e)))?;
        
        match redis::cmd("GET").arg(&cache_key).query_async::<_, Option<Vec<u8>>>(&mut conn).await {
            Ok(Some(data)) => {
                self.update_stats(CacheKeyType::Events, true, None).await;
                let result = self.deserialize_data(&data)?;
                Ok(Some(result))
            }
            Ok(None) => {
                self.update_stats(CacheKeyType::Events, false, None).await;
                Ok(None)
            }
            Err(e) => Err(Error::Database(format!("Redis GET failed: {}", e))),
        }
    }
    
    async fn set<T>(&self, key: &str, value: &T, ttl: Option<Duration>) -> Result<()>
    where
        T: Serialize + Send + Sync,
    {
        let cache_key = self.make_key(key, CacheKeyType::Events);
        let data = self.serialize_data(value)?;
        
        if data.len() > self.config.max_item_size {
            return Err(Error::Database(format!("Cache item too large: {} bytes", data.len())));
        }
        
        let mut conn = self.client.get_async_connection().await
            .map_err(|e| Error::Database(format!("Redis connection failed: {}", e)))?;
        
        let ttl_secs = ttl.unwrap_or(self.config.default_ttl).as_secs();
        
        redis::cmd("SETEX")
            .arg(&cache_key)
            .arg(ttl_secs)
            .arg(&data)
            .query_async::<_, ()>(&mut conn)
            .await
            .map_err(|e| Error::Database(format!("Redis SETEX failed: {}", e)))?;
        
        self.update_stats(CacheKeyType::Events, false, Some(data.len())).await;
        Ok(())
    }
    
    async fn delete(&self, key: &str) -> Result<bool> {
        let cache_key = self.make_key(key, CacheKeyType::Events);
        let mut conn = self.client.get_async_connection().await
            .map_err(|e| Error::Database(format!("Redis connection failed: {}", e)))?;
        
        let deleted: i32 = redis::cmd("DEL")
            .arg(&cache_key)
            .query_async(&mut conn)
            .await
            .map_err(|e| Error::Database(format!("Redis DEL failed: {}", e)))?;
        
        Ok(deleted > 0)
    }
    
    async fn exists(&self, key: &str) -> Result<bool> {
        let cache_key = self.make_key(key, CacheKeyType::Events);
        let mut conn = self.client.get_async_connection().await
            .map_err(|e| Error::Database(format!("Redis connection failed: {}", e)))?;
        
        let exists: i32 = redis::cmd("EXISTS")
            .arg(&cache_key)
            .query_async(&mut conn)
            .await
            .map_err(|e| Error::Database(format!("Redis EXISTS failed: {}", e)))?;
        
        Ok(exists > 0)
    }
    
    async fn expire(&self, key: &str, ttl: Duration) -> Result<bool> {
        let cache_key = self.make_key(key, CacheKeyType::Events);
        let mut conn = self.client.get_async_connection().await
            .map_err(|e| Error::Database(format!("Redis connection failed: {}", e)))?;
        
        let result: i32 = redis::cmd("EXPIRE")
            .arg(&cache_key)
            .arg(ttl.as_secs())
            .query_async(&mut conn)
            .await
            .map_err(|e| Error::Database(format!("Redis EXPIRE failed: {}", e)))?;
        
        Ok(result > 0)
    }
    
    async fn get_many<T>(&self, keys: &[String]) -> Result<Vec<Option<T>>>
    where
        T: DeserializeOwned + Send,
    {
        if keys.is_empty() {
            return Ok(vec![]);
        }
        
        let cache_keys: Vec<String> = keys.iter()
            .map(|k| self.make_key(k, CacheKeyType::Events))
            .collect();
        
        let mut conn = self.client.get_async_connection().await
            .map_err(|e| Error::Database(format!("Redis connection failed: {}", e)))?;
        
        let results: Vec<Option<Vec<u8>>> = redis::cmd("MGET")
            .arg(&cache_keys)
            .query_async(&mut conn)
            .await
            .map_err(|e| Error::Database(format!("Redis MGET failed: {}", e)))?;
        
        let mut deserialized = Vec::new();
        for result in results {
            match result {
                Some(data) => {
                    self.update_stats(CacheKeyType::Events, true, None).await;
                    let item = self.deserialize_data(&data)?;
                    deserialized.push(Some(item));
                }
                None => {
                    self.update_stats(CacheKeyType::Events, false, None).await;
                    deserialized.push(None);
                }
            }
        }
        
        Ok(deserialized)
    }
    
    async fn set_many<T>(&self, items: &[(String, T)], ttl: Option<Duration>) -> Result<()>
    where
        T: Serialize + Send + Sync,
    {
        if items.is_empty() {
            return Ok(());
        }
        
        let mut conn = self.client.get_async_connection().await
            .map_err(|e| Error::Database(format!("Redis connection failed: {}", e)))?;
        
        let ttl_secs = ttl.unwrap_or(self.config.default_ttl).as_secs();
        
        for (key, value) in items {
            let cache_key = self.make_key(key, CacheKeyType::Events);
            let data = self.serialize_data(value)?;
            
            if data.len() <= self.config.max_item_size {
                redis::cmd("SETEX")
                    .arg(&cache_key)
                    .arg(ttl_secs)
                    .arg(&data)
                    .query_async::<_, ()>(&mut conn)
                    .await
                    .map_err(|e| Error::Database(format!("Redis SETEX failed: {}", e)))?;
                
                self.update_stats(CacheKeyType::Events, false, Some(data.len())).await;
            }
        }
        
        Ok(())
    }
    
    async fn clear(&self) -> Result<u64> {
        let pattern = format!("{}:*", self.config.namespace);
        self.clear_pattern(&pattern).await
    }
    
    async fn clear_pattern(&self, pattern: &str) -> Result<u64> {
        let mut conn = self.client.get_async_connection().await
            .map_err(|e| Error::Database(format!("Redis connection failed: {}", e)))?;
        
        // Get keys matching pattern
        let keys: Vec<String> = redis::cmd("KEYS")
            .arg(pattern)
            .query_async(&mut conn)
            .await
            .map_err(|e| Error::Database(format!("Redis KEYS failed: {}", e)))?;
        
        if keys.is_empty() {
            return Ok(0);
        }
        
        // Delete keys in batches
        let batch_size = 1000;
        let mut total_deleted = 0u64;
        
        for chunk in keys.chunks(batch_size) {
            let deleted: i32 = redis::cmd("DEL")
                .arg(chunk)
                .query_async(&mut conn)
                .await
                .map_err(|e| Error::Database(format!("Redis DEL failed: {}", e)))?;
            total_deleted += deleted as u64;
        }
        
        Ok(total_deleted)
    }
    
    async fn stats(&self) -> Result<CacheStats> {
        Ok(self.stats.read().await.clone())
    }
    
    async fn health_check(&self) -> Result<bool> {
        let mut conn = self.client.get_async_connection().await
            .map_err(|e| Error::Database(format!("Redis connection failed: {}", e)))?;
        
        match redis::cmd("PING").query_async::<_, String>(&mut conn).await {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }
}

/// Query cache that wraps storage operations with caching
pub struct QueryCache<T: Cache> {
    cache: T,
    config: CacheConfig,
}

impl<T: Cache> QueryCache<T> {
    /// Create a new query cache
    pub fn new(cache: T, config: CacheConfig) -> Self {
        Self { cache, config }
    }
    
    /// Generate cache key for event queries
    pub fn event_query_key(&self, filter: &EventFilter, limit: Option<usize>, offset: Option<usize>) -> String {
        let filter_hash = format!("{:x}", md5::compute(serde_json::to_string(filter).unwrap_or_default()));
        format!("events:{}:{}:{}", filter_hash, limit.unwrap_or(0), offset.unwrap_or(0))
    }
    
    /// Generate cache key for aggregation queries
    pub fn aggregation_key(&self, query_hash: &str) -> String {
        format!("aggregation:{}", query_hash)
    }
    
    /// Generate cache key for text search queries
    pub fn text_search_key(&self, query: &str, filters: &str) -> String {
        let combined = format!("{}:{}", query, filters);
        let hash = format!("{:x}", md5::compute(combined));
        format!("text_search:{}", hash)
    }
    
    /// Cache event query results
    pub async fn cache_events(&self, key: &str, events: &[Box<dyn Event>]) -> Result<()> {
        // Convert to serializable format
        let serializable_events: Vec<serde_json::Value> = events.iter()
            .map(|e| serde_json::json!({
                "id": e.id(),
                "chain": e.chain(),
                "block_number": e.block_number(),
                "block_hash": e.block_hash(),
                "tx_hash": e.tx_hash(),
                "timestamp": e.timestamp().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs(),
                "event_type": e.event_type(),
                "raw_data": e.raw_data(),
            }))
            .collect();
        
        let ttl = self.config.type_ttls.get(&CacheKeyType::Events).copied();
        self.cache.set(key, &serializable_events, ttl).await
    }
    
    /// Get cached event query results
    pub async fn get_cached_events(&self, key: &str) -> Result<Option<Vec<serde_json::Value>>> {
        self.cache.get(key).await
    }
    
    /// Cache aggregation results
    pub async fn cache_aggregation(&self, key: &str, results: &Vec<AggregationResult>) -> Result<()> {
        let ttl = self.config.type_ttls.get(&CacheKeyType::Aggregation).copied();
        self.cache.set(key, results, ttl).await
    }
    
    /// Get cached aggregation results
    pub async fn get_cached_aggregation(&self, key: &str) -> Result<Option<Vec<AggregationResult>>> {
        self.cache.get(key).await
    }
    
    /// Invalidate cache entries by pattern
    pub async fn invalidate_pattern(&self, pattern: &str) -> Result<u64> {
        self.cache.clear_pattern(pattern).await
    }
    
    /// Get cache statistics
    pub async fn statistics(&self) -> Result<CacheStats> {
        self.cache.stats().await
    }
    
    /// Perform cache health check
    pub async fn health(&self) -> Result<bool> {
        self.cache.health_check().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    
    // Mock cache for testing
    struct MockCache {
        data: tokio::sync::RwLock<HashMap<String, Vec<u8>>>,
        stats: tokio::sync::RwLock<CacheStats>,
    }
    
    impl MockCache {
        fn new() -> Self {
            Self {
                data: tokio::sync::RwLock::new(HashMap::new()),
                stats: tokio::sync::RwLock::new(CacheStats {
                    hits: 0,
                    misses: 0,
                    hit_rate: 0.0,
                    item_count: 0,
                    total_size: 0,
                    memory_usage: 0.0,
                    by_type: HashMap::new(),
                }),
            }
        }
    }
    
    #[async_trait]
    impl Cache for MockCache {
        async fn get<T>(&self, key: &str) -> Result<Option<T>>
        where
            T: DeserializeOwned + Send,
        {
            let data = self.data.read().await;
            match data.get(key) {
                Some(bytes) => {
                    let mut stats = self.stats.write().await;
                    stats.hits += 1;
                    let value = serde_json::from_slice(bytes)?;
                    Ok(Some(value))
                }
                None => {
                    let mut stats = self.stats.write().await;
                    stats.misses += 1;
                    Ok(None)
                }
            }
        }
        
        async fn set<T>(&self, key: &str, value: &T, _ttl: Option<Duration>) -> Result<()>
        where
            T: Serialize + Send + Sync,
        {
            let bytes = serde_json::to_vec(value)?;
            let mut data = self.data.write().await;
            data.insert(key.to_string(), bytes.clone());
            
            let mut stats = self.stats.write().await;
            stats.item_count += 1;
            stats.total_size += bytes.len() as u64;
            
            Ok(())
        }
        
        async fn delete(&self, key: &str) -> Result<bool> {
            let mut data = self.data.write().await;
            Ok(data.remove(key).is_some())
        }
        
        async fn exists(&self, key: &str) -> Result<bool> {
            let data = self.data.read().await;
            Ok(data.contains_key(key))
        }
        
        async fn expire(&self, _key: &str, _ttl: Duration) -> Result<bool> {
            Ok(true)
        }
        
        async fn get_many<T>(&self, keys: &[String]) -> Result<Vec<Option<T>>>
        where
            T: DeserializeOwned + Send,
        {
            let mut results = Vec::new();
            for key in keys {
                results.push(self.get(key).await?);
            }
            Ok(results)
        }
        
        async fn set_many<T>(&self, items: &[(String, T)], ttl: Option<Duration>) -> Result<()>
        where
            T: Serialize + Send + Sync,
        {
            for (key, value) in items {
                self.set(key, value, ttl).await?;
            }
            Ok(())
        }
        
        async fn clear(&self) -> Result<u64> {
            let mut data = self.data.write().await;
            let count = data.len() as u64;
            data.clear();
            Ok(count)
        }
        
        async fn clear_pattern(&self, pattern: &str) -> Result<u64> {
            let mut data = self.data.write().await;
            let keys_to_remove: Vec<String> = data.keys()
                .filter(|k| k.contains(&pattern.replace("*", "")))
                .cloned()
                .collect();
            
            let count = keys_to_remove.len() as u64;
            for key in keys_to_remove {
                data.remove(&key);
            }
            Ok(count)
        }
        
        async fn stats(&self) -> Result<CacheStats> {
            Ok(self.stats.read().await.clone())
        }
        
        async fn health_check(&self) -> Result<bool> {
            Ok(true)
        }
    }
    
    #[tokio::test]
    async fn test_cache_basic_operations() {
        let cache = MockCache::new();
        
        // Test set and get
        let test_data = vec!["item1", "item2", "item3"];
        cache.set("test_key", &test_data, None).await.unwrap();
        
        let retrieved: Option<Vec<String>> = cache.get("test_key").await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap(), test_data);
        
        // Test exists
        assert!(cache.exists("test_key").await.unwrap());
        assert!(!cache.exists("nonexistent").await.unwrap());
        
        // Test delete
        assert!(cache.delete("test_key").await.unwrap());
        assert!(!cache.exists("test_key").await.unwrap());
    }
    
    #[tokio::test]
    async fn test_cache_stats() {
        let cache = MockCache::new();
        
        // Perform some operations
        cache.set("key1", &"value1", None).await.unwrap();
        cache.set("key2", &"value2", None).await.unwrap();
        
        let _: Option<String> = cache.get("key1").await.unwrap(); // Hit
        let _: Option<String> = cache.get("key3").await.unwrap(); // Miss
        
        let stats = cache.stats().await.unwrap();
        assert_eq!(stats.hits, 1);
        assert_eq!(stats.misses, 1);
        assert_eq!(stats.item_count, 2);
    }
    
    #[tokio::test]
    async fn test_query_cache() {
        let mock_cache = MockCache::new();
        let config = CacheConfig::default();
        let query_cache = QueryCache::new(mock_cache, config);
        
        // Test event query key generation
        let filter = EventFilter::new();
        let key = query_cache.event_query_key(&filter, Some(100), Some(0));
        assert!(key.starts_with("events:"));
        
        // Test aggregation key generation
        let agg_key = query_cache.aggregation_key("test_hash");
        assert_eq!(agg_key, "aggregation:test_hash");
        
        // Test text search key generation
        let search_key = query_cache.text_search_key("search query", "filters");
        assert!(search_key.starts_with("text_search:"));
    }
    
    #[test]
    fn test_cache_config_default() {
        let config = CacheConfig::default();
        
        assert_eq!(config.namespace, "almanac");
        assert_eq!(config.default_ttl, Duration::from_secs(300));
        assert_eq!(config.max_key_length, 250);
        assert!(config.compression);
        assert_eq!(config.max_item_size, 1024 * 1024);
        
        // Check type-specific TTLs
        assert_eq!(config.type_ttls.get(&CacheKeyType::Events), Some(&Duration::from_secs(300)));
        assert_eq!(config.type_ttls.get(&CacheKeyType::Aggregation), Some(&Duration::from_secs(600)));
    }
} 