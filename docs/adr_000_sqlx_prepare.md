# ADR: Use `sqlx` with Trait-Based Query Interface and SQLX Schema Validation

## Status

Accepted

## Context

Our cross-chain indexer requires high-performance and type-safe access to PostgreSQL for rich historical and relational queries. We need a database access approach that:

- Supports async execution for concurrency
- Enables compile-time type checking of query results
- Avoids the use of macros to improve transparency and reduce complexity
- Allows separation of interface from implementation to facilitate testing and modularity
- Provides low-level control without sacrificing maintainability
- Validates SQL queries against the actual database schema to catch integration issues early

## Decision

We will use the [`sqlx`](https://github.com/launchbadge/sqlx) crate to interact with PostgreSQL. We will:

1. Use plain `sqlx::query_as::<_, T>()` and `sqlx::query()` calls with `.bind(...)` and `.fetch_*()` methods.
2. Avoid macro-based query definitions such as `query!()` or `query_as!()` to retain transparency and control.
3. Implement `sqlx::FromRow` manually for application data models as needed.
4. Define domain-specific traits (e.g., `AccountRepository`, `ProcessorRepository`) that encapsulate query methods, allowing for mockable, testable abstractions.
5. Integrate **compile-time SQL validation** using `sqlx prepare`:
   - This tool will be used to statically validate SQL queries against a live database and cache a snapshot of the schema and query result types.
   - This snapshot will be committed as `.sqlx/sqlx-data.json` and used in builds with `SQLX_OFFLINE=true` to enable type-checked compilation without requiring a live database.

## Compile-Time Validation Strategy

To restore query safety without using macros, we will adopt the `sqlx prepare` workflow:

### Development and Schema Change Workflow

1. Start a local database (e.g., via Docker).
2. Apply all migrations using `refinery`, `sqlx migrate`, or similar.
3. Run:
   ```bash
   DATABASE_URL=postgres://user:pass@localhost/db sqlx prepare

   This:
   - Connects to the DB
   - Validates all SQL queries in the codebase
   - Generates .sqlx/sqlx-data.json containing schema + query metadata

4. Commit .sqlx/sqlx-data.json into version control.

All subsequent builds (local or CI) will pass SQLX_OFFLINE=true to use this cached metadata instead of hitting the database.

## CI Integration

In CI, we will:

- Set SQLX_OFFLINE=true in the environment
- Ensure .sqlx/sqlx-data.json is present
- Compile the code as normal

This allows all query validation to be type-checked in CI, without starting a database.

Optionally, we may run sqlx prepare --check as a CI step to enforce that .sqlx/sqlx-data.json is up to date with the codebase:

`DATABASE_URL=postgres://user:pass@localhost/db sqlx prepare --check`

This prevents stale .sqlx metadata from masking query errors.

## Consequences

- We retain full control over SQL structure and query behavior.
- SQL queries are validated against the real schema using sqlx prepare.
- Type mismatches, invalid columns, or incorrect bindings are caught at build time — even without macros.
- Adding a new SQL query or changing a schema requires a sqlx prepare step to regenerate the metadata.

Developers must be familiar with the sqlx prepare flow and ensure .sqlx/sqlx-data.json stays in sync with the schema.

## Alternatives Considered

- Diesel: Rejected due to lack of native async support and reliance on code generation macros.
- SeaORM: Rejected due to unnecessary abstraction and runtime-based query construction.
- tokio-postgres: Rejected due to verbosity and lack of integration with typed query result validation.

## Related Decisions

We will use refinery for database migrations, enabling explicit, versioned SQL migration files and compatibility with sqlx prepare.
