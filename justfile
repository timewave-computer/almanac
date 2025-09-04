# Almanac Multi-Chain Development Environment
# https://just.systems/man/en/

# Default recipe - show available commands
default:
    @echo "Almanac Multi-Chain Development Commands"
    @echo "======================================="
    @echo ""
    @echo "Main Commands:"
    @echo "  just init              - Initialize environment"
    @echo "  just start             - Start all services"
    @echo "  just stop              - Stop all services"
    @echo "  just status            - Check service status"
    @echo "  just logs <service>    - View service logs"
    @echo ""
    @echo "Simulation Commands (using nix/simulation):"
    @echo "  just sim-init          - Initialize simulation environment"
    @echo "  just sim-start         - Start simulation cluster"
    @echo "  just sim-stop          - Stop simulation cluster"
    @echo "  just sim-status        - Check simulation status"
    @echo "  just sim-logs <svc>    - View simulation logs"
    @echo "  just sim-quick-start   - Quick start simulation"
    @echo ""
    @echo "Individual Services:"
    @echo "  just start-ethereum    - Start Ethereum node"
    @echo "  just start-cosmos      - Start Cosmos node"
    @echo "  just start-solana      - Start Solana node"
    @echo "  just start-fdb         - Start FoundationDB (alias: just fdb)"
    @echo "  just start-api         - Start Almanac API server"
    @echo "  just start-swagger     - Start Swagger UI"
    @echo ""
    @echo "Development:"
    @echo "  just build             - Build the project"
    @echo "  just test              - Run tests"
    @echo "  just dev <chain>       - Enter chain dev environment"
    @echo ""
    @echo "System Integration:"
    @echo "  just start-system      - Start full Almanac system (FDB + API + Swagger)"
    @echo "  just stop-system       - Stop full Almanac system"
    @echo ""
    @echo "Run 'just --list' for all available commands"

# Initialize simulation environment
init chains="ethereum,cosmos,solana,fdb":
    @echo "Initializing Almanac environment for chains: {{chains}}"
    @mkdir -p logs data config
    @echo "Directories created: logs/, data/, config/"

# Start all services
start: init
    @echo "Starting Almanac services..."
    @just start-fdb
    @just start-ethereum
    @just start-cosmos
    @just start-solana
    @echo "All services started!"

# Stop all services
stop:
    @echo "Stopping Almanac services..."
    @just stop-ethereum || true
    @just stop-cosmos || true
    @just stop-solana || true
    @just stop-fdb || true
    @echo "All services stopped!"

# Check status of all services
status:
    @echo "Almanac Service Status"
    @echo "========================"
    @just status-ethereum
    @just status-cosmos
    @just status-solana
    @just status-fdb
    @just status-api
    @just status-swagger
    @echo ""
    @just status-ports
    @echo ""
    @echo "Logs: ./logs/"
    @echo "Data: ./data/"

# Check Almanac API status
status-api:
    @echo -n "Almanac API: "
    @if pgrep -f "almanac-api" >/dev/null 2>&1; then \
        echo "Running (PID: $(pgrep almanac-api))"; \
    else \
        echo "Not running"; \
    fi

# Check Swagger UI status
status-swagger:
    @echo -n "Swagger UI: "
    @if lsof -i :8081 >/dev/null 2>&1; then \
        echo "Running on port 8081"; \
    else \
        echo "Not running"; \
    fi

# Check port status for all services
status-ports:
    @echo "Port Status:"
    @echo -n "  4500 (FDB): "; lsof -i :4500 >/dev/null 2>&1 && echo "Open" || echo "Closed"
    @echo -n "  8545 (ETH): "; lsof -i :8545 >/dev/null 2>&1 && echo "Open" || echo "Closed"
    @echo -n "  8899 (SOL): "; lsof -i :8899 >/dev/null 2>&1 && echo "Open" || echo "Closed"
    @echo -n "  8080 (API): "; lsof -i :8080 >/dev/null 2>&1 && echo "Open" || echo "Closed"
    @echo -n "  8081 (Swagger): "; lsof -i :8081 >/dev/null 2>&1 && echo "Open" || echo "Closed"
    @echo -n "  26657 (Cosmos): "; lsof -i :26657 >/dev/null 2>&1 && echo "Open" || echo "Closed"

# FoundationDB commands
start-fdb:
    @mkdir -p logs data/foundationdb
    @cd nix/fdb && nix develop --command fdb-manage start || echo "FoundationDB using mock or not available"

stop-fdb:
    @cd nix/fdb && nix develop --command fdb-manage stop 2>/dev/null || true

status-fdb:
    @echo -n "FoundationDB: "
    @cd nix/fdb && nix develop --command fdb-manage status 2>&1 | grep -E "(ready|running|not responding|mock)" | head -1 || echo "Not available"

