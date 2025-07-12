-- Initial schema setup for indexer storage

-- Migrations table to track applied migrations
CREATE TABLE IF NOT EXISTS migrations (
    id SERIAL PRIMARY KEY,
    name VARCHAR(255) NOT NULL UNIQUE,
    applied_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

-- Contract schemas table
CREATE TABLE IF NOT EXISTS contract_schemas (
    id SERIAL PRIMARY KEY,
    chain VARCHAR(100) NOT NULL,
    address VARCHAR(255) NOT NULL,
    schema_data JSONB NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(chain, address)
);

-- Blocks table to track blockchain blocks
CREATE TABLE IF NOT EXISTS blocks (
    id SERIAL PRIMARY KEY,
    chain VARCHAR(100) NOT NULL,
    number BIGINT NOT NULL,
    hash VARCHAR(255) NOT NULL,
    timestamp BIGINT NOT NULL,
    status VARCHAR(50) NOT NULL, -- 'pending', 'confirmed', 'finalized'
    parent_hash VARCHAR(255),
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(chain, number)
);

-- Events table to store blockchain events
CREATE TABLE IF NOT EXISTS events (
    id SERIAL PRIMARY KEY,
    event_id VARCHAR(255) NOT NULL,
    chain VARCHAR(100) NOT NULL,
    block_number BIGINT NOT NULL,
    block_hash VARCHAR(255) NOT NULL,
    tx_hash VARCHAR(255) NOT NULL,
    timestamp BIGINT NOT NULL,
    event_type VARCHAR(255) NOT NULL,
    raw_data BYTEA NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(event_id)
);

-- Record the initial migration
INSERT INTO migrations (name) VALUES ('00_init_schema'); 