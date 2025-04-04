-- Add finality status to blocks
-- For Ethereum blocks, this will indicate whether the block has been finalized by the finality gadget
-- For Cosmos blocks, all blocks will be marked as finalized immediately due to Tendermint's instant finality

-- Add finalized column to blocks table with a default of false
ALTER TABLE blocks 
ADD COLUMN finalized BOOLEAN NOT NULL DEFAULT false;

-- Create an index on the finalized column to optimize queries filtering by finality status
CREATE INDEX idx_blocks_finalized ON blocks (chain, finalized);

-- Create a composite index for common query patterns that need to find the latest finalized block
CREATE INDEX idx_blocks_chain_finalized_block_number ON blocks (chain, finalized, block_number DESC); 