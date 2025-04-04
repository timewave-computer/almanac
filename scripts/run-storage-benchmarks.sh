#!/bin/bash
set -e

# Check if we're running in a nix shell
if [ -z "$IN_NIX_SHELL" ]; then
  echo "This script must be run within a Nix shell. Please run 'nix develop' first."
  exit 1
fi

# Check if Postgres is running and start it if not
if ! pg_isready -q; then
  echo "Starting PostgreSQL server..."
  pg_ctl -D "$PGDATA" start -l "$PGDATA/postgresql.log" -o "-k $PGDATA"
  
  # Wait for PostgreSQL to be ready
  attempt=0
  max_attempts=10
  until pg_isready -q || [ $attempt -eq $max_attempts ]; do
    attempt=$((attempt+1))
    echo "Waiting for PostgreSQL to be ready... (attempt $attempt/$max_attempts)"
    sleep 1
  done

  if [ $attempt -eq $max_attempts ]; then
    echo "Failed to connect to PostgreSQL after $max_attempts attempts."
    exit 1
  fi
fi

# Create test database if it doesn't exist
if ! psql -lqt | cut -d \| -f 1 | grep -qw indexer_test; then
  echo "Creating test database 'indexer_test'..."
  createdb indexer_test
fi

# Set the DATABASE_URL for the tests
export DATABASE_URL="postgresql://postgres@localhost/indexer_test"

echo "Initializing database schema..."

# Drop existing tables to ensure clean state
psql -d indexer_test -c "
DROP TABLE IF EXISTS contract_function_schemas CASCADE;
DROP TABLE IF EXISTS contract_event_schemas CASCADE;
DROP TABLE IF EXISTS contract_schema_versions CASCADE;
DROP TABLE IF EXISTS events CASCADE;
DROP TABLE IF EXISTS blocks CASCADE;
DROP TABLE IF EXISTS contract_schemas CASCADE;
DROP TABLE IF EXISTS migrations CASCADE;
"

# Initialize schema directly with the correct column types and names
psql -d indexer_test -c "
-- Create migrations table
CREATE TABLE IF NOT EXISTS migrations (
    id SERIAL PRIMARY KEY,
    name VARCHAR(255) NOT NULL UNIQUE,
    applied_at BIGINT NOT NULL DEFAULT extract(epoch from now())
);

-- Create contract_schemas table
CREATE TABLE IF NOT EXISTS contract_schemas (
    id SERIAL PRIMARY KEY,
    chain VARCHAR(255) NOT NULL,
    address VARCHAR(255) NOT NULL,
    name VARCHAR(255) NOT NULL,
    schema_data BYTEA NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(chain, address)
);

-- Create blocks table
CREATE TABLE IF NOT EXISTS blocks (
    id SERIAL PRIMARY KEY,
    chain VARCHAR(255) NOT NULL,
    block_number BIGINT NOT NULL,
    block_hash VARCHAR(255) NOT NULL,
    timestamp BIGINT NOT NULL,
    status VARCHAR(50) NOT NULL DEFAULT 'pending',
    parent_hash VARCHAR(255),
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(chain, block_number)
);

-- Create events table
CREATE TABLE IF NOT EXISTS events (
    id VARCHAR(255) PRIMARY KEY,
    chain VARCHAR(255) NOT NULL,
    block_number BIGINT NOT NULL,
    block_hash VARCHAR(255) NOT NULL,
    tx_hash VARCHAR(255) NOT NULL,
    timestamp BIGINT NOT NULL,
    event_type VARCHAR(255) NOT NULL,
    raw_data BYTEA NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Create contract_schema_versions table with 'version' column
CREATE TABLE IF NOT EXISTS contract_schema_versions (
    id SERIAL PRIMARY KEY,
    contract_schema_id INTEGER NOT NULL REFERENCES contract_schemas(id),
    version VARCHAR(255) NOT NULL,
    chain_id VARCHAR(255),
    contract_address VARCHAR(255),
    abi_hash VARCHAR(255),
    abi_json JSONB,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(contract_schema_id, version)
);

-- Create contract_event_schemas table
CREATE TABLE IF NOT EXISTS contract_event_schemas (
    id SERIAL PRIMARY KEY,
    contract_schema_id INTEGER NOT NULL REFERENCES contract_schemas(id),
    event_name VARCHAR(255) NOT NULL,
    event_schema JSONB NOT NULL,
    schema_json JSONB,
    event_signature VARCHAR(255),
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(contract_schema_id, event_name)
);

-- Create contract_function_schemas table
CREATE TABLE IF NOT EXISTS contract_function_schemas (
    id SERIAL PRIMARY KEY,
    contract_schema_id INTEGER NOT NULL REFERENCES contract_schemas(id),
    function_name VARCHAR(255) NOT NULL,
    function_schema JSONB NOT NULL,
    schema_json JSONB,
    function_signature VARCHAR(255),
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(contract_schema_id, function_name)
);

-- Insert initial migration record
INSERT INTO migrations (name, applied_at) VALUES ('init', extract(epoch from now())::bigint);
"

echo "Database schema initialized successfully."

echo ""
echo "==================== STORAGE BENCHMARK ENVIRONMENT SUMMARY ====================

The database schema has been successfully set up for benchmarking.

REQUIRED CODE FIXES:

1. Fix schema.rs:
   - Update lines 272-287 - SQL query accessing 'version' column needs updating 
   - Update lines 301-316 - Similar issue with SQL query
   - Fix type mismatches on lines 332-333 by adding .unwrap_or_default()

2. Run RocksDB benchmarks:
   cargo test -p indexer-storage rocks_benchmark -- --nocapture
   
3. After schema.rs fixes, run PostgreSQL benchmarks:
   cargo test -p indexer-storage postgres_benchmark -- --nocapture

4. For synchronization benchmarks:
   cargo test -p indexer-storage sync_benchmark -- --nocapture

===========================================================================" 