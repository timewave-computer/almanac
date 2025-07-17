# Nix-Based Local Development Setup Guide

This guide will help you set up a local development environment for Almanac using Nix.

## Prerequisites

1. **Install Nix**:
   ```bash
   # macOS
   sh <(curl -L https://nixos.org/nix/install)
   
   # Linux
   sh <(curl -L https://nixos.org/nix/install) --daemon
   ```

2. **Enable Flakes** (if not already enabled):
   ```bash
   echo "experimental-features = nix-command flakes" >> ~/.config/nix/nix.conf
   ```

## Getting Started

### 1. Clone the Repository

```bash
git clone https://github.com/timewave-computer/almanac.git
cd almanac
```

### 2. Enter the Development Shell

```bash
# Default development shell with all tools
nix develop

# Or use specialized shells:
nix develop .#api        # API development
nix develop .#blockchain # Blockchain development
nix develop .#ci         # Minimal CI environment
nix develop .#bench      # Benchmarking tools
```

### 3. Initialize Databases

Once in the Nix shell, initialize PostgreSQL and RocksDB:

```bash
init_databases
```

This will:
- Create data directories at `./data/postgres` and `./data/rocksdb`
- Initialize and start PostgreSQL on port 5432
- Create the `indexer` database
- Run any migrations

### 4. Build the Project

```bash
# Build all crates
cargo build

# Build specific crates
cargo build -p almanac-infrastructure-core
cargo build -p almanac-api-http
cargo build -p indexer-api

# Build in release mode
cargo build --release
```

## Development Commands

The Nix shell provides several helpful commands:

### Database Management
- `init_databases` - Initialize and start PostgreSQL and RocksDB
- `stop_databases` - Stop PostgreSQL server
- `pg_isready` - Check if PostgreSQL is running

### API Development
- `start_api_server` - Start the API HTTP server on port 8080
- `test_api` - Run API tests
- `build_api` - Build API crates in release mode
- `api_health_check` - Check if API server is running
- `watch_api` - Auto-restart API on code changes (in API shell)
- `test_watch` - Auto-run tests on code changes (in API shell)

### Testing
- `run_almanac_tests` - Run the complete test suite
- `cargo test` - Run Rust tests
- `./scripts/test-api.sh` - Run API-specific tests

### Nix Build Commands
- `generate_cargo_nix` - Generate Cargo.nix from Cargo.toml
- `update_cargo_nix` - Update existing Cargo.nix file
- `nix build` - Build packages using Nix

## Environment Variables

The development shell sets up these environment variables:

- `DATABASE_URL` - PostgreSQL connection string
- `PGHOST`, `PGPORT`, `PGUSER`, `PGPASSWORD`, `PGDATABASE` - PostgreSQL connection details
- `RUST_LOG` - Logging configuration (default: "info")
- `MACOSX_DEPLOYMENT_TARGET` - macOS deployment target (for macOS users)

## Running Services

### Start API Server

```bash
# Using the helper function
start_api_server

# Or manually
cargo run -p indexer-api -- --port 8080

# With custom configuration
API_PORT=3000 cargo run -p indexer-api
```

### Running Tests

```bash
# All tests
cargo test

# API tests only
test_api

# With output
cargo test -- --nocapture

# Specific test
cargo test -p indexer-api test_health_endpoint
cargo test -p almanac-infrastructure-core
```

### Development Workflow

```bash
# Start databases
init_databases

# Start API server in development mode
start_api_server

# In another terminal, run tests
cargo test

# Watch for changes and auto-restart
watch_api
```

## Troubleshooting

### PostgreSQL Won't Start

1. Check if another instance is running:
   ```bash
   ps aux | grep postgres
   ```

2. Check the log file:
   ```bash
   cat data/postgres/postgres.log
   ```

3. Remove data directory and reinitialize:
   ```bash
   rm -rf data/postgres
   init_databases
   ```

### Cargo.nix Issues

If you encounter build issues with Nix:

1. Regenerate Cargo.nix:
   ```bash
   generate_cargo_nix
   ```

2. Clean and rebuild:
   ```bash
   rm -rf result
   nix build
   ```

### macOS Specific Issues

If you encounter linking errors on macOS:

1. Ensure Xcode Command Line Tools are installed:
   ```bash
   xcode-select --install
   ```

2. The Nix shell automatically sets `MACOSX_DEPLOYMENT_TARGET` and clears `DEVELOPER_DIR` to avoid conflicts.

### RocksDB Issues

If RocksDB fails to initialize:

1. Check disk space:
   ```bash
   df -h
   ```

2. Clear RocksDB data:
   ```bash
   rm -rf data/rocksdb
   init_databases
   ```

## Advanced Usage

### Custom Database Configuration

Override default database settings:

```bash
export PGPORT=5433
export PGDATABASE=my_custom_db
init_databases
```

### Running Multiple Services

Use different shells for different services:

```bash
# Terminal 1 - Database
nix develop
init_databases

# Terminal 2 - API Server
nix develop .#api
watch_api

# Terminal 3 - Testing
nix develop
test_watch
```

### Production Build

Build optimized binaries:

```bash
# Using Cargo
cargo build --release

# Using Nix (creates reproducible builds)
nix build .#indexer-api
nix build .#almanac-infrastructure-core
```

### Available Nix Packages

The flake provides several packages:

```bash
# Build specific components
nix build .#indexer-api              # API service
nix build .#almanac-infrastructure-core  # Core infrastructure
nix build .#indexer-ethereum         # Ethereum integration
nix build .#indexer-cosmos          # Cosmos integration
nix build .#indexer-tools           # Development tools
```

### Development Environment Customization

Create a custom development environment:

```bash
# Create a custom shell configuration
cat > shell.nix << 'EOF'
{ pkgs ? import <nixpkgs> {} }:
pkgs.mkShell {
  buildInputs = with pkgs; [
    # Add your custom tools here
    jq
    curl
    postgresql
  ];
}
EOF

# Enter the custom shell
nix-shell
```

## Next Steps

- Read the [API documentation](./002_almanac_api.md)
- Check out [example integrations](../examples/)
- Review the [architecture guide](./001_architecture.md)
- Follow the [integration tutorial](./003_integration_tutorial.md)
- Join our community for support

## Useful Commands Reference

```bash
# Database operations
init_databases                    # Initialize and start databases
stop_databases                   # Stop databases
pg_isready                       # Check PostgreSQL status

# Development
cargo build                      # Build all packages
cargo test                       # Run all tests
cargo run -p indexer-api         # Run API server

# Nix operations
nix develop                      # Enter development shell
nix build .#indexer-api          # Build API server
generate_cargo_nix              # Update Cargo.nix
update_cargo_nix                # Update existing Cargo.nix

# API development
start_api_server                # Start API server
test_api                        # Run API tests
api_health_check               # Check API health
watch_api                      # Watch and restart API
```

This development setup provides a reproducible, consistent environment for Almanac development across different systems and contributors.