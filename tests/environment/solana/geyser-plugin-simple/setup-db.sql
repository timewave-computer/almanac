-- Database setup for Almanac
CREATE DATABASE almanac;
CREATE USER almanac WITH PASSWORD 'almanac';
GRANT ALL PRIVILEGES ON DATABASE almanac TO almanac;

-- Connect to almanac database
\c almanac

-- Create tables
CREATE TABLE IF NOT EXISTS solana_accounts (
    pubkey VARCHAR(44) PRIMARY KEY,
    owner VARCHAR(44) NOT NULL,
    lamports BIGINT NOT NULL,
    slot BIGINT NOT NULL,
    executable BOOLEAN NOT NULL,
    rent_epoch BIGINT NOT NULL,
    data BYTEA,
    write_version BIGINT NOT NULL,
    updated_at TIMESTAMP NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS solana_slots (
    slot BIGINT PRIMARY KEY,
    parent_slot BIGINT,
    status VARCHAR(20) NOT NULL,
    updated_at TIMESTAMP NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS solana_transactions (
    signature VARCHAR(88) PRIMARY KEY,
    slot BIGINT NOT NULL,
    success BOOLEAN NOT NULL,
    fee BIGINT NOT NULL,
    pre_balances JSONB,
    post_balances JSONB,
    instructions JSONB,
    created_at TIMESTAMP NOT NULL DEFAULT NOW()
);

-- Create indexes
CREATE INDEX IF NOT EXISTS idx_accounts_owner ON solana_accounts(owner);
CREATE INDEX IF NOT EXISTS idx_accounts_slot ON solana_accounts(slot);
CREATE INDEX IF NOT EXISTS idx_transactions_slot ON solana_transactions(slot);
