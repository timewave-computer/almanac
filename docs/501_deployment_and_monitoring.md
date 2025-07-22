# Deployment and Monitoring Guide

This guide covers production deployment strategies and monitoring for the Almanac multi-chain indexing system.

## Deployment Architecture

### Service Overview

Almanac uses a distributed microservices architecture with the following components:

```
┌──────────────────────────────────────────────────────────────────┐
│                      Production Deployment                       │
├──────────────────────────────────────────────────────────────────┤
│  ┌─────────────────┐    ┌─────────────────┐    ┌───────────────┐ │
│  │   Load Balancer │    │     Ingress     │    │   API Gateway │ │
│  │      :443       │    │   Controller    │    │     :8080     │ │
│  └─────────────────┘    └─────────────────┘    └───────────────┘ │
│                                  │                               │
│            ┌─────────────────────┼─────────────────────┐         │
│            │                     │                     │         │
│  ┌─────────▼─────────┐ ┌─────────▼─────────┐ ┌─────────▼───────┐ │
│  │  almanac-api      │ │  almanac-api      │ │  almanac-api    │ │
│  │   (replica-1)     │ │   (replica-2)     │ │   (replica-3)   │ │
│  │     :8080         │ │     :8080         │ │     :8080       │ │
│  └───────────────────┘ └───────────────────┘ └─────────────────┘ │
│                                  │                               │
│  ┌─────────────────────────────────────────────────────────────┐ │
│  │                  FoundationDB Cluster                       │ │
│  │     coordinator-1:4500  coordinator-2:4500  coordinator-3   │ │
│  └─────────────────────────────────────────────────────────────┘ │
│                                    ▲                             │
│  ┌───────────────┬─────────────────┼─────────────────┐           │
│  │               │                 │                 │           │
│  ▼               ▼                 ▼                 ▼           │
│ ┌────────────┐ ┌─────────────┐ ┌─────────────┐ ┌─────────────┐   │
│ │ ethereum   │ │   cosmos    │ │   solana    │ │  monitoring │   │
│ │ indexer    │ │  indexer    │ │  indexer    │ │   stack     │   │
│ │ :9091      │ │  :9092      │ │  :9093      │ │  :9090      │   │
│ └────────────┘ └─────────────┘ └─────────────┘ └─────────────┘   │
└──────────────────────────────────────────────────────────────────┘
```

## Nix-Based Deployment

### Overview

Almanac uses Nix for reproducible builds and deployments, ensuring consistent environments across development, staging, and production. All services are built and deployed using Nix packages and flakes.

### Building Services with Nix

#### Build All Services
```bash
# Build all Almanac services using cargo in nix environment
nix develop --command cargo build --all --release

# Build with specific features
nix develop --command cargo build --all --features fdb

# Build specific crate
nix develop --command cargo build -p almanac-api --features fdb
```

#### Service Packages
Each service is defined as a Nix package with all dependencies:

```nix
# flake.nix (excerpt)
{
  packages = {
    almanac-api = pkgs.rustPlatform.buildRustPackage {
      pname = "almanac-api";
      version = "1.0.0";
      src = ./.;
      cargoLock.lockFile = ./Cargo.lock;
      
      buildFeatures = [ "fdb" ];
      
      nativeBuildInputs = with pkgs; [
        pkg-config
        foundationdb
      ];
      
      buildInputs = with pkgs; [
        openssl
        foundationdb
      ];
      
      # Set runtime environment
      postInstall = ''
        wrapProgram $out/bin/almanac-api \
          --set LD_LIBRARY_PATH ${pkgs.foundationdb}/lib
      '';
    };
    
    almanac-ethereum = pkgs.rustPlatform.buildRustPackage {
      pname = "almanac-ethereum";
      version = "1.0.0";
      src = ./.;
      cargoLock.lockFile = ./Cargo.lock;
      
      buildFeatures = [ "fdb" ];
      cargoBuildFlags = [ "-p" "almanac-ethereum" ];
      
      nativeBuildInputs = with pkgs; [
        pkg-config
        foundationdb
      ];
      
      buildInputs = with pkgs; [
        openssl
        foundationdb
      ];
      
      postInstall = ''
        wrapProgram $out/bin/almanac-ethereum \
          --set LD_LIBRARY_PATH ${pkgs.foundationdb}/lib
      '';
    };
  };
}
```

