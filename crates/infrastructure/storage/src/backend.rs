// Storage backend configuration and factory
//
// This module provides flexible storage backend selection with support for
// PostgreSQL-only, RocksDB-only, or hybrid configurations.

use crate::{BoxedStorage, Result, Error};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Storage backend configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum StorageBackend {
    /// PostgreSQL only backend
    PostgresOnly {
        /// PostgreSQL configuration
        config: PostgresBackendConfig,
    },
    
    /// RocksDB only backend
    #[cfg(feature = "rocks")]
    RocksOnly {
        /// RocksDB configuration
        config: RocksBackendConfig,
    },
    
    /// Hybrid backend with hot/cold storage separation
    #[cfg(all(feature = "postgres", feature = "rocks"))]
    Hybrid {
        /// Hot storage (RocksDB) for recent data
        hot: RocksBackendConfig,
        /// Cold storage (PostgreSQL) for historical data
        cold: PostgresBackendConfig,
        /// Configuration for hybrid behavior
        hybrid_config: HybridConfig,
    },
    
    /// In-memory backend for testing
    #[cfg(feature = "memory")]
    Memory {
        /// Memory backend configuration
        config: MemoryBackendConfig,
    },
}

/// PostgreSQL backend configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostgresBackendConfig {
    /// Database connection URL
    pub url: String,
    /// Maximum number of connections in the pool
    pub max_connections: u32,
    /// Connection timeout in seconds
    pub connection_timeout: u64,
    /// Enable query logging
    pub enable_query_logging: bool,
    /// Enable automatic migrations
    pub auto_migrate: bool,
}

impl Default for PostgresBackendConfig {
    fn default() -> Self {
        Self {
            url: "postgresql://localhost/almanac".to_string(),
            max_connections: 10,
            connection_timeout: 30,
            enable_query_logging: false,
            auto_migrate: true,
        }
    }
}

/// RocksDB backend configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RocksBackendConfig {
    /// Database path
    pub path: String,
    /// Cache size in bytes
    pub cache_size: usize,
    /// Enable compression
    pub compression: bool,
    /// Number of background threads
    pub num_threads: u32,
    /// Write buffer size in bytes
    pub write_buffer_size: usize,
    /// Maximum number of write buffers
    pub max_write_buffers: u32,
}

impl Default for RocksBackendConfig {
    fn default() -> Self {
        Self {
            path: "./data/rocksdb".to_string(),
            cache_size: 512 * 1024 * 1024, // 512MB
            compression: true,
            num_threads: num_cpus::get() as u32,
            write_buffer_size: 64 * 1024 * 1024, // 64MB
            max_write_buffers: 4,
        }
    }
}

/// Memory backend configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryBackendConfig {
    /// Initial capacity for event storage
    pub initial_capacity: usize,
    /// Enable persistence simulation (for testing)
    pub simulate_persistence: bool,
}

impl Default for MemoryBackendConfig {
    fn default() -> Self {
        Self {
            initial_capacity: 10000,
            simulate_persistence: false,
        }
    }
}

/// Hybrid storage configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HybridConfig {
    /// Number of recent blocks to keep in hot storage
    pub hot_blocks: u64,
    /// Time threshold for hot storage (in seconds)
    pub hot_time_threshold: u64,
    /// Enable automatic data migration from hot to cold
    pub auto_migrate: bool,
    /// Migration check interval (in seconds)
    pub migration_interval: u64,
    /// Query routing strategy
    pub query_routing: QueryRouting,
}

impl Default for HybridConfig {
    fn default() -> Self {
        Self {
            hot_blocks: 1000,
            hot_time_threshold: 3600, // 1 hour
            auto_migrate: true,
            migration_interval: 300, // 5 minutes
            query_routing: QueryRouting::Smart,
        }
    }
}

/// Query routing strategy for hybrid storage
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QueryRouting {
    /// Always try hot storage first, fall back to cold
    HotFirst,
    /// Always try cold storage first, fall back to hot
    ColdFirst,
    /// Smart routing based on query characteristics
    Smart,
    /// Parallel query to both backends
    Parallel,
}

/// Storage backend factory
pub struct StorageFactory;

impl StorageFactory {
    /// Create a storage backend from configuration
    pub async fn create(config: StorageBackend) -> Result<BoxedStorage> {
        match config {
            StorageBackend::PostgresOnly { config } => {
                Self::create_postgres_backend(config).await
            }
            
            #[cfg(feature = "rocks")]
            StorageBackend::RocksOnly { config } => {
                Self::create_rocks_backend(config)
            }
            
            #[cfg(all(feature = "postgres", feature = "rocks"))]
            StorageBackend::Hybrid { hot, cold, hybrid_config } => {
                Self::create_hybrid_backend(hot, cold, hybrid_config).await
            }
            
            #[cfg(feature = "memory")]
            StorageBackend::Memory { config } => {
                Self::create_memory_backend(config)
            }
            
            #[allow(unreachable_patterns)]
            _ => Err(Error::generic("Storage backend not available with current features")),
        }
    }
    
