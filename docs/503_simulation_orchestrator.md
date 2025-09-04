# Multi-Chain Simulation Orchestrator

The Almanac Simulation Orchestrator provides a unified interface for managing multi-chain development environments across platforms, automatically detecting your system and using optimized configurations for each blockchain.

## Overview

The simulation orchestrator offers:

- **Cross-Platform Compatibility** - Automatic macOS and Linux/NixOS detection
- **Multi-Chain Management** - Unified control of Ethereum, Cosmos, Solana, and FoundationDB
- **Platform Optimization** - Uses tested environments from individual chain nix directories
- **Unified Interface** - Single command set that works consistently across platforms
- **Battle-Tested Components** - Leverages the same environments used in automated testing

## Architecture

```
Simulation Orchestrator
├── Platform Detection (macOS/Linux/NixOS)
├── Chain Management
│   ├── Ethereum (nix/ethereum/)
│   ├── Cosmos (nix/cosmos/)  
│   ├── Solana (nix/solana/)
│   └── FoundationDB (nix/fdb/)
├── Configuration Generation
└── Service Coordination
```

### Platform-Specific Optimizations

**macOS (Apple Silicon & Intel):**
- Uses Anvil for Ethereum (fast startup)
- Neutron v6.0.3 for Cosmos (pre-built)
- Latest Solana (native performance)
- FoundationDB development cluster file

**Linux/NixOS:**
- Full FoundationDB server on port 4500
- KVM-compatible Solana builds with patches
- Optimized blockchain environments
- Real server deployment configurations

## Quick Start

### Enter Simulation Environment

```bash
# Navigate to simulation directory
cd nix/simulation

# Enter the simulation environment
nix develop

# Initialize with all chains
sim-cluster init

# Start services (shows platform-specific commands)
sim-cluster start

# Check status
sim-cluster status
```

### Service Endpoints

When running, services are available at:

| Service | Endpoint | Additional |
|---------|----------|------------|
| Ethereum | http://localhost:8545 | WebSocket: ws://localhost:8546 |
| Cosmos | http://localhost:26657 | gRPC: :9090, API: :1317 |
| Solana | http://localhost:8899 | WebSocket: ws://localhost:8900 |
| FoundationDB | Port 4500 | Cluster file: `../../data/fdb.cluster` |

## Commands Reference

### Main Orchestrator Commands

#### `sim-cluster init [chains]`
Initialize simulation environment for specified chains.

```bash
# Initialize all chains
sim-cluster init

# Initialize specific chains
sim-cluster init ethereum,solana

# Initialize with FoundationDB
sim-cluster init ethereum,cosmos,solana,fdb
```

Default chains: `ethereum,cosmos,solana,fdb`

#### `sim-cluster start`
Display platform-specific commands to start all services.

The orchestrator shows the exact commands to run based on your platform:

**macOS Example Output:**
```bash
# FoundationDB
cd ../fdb && nix develop --command fdb-manage start

# Ethereum
cd ../ethereum && nix develop --command bash -c "anvil --host 0.0.0.0 --port 8545 > ../../logs/anvil.log 2>&1 &"

# Cosmos
cd ../cosmos && nix develop --command bash -c "neutron-start > ../../logs/neutron.log 2>&1 &"

# Solana
solana-test-validator --ledger ../../data/solana --log > ../../logs/solana.log 2>&1 &
```

#### `sim-cluster stop`
Stop all running simulation services.

```bash
sim-cluster stop
```

Safely terminates:
- All blockchain validators
- FoundationDB processes
- Background service processes

#### `sim-cluster status`
Check the status of all simulation services.

```bash
sim-cluster status
```

Shows:
- Service running status
- Port availability
- Process IDs
- Service endpoints
- Data directory status

#### `sim-cluster logs [service]`
View logs for specific services.

```bash
# View all available log options
sim-cluster logs

# View specific service logs
sim-cluster logs ethereum
sim-cluster logs cosmos
sim-cluster logs solana
sim-cluster logs fdb
```

### Quick Operations

#### `sim-quick-start`
Complete simulation initialization and startup.

```bash
sim-quick-start
```

Equivalent to:
1. `sim-cluster init`
2. `sim-cluster start`
3. `sim-cluster status`

#### `sim-clean`
Clean all simulation data and logs.

```bash
sim-clean
```

Removes:
- All blockchain data directories
- Service log files
- Generated configuration files
- Preserves the base directory structure

#### `sim-restart`
Complete simulation restart cycle.

```bash
sim-restart
```

Performs:
1. `sim-cluster stop`
2. `sim-clean`
3. `sim-quick-start`

### Configuration Management

#### `sim-config [chains]`
Generate platform-aware configuration files.

```bash
# Generate config for all chains
sim-config

# Generate config for specific chains
sim-config ethereum,solana,fdb
```

Creates `config/simulation.toml` with platform-specific settings:

```toml
# Almanac Simulation Configuration
# Generated for macOS (Apple Silicon)

[simulation]
name = "almanac-local-cluster"
chains = ["ethereum", "cosmos", "solana", "fdb"]

[storage]
backend = "foundationdb"

[services.foundationdb]
enabled = true
cluster_file = "../../data/fdb.cluster"

[services.ethereum]
enabled = true
rpc_port = 8545
chain_id = 31337

[services.cosmos]
enabled = true
rpc_port = 26657
chain_id = "test-1"

[services.solana]
enabled = true
rpc_port = 8899
```

### Individual Service Control

#### Ethereum Service
```bash
sim-ethereum status    # Check Ethereum service status
sim-ethereum start     # Start Ethereum service
sim-ethereum stop      # Stop Ethereum service
sim-ethereum logs      # View Ethereum logs
```

#### Cosmos Service
```bash
sim-cosmos status      # Check Cosmos service status
sim-cosmos start       # Start Cosmos service
sim-cosmos stop        # Stop Cosmos service
sim-cosmos logs        # View Cosmos logs
```

#### Solana Service
```bash
sim-solana status      # Check Solana service status
sim-solana start       # Start Solana service
sim-solana stop        # Stop Solana service
sim-solana logs        # View Solana logs
```

#### Almanac Services
```bash
sim-api status         # Check Almanac API status
sim-indexer status     # Check Almanac indexer status
```

## Integration with Justfile

The simulation orchestrator integrates with the main justfile:

### Justfile Simulation Commands

```bash
# Initialize simulation environment
just sim-init

# Start simulation cluster
just sim-start

# Stop simulation cluster
just sim-stop

# Check simulation status
just sim-status

# View simulation logs
just sim-logs [service]

# Quick simulation startup
just sim-quick-start

# Clean simulation environment
just sim-clean

# Restart simulation
just sim-restart
```

### Justfile Integration Example

```bash
# Complete development workflow
just sim-quick-start    # Start simulation
just build             # Build Almanac
just start-api         # Start API against simulation
just test-e2e          # Run tests against simulation
just sim-stop          # Clean up
```

## Development Workflows

### Complete Development Session

```bash
# 1. Enter simulation environment
cd nix/simulation
nix develop

# 2. Initialize simulation
sim-cluster init

# 3. Start services (run commands shown by sim-cluster start)
# Follow platform-specific instructions

# 4. Verify services are running
sim-cluster status

# 5. Test connectivity
curl http://localhost:8545 -X POST -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"eth_blockNumber","params":[],"id":1}'

curl http://localhost:26657/status

curl http://localhost:8899 -X POST -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"getHealth","params":[],"id":1}'

# 6. View logs during development
sim-cluster logs ethereum
sim-cluster logs cosmos
sim-cluster logs solana

# 7. Clean up when done
sim-cluster stop
```

### Selective Chain Development

```bash
# Work with only Ethereum and Solana
sim-cluster init ethereum,solana
sim-cluster start
# Follow instructions for only Ethereum and Solana

# Check status
sim-cluster status
```

### Testing Workflow Integration

```bash
# Start simulation for testing
sim-quick-start

# Return to main directory for testing
cd ../..

# Run tests against simulation
just test-e2e

# View simulation logs if tests fail
cd nix/simulation
sim-cluster logs ethereum
```

## Platform-Specific Behaviors

### macOS Optimizations

**Ethereum (Anvil):**
- Fast startup with pre-funded accounts
- Instant mining for testing
- WebSocket support enabled

**Cosmos (Neutron v6.0.3):**
- Pre-built binary from cache
- CosmWasm support enabled
- Standard test configuration

**Solana:**
- Native macOS performance
- Latest stable version
- Standard test validator configuration

**FoundationDB:**
- Development cluster file mode
- Suitable for testing and development
- No persistent server required

### Linux/NixOS Optimizations

**Ethereum (Anvil):**
- Same configuration as macOS
- Network optimization for containers

**Cosmos (Neutron):**
- Built from source with optimizations
- KVM compatibility ensured
- Extended timeout configurations

**Solana:**
- KVM-compatible patches applied
- Socket timeout handling
- Virtualization-friendly configuration

**FoundationDB:**
- Full server instance on port 4500
- Persistent storage configuration
- Production-ready setup

## Directory Structure

```
nix/simulation/
├── flake.nix              # Cross-platform orchestrator
├── README.md              # Detailed simulation guide
├── config/                # Generated configurations
│   └── simulation.toml    # Platform-specific config
├── data/                  # Blockchain data directories
│   ├── ethereum/          # Anvil blockchain state
│   ├── cosmos/            # Neutron chain data
│   ├── solana/            # Solana validator ledger
│   └── fdb.cluster        # FoundationDB cluster file
└── logs/                  # Service logs
    ├── anvil.log          # Ethereum/Anvil logs
    ├── neutron.log        # Cosmos/Neutron logs
    └── solana.log         # Solana validator logs
```