### Deployment Scripts

#### Production Deployment Script
```bash
#!/usr/bin/env bash
# scripts/deploy-production.sh

set -euo pipefail

ENVIRONMENT=${1:-production}
TARGET_HOST=${2:-almanac-prod.example.com}

echo "Deploying Almanac to ${ENVIRONMENT} on ${TARGET_HOST}"

# Build all services
echo "Building services..."
nix develop --command cargo build --all --release

# Copy binaries to target
echo "Copying binaries to ${TARGET_HOST}..."
scp target/release/almanac-api ${TARGET_HOST}:/opt/almanac/bin/
scp target/release/almanac-ethereum ${TARGET_HOST}:/opt/almanac/bin/
scp target/release/almanac-cosmos ${TARGET_HOST}:/opt/almanac/bin/
scp target/release/almanac-solana ${TARGET_HOST}:/opt/almanac/bin/

# Deploy systemd services
echo "Deploying systemd services..."
ssh ${TARGET_HOST} sudo systemctl stop almanac-\*.service || true

# Copy service definitions
scp systemd/almanac-*.service ${TARGET_HOST}:/tmp/
ssh ${TARGET_HOST} sudo cp /tmp/almanac-*.service /etc/systemd/system/

# Reload and restart services
ssh ${TARGET_HOST} sudo systemctl daemon-reload
ssh ${TARGET_HOST} sudo systemctl enable almanac-api.service
ssh ${TARGET_HOST} sudo systemctl enable almanac-ethereum.service
ssh ${TARGET_HOST} sudo systemctl enable almanac-cosmos.service
ssh ${TARGET_HOST} sudo systemctl enable almanac-solana.service

ssh ${TARGET_HOST} sudo systemctl start almanac-api.service
ssh ${TARGET_HOST} sudo systemctl start almanac-ethereum.service
ssh ${TARGET_HOST} sudo systemctl start almanac-cosmos.service
ssh ${TARGET_HOST} sudo systemctl start almanac-solana.service

echo "Deployment complete!"
```

### SystemD Service Deployment

#### Service Definitions

Create systemd service files for each Almanac service:

```ini
# systemd/almanac-api.service
[Unit]
Description=Almanac API Server
After=network.target foundationdb.service
Wants=foundationdb.service

[Service]
Type=simple
User=almanac
Group=almanac
WorkingDirectory=/opt/almanac
ExecStart=/opt/almanac/bin/almanac-api
Restart=always
RestartSec=10

# Environment variables
Environment=API_HOST=0.0.0.0
Environment=API_PORT=8080
Environment=METRICS_PORT=9090
Environment=STORAGE_BACKEND=foundationdb
Environment=FDB_CLUSTER_FILE=/etc/foundationdb/fdb.cluster
Environment=FDB_NAMESPACE=almanac_prod
Environment=RUST_LOG=info
Environment=ENABLE_CORS=true

# Security settings
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=/opt/almanac/logs

[Install]
WantedBy=multi-user.target
```

```ini
# systemd/almanac-ethereum.service
[Unit]
Description=Almanac Ethereum Indexer
After=network.target foundationdb.service almanac-api.service
Wants=foundationdb.service

[Service]
Type=simple
User=almanac
Group=almanac
WorkingDirectory=/opt/almanac
ExecStart=/opt/almanac/bin/almanac-ethereum --node-name=ethereum-prod-1 --config=/etc/almanac/ethereum.toml
Restart=always
RestartSec=15

# Environment variables
Environment=CHAIN_ID=ethereum
Environment=METRICS_PORT=9091
Environment=FDB_CLUSTER_FILE=/etc/foundationdb/fdb.cluster
Environment=FDB_NAMESPACE=almanac_prod
Environment=RUST_LOG=info
Environment=ETHEREUM_RPC_URL=https://mainnet.infura.io/v3/YOUR_PROJECT_ID

# Security settings
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=/opt/almanac/logs

[Install]
WantedBy=multi-user.target
```

