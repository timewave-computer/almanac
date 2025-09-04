# Server Deployment Automation

Almanac provides comprehensive automation for deploying and managing Almanac services on remote NixOS servers, including code deployment, service management, and specialized Solana validator deployment.

## Overview

The deployment automation system offers:

- **Remote Code Deployment** - Automated rsync-based code synchronization
- **Service Management** - Remote control of Almanac services
- **Solana Validator Deployment** - Specialized Solana deployment with KVM compatibility
- **Status Monitoring** - Remote service health checking and logging
- **Production Configuration** - NixOS-optimized deployment configurations

## Architecture

```
Development Machine                Remote NixOS Server
├── Local Codebase      ------>   ├── /root/almanac/
├── Justfile Commands   ------>   ├── NixOS Configuration
├── Configuration       ------>   ├── Systemd Services
└── Deployment Scripts  ------>   └── Production Environment
```

## Server Setup

### Prerequisites

1. **NixOS Server** with SSH access
2. **SSH Key Authentication** configured
3. **Server Configuration** in SSH config or justfile
4. **Root Access** or sudo privileges

### SSH Configuration

Configure SSH access in `~/.ssh/config`:

```ssh
Host tw-nix
    HostName your-server.example.com
    User root
    IdentityFile ~/.ssh/your-key
    Port 22
```

Or modify the justfile to use your server hostname:

```bash
# Update server references from 'tw-nix' to your server
sed -i 's/tw-nix/your-server/g' justfile
```

## Deployment Commands

### Code Deployment

#### `just deploy-server`
Deploy current codebase to the remote server.

```bash
just deploy-server
```

**What it does:**
- Uses `rsync -avz` for efficient file synchronization
- Excludes build artifacts (`target/`, `.git/`, `node_modules/`)
- Syncs to `/root/almanac/` on the remote server
- Preserves file permissions and timestamps

**Equivalent command:**
```bash
rsync -avz --exclude target --exclude .git --exclude node_modules ./ tw-nix:/root/almanac/
```

### Service Management

#### `just server-status`
Check status of Almanac services on the remote server.

```bash
just server-status
```

Shows:
- Running processes and PIDs
- Port availability
- Service health status
- Log file locations

#### `just server-start`
Start all Almanac services on the remote server.

```bash
just server-start
```

Equivalent to running `just start` on the remote server:
- Initializes environment
- Starts FoundationDB
- Starts blockchain nodes
- Starts Almanac services

#### `just server-stop`
Stop all Almanac services on the remote server.

```bash
just server-stop
```

Gracefully shuts down:
- Almanac API and indexers
- Blockchain validators
- FoundationDB processes

### Log Management

#### `just server-logs [service="all"]`
View logs from the remote server.

```bash
# View all available logs
just server-logs

# View specific service logs
just server-logs ethereum
just server-logs cosmos
just server-logs solana
just server-logs api
just server-logs fdb
```

## Solana Validator Deployment

### Specialized Solana Commands

#### `just deploy-solana-stable`
Deploy stable Solana validator to the NixOS server.

```bash
just deploy-solana-stable
```

**What it does:**
1. Copies `nix/solana/flake-nixos-stable.nix` to the server
2. Creates deployment directory at `/root/solana-deployment/`
3. Runs NixOS-specific deployment commands
4. Uses Solana 1.14.23 (KVM-compatible version)

**Features:**
- **KVM Compatible**: Solana 1.14.23 works in virtualized environments
- **Production Stable**: Battle-tested version for server deployment
- **NixOS Optimized**: Uses NixOS-specific configurations

#### `just server-solana-status`
Check Solana validator status on the remote server.

```bash
just server-solana-status
```

**Checks:**
- Solana validator process status
- RPC endpoint health (`http://localhost:8899/health`)
- Process identification and resource usage

**Example output:**
```bash
Solana validator is running
{
  "jsonrpc": "2.0",
  "result": "ok",
  "id": 1
}
```

#### `just server-solana-logs`
View Solana validator logs on the remote server.

```bash
just server-solana-logs
```

Equivalent to:
```bash
ssh root@tw-nix "tail -f /root/solana-deployment/validator.log"
```

