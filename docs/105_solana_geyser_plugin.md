# Solana Geyser Plugin Integration

The Almanac Geyser Plugin streams real-time Solana blockchain data directly to the Almanac database, enabling low-latency indexing and analysis of Solana accounts, transactions, and slots.

## Overview

The Geyser plugin architecture allows Solana validators to stream blockchain data to external systems in real-time as blocks are processed. Almanac's implementation provides:

- **Real-time account updates** - Stream account changes as they happen
- **Transaction monitoring** - Capture transaction details and execution results  
- **Slot tracking** - Monitor slot progression and finalization
- **Selective filtering** - Configure which accounts and transactions to capture
- **Database integration** - Direct PostgreSQL integration for efficient storage

## Architecture

```
Solana Validator → Geyser Plugin Interface → Almanac Geyser Plugin → PostgreSQL → Almanac Storage
```

### Data Flow

1. **Solana Validator** processes transactions and updates accounts
2. **Geyser Interface** streams updates to registered plugins
3. **Almanac Plugin** filters and transforms data based on configuration
4. **PostgreSQL Database** stores structured blockchain data
5. **Almanac Storage Layer** provides unified access to indexed data

## Installation and Configuration

### Prerequisites

- Solana validator with Geyser plugin support
- PostgreSQL database server
- Almanac Geyser plugin binary

### Building the Plugin

The plugin is built using the Nix environment:

```bash
# Enter Solana Nix environment with Geyser support
cd nix/solana
nix develop ./flake-nixos.nix

# Build the Geyser plugin
nix build .#geyser-plugin

# The plugin library will be available as a .so file
```

### Database Setup

Initialize the PostgreSQL database and schema:

```bash
# Using Nix environment commands
setup-almanac-db

# Or manually create database
createdb almanac
psql almanac -c "CREATE USER almanac WITH PASSWORD 'almanac';"
psql almanac -c "GRANT ALL PRIVILEGES ON DATABASE almanac TO almanac;"
```

### Plugin Configuration

Create a Geyser plugin configuration file (`geyser-config.json`):

```json
{
  "libpath": "/path/to/almanac_geyser_plugin.so",
  "database_url": "postgresql://almanac:almanac@localhost/almanac",
  "max_connections": 10,
  "accounts_selector": {
    "owners": [
      "11111111111111111111111111111112",
      "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"
    ],
    "accounts": []
  },
  "transaction_selector": {
    "mentions": []
  },
  "instruction_selector": {
    "programs": [
      "11111111111111111111111111111112",
      "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"
    ]
  }
}
```

### Starting Solana with Geyser Plugin

```bash
# Start Solana test validator with Geyser plugin
solana-test-validator \
  --geyser-plugin-config geyser-config.json \
  --ledger ./ledger \
  --log

# Or using the Nix environment helper
solana-test-validator-geyser
```

## Configuration Options

### Database Configuration

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `database_url` | string | required | PostgreSQL connection string |
| `max_connections` | number | 16 | Maximum database connection pool size |

### Account Selection

Configure which account updates to capture:

```json
{
  "accounts_selector": {
    "owners": ["pubkey1", "pubkey2"],
    "accounts": ["specific_account1", "specific_account2"]
  }
}
```

- **owners**: Capture accounts owned by these program pubkeys
- **accounts**: Capture specific account pubkeys regardless of owner

### Transaction Selection

Configure transaction monitoring:

```json
{
  "transaction_selector": {
    "mentions": ["program_pubkey1", "account_pubkey2"]
  }
}
```

- **mentions**: Capture transactions that mention these pubkeys

### Instruction Selection

Configure instruction-level monitoring:

```json
{
  "instruction_selector": {
    "programs": ["program_pubkey1", "program_pubkey2"]
  }
}
```

- **programs**: Capture instructions for these program pubkeys

## Database Schema

The plugin creates and manages three main tables:

### `solana_accounts` Table

```sql
CREATE TABLE solana_accounts (
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

CREATE INDEX idx_accounts_owner ON solana_accounts(owner);
CREATE INDEX idx_accounts_slot ON solana_accounts(slot);
```

### `solana_slots` Table

```sql
CREATE TABLE solana_slots (
    slot BIGINT PRIMARY KEY,
    parent_slot BIGINT,
    status VARCHAR(20) NOT NULL,
    updated_at TIMESTAMP NOT NULL DEFAULT NOW()
);
```

### `solana_transactions` Table

```sql
CREATE TABLE solana_transactions (
    signature VARCHAR(88) PRIMARY KEY,
    slot BIGINT NOT NULL,
    success BOOLEAN NOT NULL,
    fee BIGINT NOT NULL,
    pre_balances JSONB,
    post_balances JSONB,
    instructions JSONB,
    created_at TIMESTAMP NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_transactions_slot ON solana_transactions(slot);
```

## Development and Testing

### Local Development Setup

