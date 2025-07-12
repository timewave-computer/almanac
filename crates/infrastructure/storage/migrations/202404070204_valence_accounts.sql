-- Migration: Create Valence Account related tables

CREATE TABLE valence_accounts (
    id VARCHAR PRIMARY KEY,                         -- Unique ID (e.g., chain_id:contract_address)
    chain_id VARCHAR NOT NULL,
    contract_address VARCHAR NOT NULL,
    created_at_block BIGINT NOT NULL,
    created_at_tx VARCHAR NOT NULL,
    current_owner VARCHAR,                          -- Nullable if renounced
    pending_owner VARCHAR,
    pending_owner_expiry BIGINT,                    -- Can be block height or timestamp depending on cw_ownable config
    last_updated_block BIGINT NOT NULL,
    last_updated_tx VARCHAR NOT NULL,

    CONSTRAINT uq_valence_accounts_chain_address UNIQUE (chain_id, contract_address)
);

CREATE INDEX idx_valence_accounts_owner ON valence_accounts (current_owner);
CREATE INDEX idx_valence_accounts_chain ON valence_accounts (chain_id);

COMMENT ON COLUMN valence_accounts.id IS 'Primary key combining chain_id and contract_address';
COMMENT ON COLUMN valence_accounts.pending_owner_expiry IS 'Block height or timestamp for ownership transfer expiry';

CREATE TABLE valence_account_libraries (
    account_id VARCHAR NOT NULL REFERENCES valence_accounts(id) ON DELETE CASCADE,
    library_address VARCHAR NOT NULL,
    approved_at_block BIGINT NOT NULL,
    approved_at_tx VARCHAR NOT NULL,

    PRIMARY KEY (account_id, library_address)
);

CREATE INDEX idx_valence_account_libraries_account ON valence_account_libraries (account_id);
CREATE INDEX idx_valence_account_libraries_library ON valence_account_libraries (library_address);

COMMENT ON TABLE valence_account_libraries IS 'Stores libraries approved to act on behalf of a Valence account';

CREATE TABLE valence_account_executions (
    id BIGSERIAL PRIMARY KEY,                       -- Auto-incrementing ID
    account_id VARCHAR NOT NULL REFERENCES valence_accounts(id) ON DELETE CASCADE,
    chain_id VARCHAR NOT NULL,
    block_number BIGINT NOT NULL,
    tx_hash VARCHAR NOT NULL,
    executor_address VARCHAR NOT NULL,              -- Address that called execute_msg/execute_submsgs
    message_index INT NOT NULL,                     -- Index of the execute msg within the tx (if determinable)
    correlated_event_ids TEXT[],                    -- Array of event IDs (FK to a general events table assumed)
    raw_msgs JSONB,                                 -- Raw CosmosMsg/SubMsg array if parseable
    payload TEXT,                                   -- Payload from execute_submsgs
    executed_at TIMESTAMP WITH TIME ZONE NOT NULL
);

CREATE INDEX idx_valence_account_executions_account ON valence_account_executions (account_id);
CREATE INDEX idx_valence_account_executions_tx ON valence_account_executions (tx_hash);
CREATE INDEX idx_valence_account_executions_block ON valence_account_executions (chain_id, block_number);
CREATE INDEX idx_valence_account_executions_executor ON valence_account_executions (executor_address);

COMMENT ON TABLE valence_account_executions IS 'Historical record of executions initiated by Valence accounts';
COMMENT ON COLUMN valence_account_executions.correlated_event_ids IS 'References to related events in a main events table'; 