```ini
# systemd/almanac-cosmos.service
[Unit]
Description=Almanac Cosmos Indexer
After=network.target foundationdb.service almanac-api.service
Wants=foundationdb.service

[Service]
Type=simple
User=almanac
Group=almanac
WorkingDirectory=/opt/almanac
ExecStart=/opt/almanac/bin/almanac-cosmos --node-name=cosmos-prod-1 --config=/etc/almanac/cosmos.toml
Restart=always
RestartSec=15

# Environment variables
Environment=CHAIN_ID=cosmos
Environment=METRICS_PORT=9092
Environment=FDB_CLUSTER_FILE=/etc/foundationdb/fdb.cluster
Environment=FDB_NAMESPACE=almanac_prod
Environment=RUST_LOG=info
Environment=COSMOS_RPC_URL=https://rpc.cosmos.network

# Security settings
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=/opt/almanac/logs

[Install]
WantedBy=multi-user.target
```

### NixOS Configuration

For NixOS deployments, use a declarative configuration:

```nix
# nixos-config.nix
{ config, pkgs, ... }:

let
  almanac = import ./flake.nix;
in {
  # System packages
  environment.systemPackages = with pkgs; [
    almanac.packages.x86_64-linux.almanac-api
    almanac.packages.x86_64-linux.almanac-ethereum
    almanac.packages.x86_64-linux.almanac-cosmos
    almanac.packages.x86_64-linux.almanac-solana
    foundationdb
  ];

  # User and group
  users.users.almanac = {
    isSystemUser = true;
    group = "almanac";
    home = "/opt/almanac";
    createHome = true;
  };
  users.groups.almanac = {};

  # FoundationDB service
  services.foundationdb = {
    enable = true;
    package = pkgs.foundationdb;
    openFirewall = true;
  };

  # Almanac services
  systemd.services = {
    almanac-api = {
      description = "Almanac API Server";
      after = [ "network.target" "foundationdb.service" ];
      wants = [ "foundationdb.service" ];
      wantedBy = [ "multi-user.target" ];

      serviceConfig = {
        Type = "simple";
        User = "almanac";
        Group = "almanac";
        WorkingDirectory = "/opt/almanac";
        ExecStart = "${almanac.packages.x86_64-linux.almanac-api}/bin/almanac-api";
        Restart = "always";
        RestartSec = 10;

        # Security
        NoNewPrivileges = true;
        PrivateTmp = true;
        ProtectSystem = "strict";
        ProtectHome = true;
        ReadWritePaths = [ "/opt/almanac/logs" ];
      };

      environment = {
        API_HOST = "0.0.0.0";
        API_PORT = "8080";
        METRICS_PORT = "9090";
        STORAGE_BACKEND = "foundationdb";
        FDB_CLUSTER_FILE = "/etc/foundationdb/fdb.cluster";
        FDB_NAMESPACE = "almanac_prod";
        RUST_LOG = "info";
        ENABLE_CORS = "true";
      };
    };

    almanac-ethereum = {
      description = "Almanac Ethereum Indexer";
      after = [ "network.target" "foundationdb.service" "almanac-api.service" ];
      wants = [ "foundationdb.service" ];
      wantedBy = [ "multi-user.target" ];

      serviceConfig = {
        Type = "simple";
        User = "almanac";
        Group = "almanac";
        WorkingDirectory = "/opt/almanac";
        ExecStart = "${almanac.packages.x86_64-linux.almanac-ethereum}/bin/almanac-ethereum --node-name=ethereum-prod-1";
        Restart = "always";
        RestartSec = 15;

        # Security
        NoNewPrivileges = true;
        PrivateTmp = true;
        ProtectSystem = "strict";
        ProtectHome = true;
        ReadWritePaths = [ "/opt/almanac/logs" ];
      };

      environment = {
        CHAIN_ID = "ethereum";
        METRICS_PORT = "9091";
        FDB_CLUSTER_FILE = "/etc/foundationdb/fdb.cluster";
        FDB_NAMESPACE = "almanac_prod";
        RUST_LOG = "info";
        ETHEREUM_RPC_URL = "https://mainnet.infura.io/v3/YOUR_PROJECT_ID";
      };
    };
  };

  # Firewall configuration
  networking.firewall = {
    allowedTCPPorts = [ 8080 9090 9091 9092 9093 4500 ];
  };

  # Log rotation
  services.logrotate = {
    enable = true;
    settings = {
      "/opt/almanac/logs/*.log" = {
        frequency = "daily";
        rotate = 7;
        compress = true;
        delaycompress = true;
        missingok = true;
        notifempty = true;
        create = "644 almanac almanac";
      };
    };
  };
}

## Monitoring and Observability

### Health Check Endpoints

Each service provides comprehensive health monitoring:

#### API Server Health
- **Health Check**: `GET /health`
- **Readiness**: `GET /ready` 
- **Metrics**: `GET /metrics`

```json
{
  "status": "healthy",
  "components": {
    "storage": {
      "status": "healthy",
      "last_check": "2024-01-15T10:30:00Z"
    },
    "database": {
      "status": "healthy", 
      "last_check": "2024-01-15T10:30:00Z"
    }
  },
  "timestamp": "2024-01-15T10:30:00Z"
}
```

#### Chain Indexer Health
- **Health Check**: `GET :9091/health` (Ethereum)
- **Readiness**: `GET :9091/ready`
- **Metrics**: `GET :9091/metrics`

```json
{
  "status": "healthy",
  "chain": "ethereum",
  "node_name": "ethereum-indexer-1",
  "latest_block": 18500000,
  "indexed_block": 18499950,
  "lag": 50,
  "version": "1.0.0"
}
```

### Prometheus Monitoring

#### Service Discovery Configuration

```yaml
# prometheus/service-discovery.yml
- job_name: 'almanac-api'
  kubernetes_sd_configs:
  - role: pod
    namespaces:
      names:
      - almanac
  relabel_configs:
  - source_labels: [__meta_kubernetes_pod_annotation_prometheus_io_scrape]
    action: keep
    regex: true
  - source_labels: [__meta_kubernetes_pod_annotation_prometheus_io_path]
    action: replace
    target_label: __metrics_path__
    regex: (.+)
  - source_labels: [__address__, __meta_kubernetes_pod_annotation_prometheus_io_port]
    action: replace
    regex: ([^:]+)(?::\d+)?;(\d+)
    replacement: $1:$2
    target_label: __address__
  - action: labelmap
    regex: __meta_kubernetes_pod_label_(.+)
  - source_labels: [__meta_kubernetes_namespace]
    action: replace
    target_label: kubernetes_namespace
  - source_labels: [__meta_kubernetes_pod_name]
    action: replace
    target_label: kubernetes_pod_name

