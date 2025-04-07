// performance.rs - Query performance optimization for the indexer
//
// Purpose: Provides caching, query routing, and pagination mechanisms
// to improve query performance and user experience

use indexer_core::Error;
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use std::sync::Arc;

/// Cache entry with expiration time
struct CacheEntry<T> {
    /// Cached data
    data: T,
    
    /// When the entry was added to the cache
    inserted_at: Instant,
    
    /// Time-to-live for this entry
    ttl: Duration,
}

impl<T> CacheEntry<T> {
    /// Check if this cache entry has expired
    fn is_expired(&self) -> bool {
        self.inserted_at.elapsed() > self.ttl
    }
}

/// Query cache for storing and retrieving query results
pub struct QueryCache<K, V> 
where 
    K: std::hash::Hash + Eq + Clone + Send + Sync + 'static,
    V: Clone + Send + Sync + 'static
{
    /// Cache storage
    cache: Arc<RwLock<HashMap<K, CacheEntry<V>>>>,
    
    /// Default TTL for cache entries
    default_ttl: Duration,
    
    /// Maximum number of entries in the cache
    max_entries: usize,
}

impl<K, V> QueryCache<K, V> 
where 
    K: std::hash::Hash + Eq + Clone + Send + Sync + 'static,
    V: Clone + Send + Sync + 'static
{
    /// Create a new query cache
    pub fn new(default_ttl: Duration, max_entries: usize) -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
            default_ttl,
            max_entries,
        }
    }
    
    /// Get a value from the cache
    pub async fn get(&self, key: &K) -> Option<V> {
        let cache = self.cache.read().await;
        
        if let Some(entry) = cache.get(key) {
            if !entry.is_expired() {
                return Some(entry.data.clone());
            }
        }
        
        None
    }
    
    /// Put a value in the cache
    pub async fn put(&self, key: K, value: V) -> Result<(), Error> {
        let mut cache = self.cache.write().await;
        
        // Check if we need to evict entries
        if cache.len() >= self.max_entries && !cache.contains_key(&key) {
            // Evict the oldest entry
            self.evict_oldest(&mut cache).await?;
        }
        
        // Add the new entry
        cache.insert(key, CacheEntry {
            data: value,
            inserted_at: Instant::now(),
            ttl: self.default_ttl,
        });
        
        Ok(())
    }
    
    /// Put a value in the cache with a custom TTL
    pub async fn put_with_ttl(&self, key: K, value: V, ttl: Duration) -> Result<(), Error> {
        let mut cache = self.cache.write().await;
        
        // Check if we need to evict entries
        if cache.len() >= self.max_entries && !cache.contains_key(&key) {
            // Evict the oldest entry
            self.evict_oldest(&mut cache).await?;
        }
        
        // Add the new entry
        cache.insert(key, CacheEntry {
            data: value,
            inserted_at: Instant::now(),
            ttl,
        });
        
        Ok(())
    }
    
    /// Remove a value from the cache
    pub async fn remove(&self, key: &K) -> Result<(), Error> {
        let mut cache = self.cache.write().await;
        cache.remove(key);
        Ok(())
    }
    
    /// Clear the cache
    pub async fn clear(&self) -> Result<(), Error> {
        let mut cache = self.cache.write().await;
        cache.clear();
        Ok(())
    }
    
    /// Evict expired entries
    pub async fn evict_expired(&self) -> Result<usize, Error> {
        let mut cache = self.cache.write().await;
        let initial_size = cache.len();
        
        // Remove expired entries
        cache.retain(|_, entry| !entry.is_expired());
        
        Ok(initial_size - cache.len())
    }
    
    /// Evict the oldest entry from the cache
    async fn evict_oldest(&self, cache: &mut HashMap<K, CacheEntry<V>>) -> Result<(), Error> {
        // Find the oldest entry
        let oldest_key = cache
            .iter()
            .min_by_key(|(_, entry)| entry.inserted_at)
            .map(|(key, _)| key.clone());
        
        // Remove it
        if let Some(key) = oldest_key {
            cache.remove(&key);
        }
        
        Ok(())
    }
}

/// Cursor for paginated results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Cursor {
    /// Encoded cursor value
    pub value: String,
    
    /// Whether this is the last page
    pub is_last: bool,
}

/// Paginated query result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginatedResult<T> {
    /// List of items in this page
    pub items: Vec<T>,
    
    /// Total number of items (across all pages)
    pub total_count: usize,
    
    /// Cursor for the next page
    pub next_cursor: Option<Cursor>,
    
    /// Page size
    pub page_size: usize,
}

impl<T> PaginatedResult<T> {
    /// Create a new paginated result
    pub fn new(
        items: Vec<T>, 
        total_count: usize, 
        next_cursor: Option<Cursor>,
        page_size: usize
    ) -> Self {
        Self {
            items,
            total_count,
            next_cursor,
            page_size,
        }
    }
    
