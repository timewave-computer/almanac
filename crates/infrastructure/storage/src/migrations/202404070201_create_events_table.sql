-- Migration: Create events table and indexes

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

CREATE INDEX IF NOT EXISTS idx_events_chain ON events (chain);
CREATE INDEX IF NOT EXISTS idx_events_block_number ON events (block_number);
CREATE INDEX IF NOT EXISTS idx_events_timestamp ON events (timestamp);
CREATE INDEX IF NOT EXISTS idx_events_event_type ON events (event_type); 