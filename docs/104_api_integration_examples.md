# Integration Guide

This guide covers how to integrate with Almanac and add support for new blockchains.

## Architecture Overview

Almanac uses a distributed architecture where chain-specific nodes connect to a shared FoundationDB data layer:

```
┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐
│  Ethereum Node  │  │   Cosmos Node   │  │  Solana Node    │
│  (almanac-eth)  │  │ (almanac-cosmos)│  │ (almanac-sol)   │
└────────┬────────┘  └────────┬────────┘  └────────┬────────┘
         │                    │                    │
         └────────────────────┴────────────────────┘
                              │
                    ┌─────────▼─────────┐
                    │   FoundationDB    │
                    │  (Shared Storage) │
                    └─────────┬─────────┘
                              │
                    ┌─────────▼─────────┐
                    │   almanac-api     │
                    │  (Query Layer)    │
                    └───────────────────┘
```

## Adding a New Chain

### Step 1: Create Chain-Specific Binary

Create a new binary crate for your chain adapter:

```toml
# crates/nodes/almanac-mychain/Cargo.toml
[package]
name = "almanac-mychain"
version = "0.1.0"
edition = "2021"

[[bin]]
name = "almanac-mychain"
path = "src/main.rs"

[dependencies]
almanac-core = { path = "../../almanac-core" }
almanac-chains = { path = "../../almanac-chains" }
almanac-consensus = { path = "../../almanac-consensus" }
almanac-indexing = { path = "../../almanac-indexing" }
almanac-storage = { path = "../../almanac-storage", features = ["fdb"] }

# Chain-specific client
valence-domain-client-mychain = { git = "https://github.com/timewave-computer/valence-domain-clients" }

# Async runtime
tokio = { version = "1.32", features = ["full"] }
```

### Step 2: Implement Chain Adapter

```rust
// crates/nodes/almanac-mychain/src/adapter.rs
use almanac_chains::{ChainAdapter, ChainDataIR, BlockMetadata};
use valence_domain_client_mychain::{MychainClient, Block, Event};

pub struct MychainAdapter {
    client: MychainClient,
    chain_id: String,
}

impl MychainAdapter {
    pub async fn new(rpc_url: &str, chain_id: String) -> Result<Self> {
        let client = MychainClient::connect(rpc_url).await?;
        Ok(Self { client, chain_id })
    }
}

#[async_trait]
impl ChainAdapter for MychainAdapter {
    fn chain_id(&self) -> &str {
        &self.chain_id
    }
    
    async fn fetch_block(&self, block_number: u64) -> Result<ChainDataIR> {
        let block = self.client.get_block(block_number).await?;
        
        // Convert to normalized format
        Ok(ChainDataIR {
            block_number,
            block_hash: block.hash.to_string(),
            parent_hash: block.parent_hash.to_string(),
            timestamp: block.timestamp,
            events: self.extract_events(&block).await?,
            state_changes: self.extract_state_changes(&block).await?,
            metadata: self.build_metadata(&block),
        })
    }
    
    async fn get_latest_block(&self) -> Result<u64> {
        self.client.get_latest_block_number().await
    }
    
    async fn is_block_finalized(&self, block_number: u64) -> Result<bool> {
        // Chain-specific finality check
        let latest = self.get_latest_block().await?;
        Ok(latest - block_number >= 6) // Example: 6 confirmations
    }
}
```

### Step 3: Create Node Binary