- job_name: 'almanac-indexers'
  static_configs:
  - targets:
    - 'almanac-ethereum:9091'
    - 'almanac-cosmos:9092'
    - 'almanac-solana:9093'
  metrics_path: /metrics
  scrape_interval: 15s
```

#### Key Metrics

**API Server Metrics**:
- `http_requests_total` - HTTP request count
- `http_request_duration_seconds` - Request latency
- `storage_operations_total` - Storage operation count
- `storage_operation_duration_seconds` - Storage latency
- `api_concurrent_connections` - Active connections

**Chain Indexer Metrics**:
- `almanac_blocks_processed_total{chain="ethereum"}` - Blocks processed
- `almanac_indexing_lag_blocks{chain="ethereum"}` - Indexing lag
- `almanac_events_indexed_total{chain="ethereum"}` - Events indexed
- `almanac_rpc_requests_total{chain="ethereum"}` - RPC requests
- `almanac_rpc_errors_total{chain="ethereum"}` - RPC errors

**FoundationDB Metrics**:
- `fdb_operations_total` - FDB operations
- `fdb_transaction_duration_seconds` - Transaction latency
- `fdb_conflicts_total` - Transaction conflicts
- `fdb_storage_bytes_used` - Storage usage

### Alerting Rules

```yaml
# prometheus/alerts/almanac-alerts.yml
groups:
- name: almanac_alerts
  rules:
  
  # API Server Alerts
  - alert: AlmanacAPIDown
    expr: up{job="almanac-api"} == 0
    for: 2m
    labels:
      severity: critical
    annotations:
      summary: "Almanac API server is down"
      description: "API server {{ $labels.instance }} has been down for more than 2 minutes"
      
  - alert: AlmanacAPIHighLatency
    expr: histogram_quantile(0.95, http_request_duration_seconds_bucket{job="almanac-api"}) > 0.5
    for: 5m
    labels:
      severity: warning
    annotations:
      summary: "High API latency detected"
      description: "95th percentile latency is {{ $value }}s for {{ $labels.instance }}"
      
  - alert: AlmanacAPIHighErrorRate
    expr: rate(http_requests_total{job="almanac-api",status=~"5.."}[5m]) > 0.1
    for: 5m
    labels:
      severity: critical
    annotations:
      summary: "High API error rate"
      description: "Error rate is {{ $value }} for {{ $labels.instance }}"

  # Chain Indexer Alerts
  - alert: AlmanacIndexerDown
    expr: up{job="almanac-indexers"} == 0
    for: 5m
    labels:
      severity: critical
    annotations:
      summary: "Almanac indexer is down"
      description: "{{ $labels.chain }} indexer {{ $labels.instance }} has been down for more than 5 minutes"
      
  - alert: AlmanacIndexingLag
    expr: almanac_indexing_lag_blocks > 100
    for: 10m
    labels:
      severity: warning
    annotations:
      summary: "High indexing lag detected"
      description: "{{ $labels.chain }} indexing lag is {{ $value }} blocks on {{ $labels.instance }}"
      
  - alert: AlmanacIndexingStalled
    expr: increase(almanac_blocks_processed_total[5m]) == 0
    for: 15m
    labels:
      severity: critical
    annotations:
      summary: "Indexing appears to be stalled"
      description: "No blocks processed in the last 15 minutes for {{ $labels.chain }} on {{ $labels.instance }}"

  # FoundationDB Alerts  
  - alert: FoundationDBHighConflicts
    expr: rate(fdb_conflicts_total[5m]) > 10
    for: 10m
    labels:
      severity: warning
    annotations:
      summary: "High FoundationDB conflict rate"
      description: "Transaction conflict rate is {{ $value }} per second"
      
  - alert: FoundationDBHighLatency
    expr: histogram_quantile(0.95, fdb_transaction_duration_seconds_bucket) > 0.1
    for: 5m
    labels:
      severity: warning
    annotations:
      summary: "High FoundationDB latency"
      description: "95th percentile transaction latency is {{ $value }}s"
