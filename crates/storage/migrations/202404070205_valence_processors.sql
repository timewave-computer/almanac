-- Migration: Create Valence Processor related tables

CREATE TABLE valence_processors (
    id VARCHAR PRIMARY KEY,                         -- Unique ID (e.g., chain_id:contract_address)
    chain_id VARCHAR NOT NULL,
    contract_address VARCHAR NOT NULL,
    created_at_block BIGINT NOT NULL,
    created_at_tx VARCHAR NOT NULL,
    current_owner VARCHAR,                          -- Nullable if renounced
    -- Processor-specific configuration
    max_gas_per_message BIGINT,
    message_timeout_blocks BIGINT,
    retry_interval_blocks BIGINT,
    max_retry_count INT,
    paused BOOLEAN NOT NULL DEFAULT false,
    last_updated_block BIGINT NOT NULL,
    last_updated_tx VARCHAR NOT NULL,

    CONSTRAINT uq_valence_processors_chain_address UNIQUE (chain_id, contract_address)
);

CREATE INDEX idx_valence_processors_owner ON valence_processors (current_owner);
CREATE INDEX idx_valence_processors_chain ON valence_processors (chain_id);

COMMENT ON TABLE valence_processors IS 'Valence processor contracts that handle cross-chain messaging';
COMMENT ON COLUMN valence_processors.max_gas_per_message IS 'Maximum gas allowance for executing a message';
COMMENT ON COLUMN valence_processors.message_timeout_blocks IS 'Number of blocks after which a message is considered timed out';
COMMENT ON COLUMN valence_processors.retry_interval_blocks IS 'Blocks to wait before retrying a failed message';
COMMENT ON COLUMN valence_processors.max_retry_count IS 'Maximum number of retry attempts for failed messages';
COMMENT ON COLUMN valence_processors.paused IS 'Whether message processing is currently paused';

CREATE TYPE valence_message_status AS ENUM ('pending', 'processing', 'completed', 'failed', 'timed_out');

CREATE TABLE valence_processor_messages (
    id VARCHAR PRIMARY KEY,                         -- Unique message ID (UUID or hash)
    processor_id VARCHAR NOT NULL REFERENCES valence_processors(id) ON DELETE CASCADE,
    source_chain_id VARCHAR NOT NULL,               -- Chain where message originated
    target_chain_id VARCHAR NOT NULL,               -- Chain where message is to be processed
    sender_address VARCHAR NOT NULL,                -- Address that submitted the message
    payload TEXT NOT NULL,                          -- Message payload (could be base64/hex encoded)
    status valence_message_status NOT NULL,         -- Current status of the message
    created_at_block BIGINT NOT NULL,               -- Block when message was created
    created_at_tx VARCHAR NOT NULL,                 -- Transaction hash when message was created
    last_updated_block BIGINT NOT NULL,             -- Block when message was last updated
    processed_at_block BIGINT,                      -- Block when message was processed (if completed/failed)
    processed_at_tx VARCHAR,                        -- Transaction hash when message was processed
    retry_count INT NOT NULL DEFAULT 0,             -- Number of retry attempts so far
    next_retry_block BIGINT,                        -- Block number when message should be retried
    gas_used BIGINT,                                -- Gas used for processing the message
    error TEXT,                                     -- Error message if failed
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_valence_processor_messages_processor ON valence_processor_messages (processor_id);
CREATE INDEX idx_valence_processor_messages_source_chain ON valence_processor_messages (source_chain_id);
CREATE INDEX idx_valence_processor_messages_target_chain ON valence_processor_messages (target_chain_id);
CREATE INDEX idx_valence_processor_messages_sender ON valence_processor_messages (sender_address);
CREATE INDEX idx_valence_processor_messages_status ON valence_processor_messages (status);
CREATE INDEX idx_valence_processor_messages_next_retry ON valence_processor_messages (status, next_retry_block) 
  WHERE status = 'failed' AND next_retry_block IS NOT NULL;
CREATE INDEX idx_valence_processor_messages_created_block ON valence_processor_messages (source_chain_id, created_at_block);

COMMENT ON TABLE valence_processor_messages IS 'Cross-chain messages processed by Valence processors';
COMMENT ON COLUMN valence_processor_messages.payload IS 'Encoded message payload to be executed on target chain';
COMMENT ON COLUMN valence_processor_messages.next_retry_block IS 'Block number when this message should be retried if failed';

-- Stats table for processor performance monitoring
CREATE TABLE valence_processor_stats (
    processor_id VARCHAR NOT NULL REFERENCES valence_processors(id) ON DELETE CASCADE,
    timestamp TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    block_number BIGINT NOT NULL,
    pending_messages INT NOT NULL DEFAULT 0,
    processing_messages INT NOT NULL DEFAULT 0,
    completed_messages INT NOT NULL DEFAULT 0,
    failed_messages INT NOT NULL DEFAULT 0,
    timed_out_messages INT NOT NULL DEFAULT 0,
    avg_processing_time_ms DOUBLE PRECISION,
    avg_gas_used DOUBLE PRECISION,
    
    PRIMARY KEY (processor_id, timestamp)
);

CREATE INDEX idx_valence_processor_stats_block ON valence_processor_stats (processor_id, block_number);

COMMENT ON TABLE valence_processor_stats IS 'Performance statistics for Valence processors'; 