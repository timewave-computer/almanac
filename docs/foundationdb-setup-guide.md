# FoundationDB Setup and Usage Guide

This guide covers how to set up and use FoundationDB with the Almanac API.

## Table of Contents
- [Prerequisites](#prerequisites)
- [Installation](#installation)
- [Configuration](#configuration)
- [Running the API with FDB](#running-the-api-with-fdb)
- [API Endpoints](#api-endpoints)
- [Troubleshooting](#troubleshooting)

## Prerequisites

- macOS (Intel or Apple Silicon)
- Nix package manager
- Rust toolchain
- Project dependencies installed

## Installation

### 1. Set up FoundationDB Headers

First, set up the FoundationDB headers for compilation:

```bash
./scripts/setup-fdb-headers.sh
source .env.fdb
```

This creates a local include directory at `.fdb_include/` with the necessary headers.

### 2. Start FoundationDB Server

Using the Nix flake:

```bash
# Start FDB server
nix run .#fdb-manage -- start

# Check status
nix run .#fdb-manage -- status

# Access FDB CLI
nix run .#fdb-manage -- cli
```

The server will:
- Create data directory at `./data/foundationdb/`
- Create logs at `./logs/foundationdb/`
- Generate cluster file at `./data/fdb.cluster`

## Configuration

### Environment Variables

Configure the API to use FoundationDB:

```bash
# Storage backend selection
export STORAGE_BACKEND=foundationdb  # or "memory" for in-memory storage

# FoundationDB configuration
export FDB_CLUSTER_FILE="./data/fdb.cluster"  # Path to cluster file
export FDB_NAMESPACE="almanac"                # Namespace for key isolation

# API configuration
export API_HOST="0.0.0.0"
export API_PORT="8080"
export METRICS_PORT="9090"
export RATE_LIMIT_PER_MINUTE="100"
export ENABLE_CORS="true"
export CORS_ORIGINS="*"
```

### Cluster Configuration

The default cluster file (`data/fdb.cluster`) contains:

```
almanac_cluster:almanac_cluster@127.0.0.1:4500
```

For production clusters, modify this to include multiple coordinators.

## Running the API with FDB

### Build with FDB Support

```bash
# Build the API with FDB feature
cargo build -p almanac-api --features fdb

# Run the API
cargo run -p almanac-api --features fdb
```

### Using the Test Script

A convenience script is provided:

```bash
./scripts/test-api-fdb.sh
```

This script:
1. Checks FDB status
2. Builds the API with FDB support
3. Starts the API server
4. Runs test API calls
5. Keeps the server running for manual testing

## API Endpoints

### Semantic Metadata

**Submit Semantic Interpretation:**
```bash
POST /v1/semantics/{chain}/{contract}/{field}
{
  "developer_id": "developer-123",
  "chain_id": "ethereum-mainnet",
  "contract_address": "0x...",
  "field": "balanceOf",
  "interpretation": {
    "data_type": "uint256",
    "semantic_tags": ["balance", "token"],
    "zero_semantics": "never_written",
    "confidence_level": "high",
    "source": "contract_code",
    "metadata": {
      "description": "Token balance mapping"
    }
  }
}
```

**Get Semantic Interpretations:**
```bash
GET /v1/semantics/{chain}/{contract}/{field}?trust_policy=permissive
```

### Contract Layouts

**Submit Layout:**
```bash
POST /v1/layouts
{
  "chain_id": "ethereum-mainnet",
  "contract_address": "0x...",
  "layout_commitment": "0x...",
  "layout_data": {...},
  "compiler_version": "0.8.19",
  "developer_id": "developer-123"
}
```

**Get Layout:**
```bash
GET /v1/layouts/{chain}/{contract}?trust_policy=strict
```

**Detect Conflicts:**
```bash
GET /v1/layouts/{chain}/{contract}/conflicts
```

### Trust Management

**Grant Trust:**
```bash
POST /v1/trust/grant
{
  "trustor": "alice",
  "trustee": "bob",
  "scope": {"type": "global"},
  "trust_level": "contribute"
}
```

**Check Trust:**
```bash
GET /v1/trust/check/{trustor}/{trustee}?scope_type=global
```

**Get Trust Graph:**
```bash
GET /v1/trust/graph/{developer_id}
```

### Views

**Register View:**
```bash
POST /v1/views
{
  "view_id": "erc20-balances",
  "contract_address": "0x...",
  "chain_id": "ethereum-mainnet",
  "slots": [...],
  "trust_policy": "strict"
}
```

**Query View:**
```bash
GET /v1/views/{view_id}/query?chain={chain}&contract={contract}&block={height}
```

## Storage Schema

The FDB storage uses a tuple-based key structure:

```
[namespace, keyspace, ...dimensions]
```

### Keyspaces

1. **ContractState (1)**: Contract storage slots
   - Key: `[namespace, 1, chain_id, contract, block, storage_key]`
   - Value: Storage value bytes

2. **BlockMeta (2)**: Block headers
   - Key: `[namespace, 2, chain_id, block_identifier]`
   - Value: Block metadata

3. **StorageProofs (3)**: Merkle proofs
   - Key: `[namespace, 3, chain_id, contract, storage_key, block]`
   - Value: Proof data

4. **SemanticMeta (4)**: Semantic interpretations
   - Key: `[namespace, 4, chain_id, contract, storage_key, developer_id]`
   - Value: Semantic metadata

5. **TrustGraph (5)**: Developer trust relationships
   - Key: `[namespace, 5, trustor, trustee, scope]`
   - Value: Trust details

6. **ContractLayouts (6)**: Layout commitments
   - Key: `[namespace, 6, chain_id, contract, version]`
   - Value: Layout data

7. **ViewRegistry (7)**: View definitions
   - Key: `[namespace, 7, view_id]`
   - Value: View configuration

## Trust Policies

The system supports three trust policies:

1. **Strict**: Only return data from explicitly trusted developers
2. **Permissive**: Prefer trusted developers but accept others
3. **Explicit**: Only return data from a specific developer

Apply trust policies in queries:

```bash
# Strict - only trusted developers
GET /v1/semantics/{chain}/{contract}/{field}?trust_policy=strict&developer_id=alice

# Permissive - prefer trusted
GET /v1/semantics/{chain}/{contract}/{field}?trust_policy=permissive

# Explicit - specific developer only
GET /v1/semantics/{chain}/{contract}/{field}?trust_policy=explicit&developer_id=bob
```

## Troubleshooting

### Build Issues

**Missing Headers:**
```bash
error: couldn't read `/usr/local/include/foundationdb/fdb.options`
```

Solution: Run `./scripts/setup-fdb-headers.sh`

**FDB Not Available:**
```bash
FoundationDB not available (feature not enabled)
```

Solution: Build with `--features fdb`

### Runtime Issues

**Connection Failed:**
```bash
Failed to connect to FDB: ...
```

Solutions:
1. Check FDB is running: `nix run .#fdb-manage -- status`
2. Verify cluster file exists at `$FDB_CLUSTER_FILE`
3. Check FDB logs at `./logs/foundationdb/`

**Permission Denied:**
```bash
Error: Storage error: Permission denied
```

Solutions:
1. Check data directory permissions
2. Ensure FDB process has write access

### Performance Tuning

For production use:

1. **Configure Memory:**
   ```bash
   fdbcli> configure memory
   ```

2. **Set Process Classes:**
   ```bash
   fdbserver --class=storage --memory=8GiB
   ```

3. **Monitor Performance:**
   ```bash
   fdbcli> status details
   ```

## Next Steps

1. Set up monitoring with Prometheus metrics
2. Configure backup and restore procedures
3. Implement access control policies
4. Set up multi-region clusters
5. Enable TLS for secure communication

For more information, see the [FoundationDB documentation](https://apple.github.io/foundationdb/).