```

### Grafana Dashboards

#### API Server Dashboard

```json
{
  "dashboard": {
    "title": "Almanac API Server",
    "panels": [
      {
        "title": "Request Rate",
        "type": "graph",
        "targets": [
          {
            "expr": "rate(http_requests_total{job=\"almanac-api\"}[5m])",
            "legendFormat": "{{ instance }}"
          }
        ]
      },
      {
        "title": "Response Time (95th percentile)",
        "type": "graph", 
        "targets": [
          {
            "expr": "histogram_quantile(0.95, http_request_duration_seconds_bucket{job=\"almanac-api\"})",
            "legendFormat": "{{ instance }}"
          }
        ]
      },
      {
        "title": "Error Rate",
        "type": "graph",
        "targets": [
          {
            "expr": "rate(http_requests_total{job=\"almanac-api\",status=~\"5..\"}[5m])",
            "legendFormat": "{{ instance }}"
          }
        ]
      }
    ]
  }
}
```

#### Chain Indexer Dashboard

```json
{
  "dashboard": {
    "title": "Almanac Chain Indexers",
    "panels": [
      {
        "title": "Indexing Lag by Chain",
        "type": "graph",
        "targets": [
          {
            "expr": "almanac_indexing_lag_blocks",
            "legendFormat": "{{ chain }} - {{ instance }}"
          }
        ]
      },
      {
        "title": "Blocks Processed Rate",
        "type": "graph",
        "targets": [
          {
            "expr": "rate(almanac_blocks_processed_total[5m])",
            "legendFormat": "{{ chain }} - {{ instance }}"
          }
        ]
      },
      {
        "title": "Events Indexed Rate", 
        "type": "graph",
        "targets": [
          {
            "expr": "rate(almanac_events_indexed_total[5m])",
            "legendFormat": "{{ chain }} - {{ instance }}"
          }
        ]
      }
    ]
  }
}
```

### Logging

#### Structured Logging Configuration

```bash
# Environment variables for production logging
export RUST_LOG="almanac_api=info,almanac_ethereum=info,almanac_cosmos=info,almanac_solana=info"
export RUST_LOG_FORMAT="json"
export LOG_LEVEL="info"
```

#### Log Aggregation with Fluentd

```yaml
# fluentd/configmap.yaml
apiVersion: v1
kind: ConfigMap
metadata:
  name: fluentd-config
  namespace: almanac
