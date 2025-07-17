# Nix-Based Service Management Guide

This guide explains how to manage Almanac services using Nix, replacing Docker-based workflows.

## Overview

Almanac services are managed through:
- **Development**: Shell scripts within `nix develop`
- **Production**: NixOS modules or systemd units
- **Testing**: Ephemeral instances for CI/CD

## Development Service Management

### Starting Services

```bash
# Enter development shell
nix develop

# Start API service (foreground)
start_api_server

# Start as background daemon
./scripts/manage-api-service.sh start --daemon

# Start with custom configuration
API_PORT=9000 ./scripts/manage-api-service.sh start
```

### Service Control Commands

```bash
# Check service status
./scripts/manage-api-service.sh status

# Stop service
./scripts/manage-api-service.sh stop

# Restart service
./scripts/manage-api-service.sh restart

# View logs
./scripts/manage-api-service.sh logs

# Follow logs
./scripts/manage-api-service.sh logs --follow
```

### Health Monitoring

```bash
# Quick health check
./scripts/manage-api-service.sh health

# Continuous monitoring
./scripts/monitor-api-health.sh

# One-time health check
./scripts/monitor-api-health.sh --once

# Monitor with alerts
./scripts/monitor-api-health.sh --webhook https://hooks.slack.com/YOUR_WEBHOOK
```

## Production Deployment

### NixOS Module Deployment

For NixOS systems, use the provided module:

```nix
# /etc/nixos/configuration.nix
{ config, pkgs, ... }:

{
  imports = [
    # Import Almanac module
    "${builtins.fetchGit {
      url = "https://github.com/timewave-computer/almanac";
      ref = "main";
    }}/nix/modules/almanac-api.nix"
  ];

  # Configure service
  services.almanac-api = {
    enable = true;
    port = 8080;
    databaseUrl = "postgresql://almanac:pass@localhost/almanac_prod";
    
    # Enable HTTPS in production
    enableHttps = true;
    tlsCert = "/etc/ssl/certs/almanac.crt";
    tlsKey = "/etc/ssl/private/almanac.key";
    
    # Production settings
    rateLimitPerMinute = 1000;
    enableCors = true;
    corsOrigins = [ "https://app.example.com" ];
  };
}

# Apply configuration
sudo nixos-rebuild switch
```

### Systemd Service (Non-NixOS)

For non-NixOS systems:

```bash
# Build the service
nix build .#almanac-api-http

# Install systemd unit
sudo tee /etc/systemd/system/almanac-api.service << EOF
[Unit]
Description=Almanac API Service
After=network.target postgresql.service

[Service]
Type=notify
ExecStart=$(readlink -f ./result/bin/almanac-api-http)
Restart=always
RestartSec=5

# Environment
Environment="DATABASE_URL=postgresql://almanac:pass@localhost/almanac"
Environment="API_PORT=8080"
Environment="RUST_LOG=info"

# Security
DynamicUser=yes
ProtectSystem=strict
PrivateTmp=yes

[Install]
WantedBy=multi-user.target
EOF

# Enable and start
sudo systemctl daemon-reload
sudo systemctl enable almanac-api
sudo systemctl start almanac-api
```

## Service Commands Reference

### Docker → Nix Command Mapping

| Docker Command | Nix Equivalent |
|----------------|----------------|
| `docker-compose up` | `nix develop --command start_api_server` |
| `docker-compose up -d` | `./scripts/manage-api-service.sh start --daemon` |
| `docker-compose down` | `./scripts/manage-api-service.sh stop` |
| `docker-compose ps` | `./scripts/manage-api-service.sh status` |
| `docker-compose logs` | `./scripts/manage-api-service.sh logs` |
| `docker-compose logs -f` | `./scripts/manage-api-service.sh logs --follow` |
| `docker-compose restart` | `./scripts/manage-api-service.sh restart` |
| `docker exec <container> <cmd>` | `nix develop --command <cmd>` |
| `docker build` | `nix build .#almanac-api-http` |

### Environment Management

Instead of Docker's `.env` file handling:

```bash
# Development: Use .env file
cat > .env << EOF
API_PORT=8080
DATABASE_URL=postgresql://localhost/almanac
RUST_LOG=debug
EOF

# The service script automatically loads .env
./scripts/manage-api-service.sh start

# Or set directly
API_PORT=9000 ./scripts/manage-api-service.sh start

# Production: Use systemd environment
systemctl edit almanac-api
# Add:
[Service]
Environment="API_PORT=8080"
```

## Advanced Service Management

### Multiple Instances

Run multiple instances on different ports:

```bash
# Instance 1
API_PORT=8081 ./scripts/manage-api-service.sh start --daemon

# Instance 2
API_PORT=8082 PID_FILE=/tmp/almanac-2.pid ./scripts/manage-api-service.sh start --daemon

# Instance 3
API_PORT=8083 PID_FILE=/tmp/almanac-3.pid ./scripts/manage-api-service.sh start --daemon
```

