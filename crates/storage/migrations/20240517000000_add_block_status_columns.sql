-- Add status columns to blocks for tracking different Ethereum block states
-- This allows us to distinguish between different levels of block confirmation

-- Add status columns to blocks table
ALTER TABLE blocks 
ADD COLUMN status TEXT NOT NULL DEFAULT 'confirmed',
ADD COLUMN finalized BOOLEAN NOT NULL DEFAULT false,
ADD COLUMN justified BOOLEAN NOT NULL DEFAULT false,
ADD COLUMN safe BOOLEAN NOT NULL DEFAULT false;

-- Create indices for each block status to optimize query performance
CREATE INDEX idx_blocks_finalized ON blocks (chain, finalized);
CREATE INDEX idx_blocks_justified ON blocks (chain, justified);
CREATE INDEX idx_blocks_safe ON blocks (chain, safe);
CREATE INDEX idx_blocks_status ON blocks (chain, status);

-- Create composite indices for common query patterns
CREATE INDEX idx_blocks_chain_finalized_block_number ON blocks (chain, finalized, block_number DESC);
CREATE INDEX idx_blocks_chain_justified_block_number ON blocks (chain, justified, block_number DESC);
CREATE INDEX idx_blocks_chain_safe_block_number ON blocks (chain, safe, block_number DESC);
CREATE INDEX idx_blocks_chain_status_block_number ON blocks (chain, status, block_number DESC);

-- Add comment explaining the different status values
COMMENT ON COLUMN blocks.status IS 'Block status: "confirmed" (included in chain), "safe" (unlikely to be orphaned), "justified" (voted by validators), "finalized" (irreversible)'; 