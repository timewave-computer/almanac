# Almanac Scripts Directory

This directory contains utility scripts for development, testing, and operations.

## Script Categories

### Development Environment
- `setup-dev-env.sh` - One-command development environment setup
- `make-scripts-executable.sh` - Ensures all scripts have execute permissions

### Database Management
- `databases/setup-postgres.sh` - Initialize PostgreSQL database
- `databases/setup-rocksdb.sh` - Initialize RocksDB storage
- `databases/create-postgres-tables.sh` - Create database schema
- `databases/reset-databases.sh` - Reset databases to clean state
- `databases/populate-rocksdb.sh` - Populate RocksDB with test data
- `prepare-sqlx.sh` - Prepare SQLX offline mode

### Testing
- `test-api.sh` - Run API-specific tests
- `run-all-tests.sh` - Run comprehensive test suite
- `run-db-tests.sh` - Run database-specific tests
- `run-db-tests-nix.sh` - Run database tests in Nix environment
- `run-db-tests-simple.sh` - Simple database test runner
- `isolated-rocks-test.sh` - Test RocksDB in isolation
- `run-rocksdb-benchmark.sh` - Benchmark RocksDB performance
- `verify-api-refactor.sh` - Verify API refactor completion

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

### Other Scripts
- `run-almanac-indexer.sh` - Run the Almanac indexer
- `test-rocks-only.sh` - Test only RocksDB functionality
- `test-rocks-transactions.sh` - Test RocksDB transactions
- `test-rocks.sh` - General RocksDB tests
- `standalone-rocks-test.sh` - Standalone RocksDB test
- `run-test-with-db-fix.sh` - Run tests with database fixes
- `run-pure-rocks-test.sh` - Pure RocksDB test without dependencies
- `run_e2e_tests.sh` - Run end-to-end test suite
- `create-test-db-tables.sh` - Create test database tables
- `README-postgres.md` - PostgreSQL setup documentation

## Usage

All scripts should be run from the project root directory:

```bash
# Example: Setup development environment
./scripts/setup-dev-env.sh

# Example: Start API service
./scripts/manage-api-service.sh start

# Example: Run all tests
./scripts/run-all-tests.sh
```

## Nix Integration

Most scripts automatically detect if they're running in a Nix environment and will use `nix develop --command` if needed. This ensures consistent execution environment.

## Adding New Scripts

When adding new scripts:
1. Make them executable: `chmod +x script-name.sh`
2. Add proper error handling with `set -euo pipefail`
3. Include usage documentation at the top
4. Use the Nix detection pattern if the script requires specific tools
5. Update this README with the new script

## Maintenance

To ensure all scripts are executable:
```bash
./scripts/make-scripts-executable.sh
```