data:
  fluent.conf: |
    <source>
      @type tail
      path /var/log/containers/almanac-*.log
      pos_file /var/log/fluentd-almanac.log.pos
      tag kubernetes.almanac.*
      format json
      time_key timestamp
      time_format %Y-%m-%dT%H:%M:%S.%NZ
    </source>
    
    <filter kubernetes.almanac.**>
      @type kubernetes_metadata
    </filter>
    
    <match kubernetes.almanac.**>
      @type elasticsearch
      host elasticsearch.logging.svc.cluster.local
      port 9200
      index_name almanac-logs
      type_name _doc
    </match>
```

## Performance Tuning

### FoundationDB Tuning

```bash
# Production FoundationDB configuration
nix run .#fdb-manage -- cli --exec "configure memory"
nix run .#fdb-manage -- cli --exec "configure storage_engine=ssd-redwood-1"
nix run .#fdb-manage -- cli --exec "configure redundancy_mode=triple"

# Set process classes for dedicated roles
fdbserver --class=storage --memory=32GiB --storage_memory=8GiB
fdbserver --class=transaction --memory=8GiB
fdbserver --class=proxy --memory=8GiB
```

### API Server Tuning

```bash
# Environment variables for performance tuning
export MAX_CONNECTIONS=1000
export CONNECTION_TIMEOUT=30
export KEEPALIVE_TIMEOUT=60
export WORKER_THREADS=8
export BLOCKING_THREADS=16

# systemd service with resource limits
systemctl edit almanac-api.service
```

```ini
[Service]
# Memory limits
MemoryMax=16G
MemoryHigh=12G

# CPU limits  
CPUQuota=800%

# Connection limits
LimitNOFILE=65536
LimitNPROC=4096

# Environment tuning
Environment=MAX_CONNECTIONS=1000
Environment=CONNECTION_TIMEOUT=30
Environment=KEEPALIVE_TIMEOUT=60
```

### Chain Indexer Tuning

#### Ethereum Indexer (High Throughput)
```ini
[Service]
# Memory and CPU for high-volume chain
MemoryMax=16G
MemoryHigh=12G
CPUQuota=800%