## Troubleshooting

### Service Status Issues

```bash
# Check individual service status
sim-cluster status

# Check ports manually
netstat -tulpn | grep -E ':(4500|8545|26657|8899)'

# Check process status
ps aux | grep -E '(anvil|neutron|solana-test-validator|fdb)'
```

### Platform Detection Issues

```bash
# Force platform detection
uname -s    # Should show Darwin or Linux
uname -m    # Shows architecture

# Check Nix platform
nix eval --impure --raw --expr 'builtins.currentSystem'
```

### First Run on NixOS

The first run requires building dependencies:

**Expected Build Times:**
- **Cosmos/Neutron**: 30-60 minutes (Wasmer, libwasmvm)
- **Solana**: 15-30 minutes (with KVM patches)
- **Ethereum/FoundationDB**: Usually cached (fast)

**Monitor Build Progress:**
```bash
# Watch build progress
nix develop --show-trace

# Check cache availability
nix show-config | grep substituters
```

### Log Analysis

```bash
# Real-time log monitoring
sim-cluster logs ethereum  # tail -f logs/anvil.log
sim-cluster logs cosmos    # tail -f logs/neutron.log
sim-cluster logs solana    # tail -f logs/solana.log

# Check for common errors
grep -i error logs/*.log
grep -i "failed" logs/*.log
```

### Network Connectivity

```bash
# Test individual services
curl -s http://localhost:8545 -X POST \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"eth_blockNumber","params":[],"id":1}' | jq

curl -s http://localhost:26657/status | jq .result.sync_info

curl -s http://localhost:8899 -X POST \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"getHealth","params":[],"id":1}' | jq
```

### Clean Restart

```bash
# Complete environment reset
sim-cluster stop
sim-clean
rm -rf data/ logs/  # Optional: remove all data
sim-cluster init
sim-cluster start
```

## Integration with Almanac Development

### API Development

```bash
# Start simulation
sim-quick-start

# Start Almanac API against simulation
cd ../..
just start-api config/simulation.toml

# API is now available at http://localhost:8080
```

### Cross-Chain Testing

```bash
# Simulation provides complete multi-chain environment
sim-cluster status

# Run cross-chain tests
cd ../..
just test-e2e multi-chain-simulation
```

### Storage Backend Testing

```bash
# FoundationDB is included in simulation
sim-cluster status  # Check FDB status

# Test FoundationDB connectivity
fdbcli --cluster-file=nix/simulation/data/fdb.cluster
```

## Advanced Configuration

### Custom Chain Selection

```bash
# Development with only specific chains
sim-cluster init ethereum,fdb    # Minimal setup
sim-cluster init solana          # Solana-only development
sim-cluster init                 # All chains (default)
```

### Environment Variables

```bash
# Customize simulation behavior
export SIM_LOG_LEVEL=debug       # Increase logging
export SIM_DATA_DIR=/custom/path # Custom data directory
export SIM_FAST_MODE=true        # Skip confirmations
```

### Integration Testing

```bash
# Simulation for integration tests
sim-quick-start

# Wait for services to be ready
sleep 10

# Run integration test suite
cd ../..
cargo test --test integration_tests
```

## Performance Optimization

### Resource Usage

**macOS:**
- Ethereum: ~100MB RAM, minimal CPU
- Cosmos: ~200MB RAM, moderate CPU during startup
- Solana: ~300MB RAM, moderate CPU
- FoundationDB: ~50MB RAM (dev mode)

**Linux/NixOS:**
- Similar resource usage with optimizations for containers
- FoundationDB: ~100MB RAM (server mode)

### Startup Optimization

```bash
# Parallel startup (advanced users)
# Run multiple service start commands in background
cd ../ethereum && nix develop --command bash -c "anvil --host 0.0.0.0 --port 8545 > ../../logs/anvil.log 2>&1 &" &
cd ../cosmos && nix develop --command bash -c "neutron-start > ../../logs/neutron.log 2>&1 &" &
cd ../solana && nix develop --command bash -c "solana-test-validator --ledger ../../data/solana --log > ../../logs/solana.log 2>&1 &" &

# Wait for services to start
sleep 15
sim-cluster status
```

## Production Notes

### Development vs Production

**Simulation Environment:**
- Designed for development and testing
- Uses test configurations
- Ephemeral blockchain state
- Local-only network binding

**Production Deployment:**
- Use NixOS modules for system services
- Configure persistent storage
- Enable network security
- Use production blockchain configurations

### Security Considerations

- All services bound to localhost (127.0.0.1)
- No external network exposure by default
- Isolated data directories
- Development keys only (not for production)

The simulation orchestrator provides a comprehensive, cross-platform development environment that makes multi-chain development accessible and consistent across different systems.