## Complete Deployment Workflow

### Initial Server Setup

1. **Prepare Server:**
```bash
# Ensure NixOS is installed and configured
# SSH access is working
ssh tw-nix "nix --version"
```

2. **Deploy Almanac:**
```bash
# Deploy codebase
just deploy-server

# Check deployment
just server-status
```

3. **Start Services:**
```bash
# Start all services
just server-start

# Verify services
just server-status
```

### Production Solana Deployment

1. **Deploy Stable Solana:**
```bash
# Deploy KVM-compatible Solana
just deploy-solana-stable
```

2. **Verify Deployment:**
```bash
# Check validator status
just server-solana-status

# Monitor logs
just server-solana-logs
```

3. **Test RPC Connectivity:**
```bash
# Test from local machine
curl -X POST -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"getHealth","params":[],"id":1}' \
  http://your-server:8899
```

### Continuous Deployment Workflow

```bash
# 1. Make changes locally
git add .
git commit -m "Feature update"

# 2. Deploy to server
just deploy-server

# 3. Check status
just server-status

# 4. Restart services if needed
just server-stop
just server-start

# 5. Monitor logs
just server-logs api
```

## Server Configuration

### NixOS Configuration

Example server configuration (`/etc/nixos/configuration.nix`):

```nix
{ config, lib, pkgs, ... }:

{
  imports = [
    ./hardware-configuration.nix
    /root/almanac/nix/swagger-ui/nixos-module.nix
    /root/almanac/nix/api/nixos-module.nix
  ];

  # Enable flakes
  nix.settings.experimental-features = [ "nix-command" "flakes" ];

  # Network configuration
  networking = {
    hostName = "almanac-server";
    firewall = {
      enable = true;
      allowedTCPPorts = [ 22 80 443 8080 8081 8545 8899 26657 ];
    };
  };

  # SSH configuration
  services.openssh = {
    enable = true;
    settings.PermitRootLogin = "yes";
    settings.PasswordAuthentication = false;
  };

  # FoundationDB
  services.foundationdb = {
    enable = true;
    package = pkgs.foundationdb71;
    dataDir = "/var/lib/foundationdb";
  };

  # Almanac services
  services.almanac-api = {
    enable = true;
    port = 8080;
    configFile = "/root/almanac/config/production.toml";
  };

  services.almanac-swagger-ui = {
    enable = true;
    port = 8081;
  };

  system.stateVersion = "23.11";
}
```

### Production Configuration

Create production configuration (`config/production.toml`):

```toml
[server]
host = "0.0.0.0"
port = 8080

[storage]
backend = "foundationdb"
cluster_file = "/etc/foundationdb/fdb.cluster"

[chains.ethereum]
enabled = true
rpc_url = "http://localhost:8545"
indexer_port = 9092

[chains.solana]
enabled = true
rpc_url = "http://localhost:8899"
indexer_port = 8899

[chains.cosmos]
enabled = true
rpc_url = "http://localhost:26657"
indexer_port = 9093

[logging]
level = "info"
format = "json"

[metrics]
enabled = true
port = 9090
```

## Monitoring and Maintenance

### Health Monitoring

Create monitoring scripts:

```bash
#!/bin/bash
# server-health-check.sh

echo "=== Almanac Server Health Check ==="
echo "Date: $(date)"
echo

# Check services
just server-status

# Check Solana specifically
just server-solana-status

# Check disk space
ssh tw-nix "df -h"

# Check memory usage
ssh tw-nix "free -h"

# Check system load
ssh tw-nix "uptime"
```

### Automated Health Checks

Set up periodic health checks:

```bash
# Add to cron for regular monitoring
0 */6 * * * /path/to/server-health-check.sh >> /var/log/almanac-health.log 2>&1
```

### Log Rotation

Configure log rotation on the server:

```nix
# In NixOS configuration
services.logrotate = {
  enable = true;
  settings = {
    "/root/almanac/logs/*.log" = {
      daily = true;
      missingok = true;
      rotate = 30;
      compress = true;
      notifempty = true;
      create = "644 root root";
    };
    "/root/solana-deployment/*.log" = {
      daily = true;
      missingok = true;
      rotate = 30;
      compress = true;
      notifempty = true;
      create = "644 root root";
    };
  };
};
```

