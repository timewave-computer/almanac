{
  description = "Almanac local blockchain simulation environment";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-parts.url = "github:hercules-ci/flake-parts";
    
    # Local blockchain development environments
    fdb = {
      url = "path:../fdb";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    
    ethereum = {
      url = "path:../ethereum";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    
    cosmos = {
      url = "path:../cosmos";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    
    solana = {
      url = "path:../solana";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = inputs @ { flake-parts, ... }:
    flake-parts.lib.mkFlake { inherit inputs; } {
      systems = [ "x86_64-linux" "aarch64-linux" "x86_64-darwin" "aarch64-darwin" ];

      perSystem = { config, self', inputs', pkgs, system, lib, ... }: let
        # Import packages from blockchain subflakes
        fdb-packages = inputs.fdb.packages.${system};
        ethereum-packages = inputs.ethereum.packages.${system};
        cosmos-packages = inputs.cosmos.packages.${system};
        solana-packages = inputs.solana.packages.${system};
        
        # Common environment setup
        commonEnv = {
          PROJECT_ROOT = "$PWD";
          DATA_DIR = "$PWD/data";
          LOGS_DIR = "$PWD/logs";
          CONFIG_DIR = "$PWD/config";
        };

        # Node configuration generator
        nodeConfigGenerator = pkgs.writeShellApplication {
          name = "sim-config";
          runtimeInputs = [ pkgs.jq pkgs.coreutils pkgs.envsubst ];
          text = ''
            # Generate configuration for simulation environment
            
            CHAINS="''${1:-ethereum,cosmos,fdb}"  # Default chains to run
            
            # Set up environment variables
            DATA_DIR="$PWD/data"
            LOGS_DIR="$PWD/logs"
            CONFIG_DIR="$PWD/config"
            
            echo "Generating simulation configuration..."
            echo "Chains: $CHAINS"
            echo "Configuration directory: $CONFIG_DIR"
            
            mkdir -p "$CONFIG_DIR"
            
            # Generate consolidated TOML configuration
            cat > "$CONFIG_DIR/simulation.toml" << EOF
            # Almanac Simulation Configuration
            # Consolidated configuration for local multi-chain simulation environment
            
            [simulation]
            name = "almanac-local-cluster"
            chains = [$(echo "$CHAINS" | sed 's/,/", "/g' | sed 's/^/"/' | sed 's/$/"/')]
            
            [almanac]
            name = "local-simulation"
            api_port = 3000
            api_log_file = "$LOGS_DIR/almanac-api.log"
            
            [storage]
            backend = "foundationdb"
            cluster_file = "$DATA_DIR/fdb.cluster"
            
            [services.foundationdb]
            enabled = $(echo "$CHAINS" | grep -q "fdb" && echo "true" || echo "false")
            cluster_file = "$DATA_DIR/fdb.cluster"
            data_dir = "$DATA_DIR/foundationdb"
            log_dir = "$LOGS_DIR/foundationdb"
            
            [services.ethereum]
            enabled = $(echo "$CHAINS" | grep -q "ethereum" && echo "true" || echo "false")
            node_type = "anvil"
            rpc_port = 8545
            ws_port = 8546
            chain_id = 31337
            data_dir = "$DATA_DIR/ethereum"
            log_file = "$LOGS_DIR/anvil.log"
            
            [services.cosmos]
            enabled = $(echo "$CHAINS" | grep -q "cosmos" && echo "true" || echo "false")
            node_type = "neutron"
            rpc_port = 26657
            grpc_port = 9090
            api_port = 1317
            chain_id = "test-1"
            data_dir = "$DATA_DIR/cosmos"
            log_file = "$LOGS_DIR/neutron.log"
            
            [services.solana]
            enabled = $(echo "$CHAINS" | grep -q "solana" && echo "true" || echo "false")
            node_type = "test-validator"
            rpc_port = 8899
            ws_port = 8900
            data_dir = "$DATA_DIR/solana"
            log_file = "$LOGS_DIR/solana.log"
            EOF
            
            # Add chain-specific configurations to TOML
            if echo "$CHAINS" | grep -q "ethereum"; then
              cat >> "$CONFIG_DIR/simulation.toml" << EOF
            
            [[chains.ethereum]]
            name = "ethereum-local"
            rpc_url = "http://localhost:8545"
            ws_url = "ws://localhost:8546"
            chain_id = 31337
            starting_block = 1
            batch_size = 100
            EOF
            fi
            
            if echo "$CHAINS" | grep -q "cosmos"; then
              cat >> "$CONFIG_DIR/simulation.toml" << EOF
            
            [[chains.cosmos]]
            name = "cosmos-local"
            rpc_url = "http://localhost:26657"
            grpc_url = "http://localhost:9090"
            chain_id = "test-1"
            starting_block = 1
            batch_size = 100
            EOF
            fi
            
            if echo "$CHAINS" | grep -q "solana"; then
              cat >> "$CONFIG_DIR/simulation.toml" << EOF
            
            [[chains.solana]]
            name = "solana-local"
            rpc_url = "http://localhost:8899"
            ws_url = "ws://localhost:8900"
            starting_slot = 1
            batch_size = 100
            EOF
            fi
            
            
            echo "Configuration generated successfully"
            echo "Consolidated TOML configuration: $CONFIG_DIR/simulation.toml"
          '';
        };

        # Individual node launchers
        ethereumLauncher = pkgs.writeShellApplication {
          name = "sim-ethereum";
          runtimeInputs = [ ethereum-packages.forge pkgs.curl pkgs.toml2json pkgs.jq pkgs.envsubst ];
          text = ''
            # Launch Ethereum node (Anvil)
            
            ACTION="''${1:-start}"  # start, stop, status
            CONFIG_FILE="''${2:-./config/simulation.toml}"
            
            if [ ! -f "$CONFIG_FILE" ]; then
              echo "Error: Configuration file not found: $CONFIG_FILE"
              echo "Run 'sim-config' first to generate configuration"
              exit 1
            fi
            
            # Read configuration from TOML
            ETH_ENABLED=$(toml2json < "$CONFIG_FILE" | jq -r '.services.ethereum.enabled')
            RPC_PORT=$(toml2json < "$CONFIG_FILE" | jq -r '.services.ethereum.rpc_port')
            CHAIN_ID=$(toml2json < "$CONFIG_FILE" | jq -r '.services.ethereum.chain_id')
            DATA_DIR=$(toml2json < "$CONFIG_FILE" | jq -r '.services.ethereum.data_dir' | envsubst)
            LOG_FILE=$(toml2json < "$CONFIG_FILE" | jq -r '.services.ethereum.log_file' | envsubst)
            
            if [ "$ETH_ENABLED" != "true" ]; then
              echo "Ethereum not enabled in configuration"
              exit 0
            fi
            
            PID_FILE="/tmp/sim-ethereum.pid"
            
            case "$ACTION" in
              "start")
                echo "Starting Ethereum node (Anvil)..."
                
                # Create directories
                mkdir -p "$(dirname "$DATA_DIR")" "$(dirname "$LOG_FILE")"
                
                # Check if already running
                if [ -f "$PID_FILE" ] && kill -0 "$(cat "$PID_FILE")" 2>/dev/null; then
                  echo "Ethereum node is already running (PID: $(cat "$PID_FILE"))"
                  exit 0
                fi
                
                # Start Anvil
                anvil \
                  --host 0.0.0.0 \
                  --port "$RPC_PORT" \
                  --chain-id "$CHAIN_ID" \
                  --accounts 10 \
                  --balance 10000 \
                  --gas-limit 30000000 \
                  --gas-price 1 \
                  > "$LOG_FILE" 2>&1 &
                
                echo $! > "$PID_FILE"
                echo "Ethereum node started (PID: $!)"
                echo "RPC: http://localhost:$RPC_PORT"
                echo "Logs: $LOG_FILE"
                
                # Wait for node to be ready
                echo "Waiting for Ethereum node to be ready..."
                for ((i=1; i<=30; i++)); do
                  if curl -s -X POST -H "Content-Type: application/json" \
                     --data '{"jsonrpc":"2.0","method":"eth_chainId","params":[],"id":1}' \
                     "http://localhost:$RPC_PORT" > /dev/null; then
                    echo "Ethereum node is ready"
                    break
                  fi
                  sleep 1
                done
                ;;
                
              "stop")
                echo "Stopping Ethereum node..."
                if [ -f "$PID_FILE" ]; then
                  kill "$(cat "$PID_FILE")" 2>/dev/null || true
                  rm -f "$PID_FILE"
                  echo "Ethereum node stopped"
                else
                  echo "Ethereum node not running"
                fi
                ;;
                
              "status")
                if [ -f "$PID_FILE" ] && kill -0 "$(cat "$PID_FILE")" 2>/dev/null; then
                  echo "Ethereum node is running (PID: $(cat "$PID_FILE"))"
                  # Test RPC
                  if curl -s -X POST -H "Content-Type: application/json" \
                     --data '{"jsonrpc":"2.0","method":"eth_chainId","params":[],"id":1}' \
                     "http://localhost:$RPC_PORT" > /dev/null; then
                    echo "RPC is responding"
                  else
                    echo "RPC not responding"
                  fi
                else
                  echo "Ethereum node is not running"
                fi
                ;;
                
              *)
                echo "Usage: sim-ethereum {start|stop|status} [config_file]"
                exit 1
                ;;
            esac
          '';
        };

        cosmosLauncher = pkgs.writeShellApplication {
          name = "sim-cosmos";
          runtimeInputs = [ cosmos-packages.neutron pkgs.curl pkgs.toml2json pkgs.jq pkgs.envsubst ];
          text = ''
            # Launch Cosmos node (Neutron)
            
            ACTION="''${1:-start}"  # start, stop, status  
            CONFIG_FILE="''${2:-./config/simulation.toml}"
            
            if [ ! -f "$CONFIG_FILE" ]; then
              echo "Error: Configuration file not found: $CONFIG_FILE"
              echo "Run 'sim-config' first to generate configuration"
              exit 1
            fi
            
            # Read configuration from TOML
            COSMOS_ENABLED=$(toml2json < "$CONFIG_FILE" | jq -r '.services.cosmos.enabled')
            RPC_PORT=$(toml2json < "$CONFIG_FILE" | jq -r '.services.cosmos.rpc_port')
            GRPC_PORT=$(toml2json < "$CONFIG_FILE" | jq -r '.services.cosmos.grpc_port')
            API_PORT=$(toml2json < "$CONFIG_FILE" | jq -r '.services.cosmos.api_port')
            CHAIN_ID=$(toml2json < "$CONFIG_FILE" | jq -r '.services.cosmos.chain_id')
            DATA_DIR=$(toml2json < "$CONFIG_FILE" | jq -r '.services.cosmos.data_dir' | envsubst)
            LOG_FILE=$(toml2json < "$CONFIG_FILE" | jq -r '.services.cosmos.log_file' | envsubst)
            
            if [ "$COSMOS_ENABLED" != "true" ]; then
              echo "Cosmos not enabled in configuration"
              exit 0
            fi
            
            PID_FILE="/tmp/sim-cosmos.pid"
            
            case "$ACTION" in
              "start")
                echo "Starting Cosmos node (Neutron)..."
                
                # Create directories
                mkdir -p "$DATA_DIR" "$(dirname "$LOG_FILE")"
                
                # Check if already running
                if [ -f "$PID_FILE" ] && kill -0 "$(cat "$PID_FILE")" 2>/dev/null; then
                  echo "Cosmos node is already running (PID: $(cat "$PID_FILE"))"
                  exit 0
                fi
                
                # Initialize chain if not exists
                if [ ! -f "$DATA_DIR/config/genesis.json" ]; then
                  echo "Initializing Cosmos chain..."
                  neutron init "test-node" --chain-id "$CHAIN_ID" --home "$DATA_DIR"
                  
                  # Add a validator key
                  neutron keys add validator --keyring-backend test --home "$DATA_DIR"
                  
                  # Add genesis account
                  neutron genesis add-genesis-account validator 1000000000untrn --keyring-backend test --home "$DATA_DIR"
                  
                  # Create genesis transaction
                  neutron genesis gentx validator 1000000untrn --keyring-backend test --home "$DATA_DIR" --chain-id "$CHAIN_ID"
                  
                  # Collect genesis transactions
                  neutron genesis collect-gentxs --home "$DATA_DIR"
                  
                  # Update config for local development
                  sed -i.bak "s/minimum-gas-prices = \"\"/minimum-gas-prices = \"0untrn\"/" "$DATA_DIR/config/app.toml"
                fi
                
                # Start Neutron
                neutron start \
                  --home "$DATA_DIR" \
                  --rpc.laddr "tcp://0.0.0.0:$RPC_PORT" \
                  --grpc.address "0.0.0.0:$GRPC_PORT" \
                  --api.address "tcp://0.0.0.0:$API_PORT" \
                  --api.enable \
                  > "$LOG_FILE" 2>&1 &
                
                echo $! > "$PID_FILE"
                echo "Cosmos node started (PID: $!)"
                echo "RPC: http://localhost:$RPC_PORT"
                echo "gRPC: http://localhost:$GRPC_PORT"
                echo "API: http://localhost:$API_PORT"
                echo "Logs: $LOG_FILE"
                
                # Wait for node to be ready
                echo "Waiting for Cosmos node to be ready..."
                for ((i=1; i<=60; i++)); do
                  if curl -s "http://localhost:$RPC_PORT/status" > /dev/null; then
                    echo "Cosmos node is ready"
                    break
                  fi
                  sleep 2
                done
                ;;
                
              "stop")
                echo "Stopping Cosmos node..."
                if [ -f "$PID_FILE" ]; then
                  kill "$(cat "$PID_FILE")" 2>/dev/null || true
                  rm -f "$PID_FILE"
                  echo "Cosmos node stopped"
                else
                  echo "Cosmos node not running"
                fi
                ;;
                
              "status")
                if [ -f "$PID_FILE" ] && kill -0 "$(cat "$PID_FILE")" 2>/dev/null; then
                  echo "Cosmos node is running (PID: $(cat "$PID_FILE"))"
                  # Test RPC
                  if curl -s "http://localhost:$RPC_PORT/status" > /dev/null; then
                    echo "RPC is responding"
                  else
                    echo "RPC not responding"
                  fi
                else
                  echo "Cosmos node is not running"
                fi
                ;;
                
              *)
                echo "Usage: sim-cosmos {start|stop|status} [config_file]"
                exit 1
                ;;
            esac
          '';
        };

        # Platform-specific Solana launcher
        solanaLauncher = if (pkgs.stdenv.system != "aarch64-linux") then
          pkgs.writeShellApplication {
            name = "sim-solana";
            runtimeInputs = [ solana-packages.solana-tools pkgs.curl pkgs.toml2json pkgs.jq pkgs.envsubst ];
            text = ''
              # Launch Solana node (Test Validator)
              
              ACTION="''${1:-start}"  # start, stop, status
              CONFIG_FILE="''${2:-./config/simulation.toml}"
              
              if [ ! -f "$CONFIG_FILE" ]; then
                echo "Error: Configuration file not found: $CONFIG_FILE"
                echo "Run 'sim-config' first to generate configuration"
                exit 1
              fi
              
              # Read configuration from TOML
              SOLANA_ENABLED=$(toml2json < "$CONFIG_FILE" | jq -r '.services.solana.enabled')
              RPC_PORT=$(toml2json < "$CONFIG_FILE" | jq -r '.services.solana.rpc_port')
              # WS_PORT=$(toml2json < "$CONFIG_FILE" | jq -r '.services.solana.ws_port')  # Currently unused
              DATA_DIR=$(toml2json < "$CONFIG_FILE" | jq -r '.services.solana.data_dir' | envsubst)
              LOG_FILE=$(toml2json < "$CONFIG_FILE" | jq -r '.services.solana.log_file' | envsubst)
              
              if [ "$SOLANA_ENABLED" != "true" ]; then
                echo "Solana not enabled in configuration"
                exit 0
              fi
              
              PID_FILE="/tmp/sim-solana.pid"
              
              case "$ACTION" in
                "start")
                  echo "Starting Solana node (Test Validator)..."
                  
                  # Create directories
                  mkdir -p "$DATA_DIR" "$(dirname "$LOG_FILE")"
                  
                  # Check if already running
                  if [ -f "$PID_FILE" ] && kill -0 "$(cat "$PID_FILE")" 2>/dev/null; then
                    echo "Solana node is already running (PID: $(cat "$PID_FILE"))"
                    exit 0
                  fi
                  
                  # Start Solana test validator
                  solana-test-validator \
                    --ledger "$DATA_DIR" \
                    --rpc-port "$RPC_PORT" \
                    --rpc-bind-address 0.0.0.0 \
                    --reset \
                    --quiet \
                    > "$LOG_FILE" 2>&1 &
                  
                  echo $! > "$PID_FILE"
                  echo "Solana node started (PID: $!)"
                  echo "RPC: http://localhost:$RPC_PORT"
                  echo "Logs: $LOG_FILE"
                  
                  # Wait for node to be ready
                  echo "Waiting for Solana node to be ready..."
                  for ((i=1; i<=30; i++)); do
                    if curl -s -X POST -H "Content-Type: application/json" \
                       --data '{"jsonrpc":"2.0","method":"getHealth","id":1}' \
                       "http://localhost:$RPC_PORT" > /dev/null; then
                      echo "Solana node is ready"
                      break
                    fi
                    sleep 2
                  done
                  ;;
                  
                "stop")
                  echo "Stopping Solana node..."
                  if [ -f "$PID_FILE" ]; then
                    kill "$(cat "$PID_FILE")" 2>/dev/null || true
                    rm -f "$PID_FILE"
                    echo "Solana node stopped"
                  else
                    echo "Solana node not running"
                  fi
                  ;;
                  
                "status")
                  if [ -f "$PID_FILE" ] && kill -0 "$(cat "$PID_FILE")" 2>/dev/null; then
                    echo "Solana node is running (PID: $(cat "$PID_FILE"))"
                    # Test RPC
                    if curl -s -X POST -H "Content-Type: application/json" \
                       --data '{"jsonrpc":"2.0","method":"getHealth","id":1}' \
                       "http://localhost:$RPC_PORT" > /dev/null; then
                      echo "RPC is responding"
                    else
                      echo "RPC not responding"
                    fi
                  else
                    echo "Solana node is not running"
                  fi
                  ;;
                  
                *)
                  echo "Usage: sim-solana {start|stop|status} [config_file]"
                  exit 1
                  ;;
              esac
            '';
          }
        else
          pkgs.writeShellApplication {
            name = "sim-solana";
            text = ''
              echo "Solana test validator not available on aarch64-linux"
              echo "This platform only supports Solana development tools (anchor)"
              exit 1
            '';
          };

        # Almanac API launcher
        almanacApiLauncher = pkgs.writeShellApplication {
          name = "sim-almanac-api";
          runtimeInputs = [ pkgs.curl pkgs.toml2json pkgs.jq pkgs.envsubst ];
          text = ''
            # Launch Almanac API server
            
            ACTION="''${1:-start}"  # start, stop, status
            CONFIG_FILE="''${2:-./config/simulation.toml}"
            
            if [ ! -f "$CONFIG_FILE" ]; then
              echo "Error: Configuration file not found: $CONFIG_FILE"
              echo "Run 'sim-config' first to generate configuration"
              exit 1
            fi
            
            # Read configuration from TOML
            API_PORT=$(toml2json < "$CONFIG_FILE" | jq -r '.almanac.api_port')
            API_LOG_FILE=$(toml2json < "$CONFIG_FILE" | jq -r '.almanac.api_log_file' | envsubst)
            
            PID_FILE="/tmp/sim-almanac-api.pid"
            
            case "$ACTION" in
              "start")
                echo "Starting Almanac API server..."
                
                # Create log directory
                mkdir -p "$(dirname "$API_LOG_FILE")"
                
                # Check if already running
                if [ -f "$PID_FILE" ] && kill -0 "$(cat "$PID_FILE")" 2>/dev/null; then
                  echo "Almanac API is already running (PID: $(cat "$PID_FILE"))"
                  exit 0
                fi
                
                # Set environment variables for API
                export API_HOST="0.0.0.0"
                export API_PORT="$API_PORT"
                export STORAGE_BACKEND="foundationdb"
                export FDB_CLUSTER_FILE="$PWD/data/fdb.cluster"
                export FDB_NAMESPACE="almanac"
                export ENABLE_CORS="true"
                export CORS_ORIGINS="*"
                export RUST_LOG="almanac_api=info,tower_http=debug"
                
                # Set FoundationDB library path for runtime
                ${if pkgs.stdenv.isDarwin then ''
                  export DYLD_LIBRARY_PATH="${fdb-packages.foundationdb}/lib:''${DYLD_LIBRARY_PATH:-}"
                '' else ''
                  export LD_LIBRARY_PATH="${fdb-packages.foundationdb}/lib:''${LD_LIBRARY_PATH:-}"
                ''}
                
                # Start API server
                cargo run --bin almanac-api --features fdb \
                  > "$API_LOG_FILE" 2>&1 &
                
                echo $! > "$PID_FILE"
                echo "Almanac API started (PID: $!)"
                echo "API: http://localhost:$API_PORT"
                echo "Logs: $API_LOG_FILE"
                
                # Wait for API to be ready
                echo "Waiting for Almanac API to be ready..."
                for ((i=1; i<=30; i++)); do
                  if curl -s "http://localhost:$API_PORT/health" > /dev/null; then
                    echo "Almanac API is ready"
                    break
                  fi
                  sleep 1
                done
                ;;
                
              "stop")
                echo "Stopping Almanac API..."
                if [ -f "$PID_FILE" ]; then
                  kill "$(cat "$PID_FILE")" 2>/dev/null || true
                  rm -f "$PID_FILE"
                  echo "Almanac API stopped"
                else
                  echo "Almanac API not running"
                fi
                ;;
                
              "status")
                if [ -f "$PID_FILE" ] && kill -0 "$(cat "$PID_FILE")" 2>/dev/null; then
                  echo "Almanac API is running (PID: $(cat "$PID_FILE"))"
                  # Test API health
                  if curl -s "http://localhost:$API_PORT/health" > /dev/null; then
                    echo "API health check passed"
                  else
                    echo "API health check failed"
                  fi
                else
                  echo "Almanac API is not running"
                fi
                ;;
                
              *)
                echo "Usage: sim-almanac-api {start|stop|status} [config_file]"
                exit 1
                ;;
            esac
          '';
        };

        # Almanac indexer launcher
        almanacIndexerLauncher = pkgs.writeShellApplication {
          name = "sim-almanac-indexer";
          runtimeInputs = [ pkgs.curl pkgs.toml2json pkgs.jq pkgs.envsubst ];
          text = ''
            # Launch Almanac indexer service
            
            ACTION="''${1:-start}"  # start, stop, status
            CONFIG_FILE="''${2:-./config/simulation.toml}"
            
            if [ ! -f "$CONFIG_FILE" ]; then
              echo "Error: Configuration file not found: $CONFIG_FILE"
              echo "Run 'sim-config' first to generate configuration"
              exit 1
            fi
            
            # Read configuration from TOML
            INDEXER_CONFIG=$(toml2json < "$CONFIG_FILE" | jq -r '.almanac.indexer_config' | envsubst)
            
            PID_FILE="/tmp/sim-almanac-indexer.pid"
            LOG_FILE="$PWD/logs/almanac-indexer.log"
            
            case "$ACTION" in
              "start")
                echo "Starting Almanac indexer service..."
                
                # Create log directory
                mkdir -p "$(dirname "$LOG_FILE")"
                
                # Check if already running
                if [ -f "$PID_FILE" ] && kill -0 "$(cat "$PID_FILE")" 2>/dev/null; then
                  echo "Almanac indexer is already running (PID: $(cat "$PID_FILE"))"
                  exit 0
                fi
                
                # Set environment variables for indexer
                export RUST_LOG="almanac_indexing=info,almanac_core=debug"
                export ALMANAC_CONFIG="$INDEXER_CONFIG"
                
                # Set FoundationDB library path for runtime
                ${if pkgs.stdenv.isDarwin then ''
                  export DYLD_LIBRARY_PATH="${fdb-packages.foundationdb}/lib:''${DYLD_LIBRARY_PATH:-}"
                '' else ''
                  export LD_LIBRARY_PATH="${fdb-packages.foundationdb}/lib:''${LD_LIBRARY_PATH:-}"
                ''}
                
                # Start indexer service
                cargo run --bin almanac-indexing --features fdb \
                  > "$LOG_FILE" 2>&1 &
                
                echo $! > "$PID_FILE"
                echo "Almanac indexer started (PID: $!)"
                echo "Config: $INDEXER_CONFIG"
                echo "Logs: $LOG_FILE"
                ;;
                
              "stop")
                echo "Stopping Almanac indexer..."
                if [ -f "$PID_FILE" ]; then
                  kill "$(cat "$PID_FILE")" 2>/dev/null || true
                  rm -f "$PID_FILE"
                  echo "Almanac indexer stopped"
                else
                  echo "Almanac indexer not running"
                fi
                ;;
                
              "status")
                if [ -f "$PID_FILE" ] && kill -0 "$(cat "$PID_FILE")" 2>/dev/null; then
                  echo "Almanac indexer is running (PID: $(cat "$PID_FILE"))"
                else
                  echo "Almanac indexer is not running"
                fi
                ;;
                
              *)
                echo "Usage: sim-almanac-indexer {start|stop|status} [config_file]"
                exit 1
                ;;
            esac
          '';
        };

        # Main simulation orchestrator
        simulationOrchestrator = pkgs.writeShellApplication {
          name = "sim-cluster";
          runtimeInputs = [ 
            nodeConfigGenerator
            ethereumLauncher
            cosmosLauncher
            solanaLauncher
            almanacApiLauncher
            almanacIndexerLauncher
            fdb-packages.fdb-manage
            pkgs.curl
            pkgs.toml2json
            pkgs.jq
            pkgs.envsubst
          ];
          text = ''
            # Almanac Simulation Cluster Orchestrator
            
            ACTION="''${1:-help}"
            CHAINS="''${2:-ethereum,cosmos,fdb}"  # Default chains
            CONFIG_FILE="./config/simulation.toml"
            
            case "$ACTION" in
              "init")
                echo "Initializing Almanac simulation cluster..."
                echo "Chains: $CHAINS"
                
                # Generate configuration
                sim-config "$CHAINS"
                
                echo "Simulation cluster initialized"
                echo "Configuration: $CONFIG_FILE"
                echo ""
                echo "Next steps:"
                echo "  sim-cluster start    # Start all services"
                echo "  sim-cluster status   # Check service status"
                echo "  sim-cluster stop     # Stop all services"
                ;;
                
              "start")
                echo "Starting Almanac simulation cluster..."
                
                if [ ! -f "$CONFIG_FILE" ]; then
                  echo "Configuration not found. Running init first..."
                  sim-config "$CHAINS"
                fi
                
                # Create directories
                mkdir -p data logs config
                
                # Start FoundationDB first (if enabled)
                if toml2json < "$CONFIG_FILE" | jq -e '.services.foundationdb.enabled == true' > /dev/null; then
                  echo "Starting FoundationDB..."
                  fdb-manage start || echo "FoundationDB already running or failed to start"
                fi
                
                # Start blockchain nodes
                sim-ethereum start "$CONFIG_FILE" &
                sim-cosmos start "$CONFIG_FILE" &
                
                # Start Solana (if supported)
                if command -v solana-test-validator >/dev/null 2>&1; then
                  sim-solana start "$CONFIG_FILE" &
                fi
                
                # Wait for background processes
                wait
                
                # Start Almanac services after blockchain nodes are ready
                echo "Starting Almanac services..."
                sim-almanac-api start "$CONFIG_FILE" &
                sim-almanac-indexer start "$CONFIG_FILE" &
                
                # Wait for Almanac services
                wait
                
                echo "All services started"
                echo ""
                echo "Service endpoints:"
                toml2json < "$CONFIG_FILE" | jq -e '.services.ethereum.enabled == true' > /dev/null && \
                  echo "  Ethereum RPC: http://localhost:$(toml2json < "$CONFIG_FILE" | jq -r '.services.ethereum.rpc_port')"
                toml2json < "$CONFIG_FILE" | jq -e '.services.cosmos.enabled == true' > /dev/null && \
                  echo "  Cosmos RPC: http://localhost:$(toml2json < "$CONFIG_FILE" | jq -r '.services.cosmos.rpc_port')"
                toml2json < "$CONFIG_FILE" | jq -e '.services.solana.enabled == true' > /dev/null && \
                  echo "  Solana RPC: http://localhost:$(toml2json < "$CONFIG_FILE" | jq -r '.services.solana.rpc_port')"
                echo "  Almanac API: http://localhost:$(toml2json < "$CONFIG_FILE" | jq -r '.almanac.api_port')"
                echo ""
                echo "Logs directory: ./logs/"
                ;;
                
              "stop")
                echo "Stopping Almanac simulation cluster..."
                
                # Stop Almanac services first
                sim-almanac-indexer stop "$CONFIG_FILE" || true
                sim-almanac-api stop "$CONFIG_FILE" || true
                
                # Stop blockchain nodes
                sim-ethereum stop "$CONFIG_FILE" || true
                sim-cosmos stop "$CONFIG_FILE" || true
                sim-solana stop "$CONFIG_FILE" || true
                
                # Stop FoundationDB last
                fdb-manage stop || true
                
                echo "All services stopped"
                ;;
                
              "status")
                echo "Almanac simulation cluster status:"
                echo ""
                
                if [ ! -f "$CONFIG_FILE" ]; then
                  echo "Not initialized (run 'sim-cluster init')"
                  exit 1
                fi
                
                # Check FoundationDB
                if toml2json < "$CONFIG_FILE" | jq -e '.services.foundationdb.enabled == true' > /dev/null; then
                  echo -n "FoundationDB: "
                  if fdb-manage status > /dev/null 2>&1; then
                    echo "Running"
                  else
                    echo "Not running"
                  fi
                fi
                
                # Check blockchain nodes
                sim-ethereum status "$CONFIG_FILE"
                sim-cosmos status "$CONFIG_FILE"
                sim-solana status "$CONFIG_FILE"
                
                # Check Almanac services
                sim-almanac-api status "$CONFIG_FILE"
                sim-almanac-indexer status "$CONFIG_FILE"
                ;;
                
              "logs")
                SERVICE="''${2:-all}"
                
                if [ ! -f "$CONFIG_FILE" ]; then
                  echo "Configuration not found. Run 'sim-cluster init' first."
                  exit 1
                fi
                
                case "$SERVICE" in
                  "ethereum"|"eth")
                    LOG_FILE=$(toml2json < "$CONFIG_FILE" | jq -r '.services.ethereum.log_file' | envsubst)
                    if [ -f "$LOG_FILE" ]; then
                      tail -f "$LOG_FILE"
                    else
                      echo "Ethereum log file not found: $LOG_FILE"
                    fi
                    ;;
                  "cosmos")
                    LOG_FILE=$(toml2json < "$CONFIG_FILE" | jq -r '.services.cosmos.log_file' | envsubst)
                    if [ -f "$LOG_FILE" ]; then
                      tail -f "$LOG_FILE"
                    else
                      echo "Cosmos log file not found: $LOG_FILE"
                    fi
                    ;;
                  "solana")
                    LOG_FILE=$(toml2json < "$CONFIG_FILE" | jq -r '.services.solana.log_file' | envsubst)
                    if [ -f "$LOG_FILE" ]; then
                      tail -f "$LOG_FILE"
                    else
                      echo "Solana log file not found: $LOG_FILE"
                    fi
                    ;;
                  "api"|"almanac-api")
                    LOG_FILE=$(toml2json < "$CONFIG_FILE" | jq -r '.almanac.api_log_file' | envsubst)
                    if [ -f "$LOG_FILE" ]; then
                      tail -f "$LOG_FILE"
                    else
                      echo "Almanac API log file not found: $LOG_FILE"
                    fi
                    ;;
                  "indexer"|"almanac-indexer")
                    LOG_FILE="$PWD/logs/almanac-indexer.log"
                    if [ -f "$LOG_FILE" ]; then
                      tail -f "$LOG_FILE"
                    else
                      echo "Almanac indexer log file not found: $LOG_FILE"
                    fi
                    ;;
                  "all"|*)
                    echo "Available logs:"
                    echo "  sim-cluster logs ethereum"
                    echo "  sim-cluster logs cosmos"
                    echo "  sim-cluster logs solana"
                    echo "  sim-cluster logs api"
                    echo "  sim-cluster logs indexer"
                    ;;
                esac
                ;;
                
              "help"|*)
                cat << EOF
            Almanac Simulation Cluster Orchestrator

            Usage: sim-cluster <command> [options]

            Commands:
              init [chains]         Initialize simulation (default: ethereum,cosmos,fdb)
              start                 Start all configured services
              stop                  Stop all services
              status                Check status of all services
              logs [service]        Show logs for a service (ethereum|cosmos|solana|api|indexer|all)
              help                  Show this help message

            Available chains:
              ethereum              Anvil Ethereum node
              cosmos                Neutron Cosmos node  
              solana                Solana test validator (where supported)
              fdb                   FoundationDB

            Services:
              api                   Almanac API server
              indexer               Almanac indexing service

            Examples:
              sim-cluster init ethereum,fdb          # Ethereum + FoundationDB only
              sim-cluster init cosmos,solana,fdb     # Cosmos + Solana + FoundationDB
              sim-cluster start                      # Start all configured services
              sim-cluster logs ethereum              # Show Ethereum logs
              sim-cluster logs api                   # Show Almanac API logs
              sim-cluster logs indexer               # Show Almanac indexer logs

            Configuration is stored in: ./config/
            EOF
                ;;
            esac
          '';
        };

      in {
        packages = {
          inherit nodeConfigGenerator ethereumLauncher cosmosLauncher solanaLauncher almanacApiLauncher almanacIndexerLauncher simulationOrchestrator;
          
          # Main entry point
          sim-cluster = simulationOrchestrator;
          sim-config = nodeConfigGenerator;
          sim-ethereum = ethereumLauncher;
          sim-cosmos = cosmosLauncher;
          sim-solana = solanaLauncher;
          sim-almanac-api = almanacApiLauncher;
          sim-almanac-indexer = almanacIndexerLauncher;
        };

        apps = {
          sim-cluster = {
            type = "app";
            program = "${simulationOrchestrator}/bin/sim-cluster";
          };
          
          sim-config = {
            type = "app";
            program = "${nodeConfigGenerator}/bin/sim-config";
          };
        };
      };
    };
}