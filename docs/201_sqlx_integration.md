# SQLx Database Integration Specification

This document specifies how to use SQLx for PostgreSQL database integration in the Almanac indexer, covering query patterns, type safety, and development workflows.

## Overview

Almanac uses SQLx as the primary PostgreSQL client library with a macro-based approach for compile-time query validation and automatic type inference. This ensures type safety, eliminates boilerplate code, and catches SQL errors at compile time.

## Core Principles

### Macro-First Approach

Use SQLx macros as the primary interface for database queries:

- `sqlx::query!()` for queries returning anonymous records
- `sqlx::query_as!()` for queries mapping to custom structs
- `sqlx::query_scalar!()` for queries returning single values

### Compile-Time Validation

SQLx macros validate queries against the database schema during compilation, providing:

- SQL syntax validation
- Type checking for parameters and results
- Schema consistency verification
- Automatic type inference from database columns

### Repository Pattern

Organize database access through domain-specific repositories that encapsulate SQLx usage and provide clean interfaces for business logic.

## Basic Usage Patterns

### Simple Queries

Use `sqlx::query!()` for straightforward data retrieval:

```rust
// Query with automatic type inference
let users = sqlx::query!(
    "SELECT id, name, email, created_at FROM users WHERE active = $1",
    true
)
.fetch_all(&pool)
.await?;

// Access fields with compile-time type checking
for user in users {
    println!("User {}: {} ({})", user.id, user.name, user.email);
}
```

### Scalar Values

Use `sqlx::query_scalar!()` for single-value queries:

```rust
// Count queries
let user_count = sqlx::query_scalar!(
    "SELECT COUNT(*) FROM users WHERE active = $1",
    true
)
.fetch_one(&pool)
.await?;

// Latest block number
let latest_block = sqlx::query_scalar!(
    "SELECT MAX(block_number) FROM blocks WHERE chain_id = $1",
    "ethereum"
)
.fetch_one(&pool)
.await?;
```

### Custom Types

Use `sqlx::query_as!()` with custom structs for complex mappings:

```rust
#[derive(sqlx::FromRow)]
struct BlockSummary {
    chain_id: String,
    latest_block: i64,
    event_count: i64,
    last_updated: chrono::DateTime<chrono::Utc>,
}

let summaries = sqlx::query_as!(
    BlockSummary,
    r#"
    SELECT 
        b.chain_id,
        MAX(b.block_number) as latest_block,
        COUNT(e.id) as event_count,
        MAX(b.timestamp) as last_updated
    FROM blocks b
    LEFT JOIN events e ON b.id = e.block_id
    WHERE b.is_finalized = true
    GROUP BY b.chain_id
    "#
)
.fetch_all(&pool)
.await?;
```

## Repository Implementation

### Repository Trait Pattern

Define repository traits for each domain entity:

```rust
#[async_trait]
pub trait BlockRepository: Send + Sync {
    async fn get_latest_block(&self, chain_id: &str) -> Result<Option<i64>>;
    async fn get_blocks_by_range(&self, chain_id: &str, from: i64, to: i64) -> Result<Vec<Block>>;
    async fn store_block(&self, block: &NewBlock) -> Result<i64>;
    async fn update_finality_status(&self, chain_id: &str, block_number: i64, is_finalized: bool) -> Result<()>;
}
```

### Repository Implementation

Implement repositories using SQLx macros:

```rust
pub struct PostgresBlockRepository {
    pool: PgPool,
}

#[async_trait]
impl BlockRepository for PostgresBlockRepository {
    async fn get_latest_block(&self, chain_id: &str) -> Result<Option<i64>> {
        let block_number = sqlx::query_scalar!(
            "SELECT MAX(block_number) FROM blocks WHERE chain_id = $1",
            chain_id
        )
        .fetch_one(&self.pool)
        .await?;
        
        Ok(block_number)
    }
    
    async fn get_blocks_by_range(&self, chain_id: &str, from: i64, to: i64) -> Result<Vec<Block>> {
        let blocks = sqlx::query_as!(
            Block,
            r#"
            SELECT id, chain_id, block_number, block_hash, parent_hash, 
                   timestamp, is_finalized, is_safe
            FROM blocks 
            WHERE chain_id = $1 AND block_number BETWEEN $2 AND $3
            ORDER BY block_number ASC
            "#,
            chain_id, from, to
        )
        .fetch_all(&self.pool)
        .await?;
        
        Ok(blocks)
    }
    
    async fn store_block(&self, block: &NewBlock) -> Result<i64> {
        let block_id = sqlx::query_scalar!(
            r#"
            INSERT INTO blocks (chain_id, block_number, block_hash, timestamp)
            VALUES ($1, $2, $3, $4)
            RETURNING id
            "#,
            block.chain_id,
            block.block_number,
            block.block_hash,
            block.timestamp
        )
        .fetch_one(&self.pool)
        .await?;
        
        Ok(block_id)
    }
}
```