```rust
// crates/nodes/almanac-mychain/src/main.rs
use clap::Parser;
use almanac_mychain::adapter::MychainAdapter;
use almanac_indexing::IndexingService;
use almanac_storage::FoundationDBBackend;
use almanac_consensus::{create_fork_choice, ForkChoiceRule};

#[derive(Parser)]
struct Args {
    /// RPC endpoint URL
    #[arg(long, env = "MYCHAIN_RPC_URL")]
    rpc_url: String,
    
    /// Chain ID
    #[arg(long, default_value = "mychain")]
    chain_id: String,
    
    /// FoundationDB cluster file
    #[arg(long, env = "FDB_CLUSTER_FILE")]
    fdb_cluster: Option<String>,
    
    /// Node name for identification
    #[arg(long, env = "NODE_NAME")]
    node_name: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    
    // Initialize logging
    tracing_subscriber::fmt::init();
    
    // Create chain adapter
    let adapter = MychainAdapter::new(&args.rpc_url, args.chain_id.clone()).await?;
    
    // Connect to shared FoundationDB
    let storage = FoundationDBBackend::new(
        args.fdb_cluster.as_deref(),
        &format!("almanac/{}", args.chain_id),
    ).await?;
    
    // Create consensus handler
    let fork_choice = create_fork_choice(
        ForkChoiceRule::LongestChain,
        adapter.get_latest_block().await?,
    );
    
    // Start indexing service
    let indexer = IndexingService::builder()
        .chain_adapter(adapter)
        .storage(storage)
        .fork_choice(fork_choice)
        .node_name(&args.node_name)
        .build();
    
    indexer.run().await
}
```

## Chain Adapter Best Practices

### 1. Connection Management
```rust
impl MychainAdapter {
    async fn ensure_connected(&self) -> Result<()> {
        if !self.client.is_connected() {
            self.client.reconnect().await?;
        }
        Ok(())
    }
    
    async fn with_retry<T, F>(&self, f: F) -> Result<T>
    where
        F: Fn() -> Future<Output = Result<T>>,
    {
        let mut retries = 3;
        loop {
            match f().await {
                Ok(result) => return Ok(result),
                Err(e) if retries > 0 => {
                    retries -= 1;
                    tokio::time::sleep(Duration::from_secs(1)).await;
                }
                Err(e) => return Err(e),
            }
        }
    }
}
```

### 2. Event Extraction
```rust
async fn extract_events(&self, block: &Block) -> Result<Vec<Event>> {
    let mut events = Vec::new();
    
    for tx in &block.transactions {
        // Extract events from transaction
        let tx_events = self.client.get_transaction_events(tx.hash).await?;
        
        for event in tx_events {
            events.push(Event {
                id: format!("{}_{}", tx.hash, event.index),
                event_type: event.event_type,
                contract_address: event.address,
                data: event.data,
                topics: event.topics,
                transaction_hash: tx.hash.clone(),
                log_index: event.index,
            });
        }
    }
    
    Ok(events)
}
```

### 3. State Proof Generation
```rust
async fn generate_state_proof(
    &self,
    contract: &str,
    storage_key: &str,
    block_number: u64,
) -> Result<StorageProof> {
    let proof = self.client
        .get_proof(contract, storage_key, block_number)
        .await?;
    
    Ok(StorageProof {
        key: storage_key.to_string(),
        value: proof.value,
        proof_nodes: proof.nodes,
        verified: self.verify_proof(&proof),
    })
}
```

## Node Redundancy

### Load Balancer Configuration
```nginx
# nginx.conf for chain node load balancing
upstream mychain_nodes {
    least_conn;
    server almanac-mychain-1:9090;
    server almanac-mychain-2:9090;
    server almanac-mychain-3:9090;
    
    # Health checks
    check interval=3000 rise=2 fall=5 timeout=1000;
}
```

### Kubernetes Deployment
```yaml
# k8s/mychain-deployment.yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: almanac-mychain
spec:
  replicas: 3
  selector:
    matchLabels:
      app: almanac-mychain
  template:
    metadata:
      labels:
        app: almanac-mychain
    spec:
      containers:
      - name: indexer
        image: almanac-mychain:latest
        env:
        - name: MYCHAIN_RPC_URL
          valueFrom:
            secretKeyRef:
              name: mychain-secrets
              key: rpc-url
        - name: FDB_CLUSTER_FILE
          value: /etc/foundationdb/fdb.cluster
        - name: NODE_NAME
          valueFrom:
            fieldRef:
              fieldPath: metadata.name
        volumeMounts:
        - name: fdb-cluster
          mountPath: /etc/foundationdb
      volumes:
      - name: fdb-cluster
        configMap:
          name: fdb-cluster
```

## Chain-Specific Features