# Indexing optimization
Environment=BATCH_SIZE=1000
Environment=MAX_CONCURRENT_BATCHES=20
Environment=RPC_TIMEOUT=30
Environment=REQUEST_POOL_SIZE=50
```

#### Cosmos Indexer (Medium Throughput)
```ini
[Service]
# Moderate resource allocation
MemoryMax=8G
MemoryHigh=6G
CPUQuota=400%

# Optimized for instant finality
Environment=BATCH_SIZE=500
Environment=MAX_CONCURRENT_BATCHES=10
Environment=RPC_TIMEOUT=10
Environment=REQUEST_POOL_SIZE=20
```

#### Solana Indexer (Very High Throughput)
```ini
[Service]
# Maximum resource allocation
MemoryMax=32G
MemoryHigh=24G
CPUQuota=1600%

# Optimized for high-frequency slots
Environment=BATCH_SIZE=2000
Environment=MAX_CONCURRENT_BATCHES=50
Environment=RPC_TIMEOUT=5
Environment=REQUEST_POOL_SIZE=100
```

## Security

### Firewall Configuration

```bash
# Configure firewall for Almanac services
ufw allow 8080/tcp   # API Server
ufw allow 9090/tcp   # API Metrics
ufw allow 9091/tcp   # Ethereum Health/Metrics
ufw allow 9092/tcp   # Cosmos Health/Metrics
ufw allow 9093/tcp   # Solana Health/Metrics
ufw allow 4500/tcp   # FoundationDB
ufw allow 443/tcp    # HTTPS
ufw allow 80/tcp     # HTTP (redirect to HTTPS)

# Restrict SSH access
ufw allow from YOUR_IP_RANGE to any port 22

# Enable firewall
ufw enable
```

### TLS Configuration

#### nginx reverse proxy with TLS
```nginx
# /etc/nginx/sites-available/almanac
server {
    listen 443 ssl http2;
    server_name api.almanac.example.com;

    ssl_certificate /etc/letsencrypt/live/api.almanac.example.com/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/api.almanac.example.com/privkey.pem;

    # SSL configuration
    ssl_protocols TLSv1.2 TLSv1.3;
    ssl_ciphers ECDHE-RSA-AES256-GCM-SHA512:DHE-RSA-AES256-GCM-SHA512:ECDHE-RSA-AES256-GCM-SHA384:DHE-RSA-AES256-GCM-SHA384;
    ssl_prefer_server_ciphers off;

    # Rate limiting
    limit_req_zone $binary_remote_addr zone=api:10m rate=100r/m;
    limit_req zone=api burst=20 nodelay;

    # CORS headers
    add_header 'Access-Control-Allow-Origin' '*' always;
    add_header 'Access-Control-Allow-Methods' 'GET, POST, PUT, DELETE, OPTIONS' always;
    add_header 'Access-Control-Allow-Headers' 'Content-Type, Authorization, X-API-Key' always;

    location / {
        proxy_pass http://127.0.0.1:8080;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
        
        # WebSocket support
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "upgrade";
    }

    location /metrics {
        proxy_pass http://127.0.0.1:9090/metrics;
        allow YOUR_MONITORING_IP;
        deny all;
    }
}

# Redirect HTTP to HTTPS
server {
    listen 80;
    server_name api.almanac.example.com;
    return 301 https://$server_name$request_uri;
}
```

#### Let's Encrypt SSL Certificate
```bash
# Install certbot
apt install certbot python3-certbot-nginx

# Get certificate
certbot --nginx -d api.almanac.example.com

# Auto-renewal
crontab -e
# Add: 0 12 * * * /usr/bin/certbot renew --quiet
```

## Disaster Recovery

### Backup Strategy

```bash
#!/bin/bash
# backup/fdb-backup.sh

# FoundationDB backup
DATE=$(date +%Y%m%d_%H%M%S)
BACKUP_DIR="/backups/almanac/${DATE}"

# Create backup
fdbbackup start -t almanac_backup_${DATE} -d ${BACKUP_DIR}