### Blue-Green Deployment

```bash
#!/bin/bash
# blue-green-deploy.sh

# Build new version
nix build .#almanac-api-http -o result-new

# Start new version on different port
API_PORT=8081 ./result-new/bin/almanac-api-http &
NEW_PID=$!

# Health check new version
sleep 5
if curl -f http://localhost:8081/health; then
    # Switch load balancer to new version
    # Update nginx/haproxy config
    
    # Stop old version
    ./scripts/manage-api-service.sh stop
    
    # Move new to standard port
    kill $NEW_PID
    API_PORT=8080 ./result-new/bin/almanac-api-http --daemon
else
    kill $NEW_PID
    echo "New version health check failed"
    exit 1
fi
```

### Rolling Updates

```nix
# NixOS configuration for rolling updates
{ config, pkgs, ... }:

{
  # Run 3 instances
  services.almanac-api-1 = almanacConfig // { port = 8081; };
  services.almanac-api-2 = almanacConfig // { port = 8082; };
  services.almanac-api-3 = almanacConfig // { port = 8083; };
  
  # Load balancer
  services.nginx = {
    enable = true;
    upstreams.almanac.servers = {
      "localhost:8081" = {};
      "localhost:8082" = {};
      "localhost:8083" = {};
    };
  };
}
```

### Service Dependencies

Ensure proper startup order:

```bash
# Start dependencies first
init_databases

# Wait for database
while ! pg_isready; do
  sleep 1
done

# Start API service
./scripts/manage-api-service.sh start
```

## Monitoring and Logging

### Log Management

```bash
# View recent logs
./scripts/manage-api-service.sh logs

# Follow logs with filtering
./scripts/manage-api-service.sh logs -f | grep ERROR

# Structured logging
RUST_LOG=json ./scripts/manage-api-service.sh start

# Log rotation (systemd)
systemctl edit almanac-api
# Add:
[Service]
StandardOutput=journal
StandardError=journal
```

### Metrics Collection

```bash
# Prometheus metrics endpoint
curl http://localhost:9090/metrics

# Grafana dashboard setup
# Import dashboard from docs/grafana-dashboard.json
```

### Health Checks

```bash
# Simple health check
curl http://localhost:8080/health

# Detailed health check
curl http://localhost:8080/health/detailed

# Automated monitoring
*/5 * * * * /path/to/almanac/scripts/monitor-api-health.sh --once
```

## Troubleshooting Services

### Service Won't Start

```bash
# Check logs
./scripts/manage-api-service.sh logs

# Run in foreground for debugging
RUST_LOG=debug start_api_server

# Check port conflicts
lsof -i :8080
```

### Performance Issues

```bash
# Enable debug logging
RUST_LOG=almanac_api=trace ./scripts/manage-api-service.sh restart

# Monitor resources
htop -p $(cat data/almanac-api.pid)

# Profile the service
cargo flamegraph --bin almanac-api-http
```

### Database Connection Issues

```bash
# Test database connection
psql $DATABASE_URL -c "SELECT 1"

# Check connection pool
RUST_LOG=sqlx=debug ./scripts/manage-api-service.sh restart
```

## Best Practices

### Development

1. Always use `nix develop` for consistent environment
2. Use `.env` files for configuration
3. Run health checks after starting services
4. Monitor logs during development

### Production

1. Use NixOS modules when possible
2. Enable systemd security features
3. Set up proper monitoring and alerting
4. Use rolling updates for zero downtime
5. Regular backup of configuration

### Security

1. Run services as non-root users
2. Use systemd sandboxing features
3. Enable TLS in production
4. Rotate API keys regularly
5. Monitor for suspicious activity

## Migration from Docker

### Docker Compose Translation

Original `docker-compose.yml`:
```yaml
services:
  api:
    image: almanac-api:latest
    ports:
      - "8080:8080"
    environment:
      DATABASE_URL: postgresql://db/almanac
    depends_on:
      - db
```

Equivalent Nix approach:
```bash
# Development
nix develop --command bash -c '
  init_databases
  start_api_server
'

# Production (NixOS)
services.almanac-api = {
  enable = true;
  port = 8080;
  databaseUrl = "postgresql://localhost/almanac";
};
```

### Volume Management

Instead of Docker volumes:

```bash
# Data is stored in project directory
./data/postgres/    # PostgreSQL data
./data/rocksdb/     # RocksDB data
./logs/             # Application logs

# Backup
tar -czf backup.tar.gz data/

# Restore
tar -xzf backup.tar.gz
```

## Conclusion

Nix-based service management provides:
- Reproducible deployments
- Better resource efficiency
- Simplified debugging
- Native system integration

For more details, see:
- [Nix Development Setup](./nix-development-setup.md)
- [Deployment Guide](./nix-deployment-guide.md)
- [Troubleshooting Guide](./nix-troubleshooting-guide.md)