# Convenient alias for start-fdb
fdb: start-fdb

# Convenient alias for start-api  
almanac: start-api

# Convenient alias for start-swagger
swagger: start-swagger

# Ethereum commands
start-ethereum:
    @echo "Starting Ethereum (Anvil)..."
    @mkdir -p logs data/ethereum
    @cd nix/ethereum && nix develop --command bash -c "anvil --host 0.0.0.0 --port 8545 > ../../logs/anvil.log 2>&1 &"

stop-ethereum:
    @pkill -f "anvil.*8545" || true

status-ethereum:
    @echo -n "Ethereum: "
    @if nc -z localhost 8545 2>/dev/null; then \
        echo "Running (http://localhost:8545)"; \
    else \
        echo "Not running"; \
    fi

logs-ethereum:
    @tail -f logs/anvil.log

# Cosmos commands
start-cosmos:
    @echo "Starting Cosmos (Neutron)..."
    @mkdir -p logs data/cosmos
    @cd nix/cosmos && nix develop --command bash -c "neutron-start > ../../logs/cosmos.log 2>&1 &"

stop-cosmos:
    @pkill -f "neutrond" || true

status-cosmos:
    @echo -n "Cosmos: "
    @if nc -z localhost 26657 2>/dev/null; then \
        echo "Running (http://localhost:26657)"; \
    else \
        echo "Not running"; \
    fi

logs-cosmos:
    @tail -f logs/cosmos.log

# Solana commands
start-solana:
    @echo "Starting Solana..."
    @mkdir -p logs data/solana
    @solana-test-validator --ledger data/solana --log > logs/solana.log 2>&1 &

stop-solana:
    @pkill -f "solana-test-validator" || true

status-solana:
    @echo -n "Solana: "
    @if nc -z localhost 8899 2>/dev/null; then \
        echo "Running (http://localhost:8899)"; \
    else \
        echo "Not running"; \
    fi

logs-solana:
    @tail -f logs/solana.log

# View logs for a specific service
logs service="all":
    #!/usr/bin/env bash
    case {{service}} in
        eth|ethereum) just logs-ethereum ;;
        cosmos) just logs-cosmos ;;
        sol|solana) just logs-solana ;;
        fdb|foundationdb) tail -f logs/foundationdb.log ;;
        api|almanac) tail -f logs/almanac-api.log ;;
        all)
            echo "Available logs:"
            echo "  just logs ethereum  # Ethereum/Anvil logs"
            echo "  just logs cosmos    # Cosmos logs"
            echo "  just logs solana    # Solana logs"
            echo "  just logs fdb       # FoundationDB logs"
            echo "  just logs api       # Almanac API logs"
            ;;
        *) echo "Unknown service: {{service}}" ;;
    esac

# Build the Almanac project
build:
    cargo build --workspace

# Run tests
test filter="":
    #!/usr/bin/env bash
    if [ -n "{{filter}}" ]; then
        cargo test --workspace --exclude almanac-tests -- {{filter}}
    else
        cargo test --workspace --exclude almanac-tests
    fi

# Run E2E tests with all chains
test-e2e test_name="":
    #!/usr/bin/env bash
    set -euo pipefail
    
    # Colors
    BLUE='\033[0;34m'
    GREEN='\033[0;32m'
    RED='\033[0;31m'
    NC='\033[0m'
    
    echo -e "${BLUE}╔════════════════════════════════════════════════════════════╗${NC}"
    echo -e "${BLUE}║              Almanac E2E Test Runner                       ║${NC}"
    echo -e "${BLUE}╚════════════════════════════════════════════════════════════╝${NC}"
    echo
    
    # Clean up any existing processes
    echo "Cleaning up existing blockchain processes..."
    pkill -f "solana-test-validator" || true
    pkill -f "anvil" || true
    pkill -f "neutron" || true
    sleep 2
    
    # Run tests
    export RUST_LOG=info
    export RUST_BACKTRACE=1
    
    if [ -z "{{test_name}}" ]; then
        echo "Running all E2E tests..."
        nix develop -c cargo run --package almanac-tests --bin test-runner -- e2e multi-chain-simulation
    else
        echo "Running E2E test: {{test_name}}"
        nix develop -c cargo run --package almanac-tests --bin test-runner -- e2e {{test_name}}
    fi
    
    # Clean up after tests
    pkill -f "solana-test-validator" || true
    pkill -f "anvil" || true
    pkill -f "neutron" || true
    
    echo -e "${GREEN}E2E tests completed!${NC}"

# Start Almanac API
start-api config="config/simulation.toml":
    @mkdir -p logs
    bash -c "cargo run --bin almanac-api -- --config {{config}} > logs/almanac-api.log 2>&1 &"

