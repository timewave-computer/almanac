# Nix-Based Deployment Guide

This guide covers deploying Almanac services using Nix, replacing Docker-based deployments.

## Overview

Almanac uses Nix for reproducible builds and deployments. This ensures consistent environments across development, testing, and production.

## Prerequisites

- Nix with flakes enabled
- NixOS (for systemd service deployment) or a Nix-compatible system
- PostgreSQL database
- Appropriate network access and firewall configuration

## Building for Deployment

### Build the API Service

```bash
# Build the API HTTP service
nix build .#almanac-api-http

# The binary will be available at:
./result/bin/almanac-api-http
```

### Build All Services

```bash
# Build all Almanac components
nix build .#almanac

# Build specific components
nix build .#almanac-storage
nix build .#almanac-infrastructure-core
```

## Deployment Options

### Option 1: NixOS Module Deployment

For NixOS systems, use the provided module:

```nix
# /etc/nixos/configuration.nix
{ config, pkgs, ... }:

{
  imports = [
    # Import the Almanac flake
    (builtins.getFlake "github:timewave-computer/almanac").nixosModules.almanac-api
  ];

  # Configure the Almanac API service
  services.almanac-api = {
    enable = true;
    
    # API configuration
    port = 8080;
    host = "0.0.0.0";
    metricsPort = 9090;
    
    # Database configuration
    databaseUrl = "postgresql://almanac:password@localhost/almanac_prod";
    
    # Security configuration
    enableCors = true;
    corsOrigins = [ "https://app.example.com" ];
    rateLimitPerMinute = 100;
    
    # Optional: Admin API key for initial setup
    adminApiKey = "your-secure-admin-key";
    
    # Additional environment variables
    extraEnv = {
      RUST_LOG = "almanac_api=info,tower_http=debug";
    };
  };
  
  # Ensure PostgreSQL is available
  services.postgresql = {
    enable = true;
    package = pkgs.postgresql_15;
    ensureDatabases = [ "almanac_prod" ];
    ensureUsers = [{
      name = "almanac";
      ensurePermissions = {
        "DATABASE almanac_prod" = "ALL PRIVILEGES";
      };
    }];
  };
}
```

Then rebuild and switch:

```bash
sudo nixos-rebuild switch
```

### Option 2: Systemd Service (Non-NixOS)

For non-NixOS systems with systemd:

```bash
# Build the service
nix build .#almanac-api-http

# Create systemd service file
sudo tee /etc/systemd/system/almanac-api.service << EOF
[Unit]
Description=Almanac API Service
After=network.target postgresql.service
Wants=postgresql.service

[Service]
Type=notify
ExecStart=$(readlink -f ./result/bin/almanac-api-http)
Restart=always
RestartSec=5

# Environment
Environment="RUST_LOG=info"
Environment="DATABASE_URL=postgresql://almanac:password@localhost/almanac_prod"
Environment="API_HOST=0.0.0.0"
Environment="API_PORT=8080"
Environment="METRICS_PORT=9090"

# Security
DynamicUser=yes
NoNewPrivileges=yes
PrivateTmp=yes
ProtectSystem=strict
ProtectHome=yes
ReadWritePaths=/var/lib/almanac-api

# Resource limits
LimitNOFILE=65536

[Install]
WantedBy=multi-user.target
EOF

# Reload systemd and start service
sudo systemctl daemon-reload
sudo systemctl enable almanac-api
sudo systemctl start almanac-api
```

### Option 3: Container-like Deployment with Nix

Create a self-contained deployment:

```nix
# almanac-deployment.nix
{ pkgs ? import <nixpkgs> {} }:

let
  almanac = (builtins.getFlake "github:timewave-computer/almanac");
  almanac-api = almanac.packages.${pkgs.system}.almanac-api-http;
in
pkgs.dockerTools.buildLayeredImage {
  name = "almanac-api";
  tag = "latest";
  
  contents = with pkgs; [
    almanac-api
    cacert
    tzdata
  ];
  
  config = {
    Cmd = [ "${almanac-api}/bin/almanac-api-http" ];
    Env = [
      "RUST_LOG=info"
      "API_PORT=8080"
      "METRICS_PORT=9090"
    ];
    ExposedPorts = {
      "8080/tcp" = {};
      "9090/tcp" = {};
    };
  };
}
```

