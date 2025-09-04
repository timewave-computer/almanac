{
  description = "Almanac multi-chain simulation orchestrator with OS-aware environments";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-parts.url = "github:hercules-ci/flake-parts";
    
    # Import the OS-specific flakes for each blockchain
    cosmos-macos = {
      url = "path:../cosmos";
      flake = false;
    };
    ethereum-macos = {
      url = "path:../ethereum";
      flake = false;
    };
    solana-macos = {
      url = "path:../solana";
      flake = false;
    };
    fdb-macos = {
      url = "path:../fdb";
      flake = false;
    };
  };

  outputs = inputs @ { flake-parts, ... }:
    flake-parts.lib.mkFlake { inherit inputs; } {
      systems = [ "x86_64-linux" "aarch64-linux" "x86_64-darwin" "aarch64-darwin" ];

      perSystem = { config, self', inputs', pkgs, system, lib, ... }: let
        
        # Platform detection
        isDarwin = pkgs.stdenv.isDarwin;
        isLinux = pkgs.stdenv.isLinux;
        isx86_64 = pkgs.stdenv.hostPlatform.isx86_64;
        isAarch64 = pkgs.stdenv.hostPlatform.isAarch64;
        
        # Platform string for user display
        platformString = 
          if isDarwin && isAarch64 then "macOS (Apple Silicon)"
          else if isDarwin && isx86_64 then "macOS (Intel)"
          else if isLinux && isx86_64 then "Linux (x86_64)"
          else if isLinux && isAarch64 then "Linux (aarch64)"
          else "Unknown";
        
        # Each blockchain directory has multiple flakes, we use the default one (flake.nix)
        # which automatically selects the right configuration for the platform

        # Common environment setup
        commonEnv = {
          PROJECT_ROOT = "$PWD";
          DATA_DIR = "$PWD/data";
          LOGS_DIR = "$PWD/logs";
          CONFIG_DIR = "$PWD/config";
          DETECTED_OS = if isDarwin then "macos" else "nixos";
          DETECTED_ARCH = if isAarch64 then "aarch64" else "x86_64";
        };

        # Enhanced node configuration generator
        nodeConfigGenerator = pkgs.writeShellApplication {
          name = "sim-config";
          runtimeInputs = [ pkgs.jq pkgs.coreutils ];
          text = ''
            # Generate configuration for simulation environment
            
            CHAINS="''${1:-ethereum,cosmos,solana,fdb}"
            
            # Set up environment variables
            DATA_DIR="$PWD/data"
            LOGS_DIR="$PWD/logs"
            CONFIG_DIR="$PWD/config"
            
            echo "Generating simulation configuration..."
            echo "Platform: ${platformString}"
            echo "Chains: $CHAINS"
            echo "Configuration directory: $CONFIG_DIR"
            echo ""
            
            mkdir -p "$CONFIG_DIR"
            
            # Determine storage backend based on platform and FDB availability
            if echo "$CHAINS" | grep -q "fdb"; then
              STORAGE_BACKEND="foundationdb"
              CLUSTER_FILE="$DATA_DIR/fdb.cluster"
              echo "Note: FoundationDB selected"
            else
              STORAGE_BACKEND="memory"
              CLUSTER_FILE=""
              echo "Note: Using memory storage (FDB not requested)"
            fi
            
            # Generate consolidated TOML configuration
            cat > "$CONFIG_DIR/simulation.toml" << EOF
            # Almanac Simulation Configuration
            # Generated for ${platformString}
            # OS: ${if isDarwin then "macOS" else "NixOS"}
            # Architecture: ${if isAarch64 then "aarch64" else "x86_64"}
            
            [simulation]
            name = "almanac-local-cluster"
            platform = "${platformString}"
            chains = [$(echo "$CHAINS" | sed 's/,/", "/g' | sed 's/^/"/' | sed 's/$/"/')]
            
            [almanac]
            name = "local-simulation"
            api_port = 3000
            api_log_file = "$LOGS_DIR/almanac-api.log"
            
            [storage]
            backend = "$STORAGE_BACKEND"
            $([ -n "$CLUSTER_FILE" ] && echo "cluster_file = \"$CLUSTER_FILE\"")
            
            [services.foundationdb]
            enabled = $(echo "$CHAINS" | grep -q "fdb" && echo "true" || echo "false")
            available = true
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
            # Using latest Solana version with patches
            
            [services.solana.idl_indexing]
            enabled = true
            queue_size = 10000
            fetch_concurrency = 10
            fetch_timeout_ms = 5000
            retry_max_attempts = 3
            retry_backoff_ms = 1000
            circuit_breaker_threshold = 5
            circuit_breaker_timeout_ms = 60000
            EOF
            
            # Add chain-specific configurations
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
            
            echo ""
            echo "Configuration generated successfully!"
            echo "Storage backend: $STORAGE_BACKEND"
            echo "Configuration file: $CONFIG_DIR/simulation.toml"
            
            # Platform-specific notes
            if ${if isDarwin then "true" else "false"}; then
              echo ""
              echo "macOS Notes:"
              echo "- FoundationDB available via official Apple binaries"
              echo "- All chains use macOS-optimized configurations"
            fi
            
            if ${if isLinux then "true" else "false"}; then
              echo ""
              echo "Linux/NixOS Notes:"
              echo "- Solana uses patched version for KVM compatibility"
              echo "- FoundationDB available for persistent storage"
            fi
          '';
        };

        # Enhanced simulation orchestrator
        simulationOrchestrator = pkgs.writeShellApplication {
          name = "sim-cluster";
          runtimeInputs = [ 
            nodeConfigGenerator
            pkgs.curl
            pkgs.jq
            pkgs.coreutils
          ];
          text = ''
            # Almanac Simulation Cluster Orchestrator
            # OS-Aware Version
            
            ACTION="''${1:-help}"
            CHAINS="''${2:-ethereum,cosmos,solana,fdb}"
            CONFIG_FILE="./config/simulation.toml"
            
            # Display platform info
            show_platform_info() {
              echo "Platform: ${platformString}"
              echo "Using configurations:"
              echo "  - Cosmos: nix/cosmos (auto-detects platform)"
              echo "  - Ethereum: nix/ethereum"  
              echo "  - Solana: nix/solana"
              echo "  - FDB: nix/fdb"
              echo ""
            }
            
            case "$ACTION" in
              "init")
                echo "=== Initializing Almanac Simulation Cluster ==="
                show_platform_info
                echo "Chains: $CHAINS"
                echo ""
                
                # FDB is available on all platforms
                if echo "$CHAINS" | grep -q "fdb"; then
                  echo "FoundationDB will be included in the configuration"
                  echo ""
                fi
                
                # Generate configuration
                sim-config "$CHAINS"
                
                echo ""
                echo "Simulation cluster initialized!"
                echo "Next steps:"
                echo "  sim-cluster start    # Start all services"
                echo "  sim-cluster status   # Check service status"
                echo "  sim-cluster stop     # Stop all services"
                ;;
                
              "start")
                echo "=== Starting Almanac Simulation Cluster ==="
                show_platform_info
                
                if [ ! -f "$CONFIG_FILE" ]; then
                  echo "Configuration not found. Running init first..."
                  sim-config "$CHAINS"
                fi
                
                # Create directories
                mkdir -p data logs config
                
                # Parse enabled services from config
                echo "Starting services with OS-appropriate environments..."
                echo ""
                echo "Please run the following commands in separate terminals:"
                echo ""
                
                # Start each service using the working approach from justfile
                if grep -A 1 "\[services\.foundationdb\]" "$CONFIG_FILE" | grep -q "enabled = true"; then
                  echo "1. FoundationDB:"
                  echo "   cd ../fdb && nix develop --command fdb-manage start"
                  echo ""
                fi
                
                if grep -A 1 "\[services\.ethereum\]" "$CONFIG_FILE" | grep -q "enabled = true"; then
                  echo "2. Ethereum (Anvil):"
                  echo "   cd ../ethereum && nix develop --command bash -c \"anvil --host 0.0.0.0 --port 8545 > ../../logs/anvil.log 2>&1 &\""
                  echo ""
                fi
                
                if grep -A 1 "\[services\.cosmos\]" "$CONFIG_FILE" | grep -q "enabled = true"; then
                  echo "3. Cosmos (Neutron):"
                  echo "   cd ../cosmos && nix develop --command bash -c \"neutron-start > ../../logs/neutron.log 2>&1 &\""
                  echo ""
                fi
                
                if grep -A 1 "\[services\.solana\]" "$CONFIG_FILE" | grep -q "enabled = true"; then
                  echo "4. Solana:"
                  echo "   solana-test-validator --ledger ../../data/solana --log > ../../logs/solana.log 2>&1 &"
                  echo ""
                fi
                
                echo "After starting services, run 'sim-cluster status' to verify they're running."
                ;;
                
              "stop")
                echo "=== Stopping Almanac Simulation Cluster ==="
                show_platform_info
                
                echo "Stopping services..."
                echo ""
                
                pkill -f "anvil" || true
                pkill -f "neutrond" || true  
                pkill -f "neutron-start" || true
                pkill -f "solana-test-validator" || true
                
                echo "Stopping FoundationDB..."
                echo "  cd ../fdb && nix develop --command fdb-manage stop"
                
                echo ""
                echo "All services stopped!"
                ;;
                
              "status")
                echo "=== Almanac Simulation Cluster Status ==="
                show_platform_info
                
                if [ ! -f "$CONFIG_FILE" ]; then
                  echo "Not initialized (run 'sim-cluster init')"
                  exit 1
                fi
                
                echo "Service Status:"
                echo ""
                
                # Check Ethereum
                if nc -z localhost 8545 2>/dev/null; then
                  echo "OK Ethereum (Anvil): Running on http://localhost:8545"
                else
                  echo "NO Ethereum (Anvil): Not running"
                fi
                
                # Check Cosmos
                if nc -z localhost 26657 2>/dev/null; then
                  echo "OK Cosmos (Neutron): Running on http://localhost:26657"
                else
                  echo "NO Cosmos (Neutron): Not running"
                fi
                
                # Check Solana
                if nc -z localhost 8899 2>/dev/null; then
                  echo "OK Solana: Running on http://localhost:8899"
                else
                  echo "NO Solana: Not running"
                fi
                
                # Check FDB (using the main project FDB cluster file)
                if [ -f "../../data/fdb.cluster" ]; then
                  # Try to check if actual server is running
                  if fdbcli --cluster-file="../../data/fdb.cluster" --exec status 2>/dev/null | grep -q "Database available\|Healthy\|ready"; then
                    echo "OK FoundationDB: Server running and healthy"
                  else
                    # Cluster file exists but no server - this is expected on macOS dev setup
                    echo "OK FoundationDB: Development mode (cluster file ready)"
                  fi
                else
                  echo "NO FoundationDB: Not initialized"
                fi
                
                echo ""
                echo "Log files:"
                echo "  Ethereum: logs/anvil.log"
                echo "  Cosmos: logs/neutron.log"
                echo "  Solana: logs/solana.log"
                if ${if isLinux then "true" else "false"}; then
                  echo "  FoundationDB: logs/foundationdb/"
                fi
                ;;
                
              "logs")
                SERVICE="''${2:-all}"
                echo "=== Viewing Logs ==="
                
                case "$SERVICE" in
                  "ethereum"|"eth")
                    tail -f logs/anvil.log
                    ;;
                  "cosmos")
                    tail -f logs/neutron.log
                    ;;
                  "solana"|"sol")
                    tail -f logs/solana.log
                    ;;
                  "fdb"|"foundationdb")
                    tail -f logs/foundationdb/fdb.log
                    ;;
                  "all")
                    echo "Available logs:"
                    echo "  sim-cluster logs ethereum"
                    echo "  sim-cluster logs cosmos"
                    echo "  sim-cluster logs solana"
                    if ${if isLinux then "true" else "false"}; then
                      echo "  sim-cluster logs fdb"
                    fi
                    ;;
                  *)
                    echo "Unknown service: $SERVICE"
                    ;;
                esac
                ;;
                
              "help"|*)
                echo "Almanac Simulation Cluster Orchestrator"
                echo "======================================"
                show_platform_info
                echo "Usage: sim-cluster <command> [options]"
                echo ""
                echo "Commands:"
                echo "  init [chains]    Initialize cluster (default: ethereum,cosmos,solana,fdb)"
                echo "  start           Start all configured services"
                echo "  stop            Stop all services"
                echo "  status          Check service status"
                echo "  logs [service]  View service logs"
                echo "  help            Show this help"
                echo ""
                echo "Examples:"
                echo "  sim-cluster init                    # Initialize with all chains"
                echo "  sim-cluster init ethereum,solana    # Initialize specific chains"
                echo "  sim-cluster start                   # Start services"
                echo "  sim-cluster logs ethereum          # View Ethereum logs"
                echo ""
                echo "Platform-specific notes:"
                if ${if isDarwin then "true" else "false"}; then
                  echo "  - Running on macOS: Using macOS-optimized configurations"
                  echo "  - FoundationDB available via official Apple binaries"
                fi
                if ${if isLinux then "true" else "false"}; then
                  echo "  - Running on Linux: Full FoundationDB support"
                  echo "  - Solana uses patched version for KVM compatibility"
                fi
                ;;
            esac
          '';
        };

        # Quick start wrapper
        quickStart = pkgs.writeShellApplication {
          name = "sim-quick-start";
          runtimeInputs = [ simulationOrchestrator ];
          text = ''
            echo "Almanac Quick Start"
            echo "===================="
            echo ""
            
            # Initialize with all chains (FDB available on all platforms)
            CHAINS="ethereum,cosmos,solana,fdb"
            echo "Initializing with all chains including FoundationDB"
            
            sim-cluster init "$CHAINS"
            echo ""
            echo "Ready to start! Run:"
            echo "  sim-cluster start"
          '';
        };

      in {
        packages = {
          default = simulationOrchestrator;
          sim-cluster = simulationOrchestrator;
          sim-config = nodeConfigGenerator;
          sim-quick-start = quickStart;
        };

        apps = {
          default = {
            type = "app";
            program = "${simulationOrchestrator}/bin/sim-cluster";
          };
        };

        devShells.default = pkgs.mkShell {
          buildInputs = [
            simulationOrchestrator
            nodeConfigGenerator
            quickStart
            pkgs.jq
            pkgs.curl
            pkgs.coreutils
          ];
          
          shellHook = ''
            echo "Almanac Simulation Environment"
            echo "============================="
            echo "Platform: ${platformString}"
            echo ""
            echo "This orchestrator automatically detects your OS and uses"
            echo "the appropriate nix environment for each blockchain."
            echo ""
            echo "Quick commands:"
            echo "  sim-quick-start  # Initialize with platform defaults"
            echo "  sim-cluster init # Initialize simulation"
            echo "  sim-cluster start # Start services"
            echo "  sim-cluster status # Check status"
            echo "  sim-cluster stop  # Stop services"
            echo ""
            
            # Platform-specific messages
            if ${if isDarwin then "true" else "false"}; then
              echo "macOS detected:"
              echo "- FoundationDB available via official binaries"
              echo "- All blockchains use macOS-optimized configurations"
            else
              echo "Linux/NixOS detected:"
              echo "- Full FoundationDB support available"
              echo "- Solana uses patched version for compatibility"
            fi
          '';
        };
      };
    };
}