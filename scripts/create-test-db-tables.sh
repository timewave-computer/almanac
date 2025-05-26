#!/bin/bash
# Purpose: Create tables in the indexer_test database

set -e

echo "=== Creating tables in indexer_test database ==="

# Use nix develop to run PostgreSQL commands
nix develop --command bash -c "
  # Create the blocks table
  psql -d indexer_test -c \"
    CREATE TABLE IF NOT EXISTS blocks (
      id SERIAL PRIMARY KEY,
      chain VARCHAR(100) NOT NULL,
      number BIGINT NOT NULL,
      hash VARCHAR(255) NOT NULL,
      timestamp BIGINT NOT NULL,
      status VARCHAR(50) NOT NULL DEFAULT 'confirmed',
      parent_hash VARCHAR(255),
      created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
      updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
      CONSTRAINT blocks_chain_number_key UNIQUE (chain, number)
    );
  \"
    
  # Create the events table
  psql -d indexer_test -c \"
    CREATE TABLE IF NOT EXISTS events (
      id VARCHAR NOT NULL PRIMARY KEY,
      chain VARCHAR(100) NOT NULL,
      block_number BIGINT NOT NULL,
      block_hash VARCHAR(255) NOT NULL,
      tx_hash VARCHAR(255) NOT NULL,
      timestamp BIGINT NOT NULL,
      event_type VARCHAR(100) NOT NULL,
      raw_data BYTEA NOT NULL,
      created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
    );
  \"
    
  # Create valence accounts table
  psql -d indexer_test -c \"
    CREATE TABLE IF NOT EXISTS valence_accounts (
      id VARCHAR(255) PRIMARY KEY,
      chain_id VARCHAR(100) NOT NULL,
      contract_address VARCHAR(255) NOT NULL,
      created_at_block BIGINT NOT NULL,
      created_at_tx VARCHAR(255) NOT NULL,
      current_owner VARCHAR(255) NOT NULL,
      pending_owner VARCHAR(255),
      pending_owner_expiry BIGINT,
      last_updated_block BIGINT NOT NULL,
      last_updated_tx VARCHAR(255) NOT NULL,
      created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
    );
  \"
    
  # Create valence account libraries table
  psql -d indexer_test -c \"
    CREATE TABLE IF NOT EXISTS valence_account_libraries (
      account_id VARCHAR(255) NOT NULL,
      library_address VARCHAR(255) NOT NULL,
      approved_at_block BIGINT NOT NULL,
      approved_at_tx VARCHAR(255) NOT NULL,
      created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
      PRIMARY KEY (account_id, library_address)
    );
  \"
    
  # Create valence account executions table
  psql -d indexer_test -c \"
    CREATE TABLE IF NOT EXISTS valence_account_executions (
      id SERIAL PRIMARY KEY,
      account_id VARCHAR(255) NOT NULL,
      chain_id VARCHAR(100) NOT NULL,
      block_number BIGINT NOT NULL,
      tx_hash VARCHAR(255) NOT NULL,
      executor_address VARCHAR(255) NOT NULL,
      message_index INTEGER NOT NULL,
      correlated_event_ids VARCHAR[],
      raw_msgs JSONB,
      payload BYTEA,
      executed_at TIMESTAMP WITH TIME ZONE NOT NULL,
      created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
    );
  \"
    
  # Create contract schemas table for storing ABI info
  psql -d indexer_test -c \"
    CREATE TABLE IF NOT EXISTS contract_schemas (
      id SERIAL PRIMARY KEY,
      chain VARCHAR(100) NOT NULL,
      address VARCHAR(255) NOT NULL,
      schema_data BYTEA NOT NULL,
      created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
      CONSTRAINT contract_schemas_chain_address_key UNIQUE (chain, address)
    );
  \"
  
  echo \"✓ All tables created successfully in indexer_test database\"
  
  # Update the SQLx metadata
  echo \"Updating SQLx metadata...\"
  cd crates/storage
    
  # Export the database URL
  export DATABASE_URL=\"postgres://postgres:postgres@localhost:5432/indexer_test\"
  echo \"Using DATABASE_URL: \$DATABASE_URL\"
    
  # Force SQLX_OFFLINE to false to generate metadata
  SQLX_OFFLINE=false cargo sqlx prepare --check || SQLX_OFFLINE=false cargo sqlx prepare
  
  cd ../..
"

echo "=== Test database setup complete ===" 