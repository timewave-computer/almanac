{
  description = "Almanac Workflow Environments";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
        
        # For macOS, set a deployment target that matches minimum requirements
        darwinDeploymentTarget = "11.0";
        
        # Create a database initialization script that will be incorporated into the workflows
        init-databases-script = let
          postgresql = pkgs.postgresql;
        in pkgs.writeShellScriptBin "init-databases" ''
          #!/usr/bin/env bash
          # init_databases.sh - Initialization script for Almanac databases
          # This script initializes both PostgreSQL and RocksDB for the Almanac project
          
          set -e
          
          # Add PostgreSQL to the PATH
          export PATH="${postgresql}/bin:$PATH"
          
          # Get the project root directory
          PROJECT_ROOT=$(git rev-parse --show-toplevel 2>/dev/null || pwd)
          cd "$PROJECT_ROOT"
          
          # Make sure simulation scripts are executable
          if [ -f "simulation/make-scripts-executable.sh" ]; then
            chmod +x simulation/make-scripts-executable.sh
            ./simulation/make-scripts-executable.sh
          fi
          
          # Create data directories in the user's home directory to avoid permission issues
          DATA_DIR="$HOME/.almanac"
          PG_DATA_DIR="$DATA_DIR/postgres"
          ROCKS_DATA_DIR="$DATA_DIR/rocksdb"
          
          echo "Using data directory: $DATA_DIR"
          mkdir -p "$PG_DATA_DIR"
          mkdir -p "$ROCKS_DATA_DIR"
          
          # Create helper functions for PostgreSQL
          wait_for_postgres() {
            echo "Waiting for PostgreSQL to start..."
            local max_attempts=30
            local attempt=0
            
            while [ $attempt -lt $max_attempts ]; do
              if pg_isready -h localhost -p 5432 >/dev/null 2>&1; then
                echo "PostgreSQL is ready!"
                return 0
              fi
              
              attempt=$((attempt + 1))
              echo "Waiting for PostgreSQL to start... (attempt $attempt/$max_attempts)"
              sleep 1
            done
            
            echo "PostgreSQL failed to start after $max_attempts attempts."
            return 1
          }
          
          ensure_postgres_user() {
            echo "Ensuring PostgreSQL user 'postgres' exists..."
            local POSTGRES_RUNNING=false
            
            # Check if PostgreSQL is already running
            if pg_isready -h localhost -p 5432 >/dev/null 2>&1; then
              POSTGRES_RUNNING=true
            fi
            
            # Create postgres user if it doesn't exist
            if ! psql -h localhost -p 5432 -U postgres -c '\q' >/dev/null 2>&1; then
              echo "Creating PostgreSQL user 'postgres'..."
              
              if [ -f "$PG_DATA_DIR/postgresql.conf" ]; then
                # PostgreSQL is installed but not running
                pg_ctl -D "$PG_DATA_DIR" start -o "-c listen_addresses='localhost' -c port=5432"
                wait_for_postgres
                
                # Create the postgres user
                createuser -h localhost -p 5432 -s postgres
                
                # If we started PostgreSQL, stop it unless it was already running
                if [ "$POSTGRES_RUNNING" = false ]; then
                  pg_ctl -D "$PG_DATA_DIR" stop
                fi
              else
                echo "PostgreSQL data directory needs to be initialized first"
              fi
            else
              echo "PostgreSQL user 'postgres' already exists"
            fi
          }
          
          # Initialize PostgreSQL
          echo "Initializing PostgreSQL..."
          
          # Check if PostgreSQL is already running
          if pg_isready -h localhost -p 5432 >/dev/null 2>&1; then
            echo "PostgreSQL is already running"
          else
            if [ -f "simulation/databases/setup-postgres.sh" ]; then
              # Save the original setup script with our data path
              TMP_SCRIPT=$(mktemp)
              cat simulation/databases/setup-postgres.sh > "$TMP_SCRIPT"
              sed -i.bak "s|data/postgres|$PG_DATA_DIR|g" "$TMP_SCRIPT"
              chmod +x "$TMP_SCRIPT"
              "$TMP_SCRIPT"
              rm "$TMP_SCRIPT" "$TMP_SCRIPT.bak"
            else
              echo "PostgreSQL setup script not found. Attempting direct setup..."
              
              # Initialize PostgreSQL data directory if needed
              if [ ! -d "$PG_DATA_DIR/base" ]; then
                echo "Initializing PostgreSQL data directory..."
                initdb -D "$PG_DATA_DIR" --no-locale --encoding=UTF8
              fi
              
              # Start PostgreSQL server
              echo "Starting PostgreSQL..."
              pg_ctl -D "$PG_DATA_DIR" -l "$PG_DATA_DIR/logfile" start -o "-c listen_addresses='localhost' -c port=5432"
              
              # Wait for PostgreSQL to start
              wait_for_postgres
            fi
          fi
          
          # Ensure postgres user exists
          ensure_postgres_user
          
          # Create a script to stop PostgreSQL
          mkdir -p "$PG_DATA_DIR"
          cat > "$PG_DATA_DIR/stop-postgres.sh" << EOF
          #!/bin/bash
          pg_ctl -D "$PG_DATA_DIR" stop
          EOF
          chmod +x "$PG_DATA_DIR/stop-postgres.sh"
          
          # Create the development database if it doesn't exist
          echo "Creating development database 'indexer'..."
          if ! psql -h localhost -p 5432 -U postgres -lqt | cut -d \| -f 1 | grep -qw indexer; then
            createdb -h localhost -p 5432 -U postgres indexer
          else
            echo "Development database 'indexer' already exists"
          fi
          
          # Initialize RocksDB storage
          echo "Initializing RocksDB storage..."
          if [ -f "simulation/databases/setup-rocksdb.sh" ]; then
            # Save the original setup script with our data path
            TMP_SCRIPT=$(mktemp)
            cat simulation/databases/setup-rocksdb.sh > "$TMP_SCRIPT"
            sed -i.bak "s|data/rocksdb|$ROCKS_DATA_DIR|g" "$TMP_SCRIPT"
            chmod +x "$TMP_SCRIPT"
            "$TMP_SCRIPT"
            rm "$TMP_SCRIPT" "$TMP_SCRIPT.bak"
          else
            echo "RocksDB setup script not found. Creating RocksDB directory..."
            mkdir -p "$ROCKS_DATA_DIR"
          fi
          
          # Export the data directories so they can be used by the scripts
          export ALMANAC_PG_DATA_DIR="$PG_DATA_DIR"
          export ALMANAC_ROCKS_DATA_DIR="$ROCKS_DATA_DIR"
          
          # Save environment variables to a file for easy sourcing
          echo "Saving environment variables to $DATA_DIR/.db_env..."
          mkdir -p "$DATA_DIR"
          cat > "$DATA_DIR/.db_env" << EOF
          export POSTGRES_URL=postgresql://postgres:postgres@localhost:5432/indexer
          export POSTGRES_USER=postgres
          export POSTGRES_PASSWORD=postgres
          export POSTGRES_DB=indexer
          export POSTGRES_HOST=localhost
          export POSTGRES_PORT=5432
          export ROCKSDB_PATH=$ROCKS_DATA_DIR
          export ALMANAC_PG_DATA_DIR=$PG_DATA_DIR
          export ALMANAC_ROCKS_DATA_DIR=$ROCKS_DATA_DIR
          EOF
          
          # Also create a symlink in the project directory if possible
          if [ -w "." ]; then
            echo "Creating .db_env in project directory"
            cat > ".db_env" << EOF
          export POSTGRES_URL=postgresql://postgres:postgres@localhost:5432/indexer
          export POSTGRES_USER=postgres
          export POSTGRES_PASSWORD=postgres
          export POSTGRES_DB=indexer
          export POSTGRES_HOST=localhost
          export POSTGRES_PORT=5432
          export ROCKSDB_PATH=$ROCKS_DATA_DIR
          export ALMANAC_PG_DATA_DIR=$PG_DATA_DIR
          export ALMANAC_ROCKS_DATA_DIR=$ROCKS_DATA_DIR
          EOF
          fi
          
          echo "Database initialization completed successfully!"
          echo "PostgreSQL running at localhost:5432"
          echo "Database: indexer"
          echo "Connection URL: postgresql://postgres:postgres@localhost:5432/indexer"
          echo "PostgreSQL data directory: $PG_DATA_DIR"
          echo "RocksDB data directory: $ROCKS_DATA_DIR"
          echo ""
          echo "Environment variables saved to $DATA_DIR/.db_env"
          echo "Source with: source $DATA_DIR/.db_env"
          
          # Return the data directories for use by other scripts
          echo "$DATA_DIR"
        '';
        
        # Create simple shell script wrappers for various workflows
        
        # Stop-all script to properly stop all services
        stop-all-script = let
          postgresql = pkgs.postgresql;
        in pkgs.writeShellScriptBin "stop-all" ''
          #!/usr/bin/env bash
          # stop-all.sh - Script to stop all running services in the Almanac workflow
          
          set -e
          
          echo "Stopping all running services..."
          
          # Add PostgreSQL to the PATH
          export PATH="${postgresql}/bin:$PATH"
          
          # Stop Anvil node
          if pgrep -f "anvil.*--port 8545" > /dev/null; then
            echo "Stopping Anvil node..."
            pkill -f "anvil.*--port 8545" || echo "Failed to stop Anvil"
            if [ -f "/tmp/anvil.pid" ]; then
              ANVIL_PID=$(cat /tmp/anvil.pid)
              kill $ANVIL_PID 2>/dev/null || echo "Failed to stop Anvil with PID $ANVIL_PID"
              rm -f /tmp/anvil.pid
            fi
          else
            echo "No Anvil node found running"
          fi
          
          # Stop Reth node
          if pgrep -f "reth" > /dev/null; then
            echo "Stopping Reth node..."
            pkill -f "reth" || echo "Failed to stop Reth"
            if [ -f "/tmp/reth-almanac.pid" ]; then
              RETH_PID=$(cat /tmp/reth-almanac.pid)
              kill $RETH_PID 2>/dev/null || echo "Failed to stop Reth with PID $RETH_PID"
              rm -f /tmp/reth-almanac.pid
            fi
          fi
          
          # Stop mock-reth if running
          if pgrep -f "mock-reth" > /dev/null; then
            echo "Stopping mock-reth server..."
            pkill -f "mock-reth" || echo "Failed to stop mock-reth"
            if [ -f "/tmp/mock-reth.pid" ]; then
              MOCK_RETH_PID=$(cat /tmp/mock-reth.pid)
              kill $MOCK_RETH_PID 2>/dev/null || echo "Failed to stop mock-reth with PID $MOCK_RETH_PID"
              rm -f /tmp/mock-reth.pid
            fi
          fi
          
          # Stop wasmd node
          if pgrep -f "wasmd start" > /dev/null; then
            echo "Stopping wasmd node..."
            pkill -f "wasmd start" || echo "Failed to stop wasmd"
            if [ -f "/tmp/wasmd-almanac.pid" ]; then
              WASMD_PID=$(cat /tmp/wasmd-almanac.pid)
              kill $WASMD_PID 2>/dev/null || echo "Failed to stop wasmd with PID $WASMD_PID"
              rm -f /tmp/wasmd-almanac.pid
            fi
          else
            echo "No wasmd node found running"
          fi
          
          # Stop mock-wasmd if running
          if pgrep -f "mock-wasmd" > /dev/null; then
            echo "Stopping mock-wasmd server..."
            pkill -f "mock-wasmd" || echo "Failed to stop mock-wasmd"
            if [ -f "/tmp/mock-wasmd.pid" ]; then
              MOCK_WASMD_PID=$(cat /tmp/mock-wasmd.pid)
              kill $MOCK_WASMD_PID 2>/dev/null || echo "Failed to stop mock-wasmd with PID $MOCK_WASMD_PID"
              rm -f /tmp/mock-wasmd.pid
            fi
          fi
          
          # Stop PostgreSQL
          DATA_DIR="$HOME/.almanac"
          if [ -f "$DATA_DIR/postgres/stop-postgres.sh" ]; then
            echo "Stopping PostgreSQL server..."
            "$DATA_DIR/postgres/stop-postgres.sh" 2>/dev/null || echo "PostgreSQL was not running or failed to stop"
          else
            pg_ctl -D "$DATA_DIR/postgres" stop 2>/dev/null || echo "PostgreSQL was not running or failed to stop"
          fi
          
          # Stop any Almanac indexers
          if pgrep -f "almanac-indexer" > /dev/null; then
            echo "Stopping Almanac indexers..."
            pkill -f "almanac-indexer" || echo "Failed to stop almanac-indexer"
          fi
          
          echo "All services stopped successfully"
        '';
        
        # Setup Reth script
        setup-reth-script = pkgs.writeShellScriptBin "setup-reth" ''
          #!/usr/bin/env bash
          # setup-reth - Script to set up a Reth Ethereum node
          
          set -e
          
          echo "Setting up Reth node..."
          MOCK_MODE=false
          
          # Check if reth is available
          if command -v reth >/dev/null 2>&1; then
            echo "Found reth command in PATH"
            RETH_CMD="reth"
          elif [ -f "target/release/reth" ] && [ -x "target/release/reth" ]; then
            echo "Found locally built reth at target/release/reth"
            RETH_CMD="$(pwd)/target/release/reth"
          else
            echo "Reth binary not found, setting up mock mode"
            MOCK_MODE=true
            
            # Create a simple mock-reth server that listens on port 8545 and responds to JSON-RPC calls
            cat > /tmp/mock-reth.sh << 'EOF'
          #!/usr/bin/env bash
          
          echo "Starting mock-reth server on port 8545..."
          echo $$ > /tmp/mock-reth.pid
          
          # Create directories
          mkdir -p data/ethereum/reth
          
          # Simple JSON-RPC response function
          handle_request() {
            local request="$1"
            if echo "$request" | grep -q "eth_chainId"; then
              echo '{"jsonrpc":"2.0","id":1,"result":"0x539"}'
            elif echo "$request" | grep -q "eth_blockNumber"; then
              echo '{"jsonrpc":"2.0","id":1,"result":"0x1"}'
            else
              echo '{"jsonrpc":"2.0","id":1,"result":"0x0"}'
            fi
          }
          
          # Start a simple server on port 8545
          while true; do
            response=$(nc -l -p 8545 -c 'read request; echo "HTTP/1.1 200 OK"; echo "Content-Type: application/json"; echo ""; handle_request "$request"')
            # Sleep to prevent high CPU usage
            sleep 0.1
          done
          EOF
          
            chmod +x /tmp/mock-reth.sh
            RETH_CMD="/tmp/mock-reth.sh"
          fi
          
          # Set up Reth data directories
          mkdir -p data/ethereum/reth
          
          # Start Reth based on mode
          if [ "$MOCK_MODE" = true ]; then
            echo "Starting mock Reth server..."
            $RETH_CMD > logs/reth.log 2>&1 &
            RETH_PID=$!
            echo $RETH_PID > /tmp/reth-almanac.pid
            echo "Mock Reth started with PID: $RETH_PID"
            
            # Wait for mock server to be ready
            echo "Waiting for mock Reth server to start..."
            sleep 2
          else
            echo "Initializing Reth node..."
            $RETH_CMD init --datadir data/ethereum/reth --dev
            
            echo "Starting Reth node..."
            $RETH_CMD --datadir data/ethereum/reth --dev --http --http.addr 0.0.0.0 --http.port 8545 > logs/reth.log 2>&1 &
            RETH_PID=$!
            echo $RETH_PID > /tmp/reth-almanac.pid
            echo "Reth started with PID: $RETH_PID"
            
            # Wait for Reth to start
            echo "Waiting for Reth to start..."
            sleep 5
          fi
          
          # Verify the node is running by querying for chain ID
          echo "Verifying Reth node is running..."
          RETRY_COUNT=0
          MAX_RETRIES=5
          
          while [ $RETRY_COUNT -lt $MAX_RETRIES ]; do
            CHAIN_ID=$(curl -s -X POST -H "Content-Type: application/json" --data '{"jsonrpc":"2.0","method":"eth_chainId","params":[],"id":1}' http://localhost:8545 | grep -o '"result":"[^"]*"' | cut -d'"' -f4)
            
            if [ -n "$CHAIN_ID" ]; then
              echo "✅ Reth node is running with chain ID: $CHAIN_ID"
              break
            else
              echo "Waiting for Reth node to start... (attempt $((RETRY_COUNT+1))/$MAX_RETRIES)"
              RETRY_COUNT=$((RETRY_COUNT+1))
              sleep 2
            fi
          done
          
          if [ $RETRY_COUNT -eq $MAX_RETRIES ]; then
            echo "ERROR: Reth node failed to start or is not responding to RPC calls"
            echo "Check logs/reth.log for details"
            exit 1
          fi
          
          echo "Reth setup completed successfully!"
        '';
        
        # Setup wasmd script
        setup-wasmd-script = pkgs.writeShellScriptBin "setup-wasmd" ''
          #!/usr/bin/env bash
          # setup-wasmd - Script to set up a CosmWasm node
          
          set -e
          
          echo "Setting up wasmd node..."
          MOCK_MODE=false
          
          # Check if wasmd is available
          if command -v wasmd >/dev/null 2>&1; then
            echo "Found wasmd command in PATH"
            WASMD_CMD="wasmd"
          else
            echo "wasmd binary not found, setting up mock mode"
            MOCK_MODE=true
            
            # Create a simple mock-wasmd script
            cat > /tmp/mock-wasmd.sh << 'EOF'
#!/usr/bin/env bash

COMMAND="$1"
HOME_DIR=''${3:-"$HOME/.wasmd-test"}

case "$COMMAND" in
  version)
    echo "mock-wasmd v0.30.0"
    ;;
  init)
    echo "Initializing mock wasmd at $HOME_DIR..."
    mkdir -p "$HOME_DIR"
    mkdir -p "$HOME_DIR/config"
    # Create a simple genesis file
    echo '{"app_state":{"wasm":{"params":{"code_upload_access":{"permission":"Everybody","address":""},"instantiate_default_permission":"Everybody"}}},"chain_id":"wasmchain","genesis_time":"2023-01-01T00:00:00Z"}' > "$HOME_DIR/config/genesis.json"
    ;;
  keys)
    SUBCOMMAND="$2"
    if [ "$SUBCOMMAND" = "add" ]; then
      echo "Adding mock key: ''${4:-validator}"
      mkdir -p "$HOME_DIR/keyring-test"
      echo '{"name":"validator","type":"local","address":"cosmos14lultfckehtszvzw4ehu0apvsr77afvygyt6kx","pubkey":"cosmospub1..."}' > "$HOME_DIR/keyring-test/validator.info"
    fi
    ;;
  add-genesis-account)
    echo "Adding genesis account: $2 with $3"
    # Update the mock genesis file
    ;;
  gentx)
    echo "Generating genesis transaction..."
    mkdir -p "$HOME_DIR/config/gentx"
    echo '{"body":{"messages":[{"@type":"/cosmos.staking.v1beta1.MsgCreateValidator"}]}}' > "$HOME_DIR/config/gentx/gentx.json"
    ;;
  collect-gentxs)
    echo "Collecting genesis transactions..."
    ;;
  start)
    echo "Starting mock wasmd node..."
    echo $$ > /tmp/wasmd-almanac.pid
    # Create a simple HTTP server that responds to RPC requests
    mkdir -p /tmp/wasmd-status
    echo "Running" > /tmp/wasmd-status/status
    
    # This is just a placeholder - in a real implementation, we'd want to mock
    # the CosmWasm RPC endpoint to respond to queries
    while true; do
      sleep 10
      if [ ! -f "/tmp/wasmd-status/status" ]; then
        echo "Shutdown signal received"
        break
      fi
    done
    ;;
  *)
    echo "Unknown command: $COMMAND"
    exit 1
    ;;
