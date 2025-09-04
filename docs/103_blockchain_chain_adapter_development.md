# Chain Adapter Development Guide

This guide covers how to create new chain adapters and set up redundant node deployments.

## Overview

Almanac's distributed architecture allows each blockchain to have its own dedicated binary with isolated dependencies. This prevents version conflicts and allows optimal resource allocation per chain.

## Creating a New Chain Adapter

### Step 1: Set Up the Binary Crate

Create a new crate for your chain-specific binary:

```bash
# Create directory structure
mkdir -p crates/almanac-nodes/mychain/src
cd crates/almanac-nodes/mychain
```

Create `Cargo.toml`:
```toml
[package]
name = "almanac-mychain"
version = "0.1.0"
edition = "2021"

[[bin]]
name = "almanac-mychain"
path = "src/main.rs"

[dependencies]
# Core almanac dependencies
almanac-core = { path = "../../almanac-core" }
almanac-chains = { path = "../../almanac-chains" }
almanac-consensus = { path = "../../almanac-consensus" }
almanac-indexing = { path = "../../almanac-indexing" }
almanac-storage = { path = "../../almanac-storage", features = ["fdb"] }
almanac-config = { path = "../../almanac-config" }

# Valence domain client for your chain
valence-domain-client-mychain = { git = "https://github.com/timewave-computer/valence-domain-clients" }

# Common dependencies
tokio = { version = "1.32", features = ["full"] }
async-trait = "0.1"
serde = { version = "1.0", features = ["derive"] }
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
clap = { version = "4.4", features = ["derive"] }
anyhow = "1.0"

# Metrics
prometheus = "0.13"
axum = "0.7"  # For metrics endpoint
```

### Step 2: Implement the Chain Adapter

Create `src/adapter.rs`:
```rust
use almanac_chains::{
    ChainAdapter, ChainDataIR, BlockMetadata, Event, StateChange, 
    StorageProof, ChainAdapterError
};
use valence_domain_client_mychain::{Client as MychainClient, types::*};
use async_trait::async_trait;
use std::collections::HashMap;

pub struct MychainAdapter {
    client: MychainClient,
    chain_id: String,
    config: MychainConfig,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct MychainConfig {
    pub confirmations: u64,
    pub batch_size: usize,
    pub request_timeout: u64,
}

impl MychainAdapter {
    pub async fn new(
        endpoints: Vec<String>,
        chain_id: String,
        config: MychainConfig,
    ) -> Result<Self, ChainAdapterError> {
        // Initialize client with multiple endpoints for redundancy
        let client = MychainClient::new(&endpoints)
            .with_timeout(config.request_timeout)
            .connect()
            .await
            .map_err(|e| ChainAdapterError::Connection(e.to_string()))?;
        
        Ok(Self {
            client,
            chain_id,
            config,
        })
    }
    
    async fn extract_events(&self, block: &Block) -> Result<Vec<Event>, ChainAdapterError> {
        let mut events = Vec::new();
        
        for tx in &block.transactions {
            // Extract events from transaction
            let tx_receipt = self.client
                .get_transaction_receipt(&tx.hash)
                .await
                .map_err(|e| ChainAdapterError::Rpc(e.to_string()))?;
            
            for (idx, log) in tx_receipt.logs.iter().enumerate() {
                events.push(Event {
                    id: format!("{}_{}_{}", self.chain_id, tx.hash, idx),
                    event_type: self.decode_event_type(log),
                    contract_address: log.address.to_string(),
                    data: log.data.clone(),
                    topics: log.topics.iter().map(|t| t.to_string()).collect(),
                    transaction_hash: tx.hash.to_string(),
                    log_index: idx as u64,
                });
            }
        }
        
        Ok(events)
    }
    
    async fn extract_state_changes(&self, block: &Block) -> Result<Vec<StateChange>, ChainAdapterError> {
        // Extract state changes if your chain supports state diffs
        let mut changes = Vec::new();
        
        // Example: Get state changes from block
        if let Some(state_diff) = self.client.get_state_diff(block.number).await.ok() {
            for (address, account_diff) in state_diff.accounts {
                for (slot, value) in account_diff.storage {
                    changes.push(StateChange {
                        address: address.to_string(),
                        key: slot.to_string(),
                        value: value.to_string(),
                        proof: None, // Will be populated if needed
                    });
                }
            }
        }
        
        Ok(changes)
    }
}

#[async_trait]
impl ChainAdapter for MychainAdapter {
    fn chain_id(&self) -> &str {
        &self.chain_id
    }
    
    async fn fetch_block(&self, block_number: u64) -> Result<ChainDataIR, ChainAdapterError> {
        let block = self.client
            .get_block(block_number)
            .await
            .map_err(|e| ChainAdapterError::Rpc(e.to_string()))?
            .ok_or_else(|| ChainAdapterError::BlockNotFound(block_number))?;
        
        let events = self.extract_events(&block).await?;
        let state_changes = self.extract_state_changes(&block).await?;
        
        let metadata = BlockMetadata {
            timestamp: block.timestamp,
            gas_used: block.gas_used,
            gas_limit: block.gas_limit,
            difficulty: block.difficulty,
            total_difficulty: block.total_difficulty,
            size: block.size,
            extra_data: serde_json::json!({
                "miner": block.miner,
                "nonce": block.nonce,
                "mix_hash": block.mix_hash,
            }),
        };
        
        Ok(ChainDataIR {
            block_number,
            block_hash: block.hash.to_string(),
            parent_hash: block.parent_hash.to_string(),
            timestamp: block.timestamp,
            events,
            state_changes,
            metadata,
        })
    }
    
    async fn get_latest_block(&self) -> Result<u64, ChainAdapterError> {
        self.client
            .get_block_number()
            .await
            .map_err(|e| ChainAdapterError::Rpc(e.to_string()))
    }
    
    async fn is_block_finalized(&self, block_number: u64) -> Result<bool, ChainAdapterError> {
        let latest = self.get_latest_block().await?;
        Ok(latest.saturating_sub(block_number) >= self.config.confirmations)
    }
    
    async fn generate_proof(
        &self,
        block_number: u64,
        address: &str,
        storage_key: &str,
    ) -> Result<StorageProof, ChainAdapterError> {
        let proof = self.client
            .get_proof(address, storage_key, block_number)
            .await
            .map_err(|e| ChainAdapterError::Rpc(e.to_string()))?;
        
        Ok(StorageProof {
            key: storage_key.to_string(),
            value: proof.value,
            proof_nodes: proof.account_proof,
            storage_proof: proof.storage_proof,
            verified: true, // Client should verify
        })
    }
}
```

