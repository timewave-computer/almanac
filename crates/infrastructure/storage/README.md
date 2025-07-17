# Almanac Storage Infrastructure

This crate provides flexible storage backends for the Almanac indexer with support for PostgreSQL, RocksDB, and hybrid configurations.

## Overview

The storage infrastructure now supports multiple backend configurations:

- **PostgreSQL Only** (default): Best for complex queries and moderate workloads
- **RocksDB Only** (optional): Best for high-throughput write workloads  
- **Hybrid Mode** (optional): Combines RocksDB for hot data with PostgreSQL for cold data
- **Memory** (default): In-memory storage for testing

### Key Features

- **Flexible Backend Selection**: Choose the right storage for your workload
- **Optional RocksDB**: RocksDB is now optional, simplifying deployment
- **Hybrid Storage**: Combine backends for optimal performance
- **Automatic Backend Selection**: Let the system recommend based on workload characteristics
- **Configuration from JSON/Environment**: Easy deployment configuration

## PostgreSQL Storage

The PostgreSQL storage implementation follows best practices for Rust database access:

1. Uses `sqlx` for compile-time validated SQL queries
2. Implements a repository pattern for domain-specific operations
3. Provides a robust migration system based on `sqlx` migrations

### SQL Approach

We follow the approach outlined in ADR-000:

- Use plain `sqlx::query_as::<_, T>()` and `sqlx::query()` calls with `.bind()` methods
- Implement `sqlx::FromRow` manually for application data models
- Define domain-specific traits that encapsulate query methods
- Set up compile-time SQL validation using `sqlx prepare`

Example:

```rust
// Define a repository trait
#[async_trait]
pub trait EventRepository: Send + Sync + 'static {
    async fn store_event(&self, event: Box<dyn Event>) -> Result<()>;
    async fn get_events(&self, filters: Vec<EventFilter>) -> Result<Vec<Box<dyn Event>>>;
    async fn get_latest_block(&self, chain: &str) -> Result<u64>;
}

// Implement FromRow for data models
#[derive(Debug, FromRow)]
pub struct EventRecord {
    pub id: String,
    pub chain: String,
    pub block_number: i64,
    // ... other fields
}

// Use query! and query_as! macros for compile-time validation
let result = sqlx::query!(
    r#"SELECT MAX(block_number) as max_block FROM blocks WHERE chain = $1"#,
    chain
)
.fetch_one(&self.pool)
.await?;
```

### Migration System

We use `sqlx` migrations to manage database schema changes:

1. Migrations are stored in the `migrations/` directory as SQL files
2. Each migration follows the naming convention `{timestamp}_{description}.sql`
3. Migrations are applied automatically when the storage is initialized
4. The migration system ensures atomicity and keeps track of applied migrations

To create a new migration:

1. Create a new file in the `migrations/` directory with the format `{timestamp}_{description}.sql`
2. Add SQL commands for the migration
3. Run `cargo sqlx prepare` to update the query metadata (required for offline builds)

### Contract Schema Migrations

For contract schemas, we provide specific support for:

1. Storing and versioning contract schemas
2. Tracking events and functions per contract version
3. Mapping between different schema versions
4. Supporting historical queries across schema changes

## RocksDB Storage

The RocksDB storage implementation provides:

1. High-performance key-value storage with optimized key design
2. Schema evolution support with migration capabilities
3. Efficient range queries for time-series data

### Key Design

Keys in RocksDB are carefully designed for efficient access patterns:

```
<entity_type>:<chain_id>:<entity_id>:<property> -> <value>
```

Examples:
- `block:eth:12345:hash` -> `0x123...`
- `event:eth:0xabc:data` -> `binary data`

## Running Tests

To run tests for the storage implementations:

```bash
# Run all tests (excluding ignored ones)
cargo test -p indexer-storage

# Run specific tests with database (these tests are ignored by default)
cargo test -p indexer-storage -- --ignored test_postgres_storage_full
```

Note: PostgreSQL tests require a running PostgreSQL server. The tests will use the `DATABASE_URL` environment variable if set, or fall back to default test connections.

## Quick Start Guide

### Using PostgreSQL Only (Default)

```rust
use almanac_infrastructure_storage::{StorageBackend, StorageFactory};

// Simple PostgreSQL configuration
let storage = StorageFactory::create(
    StorageBackend::PostgresOnly {
        config: Default::default(),
    }
).await?;
```

### Enabling RocksDB Support

Add the `rocks` feature to your `Cargo.toml`:

```toml
[dependencies]
almanac-infrastructure-storage = { 
    path = "../storage", 
    features = ["rocks"] 
}
```

Then use RocksDB backend:

```rust
let storage = StorageFactory::create(
    StorageBackend::RocksOnly {
        config: RocksBackendConfig {
            path: "./data/rocksdb".to_string(),
            cache_size: 1024 * 1024 * 1024, // 1GB
            ..Default::default()
        },
    }
).await?;
```

### Using Hybrid Storage

Enable both backends and configure hybrid mode:

```rust
let storage = StorageFactory::create(
    StorageBackend::Hybrid {
        hot: RocksBackendConfig::default(),
        cold: PostgresBackendConfig::default(),
        hybrid_config: HybridConfig {
            hot_blocks: 1000,
            auto_migrate: true,
            query_routing: QueryRouting::Smart,
            ..Default::default()
        },
    }
).await?;
```

### Automatic Backend Selection

Let the system choose based on your workload:

```rust
use almanac_infrastructure_storage::{
    BackendSelector, WorkloadCharacteristics, DataVolume
};

let workload = WorkloadCharacteristics {
    write_heavy: true,
    complex_queries: false,
    data_volume: DataVolume::Large,
    real_time: true,
    analytics: false,
};

let recommended = BackendSelector::recommend_backend(&workload);
let storage = StorageFactory::create(recommended).await?;
```

## Feature Flags

| Feature | Description | Default |
|---------|-------------|---------|
| `postgres` | PostgreSQL backend support | ✓ |
| `memory` | In-memory backend for testing | ✓ |
| `rocks` | RocksDB backend support | ✗ |
| `production` | Production config (PostgreSQL + memory) | ✗ |
| `performance` | High-performance (includes RocksDB) | ✗ |
| `full` | All features enabled | ✗ |

## Migration Guide

### From RocksDB-Required to Optional

1. **No action needed** if you only use PostgreSQL
2. **Add `rocks` feature** if you currently use RocksDB:
   ```toml
   almanac-infrastructure-storage = { path = "../storage", features = ["rocks"] }
   ```
3. **Update instantiation** to use `StorageFactory`:
   ```rust
   // Old way
   let storage = RocksStorage::new(config)?;
   
   // New way
   let storage = StorageFactory::create(
       StorageBackend::RocksOnly { config }
   ).await?;
   ```

## Performance Comparison

| Backend | Write Speed | Query Complexity | Storage Efficiency | Operational Complexity |
|---------|-------------|------------------|-------------------|----------------------|
| PostgreSQL | Medium | High | Medium | Low |
| RocksDB | High | Low | High | Medium |
| Hybrid | High | High | High | High |

## Documentation

- [Storage Backend Guide](../../../docs/storage-backend-guide.md) - Detailed configuration guide
- [Example Usage](../../../examples/storage_backend_selection.rs) - Working examples 