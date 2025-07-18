# Almanac Scripts Directory

This directory contains utility scripts for development, testing, and operations.

## Script Categories

### Development Environment
- `setup-dev-env.sh` - One-command development environment setup using Nix

### Database Management
- `databases/setup-postgres.sh` - Initialize PostgreSQL database
- `databases/setup-rocksdb.sh` - Initialize RocksDB storage
- `databases/create-postgres-tables.sh` - Create database schema
- `databases/reset-databases.sh` - Reset databases to clean state
- `databases/populate-rocksdb.sh` - Populate RocksDB with test data
- `create-test-db-tables.sh` - Create test database tables
- `prepare-sqlx.sh` - Prepare SQLX offline mode

### Testing
- `test-api.sh` - Run API-specific tests
- `run-all-tests.sh` - Run comprehensive test suite
- `run-db-tests.sh` - Run database-specific tests
- `run_e2e_tests.sh` - Run end-to-end test suite

### Service Management
- `manage-api-service.sh` - Start/stop/restart API service
- `monitor-api-health.sh` - Monitor API health and performance

### Blockchain Simulation
- `simulation/start-all.sh` - Start all blockchain simulations
- `simulation/stop-all.sh` - Stop all blockchain simulations
- `simulation/index-ethereum-anvil.sh` - Index Ethereum contracts with Anvil
- `simulation/index-ethereum-reth.sh` - Index Ethereum contracts with Reth
- `simulation/index-cosmwasm.sh` - Index CosmWasm contracts
- `simulation/tests/run-e2e-test.sh` - Run end-to-end tests

### Blockchain Tools
- `run-almanac-indexer.sh` - Run the Almanac indexer

## Usage Examples

### Quick Start
```bash
# Set up development environment
./scripts/setup-dev-env.sh

# Run all tests
./scripts/run-all-tests.sh

# Start API service
./scripts/manage-api-service.sh start

# Monitor API health
./scripts/monitor-api-health.sh --once
```

### Database Operations
```bash
# Initialize databases
./scripts/databases/setup-postgres.sh

# Reset databases
./scripts/databases/reset-databases.sh

# Run database tests
./scripts/run-db-tests.sh
```

### Testing
```bash
# Run API tests
./scripts/test-api.sh

# Run comprehensive end-to-end tests
./scripts/run_e2e_tests.sh --verbose

# Run specific test category
./scripts/run_e2e_tests.sh --filter cosmos
```

### Service Management
```bash
# Start API service as daemon
./scripts/manage-api-service.sh start --daemon

# Check service status
./scripts/manage-api-service.sh status

# View service logs
./scripts/manage-api-service.sh logs --follow

# Check API health
./scripts/manage-api-service.sh health
```

## Script Requirements

All scripts are designed to work with the Nix development environment. Make sure you're in the project root directory when running scripts.

Most scripts will automatically enter the Nix shell if needed, but for best performance, run:

```bash
nix develop
```

Then execute the scripts from within the Nix environment.