Build and load:

```bash
nix build -f almanac-deployment.nix
docker load < result
```

## Configuration Management

### Environment Variables

The API service is configured via environment variables:

```bash
# Core configuration
DATABASE_URL=postgresql://user:pass@host/db
API_HOST=0.0.0.0
API_PORT=8080
METRICS_PORT=9090

# Logging
RUST_LOG=almanac_api=info,tower_http=debug

# CORS
ENABLE_CORS=true
CORS_ORIGINS=https://app.example.com,https://app2.example.com

# Rate limiting
RATE_LIMIT_PER_MINUTE=100

# Authentication
ADMIN_API_KEY=your-secure-key
```

### Configuration File (Alternative)

Create a configuration file:

```toml
# almanac-config.toml
[api]
host = "0.0.0.0"
port = 8080
metrics_port = 9090

[database]
url = "postgresql://almanac:password@localhost/almanac_prod"
max_connections = 10

[security]
enable_cors = true
cors_origins = ["https://app.example.com"]
rate_limit_per_minute = 100

[logging]
level = "info"
format = "json"
```

Load via environment:

```bash
export ALMANAC_CONFIG=/etc/almanac/config.toml
```

## Database Setup

### Initialize Database

```bash
# Connect to PostgreSQL
psql -U postgres

# Create database and user
CREATE DATABASE almanac_prod;
CREATE USER almanac WITH ENCRYPTED PASSWORD 'secure-password';
GRANT ALL PRIVILEGES ON DATABASE almanac_prod TO almanac;

# Run migrations
export DATABASE_URL="postgresql://almanac:secure-password@localhost/almanac_prod"
sqlx migrate run --source crates/infrastructure/storage/migrations
```

### Backup Strategy

```bash
# Create backup script
cat > /usr/local/bin/backup-almanac.sh << 'EOF'
#!/usr/bin/env bash
set -euo pipefail

BACKUP_DIR="/var/backups/almanac"
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
BACKUP_FILE="$BACKUP_DIR/almanac_$TIMESTAMP.sql.gz"

mkdir -p "$BACKUP_DIR"

# Backup database
pg_dump -U almanac almanac_prod | gzip > "$BACKUP_FILE"

# Keep only last 7 days of backups
find "$BACKUP_DIR" -name "almanac_*.sql.gz" -mtime +7 -delete

echo "Backup completed: $BACKUP_FILE"
EOF

chmod +x /usr/local/bin/backup-almanac.sh

# Add to cron
echo "0 2 * * * /usr/local/bin/backup-almanac.sh" | crontab -
```

## Monitoring and Health Checks

### Prometheus Integration

The API exposes metrics at `/metrics` (on the metrics port):

```yaml
# prometheus.yml
scrape_configs:
  - job_name: 'almanac-api'
    static_configs:
      - targets: ['localhost:9090']
    metrics_path: '/metrics'
```

### Health Check Endpoints

- `/health` - General health status
- `/liveness` - Kubernetes liveness probe
- `/readiness` - Kubernetes readiness probe

### Monitoring Script

```bash
#!/usr/bin/env bash
# /usr/local/bin/check-almanac.sh

# Check API health
if ! curl -sf http://localhost:8080/health > /dev/null; then
    echo "API health check failed"
    systemctl restart almanac-api
fi

# Check database connection
if ! curl -sf http://localhost:8080/readiness > /dev/null; then
    echo "API readiness check failed"
    # Alert administrators
fi
```

## Load Balancing and High Availability

### Multiple Instance Deployment

Deploy multiple API instances behind a load balancer:

```nix
# Deploy 3 instances on different ports
services.almanac-api-1 = almanacApiConfig // { port = 8081; };
services.almanac-api-2 = almanacApiConfig // { port = 8082; };
services.almanac-api-3 = almanacApiConfig // { port = 8083; };

# Configure nginx load balancer
services.nginx = {
  enable = true;
  upstreams.almanac-api.servers = {
    "localhost:8081" = {};
    "localhost:8082" = {};
    "localhost:8083" = {};
  };
  
  virtualHosts."api.almanac.example.com" = {
    locations."/" = {
      proxyPass = "http://almanac-api";
      extraConfig = ''
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
      '';
    };
  };
};
```

## Security Considerations

### TLS/SSL Configuration

Always use HTTPS in production:

```nix
services.nginx.virtualHosts."api.almanac.example.com" = {
  forceSSL = true;
  enableACME = true;
  # ... rest of config
};
```

### Firewall Rules

```nix
networking.firewall = {
  allowedTCPPorts = [ 80 443 ];  # Public access
  # Don't expose API ports directly
};
```

### API Key Management

1. Generate secure API keys:
   ```bash
   openssl rand -hex 32
   ```

2. Store keys securely:
   - Use environment variables
   - Use secrets management (HashiCorp Vault, etc.)
   - Never commit keys to version control

### Rate Limiting

Configure appropriate rate limits:

```nix
services.almanac-api = {
  rateLimitPerMinute = 100;  # Default for unauthenticated
  # Configure per-key limits via admin API
};
```

## Troubleshooting

### Common Issues

1. **Service won't start**
   ```bash
   # Check logs
   journalctl -u almanac-api -f
   
   # Verify database connection
   psql $DATABASE_URL -c "SELECT 1"
   ```

2. **High memory usage**
   ```bash
   # Add memory limits to systemd service
   LimitMEMLOCK=512M
   ```

3. **Slow queries**
   ```bash
   # Enable query logging
   export RUST_LOG="almanac_api=debug,sqlx=debug"
   ```

### Debug Mode

Run in debug mode for troubleshooting:

```bash
RUST_LOG=debug RUST_BACKTRACE=1 almanac-api-http
```

## Upgrading

### Zero-downtime Deployment

1. Build new version:
   ```bash
   nix build github:timewave-computer/almanac#almanac-api-http
   ```

2. Test new version:
   ```bash
   ./result/bin/almanac-api-http --version
   ```

3. Rolling update:
   ```bash
   # Update one instance at a time
   systemctl restart almanac-api-1
   # Wait for health check
   systemctl restart almanac-api-2
   # etc.
   ```

### Rollback Procedure

Nix makes rollbacks simple:

```bash
# List generations
nix-env --list-generations

# Rollback to previous
nix-env --rollback

# Or switch to specific generation
nix-env --switch-generation 42
```

## Performance Tuning

### Database Connection Pool

```toml
[database]
max_connections = 20
min_connections = 5
connection_timeout = 30
```

### API Optimizations

```bash
# Enable release optimizations
export CARGO_PROFILE_RELEASE_LTO=true
export CARGO_PROFILE_RELEASE_CODEGEN_UNITS=1
```

### Caching

Configure Redis for caching (optional):

```nix
services.redis = {
  enable = true;
  bind = "127.0.0.1";
};

services.almanac-api.extraEnv = {
  REDIS_URL = "redis://localhost:6379";
};
```

## Maintenance

### Log Rotation

```nix
services.logrotate.configs.almanac-api = {
  files = "/var/log/almanac-api/*.log";
  frequency = "daily";
  rotate = 7;
  compress = true;
};
```

### Automated Updates

```nix
system.autoUpgrade = {
  enable = true;
  flake = "github:timewave-computer/almanac";
  dates = "02:00";
};
```

## Support

For deployment support:
- Check the [troubleshooting guide](./troubleshooting.md)
- Review [monitoring dashboards](./monitoring.md)
- Contact the development team
- File issues at the [GitHub repository](https://github.com/timewave-computer/almanac)