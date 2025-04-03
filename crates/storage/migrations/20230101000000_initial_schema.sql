-- Initial schema for the indexer
-- Create events table
CREATE TABLE IF NOT EXISTS events (
    id TEXT PRIMARY KEY,
    chain TEXT NOT NULL,
    block_number BIGINT NOT NULL,
    block_hash TEXT NOT NULL,
    tx_hash TEXT NOT NULL,
    timestamp BIGINT NOT NULL,
    event_type TEXT NOT NULL,
    raw_data BYTEA NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Create indexes for events table
CREATE INDEX IF NOT EXISTS idx_events_chain ON events (chain);
CREATE INDEX IF NOT EXISTS idx_events_block_number ON events (block_number);
CREATE INDEX IF NOT EXISTS idx_events_timestamp ON events (timestamp);
CREATE INDEX IF NOT EXISTS idx_events_event_type ON events (event_type);

-- Create blocks table
CREATE TABLE IF NOT EXISTS blocks (
    chain TEXT NOT NULL,
    block_number BIGINT NOT NULL,
    block_hash TEXT NOT NULL,
    timestamp BIGINT NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    PRIMARY KEY (chain, block_number)
);

-- Create contract schema registry tables
CREATE TABLE IF NOT EXISTS contract_schema_versions (
    version TEXT NOT NULL,
    contract_address TEXT NOT NULL,
    chain_id TEXT NOT NULL,
    abi_json TEXT NOT NULL,
    created_at BIGINT NOT NULL,
    PRIMARY KEY (version, contract_address, chain_id)
);

CREATE TABLE IF NOT EXISTS contract_event_schemas (
    version TEXT NOT NULL,
    contract_address TEXT NOT NULL,
    chain_id TEXT NOT NULL,
    event_name TEXT NOT NULL,
    event_signature TEXT NOT NULL,
    schema_json TEXT NOT NULL,
    PRIMARY KEY (version, contract_address, chain_id, event_name),
    FOREIGN KEY (version, contract_address, chain_id) 
        REFERENCES contract_schema_versions(version, contract_address, chain_id)
);

CREATE TABLE IF NOT EXISTS contract_function_schemas (
    version TEXT NOT NULL,
    contract_address TEXT NOT NULL,
    chain_id TEXT NOT NULL,
    function_name TEXT NOT NULL,
    function_signature TEXT NOT NULL,
    schema_json TEXT NOT NULL,
    PRIMARY KEY (version, contract_address, chain_id, function_name),
    FOREIGN KEY (version, contract_address, chain_id) 
        REFERENCES contract_schema_versions(version, contract_address, chain_id)
);

-- Add index for contract schema versions
CREATE INDEX IF NOT EXISTS idx_contract_schema_versions_contract 
    ON contract_schema_versions(contract_address, chain_id); 