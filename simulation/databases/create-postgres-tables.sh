#!/bin/bash
# Purpose: Create PostgreSQL tables for Almanac

set -e

# Define colors for output
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
RED='\033[0;31m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}=== Creating PostgreSQL Tables for Almanac ===${NC}"

# Check if PostgreSQL is available
if ! command -v psql >/dev/null 2>&1; then
    echo -e "${RED}Error: PostgreSQL client (psql) not found${NC}"
    echo -e "${YELLOW}Please ensure PostgreSQL is installed and in your PATH${NC}"
    exit 1
fi

# PostgreSQL configuration
PG_HOST="localhost"
PG_PORT="5432"
PG_USER="postgres"
PG_DB="almanac"

# Check if PostgreSQL server is running
if ! pg_isready -h $PG_HOST -p $PG_PORT -U $PG_USER > /dev/null 2>&1; then
    echo -e "${RED}Error: PostgreSQL server is not running${NC}"
    echo -e "${YELLOW}Please run ./simulation/databases/setup-postgres.sh first${NC}"
    exit 1
fi

echo -e "${GREEN}✓ PostgreSQL server is running${NC}"

# Check if the database exists
if ! psql -h $PG_HOST -p $PG_PORT -U $PG_USER -lqt | cut -d \| -f 1 | grep -qw $PG_DB; then
    echo -e "${RED}Error: Database '${PG_DB}' does not exist${NC}"
    echo -e "${YELLOW}Please run ./simulation/databases/setup-postgres.sh first${NC}"
    exit 1
fi

echo -e "${GREEN}✓ Database '${PG_DB}' exists${NC}"

# Create the tables
echo -e "${BLUE}Creating tables in database '${PG_DB}'...${NC}"

# Define the SQL schema for almanac tables
SCHEMA_SQL=$(cat <<EOF
-- Drop tables if they exist (for clean setup)
DROP TABLE IF EXISTS events CASCADE;
DROP TABLE IF EXISTS chains CASCADE;
DROP TABLE IF EXISTS addresses CASCADE;
DROP TABLE IF EXISTS chain_status CASCADE;
DROP TABLE IF EXISTS migrations CASCADE;

-- Create chains table
CREATE TABLE IF NOT EXISTS chains (
    id SERIAL PRIMARY KEY,
    chain_id VARCHAR(255) UNIQUE NOT NULL,
    name VARCHAR(255) NOT NULL,
    description TEXT,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP NOT NULL,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP NOT NULL
);

-- Create addresses table
CREATE TABLE IF NOT EXISTS addresses (
    id SERIAL PRIMARY KEY,
    address VARCHAR(255) NOT NULL,
    chain_id INTEGER NOT NULL REFERENCES chains(id),
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP NOT NULL,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP NOT NULL,
    UNIQUE(address, chain_id)
);

-- Create events table
CREATE TABLE IF NOT EXISTS events (
    id SERIAL PRIMARY KEY,
    event_id VARCHAR(255) NOT NULL,
    chain_id INTEGER NOT NULL REFERENCES chains(id),
    address_id INTEGER NOT NULL REFERENCES addresses(id),
    block_number BIGINT NOT NULL,
    transaction_hash VARCHAR(255) NOT NULL,
    log_index INTEGER,
    event_type VARCHAR(255) NOT NULL,
    timestamp TIMESTAMP WITH TIME ZONE NOT NULL,
    raw_data JSONB NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP NOT NULL,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP NOT NULL,
    UNIQUE(event_id, chain_id)
);

-- Create chain_status table to track indexing status
CREATE TABLE IF NOT EXISTS chain_status (
    id SERIAL PRIMARY KEY,
    chain_id INTEGER NOT NULL REFERENCES chains(id),
    last_block_number BIGINT NOT NULL,
    last_indexed_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP NOT NULL,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP NOT NULL,
    UNIQUE(chain_id)
);

-- Create migrations table to track schema changes
CREATE TABLE IF NOT EXISTS migrations (
    id SERIAL PRIMARY KEY,
    version VARCHAR(255) NOT NULL,
    description TEXT,
    applied_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP NOT NULL,
    UNIQUE(version)
);

-- Create indexes for performance
CREATE INDEX IF NOT EXISTS idx_events_chain_id ON events(chain_id);
CREATE INDEX IF NOT EXISTS idx_events_address_id ON events(address_id);
CREATE INDEX IF NOT EXISTS idx_events_block_number ON events(block_number);
CREATE INDEX IF NOT EXISTS idx_events_timestamp ON events(timestamp);
CREATE INDEX IF NOT EXISTS idx_events_event_type ON events(event_type);
CREATE INDEX IF NOT EXISTS idx_addresses_chain_id ON addresses(chain_id);
CREATE INDEX IF NOT EXISTS idx_addresses_address ON addresses(address);

-- Insert default chains
INSERT INTO chains (chain_id, name, description) 
VALUES 
    ('31337', 'Anvil', 'Local Ethereum development chain using Anvil'),
    ('1337', 'Reth', 'Local Ethereum development chain using Reth'),
    ('wasmchain', 'WasmChain', 'Local CosmWasm development chain')
ON CONFLICT (chain_id) DO UPDATE 
SET 
    name = EXCLUDED.name,
    description = EXCLUDED.description,
    updated_at = CURRENT_TIMESTAMP;

-- Insert initial migration
INSERT INTO migrations (version, description)
VALUES ('202404010000', 'Initial schema setup')
ON CONFLICT (version) DO NOTHING;
EOF
)

# Execute the SQL schema
echo "$SCHEMA_SQL" | psql -h $PG_HOST -p $PG_PORT -U $PG_USER -d $PG_DB

echo -e "${GREEN}✓ Tables created successfully${NC}"

# Verify the tables were created
echo -e "${BLUE}Verifying tables...${NC}"
TABLE_COUNT=$(psql -h $PG_HOST -p $PG_PORT -U $PG_USER -d $PG_DB -t -c "SELECT COUNT(*) FROM information_schema.tables WHERE table_schema = 'public';")

if [ $TABLE_COUNT -ge 5 ]; then
    echo -e "${GREEN}✓ Verification successful. Found $TABLE_COUNT tables in the database.${NC}"
else
    echo -e "${RED}Error: Verification failed. Expected at least 5 tables, but found $TABLE_COUNT.${NC}"
    exit 1
fi

echo -e "${GREEN}=== PostgreSQL tables setup completed successfully! ===${NC}" 