esac
EOF
            
            chmod +x /tmp/mock-wasmd.sh
            WASMD_CMD="/tmp/mock-wasmd.sh"
          fi
          
          # Set up wasmd
          HOME_DIR="$HOME/.wasmd-test"
          CHAIN_ID="wasmchain"
          VALIDATOR_NAME="validator"
          VALIDATOR_MONIKER="wasmd-almanac"
          
          # Initialize wasmd
          echo "Initializing wasmd..."
          $WASMD_CMD init $VALIDATOR_MONIKER --chain-id=$CHAIN_ID --home=$HOME_DIR
          
          # Create validator key
          echo "Creating validator key..."
          $WASMD_CMD keys add $VALIDATOR_NAME --keyring-backend=test --home=$HOME_DIR
          
          # Add genesis account
          echo "Adding genesis account..."
          $WASMD_CMD add-genesis-account $VALIDATOR_NAME 10000000000stake --keyring-backend=test --home=$HOME_DIR
          
          # Generate a genesis transaction
          echo "Generating genesis transaction..."
          $WASMD_CMD gentx $VALIDATOR_NAME 1000000stake --chain-id=$CHAIN_ID --keyring-backend=test --home=$HOME_DIR
          
          # Collect genesis transactions
          echo "Collecting genesis transactions..."
          $WASMD_CMD collect-gentxs --home=$HOME_DIR
          
          # Update genesis parameters for CosmWasm
          if [ "$MOCK_MODE" = false ]; then
            echo "Updating genesis parameters for CosmWasm..."
            sed -i.bak 's/"code_upload_access": {.*}/"code_upload_access": {"permission": "Everybody", "address": ""}/g' $HOME_DIR/config/genesis.json
            sed -i.bak 's/"instantiate_default_permission": ".*"/"instantiate_default_permission": "Everybody"/g' $HOME_DIR/config/genesis.json
          fi
          
          # Start wasmd node
          echo "Starting wasmd node..."
          mkdir -p logs
          if [ "$MOCK_MODE" = true ]; then
            $WASMD_CMD start --home=$HOME_DIR > logs/wasmd.log 2>&1 &
          else
            $WASMD_CMD start --rpc.laddr tcp://0.0.0.0:26657 --home=$HOME_DIR > logs/wasmd.log 2>&1 &
          fi
          WASMD_PID=$!
          echo $WASMD_PID > /tmp/wasmd-almanac.pid
          
          echo "wasmd started with PID: $WASMD_PID"
          
          # Verify the node is running
          echo "Verifying wasmd node is running..."
          RETRY_COUNT=0
          MAX_RETRIES=10
          
          while [ $RETRY_COUNT -lt $MAX_RETRIES ]; do
            if [ "$MOCK_MODE" = true ]; then
              # For mock mode, we can just check if the process is running
              if ps -p $WASMD_PID > /dev/null; then
                echo "✅ Mock wasmd node is running"
                break
              fi
            else
              # For real mode, check if the RPC endpoint is responding
              if curl -s http://localhost:26657/status > /dev/null 2>&1; then
                echo "✅ wasmd node is running"
                break
              fi
            fi
            
            echo "Waiting for wasmd node to start... (attempt $((RETRY_COUNT+1))/$MAX_RETRIES)"
            RETRY_COUNT=$((RETRY_COUNT+1))
            sleep 2
          done
          
          if [ $RETRY_COUNT -eq $MAX_RETRIES ]; then
            echo "WARNING: wasmd node may not be running properly"
            echo "Check logs/wasmd.log for details"
          else
            echo "wasmd setup completed successfully!"
          fi
        '';
        
        # Setup Anvil script
        setup-anvil-script = pkgs.writeShellScriptBin "setup-anvil" ''
          #!/usr/bin/env bash
          # setup-anvil - Script to set up an Anvil Ethereum node
          
          set -e
          
          echo "Setting up Anvil node..."
          
          # Check if anvil is available
          if ! command -v anvil >/dev/null 2>&1; then
            echo "Error: anvil command not found"
            exit 1
          fi
          
          # Create logs directory if it doesn't exist
          mkdir -p logs
          
          # Start Anvil
          echo "Starting Anvil node..."
          anvil --port 8545 --block-time 1 > logs/anvil.log 2>&1 &
          ANVIL_PID=$!
          echo "Anvil started with PID: $ANVIL_PID"
          
          # Save PID for later cleanup
          echo "$ANVIL_PID" > /tmp/anvil.pid
          
          # Verify Anvil is running
          echo "Verifying Anvil node is running..."
          RETRY_COUNT=0
          MAX_RETRIES=5
          
          while [ $RETRY_COUNT -lt $MAX_RETRIES ]; do
            if curl -s -X POST -H "Content-Type: application/json" --data '{"jsonrpc":"2.0","method":"eth_chainId","params":[],"id":1}' http://localhost:8545 | grep -q "result"; then
              CHAIN_ID=$(curl -s -X POST -H "Content-Type: application/json" --data '{"jsonrpc":"2.0","method":"eth_chainId","params":[],"id":1}' http://localhost:8545 | grep -o '"result":"[^"]*"' | cut -d'"' -f4)
              echo "✅ Anvil node is running with chain ID: $CHAIN_ID"
              break
            else
              echo "Waiting for Anvil node to start... (attempt $((RETRY_COUNT+1))/$MAX_RETRIES)"
              RETRY_COUNT=$((RETRY_COUNT+1))
              sleep 2
            fi
          done
          
          if [ $RETRY_COUNT -eq $MAX_RETRIES ]; then
            echo "ERROR: Anvil node failed to start or is not responding to RPC calls"
            echo "Check logs/anvil.log for details"
            exit 1
          fi
          
          echo "Anvil setup completed successfully!"
        '';
        
        # CosmWasm workflow wrapper
        cosmwasm-wrapper = let
          postgresql = pkgs.postgresql;
        in pkgs.writeShellScriptBin "cosmwasm-workflow" ''
          #!/usr/bin/env bash
          set -e
          
          # Add PostgreSQL to the PATH
          export PATH="${postgresql}/bin:$PATH"
          
          # Set deployment target for macOS compatibility
          export MACOSX_DEPLOYMENT_TARGET="${darwinDeploymentTarget}"
          
          # Change to repo root
          REPO_ROOT=$(dirname $(dirname $(dirname $0)))
          cd "$REPO_ROOT"
          
          # Execute workflow steps
          echo "=== Running CosmWasm Workflow ==="
          
          # Stop existing services
          echo "Stopping any running services..."
          ${stop-all-script}/bin/stop-all
          
          # Initialize databases
          echo "Initializing databases..."
          DATA_DIR=$(${init-databases-script}/bin/init-databases)
          
          # Source database environment variables
          if [ -f "$DATA_DIR/.db_env" ]; then
            source "$DATA_DIR/.db_env"
          fi
          
          # Setup wasmd using our integrated script
          echo "Setting up wasmd..."
          mkdir -p logs
          ${setup-wasmd-script}/bin/setup-wasmd
          
          echo "=== CosmWasm Workflow Completed ==="
          echo "wasmd is running in the background"
          echo "To stop services, run: nix run ./nix/environments#stop-all"
        '';
        
        # Reth workflow wrapper
        reth-wrapper = let
          postgresql = pkgs.postgresql;
        in pkgs.writeShellScriptBin "reth-workflow" ''
          #!/usr/bin/env bash
          set -e
          
          # Add PostgreSQL to the PATH
          export PATH="${postgresql}/bin:$PATH"
          
          # Set deployment target for macOS compatibility
          export MACOSX_DEPLOYMENT_TARGET="${darwinDeploymentTarget}"
          
          # Change to repo root
          REPO_ROOT=$(dirname $(dirname $(dirname $0)))
          cd "$REPO_ROOT"
          
          # Execute workflow steps
          echo "=== Running Reth Workflow ==="
          
          # Stop existing services
          echo "Stopping any running services..."
          ${stop-all-script}/bin/stop-all
          
          # Initialize databases
          echo "Initializing databases..."
          DATA_DIR=$(${init-databases-script}/bin/init-databases)
          
          # Source database environment variables
          if [ -f "$DATA_DIR/.db_env" ]; then
            source "$DATA_DIR/.db_env"
          fi
          
          # Setup Reth using our integrated script
          echo "Setting up Reth..."
          mkdir -p logs
          ${setup-reth-script}/bin/setup-reth
          
          echo "=== Reth Workflow Completed ==="
          echo "Reth is running in the background"
          echo "To stop services, run: nix run ./nix/environments#stop-all"
        '';
        
        # Anvil workflow wrapper
        anvil-wrapper = let
          postgresql = pkgs.postgresql;
        in pkgs.writeShellScriptBin "anvil-workflow" ''
          #!/usr/bin/env bash
          set -e
          
          # Add PostgreSQL to the PATH
          export PATH="${postgresql}/bin:$PATH"
          
          # Set deployment target for macOS compatibility
          export MACOSX_DEPLOYMENT_TARGET="${darwinDeploymentTarget}"
          
          # Change to repo root
          REPO_ROOT=$(dirname $(dirname $(dirname $0)))
          cd "$REPO_ROOT"
          
          # Execute workflow steps
          echo "=== Running Anvil Workflow ==="
          
          # Stop existing services
          echo "Stopping any running services..."
          ${stop-all-script}/bin/stop-all
          
          # Initialize databases and get the data directory
          echo "Initializing databases..."
          DATA_DIR=$(${init-databases-script}/bin/init-databases)
          echo "Using data directory: $DATA_DIR"
          
          # Source environment variables
          if [ -f "$DATA_DIR/.db_env" ]; then
            source "$DATA_DIR/.db_env"
          fi
          
          # Set up Anvil node using our integrated script
          echo "Setting up Anvil node..."
          mkdir -p logs
          ${setup-anvil-script}/bin/setup-anvil
          
          # Verify Anvil is running
          echo "Verifying Anvil node is running..."
          RETRY_COUNT=0
          MAX_RETRIES=5
          while [ $RETRY_COUNT -lt $MAX_RETRIES ]; do
            if curl -s -X POST -H "Content-Type: application/json" --data '{"jsonrpc":"2.0","method":"eth_chainId","params":[],"id":1}' http://localhost:8545 | grep -q "result"; then
              echo "✅ Anvil node is running and responding to RPC calls"
              break
            else
              echo "Waiting for Anvil node to start... (attempt $((RETRY_COUNT+1))/$MAX_RETRIES)"
              RETRY_COUNT=$((RETRY_COUNT+1))
              sleep 2
            fi
          done
          
          if [ $RETRY_COUNT -eq $MAX_RETRIES ]; then
            echo "ERROR: Anvil node failed to start or is not responding to RPC calls"
            echo "Check logs/anvil.log for details"
            exit 1
          fi
          
          # Deploy contracts if available
          if [ -d "contracts/solidity" ] && [ -f "scripts/deploy-contracts.sh" ]; then
            echo "Deploying contracts..."
            chmod +x scripts/deploy-contracts.sh
            ./scripts/deploy-contracts.sh
          fi
          
          echo "Anvil workflow completed successfully! Ethereum node is running."
          echo "To stop services, run: nix run ./nix/environments#stop-all"
        '';
        
        # Fix the all-workflows script to properly run sequential workflows
        all-workflows-wrapper = let
          postgresql = pkgs.postgresql;
        in pkgs.writeShellScriptBin "all-workflows" ''
          #!/usr/bin/env bash
          set -e
          
          # Add PostgreSQL to the PATH
          export PATH="${postgresql}/bin:$PATH"
          
          # Set deployment target for macOS compatibility
          export MACOSX_DEPLOYMENT_TARGET="${darwinDeploymentTarget}"
          
          # Change to repo root
          REPO_ROOT=$(dirname $(dirname $(dirname $0)))
          cd "$REPO_ROOT"
          
          # Execute workflow steps
          echo "=== Running All Workflows ==="
          
          # Stop existing services
          echo "Stopping any running services..."
          ${stop-all-script}/bin/stop-all
          
          # Initialize databases
          echo "Initializing databases..."
          DATA_DIR=$(${init-databases-script}/bin/init-databases)
          
          # Source database environment variables
          if [ -f "$DATA_DIR/.db_env" ]; then
            source "$DATA_DIR/.db_env"
          fi
          
          # Setup Anvil
          echo "Setting up Anvil..."
          mkdir -p logs
          ${setup-anvil-script}/bin/setup-anvil
          
          # Setup Reth
          echo "Setting up Reth..."
          ${setup-reth-script}/bin/setup-reth
          
          # Setup wasmd
          echo "Setting up wasmd..."
          ${setup-wasmd-script}/bin/setup-wasmd
          
          echo "=== All Workflows Completed ==="
          echo "All services are running in the background"
          echo "To stop services, run: nix run ./nix/environments#stop-all"
        '';
        
        # Create a simple workflow menu script
        workflow-menu-script = let
          postgresql = pkgs.postgresql;
        in pkgs.writeShellScriptBin "workflow-menu" ''
          #!/usr/bin/env bash
          
          # Add PostgreSQL to the PATH
          export PATH="${postgresql}/bin:$PATH"
          
          # Set deployment target for macOS compatibility
          export MACOSX_DEPLOYMENT_TARGET="${darwinDeploymentTarget}"
          
          # Change to repo root
          REPO_ROOT=$(dirname $(dirname $(dirname $0)))
          cd "$REPO_ROOT"
          
          # Display menu
          echo "=== Almanac Workflow Menu ==="
          echo "1) Anvil Workflow"
          echo "2) Reth Workflow"
          echo "3) CosmWasm Workflow"
          echo "4) Run All Workflows"
          echo "0) Exit"
          echo ""
          echo -n "Select an option: "
          
          # Read user input
          read CHOICE
          
          # Process user selection
          case $CHOICE in
            1)
              echo "Starting Anvil Workflow..."
              ${pkgs.lib.getExe anvil-wrapper}
              ;;
            2)
              echo "Starting Reth Workflow..."
              ${pkgs.lib.getExe reth-wrapper}
              ;;
            3)
              echo "Starting CosmWasm Workflow..."
              ${pkgs.lib.getExe cosmwasm-wrapper}
              ;;
            4)
              echo "Running All Workflows..."
              ${pkgs.lib.getExe all-workflows-wrapper}
              ;;
            0)
              echo "Exiting..."
              exit 0
              ;;
            *)
              echo "Invalid choice. Please select a valid option."
              exit 1
              ;;
          esac
        '';
        
      in {
        packages = {
          default = workflow-menu-script;
          workflow-menu = workflow-menu-script;
          all-workflows = all-workflows-wrapper;
          anvil-workflow = anvil-wrapper;
          reth-workflow = reth-wrapper;
          cosmwasm-workflow = cosmwasm-wrapper;
          init-databases = init-databases-script;
          stop-all = stop-all-script;
          setup-anvil = setup-anvil-script;
          setup-reth = setup-reth-script;
          setup-wasmd = setup-wasmd-script;
        };
        
        apps = {
          default = {
            type = "app";
            program = "${workflow-menu-script}/bin/workflow-menu";
          };
          all = {
            type = "app";
            program = "${all-workflows-wrapper}/bin/all-workflows";
          };
          anvil = {
            type = "app";
            program = "${anvil-wrapper}/bin/anvil-workflow";
          };
          reth = {
            type = "app";
            program = "${reth-wrapper}/bin/reth-workflow";
          };
          cosmwasm = {
            type = "app";
            program = "${cosmwasm-wrapper}/bin/cosmwasm-workflow";
          };
          init-databases = {
            type = "app";
            program = "${init-databases-script}/bin/init-databases";
          };
          stop-all = {
            type = "app";
            program = "${stop-all-script}/bin/stop-all";
          };
          setup-anvil = {
            type = "app";
            program = "${setup-anvil-script}/bin/setup-anvil";
          };
          setup-reth = {
            type = "app";
            program = "${setup-reth-script}/bin/setup-reth";
          };
          setup-wasmd = {
            type = "app";
            program = "${setup-wasmd-script}/bin/setup-wasmd";
          };
        };
        
        # Expose all tools directly in the development shell
        devShells.default = pkgs.mkShell {
          buildInputs = [
            workflow-menu-script
            all-workflows-wrapper
            anvil-wrapper
            reth-wrapper
            cosmwasm-wrapper
            init-databases-script
            stop-all-script
            setup-anvil-script
            setup-reth-script
            setup-wasmd-script
            # Add PostgreSQL and other database tools
            pkgs.postgresql
          ];
          
          shellHook = ''
            echo "=== Almanac Workflow Environment ==="
            echo "Available commands:"
            echo "- workflow-menu: Run the workflow selection menu"
            echo "- all-workflows: Run all workflows sequentially"
            echo ""
            echo "Individual workflows:"
            echo "- anvil-workflow: Run the Anvil workflow"
            echo "- reth-workflow: Run the Reth workflow"
            echo "- cosmwasm-workflow: Run the CosmWasm workflow"
            echo "- init-databases: Initialize PostgreSQL and RocksDB"
            echo "- stop-all: Stop all running services"
            echo ""
            echo "Individual setup scripts:"
            echo "- setup-anvil: Set up Anvil node"
            echo "- setup-reth: Set up Reth node"
            echo "- setup-wasmd: Set up wasmd node"
            echo ""
            echo "To start, run 'workflow-menu'"
          '';
        };
      }
    );
} 