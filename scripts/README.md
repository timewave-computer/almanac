# Almanac Scripts Directory

This directory contains utility scripts for development, testing, and operations, organized by category for better maintainability.

## Organization Structure

```
scripts/
├── README.md                    # This file
├── dev/                        # Development Environment Setup
│   ├── setup-dev-env.sh        # One-command development environment setup
│   └── setup-fdb-headers.sh    # Set up FoundationDB headers for compilation
├── testing/                    # All Testing Scripts
│   ├── run-all-tests.sh        # Master test runner (orchestrates all tests)
│   ├── api-tests.sh            # Consolidated API testing (memory + FDB)
│   ├── e2e-tests.sh            # Consolidated E2E testing
│   └── fdb-tests.sh            # Consolidated FDB testing
├── services/                   # Service Management
│   ├── manage-api-service.sh   # Start/stop/restart API service
│   ├── monitor-api-health.sh   # Monitor API health and performance
│   └── run-almanac-indexer.sh  # Run the Almanac indexer
└── simulation/                 # Simulation Environment
    ├── start-all.sh            # Start all blockchain simulations
    ├── stop-all.sh             # Stop all blockchain simulations
    ├── indexers/               # Chain-specific indexing scripts
    │   ├── index-cosmwasm.sh   # Index CosmWasm contracts
    │   └── index-ethereum-anvil.sh # Index Ethereum with Anvil
    └── tests/
        └── run-e2e-test.sh     # Run simulation E2E tests
```

## Quick Start

### Development Environment Setup
```bash
# Set up complete development environment
./scripts/dev/setup-dev-env.sh

# Set up FoundationDB headers (if needed)
./scripts/dev/setup-fdb-headers.sh
```

### Testing
```bash
# Run all test suites
./scripts/testing/run-all-tests.sh

# Run only fast unit tests
./scripts/testing/run-all-tests.sh --quick

# Run specific test types
./scripts/testing/api-tests.sh              # API tests only
./scripts/testing/fdb-tests.sh              # FoundationDB tests only
./scripts/testing/e2e-tests.sh              # E2E tests only

# Test options
./scripts/testing/run-all-tests.sh --verbose --dry-run  # Show what would run
./scripts/testing/api-tests.sh --backend memory         # Test specific backend
./scripts/testing/e2e-tests.sh --simulation-only       # Simulation tests only
```

### Service Management
```bash
# API service management
./scripts/services/manage-api-service.sh start          # Start API
./scripts/services/manage-api-service.sh status         # Check status
./scripts/services/manage-api-service.sh logs           # View logs
./scripts/services/manage-api-service.sh stop           # Stop API

# Health monitoring
./scripts/services/monitor-api-health.sh --once         # Single check
./scripts/services/monitor-api-health.sh --continuous   # Continuous monitoring

# Indexer service
./scripts/services/run-almanac-indexer.sh --config config/ethereum.toml
```

## Nix Integration

Many scripts are now replaced by more sophisticated Nix packages:

### Replaced Scripts → Nix Commands

| **Old Script** | **New Nix Command** | **Description** |
|---|---|---|
| `swagger-ui-server.sh` | `nix run .#swagger-ui` | Advanced Swagger UI server |
| `swagger-static-server.sh` | `nix run .#swagger-ui` | Static OpenAPI server |
| `demo-simulation.sh` | `nix run .#sim-cluster` | Full simulation orchestrator |
| Individual indexers | `nix run .#sim-ethereum`, etc. | Chain-specific launchers |

### Nix-Based Simulation

Use the advanced simulation cluster instead of individual scripts:

```bash
# Initialize simulation environment
nix run .#sim-cluster init ethereum,cosmos,fdb

# Start all services
nix run .#sim-cluster start

# Check status
nix run .#sim-cluster status

# View logs
nix run .#sim-cluster logs ethereum
nix run .#sim-cluster logs api

# Stop all services
nix run .#sim-cluster stop
```

### Documentation and API Tools

```bash
# Start Swagger UI server
nix run .#swagger-ui

# Start on custom port
nix run .#swagger-ui 3000

# Generate OpenAPI specification
nix run .#generate-openapi-spec
```

## Script Categories

### Development Environment (`dev/`)
Scripts for setting up and configuring the development environment.

- **setup-dev-env.sh** - Complete environment setup using Nix
- **setup-fdb-headers.sh** - FoundationDB header setup for compilation

### Testing (`testing/`)
Comprehensive testing infrastructure with consolidated scripts.

#### Master Test Runner
- **run-all-tests.sh** - Orchestrates all test suites
  - Unit tests (always run)
  - API tests (memory + FDB backends)
  - FDB integration tests
  - E2E tests (optional)
  - Code quality checks (formatting, clippy)

