# Justfile Command Reference

Almanac uses [Just](https://just.systems/) as a command runner to automate development, testing, and deployment workflows. This document provides a comprehensive reference to all available commands.

## Quick Reference

Run `just` or `just default` to see the main command help menu.

## Main Commands

### Environment Management

#### `just init [chains="ethereum,cosmos,solana,fdb"]`
Initialize the Almanac environment with specified blockchain chains.
- Creates necessary directories: `logs/`, `data/`, `config/`
- Default includes all supported chains
- Example: `just init ethereum,solana`

#### `just start`
Start all configured blockchain services and dependencies.
- Automatically runs `just init` first
- Starts FoundationDB, Ethereum, Cosmos, and Solana nodes
- Shows startup confirmation messages

#### `just stop`
Stop all running Almanac services gracefully.
- Stops all blockchain nodes
- Stops FoundationDB
- Uses `|| true` to continue even if some services aren't running

#### `just status`
Check the status of all Almanac services.
- Shows running status for each service
- Displays port availability
- Shows log and data directory locations

### Individual Service Management

#### FoundationDB Commands
- `just start-fdb` / `just fdb` - Start FoundationDB using Nix environment
- `just stop-fdb` - Stop FoundationDB
- `just status-fdb` - Check FoundationDB status

#### Ethereum Commands
- `just start-ethereum` - Start Ethereum (Anvil) on port 8545
- `just stop-ethereum` - Stop Ethereum node
- `just status-ethereum` - Check Ethereum node status
- `just logs-ethereum` - View Ethereum logs

#### Cosmos Commands
- `just start-cosmos` - Start Cosmos (Neutron) on port 26657
- `just stop-cosmos` - Stop Cosmos node
- `just status-cosmos` - Check Cosmos node status
- `just logs-cosmos` - View Cosmos logs

#### Solana Commands
- `just start-solana` - Start Solana test validator on port 8899
- `just stop-solana` - Stop Solana validator
- `just status-solana` - Check Solana validator status
- `just logs-solana` - View Solana logs

### Logging

#### `just logs [service="all"]`
View logs for specific services:
- `just logs ethereum` - Ethereum/Anvil logs
- `just logs cosmos` - Cosmos logs  
- `just logs solana` - Solana logs
- `just logs fdb` - FoundationDB logs
- `just logs api` - Almanac API logs
- `just logs all` - Show available log options

## Development Commands

### Building and Testing

#### `just build`
Build the entire Almanac workspace using Cargo.

#### `just test [filter=""]`
Run tests with optional filtering.
- `just test` - Run all workspace tests (excludes almanac-tests)
- `just test storage` - Run tests matching "storage"
- Uses `--workspace --exclude almanac-tests` by default

#### `just test-e2e [test_name=""]`
Run end-to-end tests with full blockchain environment.
- Automatically cleans up existing blockchain processes
- Sets environment variables: `RUST_LOG=info`, `RUST_BACKTRACE=1`
- `just test-e2e` - Run all E2E tests
- `just test-e2e multi-chain-simulation` - Run specific test
- Cleans up blockchain processes after completion

#### `just test-all`
Run comprehensive test suite:
- Unit tests (`just test`)
- E2E tests (`just test-e2e`)
- Cosmos chain tests (`just test-cosmos`)

### Development Environments

#### `just dev [chain]`
Enter chain-specific development environments:
- `just dev ethereum` - Enter Ethereum Nix environment
- `just dev cosmos` - Enter Cosmos Nix environment
- `just dev solana` - Enter Solana Nix environment
- `just dev fdb` - Enter FoundationDB Nix environment
- `just dev simulation` - Enter simulation orchestrator environment

## Simulation Commands

### Simulation Orchestrator

#### `just sim-init [chains="ethereum,cosmos,solana,fdb"]`
Initialize simulation environment with specified chains.

#### `just sim-start`
Start the simulation cluster (shows platform-specific commands).

#### `just sim-stop`
Stop all simulation services.

#### `just sim-status`
Check simulation cluster status.

#### `just sim-logs [service="all"]`
View simulation service logs.

#### `just sim-quick-start`
Quick simulation startup:
1. Initialize simulation
2. Start cluster
3. Show status

#### `just sim-clean`
Clean simulation data and logs.

#### `just sim-restart`
Complete simulation restart: stop → clean → quick-start.

### Individual Simulation Services

#### `just sim-ethereum [cmd="status"]`
Control simulation Ethereum service.

#### `just sim-cosmos [cmd="status"]`
Control simulation Cosmos service.

#### `just sim-solana [cmd="status"]`
Control simulation Solana service.

#### `just sim-api [cmd="status"]`
Control simulation Almanac API service.

#### `just sim-indexer [cmd="status"]`
Control simulation Almanac indexer service.

## System Integration Commands

### Almanac System Services

#### `just start-system`
Start complete Almanac system (FoundationDB + API + Swagger UI):
1. Start FoundationDB
2. Wait 3 seconds for initialization
3. Start Almanac API on port 8080
4. Wait 3 seconds for API startup
5. Start Swagger UI on port 8081

#### `just stop-system`
Stop complete Almanac system in reverse order.

#### `just status-system`
Check status of all system components.

### API and Documentation

#### `just start-api [config="config/simulation.toml"]`
Start Almanac API server with specified configuration.
- Logs to `logs/almanac-api.log`
- Default uses simulation configuration

#### `just almanac`
Convenience alias for `just start-api`.

#### `just start-swagger`
Start Swagger UI documentation server.
- Runs on port 8081
- Logs to `logs/swagger-ui.log`

#### `just swagger`
Convenience alias for `just start-swagger`.

#### `just stop-swagger`
Stop Swagger UI server.

## Deployment Commands

### Server Management

#### `just deploy-server`
Deploy current codebase to NixOS server:
- Uses rsync to sync files to `tw-nix:/root/almanac/`
- Excludes `target/`, `.git/`, `node_modules/`

#### `just server-status`
Check Almanac service status on remote server.

#### `just server-start`
Start Almanac services on remote server.

#### `just server-stop`
Stop Almanac services on remote server.

#### `just server-logs [service="all"]`
View service logs from remote server.

### Solana Deployment

#### `just deploy-solana-stable`
Deploy stable Solana 1.14.23 to NixOS server:
- Copies `flake-nixos-stable.nix` to server
- Runs deployment command on server
- KVM-compatible version for production

#### `just server-solana-status`
Check Solana validator status on server:
- Checks if validator process is running
- Tests RPC endpoint health

#### `just server-solana-logs`
View Solana validator logs on remote server.

## Utility Commands

### Cleanup

#### `just clean`
Remove all data and logs:
- Deletes contents of `data/*` and `logs/*`
- Preserves directory structure

#### `just clean-all`
Deep clean including Cargo build artifacts:
- Runs `just clean`
- Runs `cargo clean` in Nix environment

### Quick Operations

#### `just quick-start`
Complete environment setup and startup:
1. Initialize environment (`just init`)
2. Start all services (`just start`)
3. Show status (`just status`)

### Development Tools

#### `just solana-hashes`
Calculate Solana git dependency hashes for Nix:
- Generates `outputHashes` for Nix package definitions
- Handles Solana's custom git dependencies
- Used for maintaining Nix flake compatibility

### Legacy Cosmos Support

#### `just install-wasmd`
Install wasmd for Cosmos development:
- Clones and builds wasmd v0.50.0
- Required for some Cosmos testing workflows

#### `just start-wasmd`
Start wasmd test chain:
- Initializes wasmd chain if needed
- Creates test accounts and genesis
- Runs on port 26657

#### `just test-cosmos`
Test Cosmos chain functionality:
- Requires running wasmd instance
- Queries chain status and balances

## Port Reference

| Service | Port | Protocol | Notes |
|---------|------|----------|-------|
| FoundationDB | 4500 | TCP | Database cluster |
| Ethereum (Anvil) | 8545 | HTTP/WS | JSON-RPC endpoint |
| Cosmos (Neutron) | 26657 | HTTP | Tendermint RPC |
| Cosmos gRPC | 9090 | gRPC | GRPC interface |
| Cosmos API | 1317 | HTTP | REST API |
| Solana | 8899 | HTTP/WS | JSON-RPC endpoint |
| Almanac API | 8080 | HTTP | REST API |
| Swagger UI | 8081 | HTTP | Documentation |

## Status Monitoring

### Service Status Commands

All status commands show:
- Process ID if running
- Port availability
- Service endpoints
- Log file locations

### Port Status Checking

`just status-ports` checks all service ports and shows:
- Open/Closed status for each port
- Quick overview of service availability

## Tips and Best Practices

### Development Workflow

1. **Start Development**: `just quick-start`
2. **Check Services**: `just status`
3. **View Logs**: `just logs [service]`
4. **Run Tests**: `just test`
5. **Clean Up**: `just stop` or `just clean`

### Testing Workflow

1. **Unit Tests**: `just test`
2. **Integration Tests**: `just test-e2e`
3. **Full Test Suite**: `just test-all`
4. **Service-Specific**: `just test-cosmos`

### Deployment Workflow

1. **Deploy Code**: `just deploy-server`
2. **Check Status**: `just server-status`
3. **Start Services**: `just server-start`
4. **Monitor**: `just server-logs`

### Simulation Development

1. **Initialize**: `just sim-quick-start`
2. **Monitor**: `just sim-status`
3. **Debug**: `just sim-logs [service]`
4. **Reset**: `just sim-restart`

## Environment Integration

The justfile integrates with:
- **Nix environments** for reproducible builds
- **Cargo workspace** for Rust development
- **Multiple blockchains** for multi-chain testing
- **FoundationDB** for persistent storage
- **Remote servers** for deployment

All commands are designed to work across platforms (macOS, Linux, NixOS) with automatic environment detection and optimization.