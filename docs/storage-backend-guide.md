# Storage Backend Selection Guide

## Overview

The Almanac infrastructure now supports flexible storage backend selection, allowing you to choose the optimal storage solution for your specific workload. You can use PostgreSQL only, RocksDB only, or a hybrid approach that combines both.

## Available Backends

### 1. PostgreSQL Only

Best for:
- Complex queries with JOINs and aggregations
- Strong consistency requirements
- Moderate write throughput
- Rich query capabilities
- ACID compliance

Configuration:
```rust
let backend = StorageBackend::PostgresOnly {
    config: PostgresBackendConfig {
        url: "postgresql://user:pass@localhost/almanac".to_string(),
        max_connections: 20,
        connection_timeout: 30,
        enable_query_logging: false,
        auto_migrate: true,
    },
};
```

### 2. RocksDB Only

Best for:
- High-throughput write workloads
- Simple key-value access patterns
- Large datasets with sequential access
- Low-latency point queries
- Embedded deployments

Configuration:
```rust
let backend = StorageBackend::RocksOnly {
    config: RocksBackendConfig {
        path: "./data/rocksdb".to_string(),
        cache_size: 1024 * 1024 * 1024, // 1GB
        compression: true,
        num_threads: 8,
        write_buffer_size: 128 * 1024 * 1024, // 128MB
        max_write_buffers: 4,
    },
};
```

### 3. Hybrid (PostgreSQL + RocksDB)

Best for:
- Mixed workloads with both high write throughput and complex queries
- Large datasets where recent data is accessed frequently
- Systems that need both real-time performance and historical analytics
- Cost optimization (hot data on fast storage, cold data on cheaper storage)

Configuration:
```rust
let backend = StorageBackend::Hybrid {
    hot: RocksBackendConfig {
        path: "./data/rocksdb-hot".to_string(),
        cache_size: 2048 * 1024 * 1024, // 2GB
        compression: false, // No compression for hot data
        num_threads: 16,
        write_buffer_size: 256 * 1024 * 1024,
        max_write_buffers: 8,
    },
    cold: PostgresBackendConfig {
        url: "postgresql://localhost/almanac_cold".to_string(),
        max_connections: 10,
        connection_timeout: 60,
        enable_query_logging: false,
        auto_migrate: true,
    },
    hybrid_config: HybridConfig {
        hot_blocks: 1000, // Keep last 1000 blocks in hot storage
        hot_time_threshold: 3600, // 1 hour
        auto_migrate: true,
        migration_interval: 300, // Check every 5 minutes
        query_routing: QueryRouting::Smart,
    },
};
```

## Automatic Backend Selection

The `BackendSelector` can recommend a backend based on your workload characteristics:

```rust
let workload = WorkloadCharacteristics {
    write_heavy: true,
    complex_queries: false,
    data_volume: DataVolume::Large,
    real_time: true,
    analytics: false,
};

let recommended_backend = BackendSelector::recommend_backend(&workload);
```

## Configuration Options

### PostgreSQL Configuration

- `url`: Database connection string
- `max_connections`: Maximum number of connections in the pool
- `connection_timeout`: Connection timeout in seconds
- `enable_query_logging`: Enable SQL query logging for debugging
- `auto_migrate`: Automatically run database migrations on startup

### RocksDB Configuration

- `path`: Directory path for RocksDB data files
- `cache_size`: Size of the block cache in bytes
- `compression`: Enable compression (reduces storage, slight CPU overhead)
- `num_threads`: Number of background compaction threads
- `write_buffer_size`: Size of write buffer before flushing to disk
- `max_write_buffers`: Maximum number of write buffers

### Hybrid Configuration

- `hot_blocks`: Number of recent blocks to keep in hot storage
- `hot_time_threshold`: Time threshold for data to be considered "hot"
- `auto_migrate`: Enable automatic migration from hot to cold storage
- `migration_interval`: How often to check for data to migrate
- `query_routing`: Strategy for routing queries:
  - `HotFirst`: Try hot storage first, fall back to cold
  - `ColdFirst`: Try cold storage first, fall back to hot
  - `Smart`: Intelligently route based on query characteristics
  - `Parallel`: Query both backends in parallel

