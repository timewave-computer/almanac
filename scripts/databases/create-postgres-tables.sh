#!/bin/bash
# Purpose: Create and populate PostgreSQL databases and tables

set -e

# Define colors for output
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
RED='\033[0;31m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}=== Creating PostgreSQL Databases and Tables ===${NC}"

# Check if PostgreSQL is running
nix develop --command bash -c "pg_isready -h localhost -p 5432" > /dev/null 2>&1
if [ $? -ne 0 ]; then
  echo -e "${RED}Error: PostgreSQL is not running. Please run simulation/databases/setup-postgres.sh first.${NC}"
  exit 1
fi

# Create and populate databases using nix develop
nix develop --command bash -c "
  # Ensure indexer database exists
  if ! psql -lqt | cut -d \| -f 1 | grep -qw indexer; then
    echo -e \"${YELLOW}Creating indexer database...${NC}\"
    createdb -O postgres indexer
  else
    echo -e \"${GREEN}✓ indexer database already exists${NC}\"
  fi
  
  # Ensure indexer_test database exists
  if ! psql -lqt | cut -d \| -f 1 | grep -qw indexer_test; then
    echo -e \"${YELLOW}Creating indexer_test database...${NC}\"
    createdb -O postgres indexer_test
  else
    echo -e \"${GREEN}✓ indexer_test database already exists${NC}\"
  fi
  
  echo -e \"${BLUE}Creating tables in indexer database...${NC}\"
  # Create the blocks table
  psql -d indexer -c \"
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
  psql -d indexer -c \"
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
  psql -d indexer -c \"
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
  psql -d indexer -c \"
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
  psql -d indexer -c \"
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
  psql -d indexer -c \"
    CREATE TABLE IF NOT EXISTS contract_schemas (
      id SERIAL PRIMARY KEY,
      chain VARCHAR(100) NOT NULL,
      address VARCHAR(255) NOT NULL,
      schema_data BYTEA NOT NULL,
      created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
      CONSTRAINT contract_schemas_chain_address_key UNIQUE (chain, address)
    );
  \"
  
  echo -e \"${GREEN}✓ Created tables in indexer database${NC}\"
  
  echo -e \"${BLUE}Creating tables in indexer_test database...${NC}\"
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
  
  echo -e \"${GREEN}✓ Created tables in indexer_test database${NC}\"
"

# Update SQLx metadata
echo -e "${BLUE}Updating SQLx metadata...${NC}"
nix develop --command bash -c "
  cd crates/storage
    
  # Export the database URL
  export DATABASE_URL=\"postgres://postgres:postgres@localhost:5432/indexer_test\"
  echo \"Using DATABASE_URL: \$DATABASE_URL\"
    
  # Force SQLX_OFFLINE to false to generate metadata
  SQLX_OFFLINE=false cargo sqlx prepare --check || SQLX_OFFLINE=false cargo sqlx prepare
  
  cd ../..
"

echo -e "${GREEN}✓ Updated SQLx metadata${NC}"
echo -e "${BLUE}=== PostgreSQL databases and tables created successfully ===${NC}" 