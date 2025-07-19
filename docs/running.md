# Running Almanac

This guide covers how to build, configure, and run the Almanac multi-chain indexing cluster.

## Architecture Overview

Almanac uses a distributed architecture with chain-specific nodes that share a FoundationDB data layer:

```
┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐
│  Ethereum Nodes │  │   Cosmos Nodes  │  │  Bitcoin Nodes  │
│  (2-3 replicas) │  │  (2-3 replicas) │  │  (2-3 replicas) │
└────────┬────────┘  └────────┬────────┘  └────────┬────────┘
         │                    │                    │
         └────────────────────┴────────────────────┘
                              │
                    ┌─────────▼─────────┐
                    │   FoundationDB    │
                    │   Cluster (3-5    │
                    │   storage nodes)  │
                    └─────────┬─────────┘
                              │
                    ┌─────────▼─────────┐
                    │   almanac-api     │
                    │  (3+ replicas)    │
                    └───────────────────┘
```

## Prerequisites

### System Requirements
- **Operating System**: Linux, macOS, or Windows with WSL2
- **Memory**: Minimum 16GB RAM for full cluster, 32GB recommended
- **Storage**: 100GB+ available disk space (SSD recommended)
- **CPU**: 8+ cores recommended for production
- **Network**: Low-latency network between nodes