    /// Create PostgreSQL backend
    async fn create_postgres_backend(config: PostgresBackendConfig) -> Result<BoxedStorage> {
        #[cfg(feature = "postgres")]
        {
            use crate::postgres::{PostgresConfig, PostgresStorage};
            
            let postgres_config = PostgresConfig {
                url: config.url,
                max_connections: config.max_connections,
                connection_timeout: config.connection_timeout,
            };
            
            let storage = PostgresStorage::new(postgres_config).await?;
            
            // TODO: Add migration support
            // if config.auto_migrate {
            //     storage.run_migrations().await?;
            // }
            
            Ok(Arc::new(storage))
        }
        
        #[cfg(not(feature = "postgres"))]
        {
            Err(Error::generic("PostgreSQL backend not available: enable 'postgres' feature"))
        }
    }
    
    /// Create RocksDB backend
    #[cfg(feature = "rocks")]
    fn create_rocks_backend(config: RocksBackendConfig) -> Result<BoxedStorage> {
        use crate::rocks::{RocksConfig, RocksStorage, CompressionType};
        
        let rocks_config = RocksConfig {
            path: config.path,
            cache_size_mb: config.cache_size / (1024 * 1024), // Convert bytes to MB
            compression: if config.compression { 
                CompressionType::Snappy 
            } else { 
                CompressionType::None 
            },
            write_buffer_size_mb: config.write_buffer_size / (1024 * 1024), // Convert bytes to MB
            max_write_buffer_number: config.max_write_buffers as i32,
            ..Default::default()
        };
        
        let storage = RocksStorage::new(rocks_config)?;
        Ok(Arc::new(storage))
    }
    
    /// Create hybrid backend
    #[cfg(all(feature = "postgres", feature = "rocks"))]
    async fn create_hybrid_backend(
        hot_config: RocksBackendConfig,
        cold_config: PostgresBackendConfig,
        hybrid_config: HybridConfig,
    ) -> Result<BoxedStorage> {
        use crate::hybrid::HybridStorage;
        
        let hot_storage = Self::create_rocks_backend(hot_config)?;
        let cold_storage = Self::create_postgres_backend(cold_config).await?;
        
        let hybrid_storage = HybridStorage::new(
            hot_storage,
            cold_storage,
            hybrid_config,
        )?;
        
        Ok(Arc::new(hybrid_storage))
    }
    
    /// Create memory backend
    #[cfg(feature = "memory")]
    fn create_memory_backend(config: MemoryBackendConfig) -> Result<BoxedStorage> {
        use crate::memory::MemoryStorage;
        
        let storage = MemoryStorage::with_capacity(config.initial_capacity);
        Ok(Arc::new(storage))
    }
}

/// Storage backend selection helper
pub struct BackendSelector;

impl BackendSelector {
    /// Recommend a backend based on workload characteristics
    pub fn recommend_backend(workload: &WorkloadCharacteristics) -> StorageBackend {
        match (workload.write_heavy, workload.complex_queries, workload.data_volume) {
            // Write-heavy with simple queries and large volume -> RocksDB
            #[cfg(feature = "rocks")]
            (true, false, DataVolume::Large) => {
                StorageBackend::RocksOnly {
                    config: RocksBackendConfig::default(),
                }
            }
            
            // Complex queries with any volume -> PostgreSQL
            (_, true, _) => {
                StorageBackend::PostgresOnly {
                    config: PostgresBackendConfig::default(),
                }
            }
            

            
            // Default to PostgreSQL for other cases
            _ => {
                StorageBackend::PostgresOnly {
                    config: PostgresBackendConfig::default(),
                }
            }
        }
    }
}

/// Workload characteristics for backend selection
#[derive(Debug, Clone)]
pub struct WorkloadCharacteristics {
    /// Whether the workload is write-heavy
    pub write_heavy: bool,
    /// Whether complex queries are needed
    pub complex_queries: bool,
    /// Expected data volume
    pub data_volume: DataVolume,
    /// Whether real-time performance is critical
    pub real_time: bool,
    /// Whether analytics queries are needed
    pub analytics: bool,
}

/// Data volume categories
#[derive(Debug, Clone, Copy)]
pub enum DataVolume {
    /// Less than 1GB
    Small,
    /// 1GB - 100GB
    Medium,
    /// More than 100GB
    Large,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_backend_selection() {
        // Test write-heavy workload
        let workload = WorkloadCharacteristics {
            write_heavy: true,
            complex_queries: false,
            data_volume: DataVolume::Large,
            real_time: true,
            analytics: false,
        };
        
        let backend = BackendSelector::recommend_backend(&workload);
        match backend {
            #[cfg(feature = "rocks")]
            StorageBackend::RocksOnly { .. } => {
                // Expected for write-heavy workload
            }
            #[cfg(not(feature = "rocks"))]
            StorageBackend::PostgresOnly { .. } => {
                // Fallback when RocksDB not available
            }
            _ => panic!("Unexpected backend recommendation"),
        }
    }
    
    #[test]
    fn test_config_serialization() {
        let config = StorageBackend::PostgresOnly {
            config: PostgresBackendConfig::default(),
        };
        
        let json = serde_json::to_string(&config).unwrap();
        let deserialized: StorageBackend = serde_json::from_str(&json).unwrap();
        
        match deserialized {
            StorageBackend::PostgresOnly { .. } => {
                // Successfully deserialized
            }
            _ => panic!("Deserialization failed"),
        }
    }
}