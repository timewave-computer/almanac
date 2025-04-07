-- Migration: Create blocks table

CREATE TABLE IF NOT EXISTS blocks (
    chain TEXT NOT NULL,
    block_number BIGINT NOT NULL,
    block_hash TEXT NOT NULL,
    timestamp BIGINT NOT NULL,
    status TEXT DEFAULT 'confirmed',
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    PRIMARY KEY (chain, block_number)
); 