## Security Considerations

### SSH Security

- **Key-based Authentication**: Disable password authentication
- **Limited Users**: Use dedicated deployment user if not root
- **Firewall Rules**: Restrict SSH access to known IPs

### Service Security

- **Process Isolation**: Use systemd security features
- **Network Security**: Bind services to specific interfaces
- **File Permissions**: Restrict access to configuration files

### Data Security

- **Backup Strategy**: Regular backups of FoundationDB data
- **SSL/TLS**: Use HTTPS for external API access
- **Access Control**: Implement proper access controls

## Troubleshooting

### Common Deployment Issues

**SSH Connection Failed:**
```bash
# Test SSH connectivity
ssh tw-nix "echo 'Connection successful'"

# Check SSH configuration
ssh -v tw-nix
```

**Rsync Permission Errors:**
```bash
# Check remote directory permissions
ssh tw-nix "ls -la /root/"

# Create directory if needed
ssh tw-nix "mkdir -p /root/almanac"
```

**Service Start Failures:**
```bash
# Check service status on server
just server-status

# Check detailed logs
just server-logs api

# Check NixOS configuration
ssh tw-nix "nixos-rebuild dry-build"
```

### Solana Deployment Issues

**KVM Compatibility:**
```bash
# Check if virtualization is detected
ssh tw-nix "dmesg | grep -i kvm"

# Verify Solana version compatibility
just server-solana-status
```

**Port Conflicts:**
```bash
# Check port availability
ssh tw-nix "ss -tlnp | grep 8899"

# Check firewall rules
ssh tw-nix "iptables -L"
```

### Performance Issues

**Resource Monitoring:**
```bash
# Check resource usage
ssh tw-nix "htop"
ssh tw-nix "iostat 1 5"
ssh tw-nix "sar -u 1 5"

# Check service resource usage
ssh tw-nix "systemctl status almanac-api"
```

**Network Issues:**
```bash
# Test network connectivity
curl -v http://your-server:8080/health

# Check service endpoints
ssh tw-nix "netstat -tlnp | grep -E ':(8080|8899|26657)'"
```

## Advanced Deployment

### Multi-Server Deployment

Deploy to multiple servers:

```bash
# Update justfile for multiple servers
deploy-all:
    just deploy-server-1
    just deploy-server-2
    just deploy-server-3

deploy-server-1:
    rsync -avz --exclude target ./ server1:/root/almanac/

deploy-server-2:
    rsync -avz --exclude target ./ server2:/root/almanac/

deploy-server-3:
    rsync -avz --exclude target ./ server3:/root/almanac/
```

### Blue-Green Deployment

Implement blue-green deployment:

```bash
# Deploy to staging
deploy-staging:
    rsync -avz --exclude target ./ staging-server:/root/almanac/
    ssh staging-server "cd /root/almanac && just start"

# Promote to production after testing
promote-to-production:
    rsync -avz --exclude target ./ production-server:/root/almanac/
    ssh production-server "cd /root/almanac && just server-stop && just start"
```

### Automated Testing

Add deployment testing:

```bash
# Test deployment
test-deployment:
    just deploy-server
    just server-start
    sleep 30  # Wait for services to start
    curl -f http://your-server:8080/health
    just server-solana-status
    echo "Deployment test successful"
```

## Integration with CI/CD

### GitHub Actions Example

```yaml
name: Deploy to Production
on:
  push:
    branches: [main]

jobs:
  deploy:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      
      - name: Setup SSH
        uses: webfactory/ssh-agent@v0.5.4
        with:
          ssh-private-key: ${{ secrets.SSH_PRIVATE_KEY }}
      
      - name: Deploy to server
        run: |
          just deploy-server
          just server-start
          
      - name: Health check
        run: |
          sleep 30
          curl -f http://your-server:8080/health
```

The server deployment automation provides a robust foundation for managing Almanac infrastructure across development, staging, and production environments.