# Monitor backup progress
fdbbackup status -t almanac_backup_${DATE}

# Upload to cloud storage
aws s3 sync ${BACKUP_DIR} s3://almanac-backups/${DATE}/
```

### Recovery Procedures

```bash
#!/bin/bash
# recovery/restore-fdb.sh

BACKUP_DATE=$1
BACKUP_DIR="/backups/almanac/${BACKUP_DATE}"

# Download from cloud storage
aws s3 sync s3://almanac-backups/${BACKUP_DATE}/ ${BACKUP_DIR}

# Restore FoundationDB
fdbrestore start -r ${BACKUP_DIR} -w

# Verify restoration
nix run .#fdb-manage -- cli --exec "status"
```

## Scaling

### Load Balancer Configuration

#### nginx Load Balancing
```nginx
# /etc/nginx/conf.d/almanac-upstream.conf
upstream almanac_api {
    least_conn;
    server 127.0.0.1:8080 max_fails=3 fail_timeout=30s;
    server 127.0.0.1:8081 max_fails=3 fail_timeout=30s;
    server 127.0.0.1:8082 max_fails=3 fail_timeout=30s;
}

server {
    listen 443 ssl http2;
    server_name api.almanac.example.com;

    location / {
        proxy_pass http://almanac_api;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
    }
}
```

### Multi-Instance Deployment

#### Multiple API Server Instances
```bash
# Create multiple systemd services
for i in {0..2}; do
    PORT=$((8080 + i))
    METRICS_PORT=$((9090 + i))
    
    cat > /etc/systemd/system/almanac-api-${i}.service << EOF
[Unit]
Description=Almanac API Server Instance ${i}
After=network.target foundationdb.service
Wants=foundationdb.service

[Service]
Type=simple
User=almanac
Group=almanac
WorkingDirectory=/opt/almanac
ExecStart=/opt/almanac/bin/almanac-api
Restart=always
RestartSec=10

Environment=API_HOST=0.0.0.0
Environment=API_PORT=${PORT}
Environment=METRICS_PORT=${METRICS_PORT}
Environment=STORAGE_BACKEND=foundationdb
Environment=FDB_CLUSTER_FILE=/etc/foundationdb/fdb.cluster
Environment=FDB_NAMESPACE=almanac_prod
Environment=RUST_LOG=info

[Install]
WantedBy=multi-user.target
EOF

    systemctl enable almanac-api-${i}.service
    systemctl start almanac-api-${i}.service
done
```

### Auto-scaling with systemd

#### CPU-based scaling script
```bash
#!/bin/bash
# scripts/auto-scale-api.sh

SCALE_UP_THRESHOLD=80
SCALE_DOWN_THRESHOLD=30
MIN_INSTANCES=2
MAX_INSTANCES=8

while true; do
    # Get current CPU usage
    CPU_USAGE=$(top -bn1 | grep "almanac-api" | awk '{print $9}' | head -1)
    
    # Count running instances
    RUNNING_INSTANCES=$(systemctl list-units --state=running | grep almanac-api | wc -l)
    
    if (( $(echo "$CPU_USAGE > $SCALE_UP_THRESHOLD" | bc -l) )) && [ $RUNNING_INSTANCES -lt $MAX_INSTANCES ]; then
        # Scale up
        NEXT_INSTANCE=$RUNNING_INSTANCES
        echo "Scaling up: starting almanac-api-${NEXT_INSTANCE}"
        systemctl start almanac-api-${NEXT_INSTANCE}.service
        
    elif (( $(echo "$CPU_USAGE < $SCALE_DOWN_THRESHOLD" | bc -l) )) && [ $RUNNING_INSTANCES -gt $MIN_INSTANCES ]; then
        # Scale down
        LAST_INSTANCE=$((RUNNING_INSTANCES - 1))
        echo "Scaling down: stopping almanac-api-${LAST_INSTANCE}"
        systemctl stop almanac-api-${LAST_INSTANCE}.service
    fi
    
    sleep 60
done
```