### Software Dependencies
- **Rust**: 1.70+ (install via [rustup](https://rustup.rs/))
- **FoundationDB**: 7.1+ (required for shared storage)
- **Docker & Kubernetes**: (recommended for production deployment)

### Installing FoundationDB
FoundationDB provides the shared storage layer between chain nodes:

```bash
# macOS
brew install foundationdb

# Ubuntu/Debian
wget https://www.foundationdb.org/downloads/7.1.0/ubuntu/installers/foundationdb-clients_7.1.0-1_amd64.deb
wget https://www.foundationdb.org/downloads/7.1.0/ubuntu/installers/foundationdb-server_7.1.0-1_amd64.deb
sudo dpkg -i foundationdb-clients_7.1.0-1_amd64.deb
sudo dpkg -i foundationdb-server_7.1.0-1_amd64.deb

# Configure cluster (for multi-node setup)
sudo nano /etc/foundationdb/fdb.cluster
# Format: clustername:clusterid@node1:port,node2:port,node3:port
```

## Building Chain-Specific Nodes

### Clone the Repository
```bash
git clone https://github.com/timewave-computer/almanac
cd almanac
```

### Build Chain-Specific Binaries
Each blockchain requires its own binary to avoid dependency conflicts:

```bash
# Build Ethereum node
cargo build -p almanac-ethereum --release --no-default-features --features ethereum

# Build Cosmos node
cargo build -p almanac-cosmos --release --no-default-features --features cosmos

# Build Solana node
cargo build -p almanac-solana --release --no-default-features --features solana

# Build API server
cargo build -p almanac-api --release
```

### Install Binaries
```bash
# Install to system
sudo cp target/release/almanac-ethereum /usr/local/bin/
sudo cp target/release/almanac-cosmos /usr/local/bin/
sudo cp target/release/almanac-api /usr/local/bin/

# Or create aliases
alias almanac-eth='./target/release/almanac-ethereum'
alias almanac-cosmos='./target/release/almanac-cosmos'
```

## Cluster Configuration

### FoundationDB Setup
Configure FoundationDB for the shared data layer:

```bash
# Create cluster configuration
sudo nano /etc/foundationdb/fdb.cluster
# Example: almanac:randomid@fdb1.local:4500,fdb2.local:4500,fdb3.local:4500

# Configure storage servers
fdbcli --exec "configure new single ssd"
fdbcli --exec "configure storage_engine=ssd-2"
fdbcli --exec "configure logs=6"
```

### Chain Node Configuration
Each chain node needs its own configuration:

#### Ethereum Node (`config/ethereum.toml`)
```toml
# Node identification
[node]
name = "ethereum-indexer-1"
chain_id = "ethereum"

# RPC Configuration
[rpc]
# Using valence-domain-clients
endpoints = [
  "https://eth-mainnet.g.alchemy.com/v2/KEY1",
  "https://mainnet.infura.io/v3/KEY2",
  "wss://mainnet.infura.io/ws/v3/KEY3"
]
request_timeout = 30
max_retries = 3

# Storage Configuration
[storage]
backend = "fdb"
namespace = "almanac/ethereum"

[storage.fdb]
cluster_file = "/etc/foundationdb/fdb.cluster"
transaction_timeout = 5000
transaction_retry_limit = 10

# Indexing Configuration
[indexing]
start_block = "latest"  # or specific block number
batch_size = 100
max_concurrent_batches = 5
reindex_on_startup = false

# Consensus Configuration
[consensus]
fork_choice = "longest_chain"
finality_levels = { confirmed = 1, safe = 6, justified = 32, finalized = 64 }
reorg_detection = true
max_reorg_depth = 100
```

#### Cosmos Node (`config/cosmos.toml`)
```toml
[node]
name = "cosmos-indexer-1"
chain_id = "cosmoshub-4"

[rpc]
endpoints = [
  "https://rpc.cosmos.network",
  "https://cosmos-rpc.quickapi.com"
]
request_timeout = 30

[storage]
backend = "fdb"
namespace = "almanac/cosmos"

[consensus]
fork_choice = "tendermint_bft"
finality_levels = { confirmed = 1, finalized = 2 }
reorg_detection = true
max_reorg_depth = 10
```

### API Server Configuration (`config/api.toml`)
```toml
[server]
host = "0.0.0.0"
port = 8080
workers = 4

[auth]
enabled = true
token_expiry = 3600

[storage]
backend = "fdb"
namespace = "almanac"

[storage.fdb]
cluster_file = "/etc/foundationdb/fdb.cluster"
read_only = true

[cache]
enabled = true
ttl = 300
max_size = 10000
```

## Running the Cluster

### Start Chain-Specific Nodes

#### Ethereum Nodes
```bash
# Start first Ethereum node
almanac-ethereum --config config/ethereum.toml --node-name eth-1

# Start additional replicas (on different machines)
almanac-ethereum --config config/ethereum.toml --node-name eth-2
almanac-ethereum --config config/ethereum.toml --node-name eth-3
```

#### Cosmos Nodes
```bash
# Start Cosmos nodes
almanac-cosmos --config config/cosmos.toml --node-name cosmos-1
almanac-cosmos --config config/cosmos.toml --node-name cosmos-2
```

#### Monitoring Node Status
```bash
# Check node health
curl http://eth-node-1:9091/health
curl http://cosmos-node-1:9092/health

# View indexing progress
almanac-ethereum status --config config/ethereum.toml
```

### Start API Servers
```bash
# Start multiple API instances for load balancing
almanac-api --config config/api.toml --port 8080
almanac-api --config config/api.toml --port 8081
almanac-api --config config/api.toml --port 8082
```

### Load Balancer Setup
Use nginx or HAProxy to distribute requests:

```nginx
# /etc/nginx/sites-enabled/almanac
upstream almanac_api {
    least_conn;
    server localhost:8080;
    server localhost:8081;
    server localhost:8082;
}

server {
    listen 80;
    server_name almanac.example.com;

    location / {
        proxy_pass http://almanac_api;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "upgrade";
    }
}
```

## Production Deployment

### Docker Images for Chain Nodes

#### Multi-Stage Dockerfile
```dockerfile
# Base builder
FROM rust:1.70 as builder
WORKDIR /app
COPY . .

# Ethereum node
FROM builder as ethereum-builder
RUN cargo build -p almanac-ethereum --release --no-default-features --features ethereum

# Cosmos node
FROM builder as cosmos-builder
RUN cargo build -p almanac-cosmos --release --no-default-features --features cosmos

# API server
FROM builder as api-builder
RUN cargo build -p almanac-api --release

# Runtime images
FROM debian:bookworm-slim as runtime
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

# Ethereum runtime
FROM runtime as ethereum
COPY --from=ethereum-builder /app/target/release/almanac-ethereum /usr/local/bin/
CMD ["almanac-ethereum"]

# Cosmos runtime
FROM runtime as cosmos
COPY --from=cosmos-builder /app/target/release/almanac-cosmos /usr/local/bin/
CMD ["almanac-cosmos"]

# API runtime
FROM runtime as api
COPY --from=api-builder /app/target/release/almanac-api /usr/local/bin/
EXPOSE 8080
CMD ["almanac-api"]
```

### Kubernetes Deployment

#### FoundationDB StatefulSet
```yaml
apiVersion: apps/v1
kind: StatefulSet
metadata:
  name: foundationdb
spec:
  serviceName: foundationdb
  replicas: 3
  selector:
    matchLabels:
      app: foundationdb
  template:
    metadata:
      labels:
        app: foundationdb
    spec:
      containers:
      - name: foundationdb
        image: foundationdb/foundationdb:7.1.0
        ports:
        - containerPort: 4500
        volumeMounts:
        - name: data
          mountPath: /var/lib/foundationdb
        - name: config
          mountPath: /etc/foundationdb
  volumeClaimTemplates:
  - metadata:
      name: data
    spec:
      accessModes: ["ReadWriteOnce"]
      resources:
        requests:
          storage: 100Gi
```

#### Chain Node Deployments
```yaml
# ethereum-deployment.yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: almanac-ethereum
spec:
  replicas: 3
  selector:
    matchLabels:
      app: almanac-ethereum
  template:
    metadata:
      labels:
        app: almanac-ethereum
    spec:
      containers:
      - name: indexer
        image: almanac:ethereum
        env:
        - name: NODE_NAME
          valueFrom:
            fieldRef:
              fieldPath: metadata.name
        - name: ETHEREUM_RPC_URL
          valueFrom:
            secretKeyRef:
              name: ethereum-secrets
              key: rpc-url
        volumeMounts:
        - name: config
          mountPath: /etc/almanac
        - name: fdb-cluster
          mountPath: /etc/foundationdb
        livenessProbe:
          httpGet:
            path: /health
            port: 9091
          initialDelaySeconds: 30
        resources:
          requests:
            memory: "4Gi"
            cpu: "2"
          limits:
            memory: "8Gi"
            cpu: "4"
      volumes:
      - name: config
        configMap:
          name: ethereum-config
      - name: fdb-cluster
        configMap:
          name: fdb-cluster
---
# Similar deployment for Cosmos nodes
```

### Docker Compose for Development
```yaml
version: '3.8'

services:
  # FoundationDB cluster
  fdb1:
    image: foundationdb/foundationdb:7.1.0
    hostname: fdb1
    volumes:
      - fdb1-data:/var/lib/foundationdb
      - ./fdb-cluster:/etc/foundationdb

  fdb2:
    image: foundationdb/foundationdb:7.1.0
    hostname: fdb2
    volumes:
      - fdb2-data:/var/lib/foundationdb
      - ./fdb-cluster:/etc/foundationdb

  fdb3:
    image: foundationdb/foundationdb:7.1.0
    hostname: fdb3
    volumes:
      - fdb3-data:/var/lib/foundationdb
      - ./fdb-cluster:/etc/foundationdb

  # Ethereum nodes
  eth1:
    image: almanac:ethereum
    depends_on: [fdb1, fdb2, fdb3]
    environment:
      NODE_NAME: eth1
      ETHEREUM_RPC_URL: ${ETHEREUM_RPC_URL}
    volumes:
      - ./config/ethereum.toml:/etc/almanac/config.toml
      - ./fdb-cluster:/etc/foundationdb

  eth2:
    image: almanac:ethereum
    depends_on: [fdb1, fdb2, fdb3]
    environment:
      NODE_NAME: eth2
      ETHEREUM_RPC_URL: ${ETHEREUM_RPC_URL_BACKUP}
    volumes:
      - ./config/ethereum.toml:/etc/almanac/config.toml
      - ./fdb-cluster:/etc/foundationdb

  # Cosmos nodes
  cosmos1:
    image: almanac:cosmos
    depends_on: [fdb1, fdb2, fdb3]
    environment:
      NODE_NAME: cosmos1
    volumes:
      - ./config/cosmos.toml:/etc/almanac/config.toml
      - ./fdb-cluster:/etc/foundationdb

  # API servers
  api1:
    image: almanac:api
    depends_on: [fdb1, fdb2, fdb3]
    ports:
      - "8080:8080"
    volumes:
      - ./config/api.toml:/etc/almanac/config.toml
      - ./fdb-cluster:/etc/foundationdb

  api2:
    image: almanac:api
    depends_on: [fdb1, fdb2, fdb3]
    ports:
      - "8081:8080"
    volumes:
      - ./config/api.toml:/etc/almanac/config.toml
      - ./fdb-cluster:/etc/foundationdb

  # Load balancer
  nginx:
    image: nginx:alpine
    ports:
      - "80:80"
    volumes:
      - ./nginx.conf:/etc/nginx/nginx.conf
    depends_on: [api1, api2]

volumes:
  fdb1-data:
  fdb2-data:
  fdb3-data:
```

## Monitoring and Operations

### Health Monitoring

#### Chain Node Health
```bash
# Check Ethereum node health
curl http://eth-node-1:9091/health
{
  "status": "healthy",
  "chain": "ethereum",
  "latest_block": 18500000,
  "indexed_block": 18499950,
  "lag": 50,
  "peers": 25
}

# Check all nodes
for node in eth-1 eth-2 cosmos-1 cosmos-2; do
  echo "Checking $node..."
  curl -s http://$node:9091/health | jq .
done
```

#### Cluster-Wide Monitoring
```yaml
# prometheus.yml
global:
  scrape_interval: 15s

scrape_configs:
  # Chain nodes
  - job_name: 'almanac-ethereum'
    static_configs:
      - targets: ['eth-1:9091', 'eth-2:9091', 'eth-3:9091']
    relabel_configs:
      - source_labels: [__address__]
        target_label: node_name
        regex: '([^:]+):.*'

  - job_name: 'almanac-cosmos'
    static_configs:
      - targets: ['cosmos-1:9092', 'cosmos-2:9092']

  # API servers
  - job_name: 'almanac-api'
    static_configs:
      - targets: ['api-1:8080', 'api-2:8080', 'api-3:8080']

  # FoundationDB
  - job_name: 'foundationdb'
    static_configs:
      - targets: ['fdb-1:4500', 'fdb-2:4500', 'fdb-3:4500']
```

### Operational Commands

#### Managing Chain Nodes
```bash
# View indexing status
almanac-ethereum status
almanac-cosmos status

# Pause/resume indexing
almanac-ethereum pause
almanac-ethereum resume

# Force reindex from specific block
almanac-ethereum reindex --from-block 18000000

# Export chain data
almanac-ethereum export --from-block 18000000 --to-block 18100000 --format parquet
```

#### FoundationDB Operations
```bash
# Check cluster status
fdbcli --exec "status details"

# View storage usage by namespace
fdbcli --exec "getrange \\xff/almanac/ \\xff/almanac/\\xff 1000"

# Backup data
fdbbackup start -d /backup/almanac-$(date +%Y%m%d)

# Restore from backup
fdbrestore start -r /backup/almanac-20240115
```

### Scaling Operations

#### Adding New Chain Nodes
```bash
# 1. Deploy new node
docker run -d --name eth-4 \
  -v /etc/foundationdb:/etc/foundationdb \
  -v /etc/almanac:/etc/almanac \
  almanac:ethereum \
  --node-name eth-4

# 2. Wait for sync
while ! almanac-ethereum status --node eth-4 | grep -q "synced"; do
  sleep 60
done

# 3. Add to load balancer
# Update nginx/haproxy configuration
```

#### Horizontal Scaling
```yaml
# kubernetes horizontal pod autoscaler
apiVersion: autoscaling/v2
kind: HorizontalPodAutoscaler
metadata:
  name: almanac-ethereum-hpa
spec:
  scaleTargetRef:
    apiVersion: apps/v1
    kind: Deployment
    name: almanac-ethereum
  minReplicas: 2
  maxReplicas: 10
  metrics:
  - type: Resource
    resource:
      name: cpu
      target:
        type: Utilization
        averageUtilization: 70
  - type: Pods
    pods:
      metric:
        name: indexing_lag_blocks
      target:
        type: AverageValue
        averageValue: "100"
```

## Troubleshooting

### Common Issues

#### Chain Node Issues

**RPC Connection Failed**
```bash
# Check RPC endpoints
for url in $ETHEREUM_RPC_URLS; do
  curl -X POST $url \
    -H "Content-Type: application/json" \
    -d '{"jsonrpc":"2.0","method":"eth_blockNumber","params":[],"id":1}'
done

# Switch to backup RPC
almanac-ethereum set-rpc --url https://backup-rpc.example.com
```

**Indexing Lag**
```bash
# Check lag across all nodes
almanac-cluster status --format json | jq '.nodes[] | {name, chain, lag}'

# Increase parallelism
almanac-ethereum set-config --batch-size 200 --concurrent-batches 10
```

#### FoundationDB Issues

**Transaction Conflicts**
```bash
# Monitor conflicts
fdbcli --exec "status details" | grep -i conflict

# Adjust conflict ranges
almanac-admin optimize-fdb --analyze-conflicts
```

**Storage Full**
```bash
# Check usage
fdbcli --exec "status details" | grep -i storage

# Clean old data
almanac-admin cleanup --before "30 days ago"
```

### Performance Optimization

#### Chain Node Tuning
```toml
# High-performance configuration
[indexing]
batch_size = 500
max_concurrent_batches = 20
prefetch_blocks = 100

[cache]
block_cache_size = 10000
event_cache_size = 100000
state_cache_size = 50000

[rpc]
connection_pool_size = 50
request_timeout = 10
retry_backoff = "exponential"
```

#### System Tuning
```bash
# CPU affinity for chain nodes
taskset -c 0-7 almanac-ethereum --node-name eth-1
taskset -c 8-15 almanac-cosmos --node-name cosmos-1

# Memory limits
systemd-run --uid=almanac --property=MemoryMax=8G almanac-ethereum

# Network optimization
tc qdisc add dev eth0 root fq
echo 'net.core.default_qdisc=fq' >> /etc/sysctl.conf
echo 'net.ipv4.tcp_congestion_control=bbr' >> /etc/sysctl.conf
```

## Backup and Recovery

### Automated Backups
```bash
#!/bin/bash
# backup-almanac.sh
DATE=$(date +%Y%m%d-%H%M%S)
BACKUP_DIR="/backups/almanac/$DATE"

# Backup FoundationDB
fdbbackup start -d "$BACKUP_DIR/fdb" --log

# Export chain metadata
for chain in ethereum cosmos; do
  almanac-$chain export --metadata-only -o "$BACKUP_DIR/$chain-meta.json"
done

# Backup configurations
cp -r /etc/almanac "$BACKUP_DIR/config"

# Upload to S3
aws s3 sync "$BACKUP_DIR" "s3://almanac-backups/$DATE/"
```

### Disaster Recovery
```bash
# 1. Restore FoundationDB
fdbrestore start -r s3://almanac-backups/20240115/fdb

# 2. Verify data integrity
almanac-admin verify --deep-check

# 3. Resume indexing from last checkpoint
for chain in ethereum cosmos; do
  almanac-$chain resume --from-checkpoint
done
```