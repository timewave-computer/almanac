/// Query result caching functionality using Redis
use almanac_infrastructure_core::{Result, Error};

use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use serde::de::DeserializeOwned;
use std::time::{Duration, SystemTime};

/// Cache key types for different query patterns
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CacheKey {
    /// Event query key
    EventQuery {
        chain_id: String,
        contract_address: Option<String>,
        event_type: Option<String>,
        block_range: Option<(u64, u64)>,
    },
    /// Aggregation query key
    AggregationQuery {
        base_filter: String,
        aggregation_type: String,
        time_window: Option<Duration>,
    },
    /// Custom query key
    Custom(String),
}

/// Simple cache implementation for query results
pub struct SimpleCache {
    /// In-memory cache for small results
    memory_cache: HashMap<String, CacheEntry>,
    /// Maximum memory cache size
    max_memory_size: usize,
    /// Current memory usage
    current_memory_usage: usize,
}

#[derive(Debug, Clone)]
struct CacheEntry {
    data: Vec<u8>,
    expires_at: SystemTime,
    last_accessed: SystemTime,
    access_count: u64,
}

impl SimpleCache {
    pub fn new(max_memory_size: usize) -> Self {
        Self {
            memory_cache: HashMap::new(),
            max_memory_size,
            current_memory_usage: 0,
        }
    }

    pub async fn get<T: DeserializeOwned>(&mut self, key: &str) -> Result<Option<T>> {
        // Check memory cache first
        if let Some(entry) = self.memory_cache.get_mut(key) {
            if entry.expires_at > SystemTime::now() {
                entry.last_accessed = SystemTime::now();
                entry.access_count += 1;
                
                let data: T = serde_json::from_slice(&entry.data)
                    .map_err(|e| Error::deserialization(e.to_string()))?;
                return Ok(Some(data));
            } else {
                // Entry expired, remove it
                self.memory_cache.remove(key);
            }
        }

        Ok(None)
    }

    pub async fn set<T: Serialize>(&mut self, key: &str, value: &T, ttl: Duration) -> Result<()> {
        let data = serde_json::to_vec(value)
            .map_err(|e| Error::serialization(e.to_string()))?;
        
        let entry = CacheEntry {
            data: data.clone(),
            expires_at: SystemTime::now() + ttl,
            last_accessed: SystemTime::now(),
            access_count: 1,
        };

        // Check if we need to evict some entries
        if self.current_memory_usage + data.len() > self.max_memory_size {
            self.evict_lru(data.len()).await;
        }

        // Insert new entry
        if let Some(old_entry) = self.memory_cache.insert(key.to_string(), entry) {
            self.current_memory_usage -= old_entry.data.len();
        }
        self.current_memory_usage += data.len();

        Ok(())
    }

    pub async fn remove(&mut self, key: &str) -> Result<bool> {
        if let Some(entry) = self.memory_cache.remove(key) {
            self.current_memory_usage -= entry.data.len();
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub async fn clear(&mut self) -> Result<()> {
        self.memory_cache.clear();
        self.current_memory_usage = 0;
        Ok(())
    }

    async fn evict_lru(&mut self, needed_space: usize) {
        // Collect entries sorted by last access time
        let mut entries: Vec<(String, SystemTime, usize)> = self.memory_cache
            .iter()
            .map(|(k, v)| (k.clone(), v.last_accessed, v.data.len()))
            .collect();
        
        entries.sort_by(|a, b| a.1.cmp(&b.1));

        let mut freed_space = 0;
        for (key, _, size) in entries {
            if freed_space >= needed_space {
                break;
            }
            
            self.memory_cache.remove(&key);
            self.current_memory_usage -= size;
            freed_space += size;
        }
    }

    pub fn cache_stats(&self) -> CacheStats {
        CacheStats {
            total_entries: self.memory_cache.len(),
            memory_usage: self.current_memory_usage,
            max_memory: self.max_memory_size,
            hit_ratio: 0.0, // Would need to track hits/misses
        }
    }
}

/// Cache statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStats {
    pub total_entries: usize,
    pub memory_usage: usize,
    pub max_memory: usize,
    pub hit_ratio: f64,
}

#[cfg(feature = "redis")]
pub struct RedisQueryCache {
    client: redis::Client,
    default_ttl: Duration,
    key_prefix: String,
}

#[cfg(feature = "redis")]
impl RedisQueryCache {
    pub fn new(config: &RedisCacheConfig) -> Result<Self> {
        let client = redis::Client::open(config.redis_url.as_str())
            .map_err(|e| Error::generic(format!("Failed to connect to Redis: {}", e)))?;

        Ok(Self {
            client,
            default_ttl: config.default_ttl,
            key_prefix: config.key_prefix.clone(),
        })
    }

    async fn _test_connection(&self) -> Result<()> {
        let mut conn = self.client.get_multiplexed_async_connection().await
            .map_err(|e| Error::generic(format!("Redis connection failed: {}", e)))?;

        redis::cmd("PING").query_async::<_, String>(&mut conn).await
            .map_err(|e| Error::generic(format!("Redis ping failed: {}", e)))?;

        Ok(())
    }

    fn make_cache_key(&self, key: &str) -> String {
        format!("{}:{}", self.key_prefix, key)
    }

    pub async fn get<T: DeserializeOwned>(&self, key: &str) -> Result<Option<T>> {
        let mut conn = self.client.get_multiplexed_async_connection().await
            .map_err(|e| Error::generic(format!("Redis connection failed: {}", e)))?;

        let cache_key = self.make_cache_key(key);
        
        match redis::cmd("GET").arg(&cache_key).query_async::<_, Option<Vec<u8>>>(&mut conn).await {
            Ok(Some(data)) => {
                let result: T = serde_json::from_slice(&data)
                    .map_err(|e| Error::deserialization(e.to_string()))?;
                Ok(Some(result))
            },
            Ok(None) => {
                Ok(None)
            },
            Err(e) => Err(Error::generic(format!("Redis GET failed: {}", e))),
        }
    }

    pub async fn set<T: Serialize>(&self, key: &str, value: &T, ttl: Option<Duration>) -> Result<()> {
        let mut conn = self.client.get_multiplexed_async_connection().await
            .map_err(|e| Error::generic(format!("Redis connection failed: {}", e)))?;

        let cache_key = self.make_cache_key(key);
        let data = serde_json::to_vec(value)
            .map_err(|e| Error::serialization(e.to_string()))?;
        
        let ttl = ttl.unwrap_or(self.default_ttl);
        
        redis::cmd("SETEX")
            .arg(&cache_key)
            .arg(ttl.as_secs() as usize)
            .arg(&data)
            .query_async::<_, ()>(&mut conn)
            .await
            .map_err(|e| Error::generic(format!("Redis SETEX failed: {}", e)))?;

        Ok(())
    }
}

#[cfg(feature = "redis")]
#[derive(Debug, Clone)]
pub struct RedisCacheConfig {
    pub redis_url: String,
    pub default_ttl: Duration,
    pub key_prefix: String,
}

 