#### Specialized Test Scripts
- **api-tests.sh** - API functionality testing
  - Memory backend testing
  - FoundationDB backend testing
  - Live API endpoint testing
  
- **fdb-tests.sh** - FoundationDB testing
  - Header and build tests
  - Connectivity tests
  - Storage integration tests
  - E2E functionality tests
  
- **e2e-tests.sh** - End-to-end testing
  - Basic E2E tests
  - Component integration tests
  - Multi-chain simulation tests

### Service Management (`services/`)
Scripts for managing Almanac services in development and production.

- **manage-api-service.sh** - Complete API service lifecycle management
- **monitor-api-health.sh** - Health monitoring and alerting
- **run-almanac-indexer.sh** - Indexer service launcher

### Simulation Environment (`simulation/`)
Scripts for blockchain simulation environment (Legacy - prefer Nix commands).

- **start-all.sh / stop-all.sh** - Basic simulation control
- **indexers/** - Chain-specific indexing scripts
- **tests/** - Simulation-specific tests

## Script Features

### Consistent Interface
All scripts support standardized options:
- `--help` - Usage information and examples
- `--verbose` - Detailed output for debugging
- `--dry-run` - Show what would be executed without running

### Environment Integration
- **Nix-aware** - Automatically enters nix develop if needed
- **Path-agnostic** - Works from any directory
- **Error handling** - Proper exit codes and error messages

### Output Formatting
- **Color-coded output** - Easy to scan success/failure status
- **Progress tracking** - Clear indication of test progress
- **Summary reports** - Detailed results at completion

## Usage Examples

### Development Workflow
```bash
# 1. Setup environment
./scripts/dev/setup-dev-env.sh

# 2. Run quick tests during development
./scripts/testing/run-all-tests.sh --quick

# 3. Before committing - run full test suite
./scripts/testing/run-all-tests.sh

# 4. Test specific functionality
./scripts/testing/api-tests.sh --backend fdb --verbose
```

### CI/CD Integration
```bash
# Fast feedback - unit tests only
./scripts/testing/run-all-tests.sh --quick --verbose

# Full validation - all tests
./scripts/testing/run-all-tests.sh --verbose

# Specific test categories
./scripts/testing/fdb-tests.sh --build-only  # Build validation
./scripts/testing/e2e-tests.sh --components-only  # Integration only
```

### Production Deployment
```bash
# Service management
./scripts/services/manage-api-service.sh start --daemon
./scripts/services/monitor-api-health.sh --continuous &

# Health checks
./scripts/services/manage-api-service.sh health
```

## Troubleshooting

### Common Issues

**Scripts not executable**
```bash
chmod +x scripts/**/*.sh
```

**Nix environment issues**
```bash
# Ensure nix develop works
nix develop

# Check if foundationdb is available
nix develop .#fdb
```

**Test failures**
```bash
# Debug with verbose output
./scripts/testing/run-all-tests.sh --verbose

# Test individual components
./scripts/testing/api-tests.sh --backend memory --verbose
./scripts/testing/fdb-tests.sh --build-only --verbose
```

**Service startup issues**
```bash
# Check service logs
./scripts/services/manage-api-service.sh logs

# Verify configuration
./scripts/services/manage-api-service.sh status
```

### Debug Mode

Enable debug output for any script:
```bash
# Bash debug mode
bash -x ./scripts/testing/run-all-tests.sh --dry-run

# Environment debugging
RUST_LOG=debug ./scripts/testing/api-tests.sh --verbose
```

## Further Reading

- [API Documentation](../docs/301_querying_blockchain_data_api.md) - Complete API usage guide
- [FoundationDB Setup](../docs/003_foundationdb_setup_and_usage.md) - FDB configuration
- [Testing Guide](../docs/201_comprehensive_testing_guide.md) - Comprehensive testing documentation
- [Running Guide](../docs/002_running_and_configuration.md) - Service deployment and management

## Migration from Old Scripts

If you have references to old scripts, here's the migration guide:

| **Old Script** | **New Script** |
|---|---|
| `test-api.sh` | `testing/api-tests.sh` |
| `test-api-fdb.sh` | `testing/api-tests.sh --backend fdb` |
| `run_e2e_tests.sh` | `testing/e2e-tests.sh` |
| `run-multi-chain-e2e.sh` | `testing/e2e-tests.sh --simulation-only` |
| `test-fdb-build.sh` | `testing/fdb-tests.sh --build-only` |
| `test-fdb-e2e.sh` | `testing/fdb-tests.sh --e2e-only` |
| `run-all-tests.sh` | `testing/run-all-tests.sh` |

All old scripts have been removed or consolidated for better maintainability.