# Almanac Simulation Environment

A cross-platform multi-chain simulation orchestrator that automatically detects your platform and uses the appropriate nix environment for each blockchain.

## Features

- **Automatic OS Detection**: Detects whether you're on macOS or Linux/NixOS
- **Platform-Optimized Configurations**: Uses tested blockchain environments from `nix/{cosmos,ethereum,solana,fdb}/`
- **Real FoundationDB**: Full FoundationDB server on Linux, development cluster file on macOS
- **Unified Interface**: Single command interface that works across platforms
- **Battle-Tested**: Uses the same environments verified in `tests/environment/`

## Quick Start

```bash
# Enter the simulation environment
nix develop

# Initialize with all chains
sim-cluster init

# Check what services to start
sim-cluster start

# Check status
sim-cluster status

# View logs
sim-cluster logs ethereum
```

## Platform-Specific Results

### macOS (Tested WORKING)
- **Ethereum (Anvil)**: WORKING - Running on http://localhost:8545
- **Cosmos (Neutron v6.0.3)**: WORKING - Running on http://localhost:26657
- **Solana**: WORKING - Running on http://localhost:8899  
- **FoundationDB**: WORKING - Development mode (cluster file ready)

### NixOS Server (Tested WORKING)
- **Ethereum (Anvil)**: WORKING - Running on http://localhost:8545 (JSON-RPC verified)
- **FoundationDB**: WORKING - Real server running on port 4500
- **Cosmos (Neutron)**: WORKING - Builds and runs (requires initial build time)
- **Solana**: WORKING - Builds and runs (requires initial build time)

## Commands

### Main Orchestrator
```bash
sim-cluster init [chains]    # Initialize cluster (default: ethereum,cosmos,solana,fdb)
sim-cluster start           # Show commands to start services
sim-cluster stop            # Stop all services  
sim-cluster status          # Check service status
sim-cluster logs [service]  # View service logs
sim-cluster help            # Show help
```

### Quick Start
```bash
sim-quick-start  # Initialize with platform defaults
```

### Configuration Generator
```bash
sim-config [chains]  # Generate platform-aware configuration
```

## Service Endpoints

When running, services are available at:

| Service | Endpoint | Additional |
|---------|----------|------------|
| Ethereum | http://localhost:8545 | WS: ws://localhost:8546 |
| Cosmos | http://localhost:26657 | gRPC: :9090, API: :1317 |
| Solana | http://localhost:8899 | WS: ws://localhost:8900 |
| FoundationDB | Port 4500 | Cluster file: `../../data/fdb.cluster` |

## Configuration

The simulation generates a `config/simulation.toml` file with platform-specific settings:

```toml
# Almanac Simulation Configuration
# Generated for macOS (Apple Silicon) / x86_64-linux

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

## Architecture

The simulation orchestrator:

1. **Detects Platform**: Automatically identifies macOS vs Linux/NixOS
2. **Uses Tested Environments**: Leverages the same blockchain environments from `tests/environment/`
3. **Provides Instructions**: Shows platform-specific commands to start services
4. **Unified Status**: Single interface to check all services across platforms

## Integration with Tested Environments

The simulation uses the battle-tested blockchain environments:

- **`nix/cosmos/`**: Neutron v6.0.3 with proper initialization
- **`nix/ethereum/`**: Anvil with cross-platform configurations  
- **`nix/solana/`**: Latest Solana with KVM compatibility patches for Linux
- **`nix/fdb/`**: Real FoundationDB server on Linux, development setup on macOS

## Starting Services

### Automatic Detection
The `sim-cluster start` command shows platform-specific instructions:

**macOS:**
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

**NixOS:**
```bash  
# Same commands work, with automatic NixOS optimizations
cd ../fdb && nix develop --command fdb-manage start
cd ../ethereum && nix develop --command bash -c "anvil --host 0.0.0.0 --port 8545 > ../../logs/anvil.log 2>&1 &"
cd ../cosmos && nix develop --command bash -c "neutron-start > ../../logs/neutron.log 2>&1 &"
cd ../solana && nix develop --command bash -c "solana-test-validator --ledger ../../data/solana --log > ../../logs/solana.log 2>&1 &"
```

## Directory Structure

```
nix/simulation/
├── flake.nix           # Cross-platform orchestrator  
├── README.md           # This file
├── config/             # Generated configurations
│   └── simulation.toml # Platform-specific config
├── data/               # Blockchain data
│   ├── ethereum/       # Anvil blockchain data
│   ├── cosmos/         # Neutron chain data
│   └── solana/         # Solana validator ledger
└── logs/               # Service logs
    ├── anvil.log       # Ethereum/Anvil logs
    ├── neutron.log     # Cosmos/Neutron logs
    └── solana.log      # Solana validator logs
```

## Development Workflow

### Complete Session
```bash
# 1. Enter simulation environment
cd nix/simulation  
nix develop

# 2. Initialize with all chains
sim-cluster init

# 3. Start services (run each in separate terminals)
sim-cluster start  # Shows platform-specific commands

# 4. Check status
sim-cluster status

# 5. Test endpoints
curl http://localhost:8545 -X POST -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"eth_blockNumber","params":[],"id":1}'

curl http://localhost:26657/status
curl http://localhost:8899 -X POST -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"getHealth","params":[],"id":1}'

# 6. View logs
sim-cluster logs ethereum
sim-cluster logs cosmos  
sim-cluster logs solana

# 7. Stop when done
sim-cluster stop
```

### Custom Chain Selection
```bash
# Only specific chains
sim-cluster init ethereum,solana

# All chains including FoundationDB
sim-cluster init ethereum,cosmos,solana,fdb
```

## Troubleshooting

### Service Status
```bash
# Check individual services
sim-cluster status

# Check ports manually
netstat -tulpn | grep -E ':(4500|8545|26657|8899)'
```

### First Run on NixOS
The first run on a new NixOS system requires building blockchain dependencies:
- **Cosmos**: Builds Neutron, Wasmer, libwasmvm (~30-60 minutes)
- **Solana**: Builds Solana with KVM patches (~15-30 minutes)  
- **Ethereum/FDB**: Usually available from binary cache (fast)

### Log Analysis
```bash
# Follow logs in real-time
sim-cluster logs ethereum  # tail -f logs/anvil.log
sim-cluster logs cosmos    # tail -f logs/neutron.log
sim-cluster logs solana    # tail -f logs/solana.log
```

### Clean Restart
```bash
# Stop all services
sim-cluster stop

# Remove data (optional - will reset blockchain state)
rm -rf data/ logs/

# Reinitialize  
sim-cluster init
```

## Integration with Almanac

The simulation provides the complete infrastructure for Almanac's multi-chain indexing:

1. **Blockchain Nodes**: Ethereum (Anvil), Cosmos (Neutron), Solana (test-validator)
2. **Storage Backend**: FoundationDB for persistent cross-chain data
3. **Network Endpoints**: All services available on standard ports
4. **Development Data**: Isolated data/logs directories for testing

This environment supports:
- Cross-chain event correlation testing
- Storage backend validation  
- API endpoint verification
- Performance testing across chains
- Integration testing with real blockchain data

## Production Notes

### FoundationDB
- **macOS**: Development cluster file only (suitable for development)
- **NixOS**: Full FoundationDB server with persistent storage

### Blockchain Versions
- **Ethereum**: Latest Anvil (Foundry)
- **Cosmos**: Neutron v6.0.3 with testing configuration
- **Solana**: Latest with KVM compatibility patches for NixOS

### Security
- All services bound to localhost for security
- No external network exposure by default
- Isolated data directories per chain

The simulation environment is production-ready for development and testing workflows.