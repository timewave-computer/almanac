# NixOS Deployment Modules

Almanac provides NixOS modules for production deployment, enabling system-level service integration with systemd, automatic startup, firewall configuration, and security hardening.

## Overview

The NixOS modules provide:

- **Systemd Service Integration** - Run Almanac services as system services
- **Automatic Startup** - Services start on boot and restart on failure
- **Firewall Management** - Automatic port opening and security rules
- **Security Hardening** - Process isolation and privilege reduction
- **Configuration Management** - Declarative service configuration
- **Monitoring Integration** - System-level logging and status reporting

## Available Modules

### Almanac Swagger UI Module

Location: `nix/swagger-ui/nixos-module.nix`

Provides system-level Swagger UI documentation service for the Almanac API.

#### Module Configuration

Add to your NixOS configuration (`configuration.nix` or separate module file):

```nix
{ config, lib, pkgs, ... }:

{
  # Import the Almanac Swagger UI module
  imports = [
    ./path/to/almanac/nix/swagger-ui/nixos-module.nix
  ];

  # Configure the Swagger UI service
  services.almanac-swagger-ui = {
    enable = true;
    port = 8081;
    host = "0.0.0.0";
    openFirewall = true;
    configFile = "/etc/almanac/simulation.toml";
    environmentFile = "/etc/almanac/swagger-env";
  };
}
```

#### Configuration Options

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `enable` | boolean | false | Enable the Almanac Swagger UI service |
| `port` | port | 8080 | Port number for the Swagger UI server |
| `host` | string | "0.0.0.0" | Host address to bind (0.0.0.0 for all interfaces) |
| `configFile` | path\|null | null | Path to Almanac configuration file |
| `openFirewall` | boolean | true | Whether to open the firewall port automatically |
| `environmentFile` | path\|null | null | Path to environment file with additional variables |

#### Service Management

Once configured, manage the service using systemd:

```bash
# Check service status
systemctl status almanac-swagger-ui

# Start/stop service
systemctl start almanac-swagger-ui
systemctl stop almanac-swagger-ui

# Enable/disable auto-start
systemctl enable almanac-swagger-ui
systemctl disable almanac-swagger-ui

# View service logs
journalctl -u almanac-swagger-ui -f

# Restart after configuration changes
systemctl restart almanac-swagger-ui
```

#### Service Features

**Automatic Restart:**
- Service restarts automatically on failure
- 10-second delay between restart attempts
- Systemd manages service lifecycle

**Security Hardening:**
- Runs with dynamic user (no system user required)
- Private temporary directory
- Protected system directories
- Home directory protection
- No new privileges allowed

**Environment Management:**
- Configurable environment variables
- Support for environment files
- Rust logging configuration

## Creating Additional Modules

### Module Template

Create new NixOS modules following this template:

```nix
# nix/[service]/nixos-module.nix
{ config, lib, pkgs, ... }:

with lib;

let
  cfg = config.services.almanac-[service];
in {
  options.services.almanac-[service] = {
    enable = mkEnableOption "Almanac [Service] service";
    
    port = mkOption {
      type = types.port;
      default = 8080;
      description = "Port number for the [Service] server";
    };
    
    # Add service-specific options here
  };
  
  config = mkIf cfg.enable {
    # Firewall configuration
    networking.firewall.allowedTCPPorts = mkIf cfg.openFirewall [ cfg.port ];
    
    # Systemd service definition
    systemd.services.almanac-[service] = {
      description = "Almanac [Service] Service";
      after = [ "network.target" ];
      wantedBy = [ "multi-user.target" ];
      
      serviceConfig = {
        Type = "simple";
        ExecStart = "${pkgs.almanac}/bin/almanac-[service] --port ${toString cfg.port}";
        
        Restart = "always";
        RestartSec = "10s";
        
        # Security hardening
        DynamicUser = true;
        StateDirectory = "almanac-[service]";
        RuntimeDirectory = "almanac-[service]";
        PrivateTmp = true;
        ProtectSystem = "strict";
        ProtectHome = true;
        NoNewPrivileges = true;
      };
    };
  };
}
```

### Almanac API Module

Example module for the Almanac API service:

```nix
# nix/api/nixos-module.nix
{ config, lib, pkgs, ... }:

with lib;

let
  cfg = config.services.almanac-api;
in {
  options.services.almanac-api = {
    enable = mkEnableOption "Almanac API service";
    
    port = mkOption {
      type = types.port;
      default = 8080;
      description = "Port number for the API server";
    };
    
    configFile = mkOption {
      type = types.path;
      default = "/etc/almanac/config.toml";
      description = "Path to the Almanac configuration file";
    };
    
    foundationdbClusterFile = mkOption {
      type = types.nullOr types.path;
      default = null;
      description = "Path to FoundationDB cluster file";
    };
    
    logLevel = mkOption {
      type = types.enum ["error" "warn" "info" "debug" "trace"];
      default = "info";
      description = "Rust log level";
    };
  };
  
  config = mkIf cfg.enable {
    networking.firewall.allowedTCPPorts = [ cfg.port ];
    
    systemd.services.almanac-api = {
      description = "Almanac Multi-Chain API Service";
      after = [ "network.target" "foundationdb.service" ];
      wants = [ "foundationdb.service" ];
      wantedBy = [ "multi-user.target" ];
      
      environment = {
        RUST_LOG = cfg.logLevel;
        FDB_CLUSTER_FILE = mkIf (cfg.foundationdbClusterFile != null) cfg.foundationdbClusterFile;
      };
      
      serviceConfig = {
        Type = "simple";
        ExecStart = "${pkgs.almanac}/bin/almanac-api --config ${cfg.configFile} --port ${toString cfg.port}";
        
        Restart = "always";
        RestartSec = "10s";
        
        # Security hardening
        DynamicUser = true;
        StateDirectory = "almanac-api";
        RuntimeDirectory = "almanac-api";
        PrivateTmp = true;
        ProtectSystem = "strict";
        ProtectHome = true;
        NoNewPrivileges = true;
        
        # Allow access to FoundationDB
        SupplementaryGroups = [ "foundationdb" ];
      };
    };
  };
}
```

## Production Deployment

### Complete NixOS Configuration

Example complete configuration for an Almanac server:

```nix
# /etc/nixos/configuration.nix
{ config, lib, pkgs, ... }:

{
  imports = [
    ./hardware-configuration.nix
    /path/to/almanac/nix/swagger-ui/nixos-module.nix
    /path/to/almanac/nix/api/nixos-module.nix
  ];

  # FoundationDB service
  services.foundationdb = {
    enable = true;
    package = pkgs.foundationdb71;
    dataDir = "/var/lib/foundationdb";
    logDir = "/var/log/foundationdb";
  };

  # Almanac API service
  services.almanac-api = {
    enable = true;
    port = 8080;
    configFile = "/etc/almanac/production.toml";
    foundationdbClusterFile = "/etc/foundationdb/fdb.cluster";
    logLevel = "info";
  };

  # Almanac Swagger UI
  services.almanac-swagger-ui = {
    enable = true;
    port = 8081;
    host = "0.0.0.0";
    openFirewall = true;
    configFile = "/etc/almanac/production.toml";
  };

  # Additional system configuration
  system.stateVersion = "23.11";
  
  # Enable automatic updates
  system.autoUpgrade = {
    enable = true;
    allowReboot = false;
  };
  
  # Configure firewall
  networking.firewall = {
    enable = true;
    allowedTCPPorts = [ 22 80 443 ];
  };
  
  # Enable monitoring
  services.prometheus.exporters = {
    node.enable = true;
    systemd.enable = true;
  };
}
```

### Configuration Files

Create Almanac configuration files:

```toml
# /etc/almanac/production.toml
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
```

### Deployment Process

1. **Prepare Configuration:**
```bash
# Copy configuration files
sudo mkdir -p /etc/almanac
sudo cp config/production.toml /etc/almanac/

# Copy NixOS modules
sudo cp -r nix/ /etc/nixos/almanac-modules/
```

2. **Update NixOS Configuration:**
```bash
sudo nano /etc/nixos/configuration.nix
# Add module imports and service configuration
```

3. **Deploy Changes:**
```bash
# Test configuration
sudo nixos-rebuild dry-build

# Apply configuration
sudo nixos-rebuild switch

# Check service status
systemctl status almanac-api almanac-swagger-ui
```

### Service Dependencies

Configure service dependencies for proper startup order:

```nix
systemd.services.almanac-api = {
  after = [ "network.target" "foundationdb.service" ];
  wants = [ "foundationdb.service" ];
  requires = [ "foundationdb.service" ];
};

systemd.services.almanac-indexer = {
  after = [ "almanac-api.service" ];
  wants = [ "almanac-api.service" ];
};
```

## Monitoring and Logging

### Systemd Integration

All modules integrate with systemd for monitoring:

```bash
# Service status
systemctl status almanac-*

# Service logs
journalctl -u almanac-api -f
journalctl -u almanac-swagger-ui --since "1 hour ago"

# System overview
systemctl list-units "almanac-*"
```

### Log Management

Configure structured logging:

```nix
services.almanac-api = {
  environment = {
    RUST_LOG = "almanac=info,tower_http=debug";
    RUST_LOG_FORMAT = "json";
  };
};

# Configure log rotation
services.logrotate.settings.almanac = {
  "/var/log/almanac/*.log" = {
    daily = true;
    missingok = true;
    rotate = 30;
    compress = true;
    notifempty = true;
    create = "644 almanac-api almanac-api";
  };
};
```

### Health Monitoring

Implement health checks:

```nix
systemd.services.almanac-api-healthcheck = {
  description = "Almanac API Health Check";
  serviceConfig = {
    Type = "oneshot";
    ExecStart = "${pkgs.curl}/bin/curl -f http://localhost:8080/health";
  };
};

systemd.timers.almanac-api-healthcheck = {
  description = "Almanac API Health Check Timer";
  timerConfig = {
    OnCalendar = "*:0/5"; # Every 5 minutes
    Persistent = true;
  };
  wantedBy = [ "timers.target" ];
};
```

## Security Considerations

### Process Isolation

All modules implement security hardening:

- **DynamicUser**: Creates ephemeral users for each service
- **PrivateTmp**: Isolated temporary directories
- **ProtectSystem**: Read-only system directories
- **ProtectHome**: No access to user home directories
- **NoNewPrivileges**: Prevents privilege escalation

### Network Security

- **Firewall Integration**: Automatic port management
- **Interface Binding**: Configurable host binding
- **SSL/TLS Support**: Certificate management integration

### Data Security

- **StateDirectory**: Persistent data directories with proper permissions
- **RuntimeDirectory**: Temporary runtime data isolation
- **Environment Files**: Secure secret management

## Advanced Configuration

### Custom Package Sources

Use custom Almanac packages:

```nix
nixpkgs.overlays = [
  (final: prev: {
    almanac = prev.callPackage ./almanac.nix {
      src = /path/to/almanac/source;
    };
  })
];
```

### Environment-Specific Configurations

```nix
# Development configuration
services.almanac-api = mkIf (config.networking.hostName == "almanac-dev") {
  enable = true;
  logLevel = "debug";
  configFile = "/etc/almanac/development.toml";
};

# Production configuration  
services.almanac-api = mkIf (config.networking.hostName == "almanac-prod") {
  enable = true;
  logLevel = "info";
  configFile = "/etc/almanac/production.toml";
};
```

### Multi-Instance Deployment

Deploy multiple Almanac instances:

```nix
services.almanac-api-primary = {
  enable = true;
  port = 8080;
  configFile = "/etc/almanac/primary.toml";
};

services.almanac-api-secondary = {
  enable = true;
  port = 8081;
  configFile = "/etc/almanac/secondary.toml";
};
```

## Troubleshooting

### Common Issues

**Service fails to start:**
```bash
# Check service status and logs
systemctl status almanac-api
journalctl -u almanac-api --since "10 minutes ago"

# Verify configuration
nixos-rebuild dry-build
```

**Permission errors:**
```bash
# Check service user permissions
sudo -u almanac-api ls -la /var/lib/almanac-api/

# Verify StateDirectory permissions
ls -la /var/lib/ | grep almanac
```

**Network connectivity:**
```bash
# Check port binding
ss -tlnp | grep 8080

# Test service endpoints
curl http://localhost:8080/health
```

### Module Development

When developing custom modules:

1. **Test Configuration:**
```bash
sudo nixos-rebuild dry-build
```

2. **Debug Services:**
```bash
systemctl cat almanac-api
systemd-analyze verify almanac-api.service
```

3. **Validate Security:**
```bash
systemctl show almanac-api | grep -E "(User|Group|ProtectSystem)"
```

The NixOS modules provide a robust foundation for production Almanac deployments with enterprise-grade security, monitoring, and reliability features.