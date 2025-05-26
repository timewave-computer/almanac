-- Migration: Create contract_schemas table

CREATE TABLE IF NOT EXISTS contract_schemas (
    id SERIAL PRIMARY KEY,
    chain TEXT NOT NULL,
    address TEXT NOT NULL,
    schema_data BYTEA NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    UNIQUE(chain, address)
); 