## Feature Flags

Storage backends are controlled by Cargo feature flags:

```toml
[dependencies]
almanac-infrastructure-storage = { 
    path = "../storage",
    features = ["postgres", "rocks"] # Enable both for hybrid support
}
```

Available features:
- `postgres`: Enable PostgreSQL backend
- `rocks`: Enable RocksDB backend
- `memory`: Enable in-memory backend (for testing)

## Migration Path

### From Docker to Flexible Storage

1. **Assess Current Workload**
   ```rust
   // Analyze your current usage patterns
   let workload = WorkloadCharacteristics {
       write_heavy: true,  // Do you have high write throughput?
       complex_queries: false,  // Do you need complex SQL queries?
       data_volume: DataVolume::Large,  // How much data?
       real_time: true,  // Need low latency?
       analytics: false,  // Need analytics queries?
   };
   ```

2. **Choose Backend**
   - Start with PostgreSQL-only if unsure
   - Move to RocksDB-only for write-heavy workloads
   - Adopt hybrid for mixed workloads

3. **Configure Storage**
   ```rust
   // Start simple
   let storage = StorageFactory::create(
       StorageBackend::PostgresOnly {
           config: PostgresBackendConfig::default(),
       }
   ).await?;
   ```

4. **Monitor and Optimize**
   - Monitor query performance
   - Check write throughput
   - Adjust configuration as needed

## Performance Considerations

### PostgreSQL
- **Pros**: Complex queries, transactions, mature tooling
- **Cons**: Lower write throughput, higher latency for simple queries
- **Best Practice**: Use for analytical queries and complex data relationships

### RocksDB
- **Pros**: High write throughput, low latency, efficient compression
- **Cons**: Limited query capabilities, no transactions across keys
- **Best Practice**: Use for high-frequency event storage and caching

### Hybrid
- **Pros**: Best of both worlds, cost-effective, scalable
- **Cons**: More complex to operate, data migration overhead
- **Best Practice**: Use when you have clear hot/cold data patterns

## Example Use Cases

### 1. High-Frequency Trading Data
```rust
// RocksDB for real-time market data
StorageBackend::RocksOnly {
    config: RocksBackendConfig {
        cache_size: 4 * 1024 * 1024 * 1024, // 4GB cache
        compression: false, // Minimize latency
        ..Default::default()
    },
}
```

### 2. Blockchain Analytics Platform
```rust
// PostgreSQL for complex analytics
StorageBackend::PostgresOnly {
    config: PostgresBackendConfig {
        max_connections: 50, // Handle many concurrent queries
        enable_query_logging: true, // Debug slow queries
        ..Default::default()
    },
}
```

### 3. Full Blockchain Indexer
```rust
// Hybrid for optimal performance and cost
StorageBackend::Hybrid {
    hot: RocksBackendConfig {
        path: "/fast-ssd/rocksdb".to_string(),
        cache_size: 8 * 1024 * 1024 * 1024, // 8GB
        ..Default::default()
    },
    cold: PostgresBackendConfig {
        url: "postgresql://cold-storage/almanac".to_string(),
        ..Default::default()
    },
    hybrid_config: HybridConfig {
        hot_blocks: 10_000, // Keep 10k recent blocks hot
        auto_migrate: true,
        ..Default::default()
    },
}
```

## Monitoring and Debugging

### Storage Metrics
- Query latency by backend
- Write throughput
- Cache hit rates (RocksDB)
- Connection pool usage (PostgreSQL)
- Migration statistics (Hybrid)

### Debug Configuration
```rust
// Enable detailed logging
env_logger::Builder::from_env(
    env_logger::Env::default()
        .default_filter_or("almanac_infrastructure_storage=debug")
).init();
```

## Conclusion

The flexible storage backend system allows you to optimize for your specific use case:

1. **Start simple**: Use PostgreSQL-only unless you have specific requirements
2. **Measure**: Monitor your actual workload patterns
3. **Optimize**: Switch to RocksDB or hybrid based on real data
4. **Iterate**: Adjust configuration as your system grows

Remember: RocksDB is now optional. You can run a fully functional system with just PostgreSQL, making deployment simpler while keeping the door open for future optimization.