### Step 3: Create the Main Binary

Create `src/main.rs`:
```rust
mod adapter;
mod health;
mod metrics;

use adapter::{MychainAdapter, MychainConfig};
use almanac_config::Config;
use almanac_consensus::{create_fork_choice, ForkChoiceRule};
use almanac_indexing::IndexingService;
use almanac_storage::FoundationDBBackend;
use clap::Parser;
use std::sync::Arc;
use tracing::{info, error};

#[derive(Parser)]
#[command(name = "almanac-mychain")]
#[command(about = "Almanac indexer for MyChain")]
struct Args {
    /// Configuration file path
    #[arg(short, long, default_value = "config/mychain.toml")]
    config: String,
    
    /// Node name for identification
    #[arg(long, env = "NODE_NAME")]
    node_name: String,
    
    /// Metrics port
    #[arg(long, default_value = "9090")]
    metrics_port: u16,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .json()
        .init();
    
    let args = Args::parse();
    info!("Starting almanac-mychain node: {}", args.node_name);
    
    // Load configuration
    let config = Config::load(&args.config)?;
    let chain_config: MychainConfig = config.get("chain")?;
    
    // Create chain adapter
    let adapter = MychainAdapter::new(
        config.get::<Vec<String>>("rpc.endpoints")?,
        config.get("chain.id")?,
        chain_config,
    ).await?;
    
    // Connect to FoundationDB
    let storage = FoundationDBBackend::new(
        config.get::<Option<String>>("storage.fdb.cluster_file")?.as_deref(),
        &format!("almanac/{}", config.get::<String>("chain.id")?),
    ).await?;
    
    // Create consensus handler
    let fork_choice_rule = config.get::<ForkChoiceRule>("consensus.fork_choice")?;
    let initial_block = adapter.get_latest_block().await?;
    let fork_choice = create_fork_choice(fork_choice_rule, initial_block);
    
    // Start metrics server
    let metrics_handle = tokio::spawn(metrics::serve(args.metrics_port));
    
    // Start health check endpoint
    let health_handle = tokio::spawn(health::serve(
        args.metrics_port + 1,
        Arc::new(adapter.clone()),
        Arc::new(storage.clone()),
    ));
    
    // Create and run indexing service
    let indexer = IndexingService::builder()
        .chain_adapter(adapter)
        .storage(storage)
        .fork_choice(fork_choice)
        .node_name(&args.node_name)
        .batch_size(config.get("indexing.batch_size")?)
        .start_block(config.get("indexing.start_block")?)
        .build();
    
    // Run indexer with graceful shutdown
    let indexer_handle = tokio::spawn(async move {
        if let Err(e) = indexer.run().await {
            error!("Indexer error: {}", e);
        }
    });
    
    // Wait for shutdown signal
    tokio::signal::ctrl_c().await?;
    info!("Shutting down...");
    
    // Clean shutdown
    indexer_handle.abort();
    metrics_handle.abort();
    health_handle.abort();
    
    Ok(())
}
```