```bash
# 1. Enter Solana Nix environment
cd nix/solana
nix develop

# 2. Setup PostgreSQL database
setup-almanac-db

# 3. Build the Geyser plugin
cargo build --package geyser-plugin

# 4. Create test configuration
cat > test-geyser-config.json << EOF
{
  "libpath": "./target/debug/libgeyser_plugin.so",
  "database_url": "postgresql://almanac:almanac@localhost/almanac",
  "accounts_selector": {
    "owners": ["11111111111111111111111111111112"],
    "accounts": []
  },
  "transaction_selector": { "mentions": [] },
  "instruction_selector": { "programs": [] }
}
EOF

# 5. Start validator with plugin
solana-test-validator \
  --geyser-plugin-config test-geyser-config.json \
  --reset
```

### Testing Plugin Functionality

```bash
# Check database for captured data
psql almanac -c "SELECT COUNT(*) FROM solana_accounts;"
psql almanac -c "SELECT COUNT(*) FROM solana_slots;"
psql almanac -c "SELECT COUNT(*) FROM solana_transactions;"

# Monitor real-time updates
psql almanac -c "SELECT pubkey, owner, lamports, slot FROM solana_accounts ORDER BY updated_at DESC LIMIT 10;"
```

### Common Program Pubkeys

| Program | Pubkey | Description |
|---------|--------|-------------|
| System Program | `11111111111111111111111111111112` | Core Solana system operations |
| SPL Token | `TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA` | Token program |
| Associated Token | `ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL` | Associated token accounts |
| Stake Program | `Stake11111111111111111111111111111111111112` | Staking operations |

## Performance Optimization

### Database Tuning

For high-throughput scenarios:

```sql
-- Increase connection limits
ALTER SYSTEM SET max_connections = 200;

-- Optimize for writes
ALTER SYSTEM SET synchronous_commit = off;
ALTER SYSTEM SET wal_buffers = '64MB';
ALTER SYSTEM SET checkpoint_segments = 64;

-- Restart PostgreSQL after changes
```

### Plugin Configuration

```json
{
  "max_connections": 20,
  "accounts_selector": {
    "owners": [
      // Only monitor specific high-value programs
      "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"
    ]
  }
}
```

### Monitoring

Monitor plugin performance:

```bash
# Check plugin logs
journalctl -u solana-validator | grep -i geyser

# Monitor database performance
psql almanac -c "SELECT schemaname,tablename,n_tup_ins,n_tup_upd,n_tup_del FROM pg_stat_user_tables;"

# Check connection usage
psql almanac -c "SELECT count(*) FROM pg_stat_activity WHERE datname='almanac';"
```

## Production Deployment

### NixOS Module Integration

The Geyser plugin integrates with Almanac's NixOS modules for production deployment:

```nix
# In your NixOS configuration
services.almanac-solana = {
  enable = true;
  geyserPlugin.enable = true;
  geyserPlugin.config = {
    database_url = "postgresql://almanac:almanac@localhost/almanac";
    accounts_selector.owners = [
      "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"
    ];
  };
};
```

### Security Considerations

- **Database Access**: Use dedicated database user with minimal privileges
- **Network Security**: Restrict database access to validator host only
- **Data Retention**: Implement data archival for long-running validators
- **Monitoring**: Set up alerting for plugin failures or database issues

### Backup and Recovery

```bash
# Backup Geyser data
pg_dump almanac > geyser-backup-$(date +%Y%m%d).sql

# Restore from backup
psql almanac < geyser-backup-20240101.sql
```

## Integration with Almanac Storage

The Geyser plugin data integrates with Almanac's storage layer:

```rust
// Access Geyser data through Almanac storage
let storage = AlmanacStorage::new().await?;

// Query Solana account data
let accounts = storage.query_solana_accounts(
    &owner_pubkey,
    Some(slot_range)
).await?;

// Query transaction history  
let transactions = storage.query_solana_transactions(
    &signature_filter,
    Some(time_range)
).await?;
```

## Troubleshooting

### Common Issues

**Plugin fails to load:**
```bash
# Check library path and permissions
ls -la /path/to/almanac_geyser_plugin.so
ldd /path/to/almanac_geyser_plugin.so
```

**Database connection errors:**
```bash
# Test database connectivity
psql "postgresql://almanac:almanac@localhost/almanac" -c "SELECT 1;"
```

**High memory usage:**
```json
// Reduce connection pool size
{
  "max_connections": 5
}
```

### Debugging

Enable debug logging in the plugin configuration:

```bash
# Set Rust log level
export RUST_LOG=almanac_geyser_plugin=debug

# Start validator with debug output
solana-test-validator --geyser-plugin-config config.json --log-level debug
```

### Performance Issues

Monitor database performance:

```sql
-- Check slow queries
SELECT query, mean_time, calls 
FROM pg_stat_statements 
ORDER BY mean_time DESC 
LIMIT 10;

-- Check table sizes
SELECT schemaname,tablename,pg_size_pretty(pg_total_relation_size(schemaname||'.'||tablename)) 
FROM pg_tables 
WHERE schemaname='public';
```

## Future Enhancements

Planned improvements to the Geyser plugin:

- **FoundationDB Integration** - Direct streaming to FoundationDB
- **Compression** - Account data compression for storage efficiency  
- **Filtered Streaming** - More sophisticated filtering options
- **Metrics** - Prometheus metrics for monitoring
- **Horizontal Scaling** - Multi-instance plugin coordination

The Geyser plugin provides a powerful foundation for real-time Solana data processing within the Almanac ecosystem.