### Consensus Configuration
```toml
# config/chains/mychain.toml
[consensus]
# Fork choice rule
fork_choice = "longest_chain"  # or "ghost", "casper_ffg", etc.

# Finality levels (blocks)
[consensus.finality_levels]
confirmed = 1
safe = 6
justified = 32
finalized = 64

# Reorg handling
max_reorg_depth = 100
reorg_detection_enabled = true
```

### Performance Tuning
```toml
[indexing]
# Batch processing
batch_size = 100
max_concurrent_blocks = 10

# Rate limiting
requests_per_second = 50
burst_size = 100

# Caching
block_cache_size = 1000
event_cache_ttl = 300  # seconds
```

## Testing Chain Adapters

### Unit Tests
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use almanac_testing::mocks::MockRpcClient;
    
    #[tokio::test]
    async fn test_block_fetching() {
        let mock_client = MockRpcClient::new()
            .with_block(100, test_block());
        
        let adapter = MychainAdapter::with_client(mock_client);
        let block = adapter.fetch_block(100).await.unwrap();
        
        assert_eq!(block.block_number, 100);
    }
}
```

### Integration Tests
```rust
#[tokio::test]
#[ignore] // Run with --ignored flag
async fn test_real_chain_connection() {
    let adapter = MychainAdapter::new(
        "https://rpc.mychain.io",
        "mychain-mainnet"
    ).await.unwrap();
    
    let latest = adapter.get_latest_block().await.unwrap();
    assert!(latest > 0);
}
```

## Monitoring

### Metrics
```rust
use prometheus::{Counter, Histogram};

lazy_static! {
    static ref BLOCKS_PROCESSED: Counter = Counter::new(
        "almanac_mychain_blocks_processed_total",
        "Total blocks processed"
    ).unwrap();
    
    static ref BLOCK_FETCH_DURATION: Histogram = Histogram::new(
        "almanac_mychain_block_fetch_duration_seconds",
        "Block fetch duration"
    ).unwrap();
}
```

### Health Checks
```rust
#[derive(Serialize)]
struct HealthStatus {
    chain: String,
    connected: bool,
    latest_block: u64,
    indexing_lag: u64,
}

async fn health_check(&self) -> HealthStatus {
    let connected = self.client.is_connected();
    let latest = self.get_latest_block().await.unwrap_or(0);
    let indexed = self.storage.get_latest_indexed_block().await.unwrap_or(0);
    
    HealthStatus {
        chain: self.chain_id.clone(),
        connected,
        latest_block: latest,
        indexing_lag: latest.saturating_sub(indexed),
    }
}
```

## API Integration

### Querying Chain Data
```bash
# Get events from specific chain
curl -X GET "http://localhost:8080/api/v1/events?chain_id=mychain&block_number=100"

# Get chain status
curl -X GET "http://localhost:8080/api/v1/chains/mychain/status"

# Subscribe to events
ws://localhost:8080/ws/events?chain_id=mychain
```

### Client Libraries
```typescript
// TypeScript client example
import { AlmanacClient } from '@timewave/almanac-client';

const client = new AlmanacClient({
  endpoint: 'http://localhost:8080',
  apiKey: 'your-api-key'
});

// Query events
const events = await client.events.query({
  chainId: 'mychain',
  contractAddress: '0x1234...',
  eventType: 'Transfer',
  fromBlock: 100,
  toBlock: 200
});

// Subscribe to real-time events
client.events.subscribe({
  chainId: 'mychain',
  filters: { eventType: 'Transfer' }
}, (event) => {
  console.log('New event:', event);
});
```

## Troubleshooting

### Common Issues

#### RPC Connection Failures
- Check RPC endpoint availability
- Verify API key/credentials
- Monitor rate limits
- Use connection pooling

#### Indexing Lag
- Scale up node replicas
- Increase batch size
- Optimize RPC queries
- Add caching layer

#### FoundationDB Performance
- Check cluster status: `nix run .#fdb-manage -- cli --exec status`
- Monitor transaction conflicts
- Adjust transaction size limits
- Scale FDB cluster if needed

### Debug Commands
```bash
# Check node status
almanac-mychain status

# Force reindex from block
almanac-mychain reindex --from-block 1000

# Export chain data
almanac-mychain export --format json --output mychain-data.json

# Validate chain data
almanac-mychain validate --from-block 1000 --to-block 2000
```