# Storage Crate

This crate provides storage implementations for the indexer, including:

- **RocksDB** for high-performance key-value storage
- **PostgreSQL** for rich querying capabilities and analysis

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