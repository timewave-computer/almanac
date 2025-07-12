# PostgreSQL and RocksDB Testing Guide

This document provides guidance on testing the storage layer of the Almanac project, which uses both PostgreSQL and RocksDB.

## Current Issues

The primary issue we're facing is that the PostgreSQL connection in the test environment fails with the error:

```
error: error returned from database: role "postgres" does not exist
```

This happens because:

1. The PostgreSQL setup in the Nix flake doesn't properly create the `postgres` role with the expected permissions
2. SQLx macros attempt to verify SQL queries at compile time, connecting to the database
3. Even when using `SQLX_OFFLINE=true`, SQLx requires query metadata to be prepared

## Solutions

### Solution 1: Fix PostgreSQL User Setup

To properly set up the PostgreSQL user for testing:

1. Create a user named `postgres` with appropriate permissions:
   ```sql
   CREATE ROLE postgres WITH LOGIN SUPERUSER PASSWORD 'postgres';
   ```

2. Create a test database:
   ```sql
   CREATE DATABASE indexer_test OWNER postgres;
   ```

3. Run migrations on the test database:
   ```bash
   cd crates/storage
   sqlx migrate run --database-url postgres://postgres:postgres@localhost:5432/indexer_test
   ```

4. To prepare query metadata for offline mode:
   ```bash
   cd crates/storage
   cargo sqlx prepare -- --check
   ```

### Solution 2: Standalone RocksDB Testing

For testing just the RocksDB functionality without requiring PostgreSQL:

1. Use the standalone RocksDB test script:
   ```bash
   ./scripts/standalone-rocks-test.sh
   ```

This script:
- Creates a completely isolated Rust project with only RocksDB dependencies
- Implements a simplified version of the storage layer focused on RocksDB
- Runs the tests in the Nix environment, but without PostgreSQL dependencies
- Cleans up temporary files after execution

## Available Scripts

The following scripts are available for testing:

1. `scripts/link-migrations.sh`: Links PostgreSQL migration files for tests
2. `scripts/run-db-tests.sh`: Runs database tests (requires PostgreSQL setup)
3. `scripts/setup-postgres-test.sh`: Attempts to set up PostgreSQL test user
4. `scripts/direct-test.sh`: Tries to run tests with the Nix flake's PostgreSQL config
5. `scripts/test-rocks.sh`: Runs RocksDB-only tests (still requires PostgreSQL)
6. `scripts/isolated-rocks-test.sh`: Attempts to run isolated RocksDB tests with SQLX_OFFLINE=true
7. `scripts/standalone-rocks-test.sh`: Runs completely standalone RocksDB tests

## Recommended Approach

For now, we recommend:

1. For local development, use the standalone RocksDB tests to verify RocksDB functionality
2. For full integration testing, ensure PostgreSQL is properly set up with the `postgres` role
3. Work on improving the Nix flake to automatically create the PostgreSQL user and database

## Future Improvements

1. Update the Nix flake to properly initialize PostgreSQL with the required users and permissions
2. Refactor the storage layer to allow optional PostgreSQL dependencies
3. Add an SQLx prepare step to the CI pipeline to generate metadata for offline mode
4. Consider splitting the storage module into separate PostgreSQL and RocksDB modules 