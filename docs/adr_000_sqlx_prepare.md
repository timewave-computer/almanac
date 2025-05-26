# ADR: Use SQLx with Macro-Based Query Interface and Compile-Time Validation

## Status

Accepted (Revised)

## Context

Our cross-chain indexer requires high-performance and type-safe access to PostgreSQL for rich historical and relational queries. We need a database access approach that:

- Supports async execution for concurrency
- Enables compile-time type checking of query results
- Uses SQLx macros for automatic type inference and validation
- Provides excellent developer experience with minimal boilerplate
- Validates SQL queries against the actual database schema to catch integration issues early
- Allows for easy testing and development workflows

## Decision

We will use the [`sqlx`](https://github.com/launchbadge/sqlx) crate to interact with PostgreSQL with a **macro-first approach**. We will:

1. **Use SQLx macros** (`sqlx::query!()`, `sqlx::query_as!()`, and `sqlx::query_scalar!()`) as the primary interface for database queries.
2. **Automatic type inference**: Let SQLx macros automatically infer result types from the database schema at compile time.
3. **Minimal manual implementations**: Avoid manual `FromRow` implementations where macros can handle the mapping automatically.
4. **Repository pattern**: Define domain-specific repository traits (e.g., `AccountRepository`, `ProcessorRepository`) that use SQLx macros internally.
5. **Compile-time validation**: SQLx macros will validate SQL queries against a live database during development and use cached metadata for CI/offline builds.

## Development Workflow

### With Live Database (Development)

During development with a live database available:

1. Start a local database via Nix
2. Apply all migrations using `sqlx migrate` or similar
3. Write queries using SQLx macros:
   ```rust
   let users = sqlx::query!(
       "SELECT id, name, email FROM users WHERE active = $1",
       true
   )
   .fetch_all(&pool)
   .await?;
   
   // users[0].id, users[0].name, users[0].email are automatically typed
   ```
4. SQLx macros will connect to the database at compile time to validate queries and infer types

### Offline Mode (CI/Production)

For CI and environments without a live database:

1. Use `SQLX_OFFLINE=true` environment variable
2. SQLx will use cached metadata from the `.sqlx/` directory
3. Run `cargo sqlx prepare` after schema changes to update the metadata cache

## Example Usage

```rust
// Simple query with automatic type inference
let user_count = sqlx::query_scalar!(
    "SELECT COUNT(*) FROM users WHERE active = $1",
    true
)
.fetch_one(&pool)
.await?;

// Query with automatic struct mapping
let users = sqlx::query!(
    "SELECT id, name, email, created_at FROM users WHERE active = $1",
    true
)
.fetch_all(&pool)
.await?;

// Each user has fields: id, name, email, created_at with correct types

// Custom return type when needed
#[derive(sqlx::FromRow)]
struct UserSummary {
    id: i64,
    name: String,
    user_count: i64,
}

let summaries = sqlx::query_as!(
    UserSummary,
    "SELECT u.id, u.name, COUNT(p.id) as user_count 
     FROM users u LEFT JOIN posts p ON u.id = p.user_id 
     WHERE u.active = $1 
     GROUP BY u.id, u.name",
    true
)
.fetch_all(&pool)
.await?;
```

## Consequences

### Benefits

- **Zero boilerplate**: No manual `FromRow` implementations needed for simple queries
- **Compile-time safety**: SQL syntax errors, type mismatches, and schema inconsistencies caught at compile time
- **Automatic type inference**: Result types automatically inferred from database schema
- **Excellent IDE support**: Full autocomplete and type checking in IDEs
- **Minimal maintenance**: Schema changes automatically reflected in query types
- **Performance**: No runtime query parsing or type conversion overhead

### Trade-offs

- **Database dependency**: Compilation requires either a live database or cached metadata
- **Compile-time overhead**: Initial compilation may be slower due to database queries
- **Limited dynamic queries**: Complex dynamic query building requires fallback to `sqlx::query()` with manual typing

### Migration Strategy

For existing code using `sqlx::query()` without macros:

1. Replace with appropriate macro (`query!`, `query_as!`, or `query_scalar!`)
2. Remove manual `FromRow` implementations where macros handle the mapping
3. Update repository implementations to use macros
4. Run `cargo sqlx prepare` to generate metadata for the new queries

## CI Integration

In CI pipelines:

1. Set `SQLX_OFFLINE=true` environment variable
2. Ensure `.sqlx/` metadata directory is committed and up-to-date
3. Optionally run `cargo sqlx prepare --check` to verify metadata is current
4. Compile and test as normal

## Alternatives Considered

- **Manual SQLx usage**: Rejected due to excessive boilerplate and lack of compile-time validation
- **Diesel**: Rejected due to lack of native async support and complex schema management
- **SeaORM**: Rejected due to unnecessary abstraction and runtime overhead
- **tokio-postgres**: Rejected due to lack of compile-time safety

## Related Decisions

We will continue using `sqlx migrate` for database migrations, which integrates seamlessly with SQLx macros and compile-time validation.