### Step 4: Add Health and Metrics

Create `src/health.rs`:
```rust
use axum::{Router, Json, response::IntoResponse};
use serde::Serialize;
use std::sync::Arc;

#[derive(Serialize)]
struct HealthStatus {
    status: &'static str,
    chain: String,
    node_name: String,
    latest_block: u64,
    indexed_block: u64,
    lag: u64,
    version: &'static str,
}

pub async fn serve(
    port: u16,
    adapter: Arc<dyn ChainAdapter>,
    storage: Arc<dyn Storage>,
) -> anyhow::Result<()> {
    let app = Router::new()
        .route("/health", axum::routing::get(health_check))
        .route("/ready", axum::routing::get(readiness_check));
    
    let listener = tokio::net::TcpListener::bind(("0.0.0.0", port)).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

async fn health_check() -> impl IntoResponse {
    Json(HealthStatus {
        status: "healthy",
        chain: "mychain".to_string(),
        node_name: std::env::var("NODE_NAME").unwrap_or_default(),
        latest_block: 0, // Fetch from adapter
        indexed_block: 0, // Fetch from storage
        lag: 0,
        version: env!("CARGO_PKG_VERSION"),
    })
}
```

## Node Redundancy Setup

### 1. Multi-Node Deployment

Deploy multiple instances of each chain adapter for redundancy:

```yaml
# kubernetes/mychain-deployment.yaml
apiVersion: apps/v1
kind: StatefulSet
metadata:
  name: almanac-mychain
spec:
  serviceName: almanac-mychain
  replicas: 3
  podManagementPolicy: Parallel
  selector:
    matchLabels:
      app: almanac-mychain
  template:
    metadata:
      labels:
        app: almanac-mychain
    spec:
      affinity:
        podAntiAffinity:
          requiredDuringSchedulingIgnoredDuringExecution:
          - labelSelector:
              matchExpressions:
              - key: app
                operator: In
                values:
                - almanac-mychain
            topologyKey: kubernetes.io/hostname
      containers:
      - name: indexer
        image: almanac-mychain:latest
        env:
        - name: NODE_NAME
          valueFrom:
            fieldRef:
              fieldPath: metadata.name
        - name: RPC_ENDPOINTS
          value: "https://rpc1.mychain.io,https://rpc2.mychain.io,https://rpc3.mychain.io"
        ports:
        - containerPort: 9090
          name: metrics
        - containerPort: 9091
          name: health
        livenessProbe:
          httpGet:
            path: /health
            port: 9091
          initialDelaySeconds: 30
          periodSeconds: 10
        readinessProbe:
          httpGet:
            path: /ready
            port: 9091
          initialDelaySeconds: 10
          periodSeconds: 5
        resources:
          requests:
            cpu: "2"
            memory: "4Gi"
          limits:
            cpu: "4"
            memory: "8Gi"
```

### 2. RPC Endpoint Redundancy

Configure multiple RPC endpoints with automatic failover:

```toml
# config/mychain.toml
[chain]
id = "mychain"
confirmations = 6

[rpc]
# Multiple endpoints for redundancy
endpoints = [
    "https://rpc1.mychain.io",
    "https://rpc2.mychain.io", 
    "https://rpc3.mychain.io",
    "wss://ws.mychain.io"  # WebSocket for real-time updates
]

# Connection settings
request_timeout = 30
max_retries = 3
retry_delay = 1000  # ms
connection_pool_size = 10

# Health check settings
health_check_interval = 30  # seconds
max_consecutive_failures = 3
```

### 3. Load Distribution

Use FoundationDB's atomic operations to distribute work:

```rust
// src/work_distribution.rs
use foundationdb::{Transaction, RangeOption};

pub struct WorkDistributor {
    namespace: String,
}

impl WorkDistributor {
    pub async fn claim_block_range(
        &self,
        tx: &Transaction,
        node_name: &str,
        batch_size: u64,
    ) -> Result<(u64, u64)> {
        // Atomic increment to claim next block range
        let counter_key = format!("{}/next_block", self.namespace);
        let current = tx.atomic_op(
            &counter_key,
            &batch_size.to_le_bytes(),
            foundationdb::options::MutationType::Add,
        ).await?;
        
        let start_block = u64::from_le_bytes(current);
        let end_block = start_block + batch_size;
        
        // Record claim
        let claim_key = format!("{}/claims/{}/{}", self.namespace, node_name, start_block);
        tx.set(&claim_key, &end_block.to_le_bytes()).await?;
        
        Ok((start_block, end_block))
    }
}
```

### 4. Monitoring Node Health

Set up comprehensive monitoring:

```yaml
# prometheus/rules/mychain-alerts.yml
groups:
- name: mychain_alerts
  rules:
  - alert: MychainNodeDown
    expr: up{job="almanac-mychain"} == 0
    for: 5m
    labels:
      severity: critical
    annotations:
      summary: "MyChain node {{ $labels.instance }} is down"
      
  - alert: MychainIndexingLag
    expr: almanac_indexing_lag_blocks{chain="mychain"} > 100
    for: 10m
    labels:
      severity: warning
    annotations:
      summary: "MyChain indexing lag is {{ $value }} blocks"
      
  - alert: MychainNoHealthyNodes
    expr: count(up{job="almanac-mychain"} == 1) == 0
    for: 1m
    labels:
      severity: critical
    annotations:
      summary: "No healthy MyChain nodes available"
```

### 5. Automatic Failover

Implement health-based RPC failover:

```rust
// src/rpc_pool.rs
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct RpcPool {
    endpoints: Vec<RpcEndpoint>,
    health_states: Arc<RwLock<Vec<bool>>>,
}

struct RpcEndpoint {
    url: String,
    client: MychainClient,
}

impl RpcPool {
    pub async fn get_healthy_client(&self) -> Option<&MychainClient> {
        let states = self.health_states.read().await;
        
        // Try to find a healthy endpoint
        for (idx, endpoint) in self.endpoints.iter().enumerate() {
            if states.get(idx).copied().unwrap_or(false) {
                return Some(&endpoint.client);
            }
        }
        
        None
    }
    
    pub async fn mark_unhealthy(&self, url: &str) {
        let mut states = self.health_states.write().await;
        if let Some(idx) = self.endpoints.iter().position(|e| e.url == url) {
            states[idx] = false;
        }
    }
    
    // Background health checker
    pub async fn health_check_loop(&self) {
        loop {
            for (idx, endpoint) in self.endpoints.iter().enumerate() {
                let healthy = endpoint.client
                    .get_block_number()
                    .await
                    .is_ok();
                
                let mut states = self.health_states.write().await;
                states[idx] = healthy;
            }
            
            tokio::time::sleep(Duration::from_secs(30)).await;
        }
    }
}
```

## Testing Your Chain Adapter

### Unit Tests
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use almanac_testing::mocks::*;
    
    #[tokio::test]
    async fn test_block_extraction() {
        let mock_client = MockMychainClient::new()
            .with_block(100, mock_block())
            .with_receipt(mock_receipt());
        
        let adapter = MychainAdapter::with_client(mock_client);
        let ir = adapter.fetch_block(100).await.unwrap();
        
        assert_eq!(ir.block_number, 100);
        assert_eq!(ir.events.len(), 3);
    }
    
    #[tokio::test]
    async fn test_failover() {
        let pool = RpcPool::new(vec![
            "https://failing.rpc",
            "https://working.rpc",
        ]);
        
        // First endpoint fails
        pool.mark_unhealthy("https://failing.rpc").await;
        
        let client = pool.get_healthy_client().await;
        assert!(client.is_some());
    }
}
```

### Integration Tests
```rust
#[tokio::test]
#[ignore] // Run with --ignored
async fn test_real_mychain_connection() {
    let adapter = MychainAdapter::new(
        vec!["https://testnet.mychain.io".to_string()],
        "mychain-testnet".to_string(),
        Default::default(),
    ).await.unwrap();
    
    let latest = adapter.get_latest_block().await.unwrap();
    let block = adapter.fetch_block(latest - 1).await.unwrap();
    
    assert!(!block.events.is_empty());
}
```

## Deployment Checklist

- [ ] Create chain-specific binary crate
- [ ] Implement ChainAdapter trait
- [ ] Add health and metrics endpoints
- [ ] Configure multiple RPC endpoints
- [ ] Set up Kubernetes manifests
- [ ] Configure monitoring alerts
- [ ] Test failover scenarios
- [ ] Document chain-specific configuration
- [ ] Add integration tests
- [ ] Set up CI/CD pipeline