    /// Check if this is the last page
    pub fn is_last_page(&self) -> bool {
        self.next_cursor.is_none() || 
        self.next_cursor.as_ref().map_or(false, |c| c.is_last)
    }
}

/// Query routing strategy
#[derive(Debug, Clone, Copy)]
pub enum RoutingStrategy {
    /// Route to the primary storage backend
    Primary,
    
    /// Route to the replica storage backend
    Replica,
    
    /// Route to the cache if available, otherwise to the primary
    CacheFirst,
    
    /// Route to the primary and update the cache
    CacheWrite,
    
    /// Route based on query complexity
    Auto,
}

/// Query router that directs queries to the appropriate storage backend
pub struct QueryRouter<T, Q> {
    /// Cache for query results
    cache: Option<QueryCache<Q, Vec<T>>>,
    
    /// Current routing strategy
    strategy: RoutingStrategy,
}

impl<T, Q> QueryRouter<T, Q>
where
    T: Clone + Send + Sync + 'static,
    Q: std::hash::Hash + Eq + Clone + Send + Sync + 'static
{
    /// Create a new query router
    pub fn new(strategy: RoutingStrategy) -> Self {
        Self {
            cache: None,
            strategy,
        }
    }
    
    /// Create a new query router with caching
    pub fn with_cache(strategy: RoutingStrategy, cache_ttl: Duration, max_entries: usize) -> Self {
        Self {
            cache: Some(QueryCache::new(cache_ttl, max_entries)),
            strategy,
        }
    }
    
    /// Set the routing strategy
    pub fn set_strategy(&mut self, strategy: RoutingStrategy) {
        self.strategy = strategy;
    }
    
    /// Enable caching for this router
    pub fn enable_cache(&mut self, cache_ttl: Duration, max_entries: usize) {
        self.cache = Some(QueryCache::new(cache_ttl, max_entries));
    }
    
    /// Disable caching for this router
    pub fn disable_cache(&mut self) {
        self.cache = None;
    }
    
    /// Route a query to the appropriate backend
    pub async fn route<F, Fut>(&self, query: Q, executor: F) -> Result<Vec<T>, Error>
    where
        F: Fn(Q, RoutingStrategy) -> Fut,
        Fut: std::future::Future<Output = Result<Vec<T>, Error>>,
    {
        match self.strategy {
            RoutingStrategy::Primary => {
                // Execute on primary
                executor(query, RoutingStrategy::Primary).await
            }
            RoutingStrategy::Replica => {
                // Execute on replica
                executor(query.clone(), RoutingStrategy::Replica).await
            }
            RoutingStrategy::CacheFirst => {
                // Try cache first
                if let Some(cache) = &self.cache {
                    if let Some(result) = cache.get(&query).await {
                        return Ok(result);
                    }
                }
                
                // If not in cache, execute on primary
                let result = executor(query.clone(), RoutingStrategy::Primary).await?;
                
                // Cache the result
                if let Some(cache) = &self.cache {
                    cache.put(query, result.clone()).await?;
                }
                
                Ok(result)
            }
            RoutingStrategy::CacheWrite => {
                // Execute on primary
                let result = executor(query.clone(), RoutingStrategy::Primary).await?;
                
                // Cache the result
                if let Some(cache) = &self.cache {
                    cache.put(query, result.clone()).await?;
                }
                
                Ok(result)
            }
            RoutingStrategy::Auto => {
                // Logic to determine the best strategy based on query complexity
                // For simplicity, we'll use CacheFirst for now
                self.route(query, executor).await
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_query_cache() {
        // Create a cache
        let cache: QueryCache<String, Vec<i32>> = QueryCache::new(Duration::from_secs(60), 10);
        
        // Put a value
        cache.put("key1".to_string(), vec![1, 2, 3]).await.unwrap();
        
        // Get the value
        let value = cache.get(&"key1".to_string()).await;
        assert_eq!(value, Some(vec![1, 2, 3]));
        
        // Try a non-existent key
        let value = cache.get(&"key2".to_string()).await;
        assert_eq!(value, None);
    }
    
    #[test]
    fn test_paginated_result() {
        // Create a paginated result
        let result = PaginatedResult::new(
            vec![1, 2, 3],
            10,
            Some(Cursor {
                value: "next".to_string(),
                is_last: false,
            }),
            3
        );
        
        // Check properties
        assert_eq!(result.items, vec![1, 2, 3]);
        assert_eq!(result.total_count, 10);
        assert_eq!(result.page_size, 3);
        assert!(!result.is_last_page());
        
        // Create a last page
        let last_page = PaginatedResult::new(
            vec![8, 9, 10],
            10,
            Some(Cursor {
                value: "last".to_string(),
                is_last: true,
            }),
            3
        );
        
        // Check is_last_page
        assert!(last_page.is_last_page());
    }
} 