# Clean all data and logs
clean:
    rm -rf data/* logs/*

# Deep clean including Cargo build artifacts
clean-all: clean
    nix develop --command cargo clean

# Development shell for a specific chain
dev chain:
    #!/usr/bin/env bash
    case {{chain}} in
        eth|ethereum) cd nix/ethereum && nix develop ;;
        cosmos) cd nix/cosmos && nix develop ;;
        sol|solana) cd nix/solana && nix develop ;;
        fdb|foundationdb) cd nix/fdb && nix develop ;;
        sim|simulation) cd nix/simulation && nix develop ;;
        *) 
            echo "Unknown chain: {{chain}}"
            echo "Available: ethereum, cosmos, solana, fdb, simulation"
            ;;
    esac

# Simulation orchestrator commands
sim-init chains="ethereum,cosmos,solana,fdb":
    cd nix/simulation && nix run .#sim-cluster -- init {{chains}}

sim-start:
    cd nix/simulation && nix run .#sim-cluster -- start

sim-stop:
    cd nix/simulation && nix run .#sim-cluster -- stop

sim-status:
    cd nix/simulation && nix run .#sim-cluster -- status

sim-logs service="all":
    cd nix/simulation && nix run .#sim-cluster -- logs {{service}}

# Generate simulation configuration
sim-config chains="ethereum,cosmos,solana,fdb":
    cd nix/simulation && nix run .#sim-config -- {{chains}}

# Individual simulation service commands
sim-ethereum cmd="status":
    cd nix/simulation && nix run .#sim-ethereum -- {{cmd}}

sim-cosmos cmd="status":
    cd nix/simulation && nix run .#sim-cosmos -- {{cmd}}

sim-solana cmd="status":
    cd nix/simulation && nix run .#sim-solana -- {{cmd}}

# Almanac services in simulation
sim-api cmd="status":
    cd nix/simulation && nix run .#sim-almanac-api -- {{cmd}}

sim-indexer cmd="status":
    cd nix/simulation && nix run .#sim-almanac-indexer -- {{cmd}}

# Quick simulation commands
sim-quick-start:
    @just sim-init
    @just sim-start
    @just sim-status

sim-clean:
    @echo "Cleaning simulation data and logs..."
    rm -rf nix/simulation/data nix/simulation/logs
    @echo "Simulation environment cleaned"

sim-restart: sim-stop sim-clean sim-quick-start

# Quick start - initialize and start everything
quick-start:
    @just init
    @just start
    @just status

# Server deployment helpers (for NixOS server)
deploy-server:
    rsync -avz --exclude target --exclude .git --exclude node_modules ./ tw-nix:/root/almanac/

server-status:
    ssh tw-nix "cd /root/almanac && just status"

server-start:
    ssh tw-nix "cd /root/almanac && just start"

server-stop:
    ssh tw-nix "cd /root/almanac && just stop"

server-logs service="all":
    ssh tw-nix "cd /root/almanac && just logs {{service}}"

# Calculate Solana git dependency hashes for Nix
solana-hashes:
    #!/usr/bin/env bash
    echo "Calculating hashes for Solana git dependencies..."
    echo ""
    
    declare -A git_deps=(
        ["aes-gcm-siv-0.10.3"]="https://github.com/RustCrypto/AEADs 6105d7a5591aefa646a95d12b5e8d3f55a9214ef"
        ["crossbeam-epoch-0.9.5"]="https://github.com/anza-xyz/crossbeam fd279d707025f0e60951e429bf778b4813d1b6bf"
        ["curve25519-dalek-3.2.1"]="https://github.com/anza-xyz/curve25519-dalek 0382b672560493840f453f2a0e24c4a129abd3a4"
        ["tokio-1.29.1"]="https://github.com/anza-xyz/solana-tokio 7cf47705faacf7bf0e43e4131a5377b3291fce21"
        ["tokio-macros-2.1.0"]="https://github.com/anza-xyz/solana-tokio 7cf47705faacf7bf0e43e4131a5377b3291fce21"
    )
    
    echo "outputHashes = {"
    for dep in "${!git_deps[@]}"; do
        IFS=' ' read -r url rev <<< "${git_deps[$dep]}"
        echo -n "  Calculating hash for $dep... " >&2
        
        hash=$(nix-shell -p nix-prefetch-git --run "nix-prefetch-git --quiet '$url' '$rev'" 2>/dev/null | grep '"sha256"' | cut -d'"' -f4)
        
        if [ -n "$hash" ]; then
            echo "done" >&2
            echo "  \"$dep\" = \"$hash\";"
        else
            echo "FAILED" >&2
            echo "  # FAILED: $dep"
        fi
    done
    echo "};"

# Deploy stable Solana 1.14.23 to NixOS server
deploy-solana-stable:
    #!/usr/bin/env bash
    echo "Deploying stable Solana 1.14.23 to NixOS server..."
    
    # Copy flake to server
    scp nix/solana/flake-nixos-stable.nix root@tw-nix:/root/solana-deployment/
    
    # Deploy on server
    ssh root@tw-nix "cd /root/solana-deployment && nix develop -f flake-nixos-stable.nix --command deploy"
    
    echo "Deployment complete! Check with: just server-solana-status"

# Check Solana status on server
server-solana-status:
    ssh root@tw-nix "pgrep -f solana-test-validator && echo 'Solana validator is running' || echo 'Solana validator is not running'"
    ssh root@tw-nix "curl -s http://localhost:8899/health | jq . || echo 'RPC not responding'"

# View Solana logs on server
server-solana-logs:
    ssh root@tw-nix "tail -f /root/solana-deployment/validator.log"

# Install wasmd for Cosmos development
install-wasmd:
    #!/usr/bin/env bash
    echo "Installing wasmd..."
    git clone https://github.com/CosmWasm/wasmd.git /tmp/wasmd
    cd /tmp/wasmd
    git checkout v0.50.0
    make install
    echo "wasmd installed successfully"

# Start wasmd test chain
start-wasmd:
    #!/usr/bin/env bash
    mkdir -p data/wasmd logs
    
    if [ ! -f "$HOME/.wasmd/config/genesis.json" ]; then
        echo "Initializing wasmd chain..."
        wasmd init test-node --chain-id test-chain
        wasmd keys add test-account --keyring-backend test
        wasmd genesis add-genesis-account test-account 1000000000stake --keyring-backend test
        wasmd genesis gentx test-account 100000000stake --keyring-backend test
        wasmd genesis collect-gentxs
    fi
    
    echo "Starting wasmd..."
    wasmd start --log_level info > logs/wasmd.log 2>&1 &

# Start Swagger UI
start-swagger:
    @mkdir -p logs
    cd nix/swagger-ui && nix run . > ../../logs/swagger-ui.log 2>&1 &

# Stop Swagger UI
stop-swagger:
    @pkill -f "swagger-static-server" || true

# Start complete Almanac system (FoundationDB + API + Swagger UI)
start-system:
    @echo "Starting complete Almanac system..."
    @just start-fdb
    @echo "Waiting for FoundationDB to initialize..."
    @sleep 3
    @just start-api
    @echo "Waiting for API server to start..."  
    @sleep 3
    @just start-swagger
    @echo "Complete Almanac system started!"
    @echo "Services:"
    @echo "  - FoundationDB: Running on default ports"
    @echo "  - Almanac API: http://localhost:8080 (logs: logs/almanac-api.log)"
    @echo "  - Swagger UI: http://localhost:8081 (logs: logs/swagger-ui.log)"

# Stop complete Almanac system
stop-system:
    @echo "Stopping complete Almanac system..."
    @just stop-swagger
    @pkill -f "almanac-api" || true
    @just stop-fdb
    @echo "Complete Almanac system stopped!"

# Check system status
status-system:
    @echo "Almanac System Status:"
    @echo "======================"
    @just status-fdb
    @echo -n "Almanac API: "
    @if pgrep -f "almanac-api" >/dev/null 2>&1; then \
        echo "Running (PID: $(pgrep almanac-api))"; \
    else \
        echo "Not running"; \
    fi
    @echo -n "Swagger UI: "
    @if pgrep -f "swagger" >/dev/null 2>&1; then \
        echo "Running (PID: $(pgrep -f swagger | head -1))"; \
    else \
        echo "Not running"; \
    fi

# Test Cosmos chain functionality
test-cosmos:
    #!/usr/bin/env bash
    echo "Testing Cosmos chain..."
    
    # Check if wasmd is running
    if ! nc -z localhost 26657; then
        echo "Cosmos chain not running. Start with: just start-cosmos"
        exit 1
    fi
    
    # Query chain
    wasmd query bank balances $(wasmd keys show test-account -a --keyring-backend test)

# Run all tests
test-all:
    @just test
    @just test-e2e
    @just test-cosmos

# Check deployment status with detailed info
check-deployment:
    @just status
    @echo ""
    @echo "Binary Availability:"
    @echo -n "  almanac-api: "; [ -f "target/release/almanac-api" ] && echo "Built" || echo "Not built"
    @echo -n "  fdbcli: "; [ -f "fdb-build/bin/fdbcli" ] && echo "Available" || echo "Not found"
    @echo ""
    @echo "Recent Logs:"
    @for log in fdb.log anvil.log solana.log wasmd.log api.log swagger-ui.log; do \
        if [ -f "logs/$$log" ]; then \
            echo "  $$log: $$(wc -l < logs/$$log) lines"; \
        fi; \
    done