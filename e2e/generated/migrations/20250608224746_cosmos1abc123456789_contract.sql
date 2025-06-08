-- Migration for contract: cosmos1abc123456789
-- Generated at: 2025-06-08 22:47:46 UTC

CREATE TABLE IF NOT EXISTS contract_state (
    id BIGSERIAL PRIMARY KEY,
    contract_address TEXT NOT NULL,
    block_height BIGINT NOT NULL,
    state_data JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