## Development Workflow

### Local Development

For development with a live database:

1. **Start PostgreSQL**: Use Nix shell to start the database
   ```bash
   nix develop
   # Database starts automatically in development environment
   ```

2. **Run Migrations**: Apply schema changes
   ```bash
   sqlx migrate run --database-url postgresql://localhost/almanac_dev
   ```

3. **Write Queries**: SQLx macros automatically validate against the live database
   ```rust
   // This query is validated at compile time
   let result = sqlx::query!("SELECT * FROM blocks WHERE chain_id = $1", "ethereum")
       .fetch_all(&pool)
       .await?;
   ```

### CI and Offline Builds

For environments without a live database:

1. **Enable Offline Mode**: Set environment variable
   ```bash
   export SQLX_OFFLINE=true
   ```

2. **Use Cached Metadata**: SQLx uses the `.sqlx/` directory for type information
   ```bash
   # Generate/update query metadata
   cargo sqlx prepare --database-url postgresql://localhost/almanac_dev
   
   # Verify metadata is current
   cargo sqlx prepare --check --database-url postgresql://localhost/almanac_dev
   ```

3. **Commit Metadata**: Include `.sqlx/` directory in version control
   ```bash
   git add .sqlx/
   git commit -m "Update SQLx query metadata"
   ```

## Advanced Patterns

### Complex Queries with JSON

Handle JSONB columns and complex data structures:

```rust
let events = sqlx::query!(
    r#"
    SELECT 
        id,
        contract_address,
        event_type,
        data as "data: sqlx::types::Json<serde_json::Value>",
        timestamp
    FROM events 
    WHERE contract_address = $1 
    AND data @> $2
    "#,
    contract_address,
    serde_json::json!({"type": "Transfer"})
)
.fetch_all(&pool)
.await?;

for event in events {
    let event_data: serde_json::Value = event.data.0;
    // Process JSON data...
}
```

### Transaction Management

Use SQLx transactions for atomic operations:

```rust
pub async fn store_block_with_events(&self, block: &NewBlock, events: &[NewEvent]) -> Result<()> {
    let mut tx = self.pool.begin().await?;
    
    // Store block
    let block_id = sqlx::query_scalar!(
        "INSERT INTO blocks (chain_id, block_number, block_hash, timestamp) VALUES ($1, $2, $3, $4) RETURNING id",
        block.chain_id, block.block_number, block.block_hash, block.timestamp
    )
    .fetch_one(&mut *tx)
    .await?;
    
    // Store events
    for event in events {
        sqlx::query!(
            "INSERT INTO events (block_id, contract_address, event_type, data) VALUES ($1, $2, $3, $4)",
            block_id, event.contract_address, event.event_type, event.data
        )
        .execute(&mut *tx)
        .await?;
    }
    
    tx.commit().await?;
    Ok(())
}
```

### Batch Operations

Optimize for bulk inserts and updates:

```rust
pub async fn store_events_batch(&self, events: &[NewEvent]) -> Result<()> {
    let mut block_ids = Vec::new();
    let mut contract_addresses = Vec::new();
    let mut event_types = Vec::new();
    let mut data_values = Vec::new();
    
    for event in events {
        block_ids.push(event.block_id);
        contract_addresses.push(&event.contract_address);
        event_types.push(&event.event_type);
        data_values.push(&event.data);
    }
    
    sqlx::query!(
        r#"
        INSERT INTO events (block_id, contract_address, event_type, data)
        SELECT * FROM UNNEST($1::bigint[], $2::text[], $3::text[], $4::jsonb[])
        "#,
        &block_ids,
        &contract_addresses,
        &event_types,
        &data_values as &[serde_json::Value]
    )
    .execute(&self.pool)
    .await?;
    
    Ok(())
}
```

## Error Handling

### Custom Error Types

Define domain-specific error handling:

```rust
#[derive(thiserror::Error, Debug)]
pub enum DatabaseError {
    #[error("Database connection error: {0}")]
    Connection(#[from] sqlx::Error),
    
    #[error("Block not found: chain {chain}, block {block_number}")]
    BlockNotFound { chain: String, block_number: i64 },
    
    #[error("Constraint violation: {0}")]
    ConstraintViolation(String),
}

impl From<sqlx::Error> for DatabaseError {
    fn from(err: sqlx::Error) -> Self {
        match err {
            sqlx::Error::RowNotFound => DatabaseError::BlockNotFound {
                chain: "unknown".to_string(),
                block_number: 0,
            },
            _ => DatabaseError::Connection(err),
        }
    }
}
```

### Query Result Handling

Handle optional results and empty sets appropriately:

```rust
// Handle optional single result
pub async fn get_block_by_hash(&self, chain_id: &str, block_hash: &str) -> Result<Option<Block>> {
    let block = sqlx::query_as!(
        Block,
        "SELECT * FROM blocks WHERE chain_id = $1 AND block_hash = $2",
        chain_id, block_hash
    )
    .fetch_optional(&self.pool)
    .await?;
    
    Ok(block)
}

// Handle required single result
pub async fn get_latest_finalized_block(&self, chain_id: &str) -> Result<Block> {
    let block = sqlx::query_as!(
        Block,
        "SELECT * FROM blocks WHERE chain_id = $1 AND is_finalized = true ORDER BY block_number DESC LIMIT 1",
        chain_id
    )
    .fetch_one(&self.pool)
    .await
    .map_err(|e| match e {
        sqlx::Error::RowNotFound => DatabaseError::BlockNotFound {
            chain: chain_id.to_string(),
            block_number: 0,
        },
        e => DatabaseError::Connection(e),
    })?;
    
    Ok(block)
}
```

## Testing

### Test Database Setup

Use a separate test database for integration tests:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    async fn setup_test_db() -> PgPool {
        let database_url = std::env::var("TEST_DATABASE_URL")
            .unwrap_or_else(|_| "postgresql://localhost/almanac_test".to_string());
        
        let pool = PgPool::connect(&database_url).await.unwrap();
        
        // Run migrations
        sqlx::migrate!("./migrations").run(&pool).await.unwrap();
        
        pool
    }
    
    #[tokio::test]
    async fn test_store_and_retrieve_block() {
        let pool = setup_test_db().await;
        let repo = PostgresBlockRepository::new(pool);
        
        let new_block = NewBlock {
            chain_id: "ethereum".to_string(),
            block_number: 18000000,
            block_hash: "0xabc123...".to_string(),
            parent_hash: "0xdef456...".to_string(),
            timestamp: chrono::Utc::now(),
        };
        
        let block_id = repo.store_block(&new_block).await.unwrap();
        assert!(block_id > 0);
        
        let retrieved = repo.get_latest_block("ethereum").await.unwrap();
        assert_eq!(retrieved, Some(18000000));
    }
}
```

### Mock Repository Pattern

For unit tests, provide mock implementations:

```rust
#[cfg(test)]
pub struct MockBlockRepository {
    blocks: std::sync::Mutex<Vec<Block>>,
}

#[cfg(test)]
#[async_trait]
impl BlockRepository for MockBlockRepository {
    async fn get_latest_block(&self, chain_id: &str) -> Result<Option<i64>> {
        let blocks = self.blocks.lock().unwrap();
        let latest = blocks
            .iter()
            .filter(|b| b.chain_id == chain_id)
            .map(|b| b.block_number)
            .max();
        Ok(latest)
    }
    
    // ... other mock implementations
}
```

## Best Practices

### Query Organization

- Keep complex queries in separate files or modules
- Use descriptive names for query functions
- Document query parameters and return types
- Group related queries in the same repository

### Performance Considerations

- Use `fetch_optional()` instead of `fetch_one()` when the result might not exist
- Prefer `fetch_many()` or `fetch_all()` for multiple results
- Use prepared statements (automatic with SQLx macros) for repeated queries
- Consider connection pooling settings for high-throughput scenarios

### Schema Evolution

- Always run `cargo sqlx prepare` after schema changes
- Commit `.sqlx/` metadata with schema changes
- Test queries against both old and new schemas during migrations
- Use database transactions for complex schema updates

This specification provides the foundation for consistent, type-safe database access throughout the Almanac indexer while maintaining excellent developer experience and compile-time safety guarantees.
