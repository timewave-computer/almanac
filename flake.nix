{
  description = "Almanac: Cross-chain event indexer and processor";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    
    # Blockchain development environments
    fdb = {
      url = "path:./nix/fdb";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    
    ethereum = {
      url = "path:./nix/ethereum";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    
    cosmos = {
      url = "path:./nix/cosmos";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    
    solana = {
      url = "path:./nix/solana";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    
    simulation = {
      url = "path:./nix/simulation";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = inputs@{ self, nixpkgs, flake-utils, fdb, ethereum, cosmos, solana, simulation, ... }:
    flake-utils.lib.eachSystem [ "aarch64-darwin" "x86_64-linux" ] (system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
        lib = pkgs.lib;
        
        # For macOS, set a deployment target
        darwinDeploymentTarget = "11.0";
        
        # Import packages from subflakes
        fdb-packages = fdb.packages.${system};
        ethereum-packages = ethereum.packages.${system};
        cosmos-packages = cosmos.packages.${system};
        solana-packages = solana.packages.${system};
        simulation-packages = simulation.packages.${system};
        
        # Create a set of common environment variables
        commonEnv = {
          MACOSX_DEPLOYMENT_TARGET = darwinDeploymentTarget;
          DEVELOPER_DIR = "";
        };
        
        # Combined development shell with all blockchain tools
        combinedDevShell = pkgs.mkShell {
          name = "almanac-development";
          
          packages = [ 
            # Rust toolchain
            pkgs.rustc
            pkgs.cargo
            pkgs.pkg-config 
            pkgs.openssl
            pkgs.curl
            pkgs.jq
            
            # FoundationDB
            fdb-packages.foundationdb
            fdb-packages.fdb-manage
            fdb-packages.fdb-init
            fdb-packages.fdb-config-generator
            
            # Ethereum tools
            ethereum-packages.geth
            ethereum-packages.lighthouse
            ethereum-packages.forge
            
            # Cosmos tools
            cosmos-packages.neutron
            
            # Solana tools
            solana-packages.solana-tools
          ];
          
          shellHook = ''
            # Set macOS deployment target for Rust builds
            export MACOSX_DEPLOYMENT_TARGET="${darwinDeploymentTarget}"
            
            # Clear the DEVELOPER_DIR variable to fix macOS linking issues
            unset DEVELOPER_DIR
            unset DEVELOPER_DIR_aarch64_apple_darwin
            
            # FoundationDB environment setup
            export PROJECT_ROOT="$(pwd)"
            export FDB_CLUSTER_FILE="$PROJECT_ROOT/data/fdb.cluster"
            export FDB_NETWORK_OPTION_TLS_CERT_PATH=""
            export FDB_NETWORK_OPTION_TLS_KEY_PATH=""
            export FDB_NETWORK_OPTION_TLS_CA_PATH=""
            
            # Set FoundationDB compilation environment for Rust
            export BINDGEN_EXTRA_CLANG_ARGS="-I${fdb-packages.foundationdb}/include"
            export C_INCLUDE_PATH="${fdb-packages.foundationdb}/include:$${C_INCLUDE_PATH:-}"
            export CPLUS_INCLUDE_PATH="${fdb-packages.foundationdb}/include:$${CPLUS_INCLUDE_PATH:-}"
            export LIBRARY_PATH="${fdb-packages.foundationdb}/lib:$${LIBRARY_PATH:-}"
            export LD_LIBRARY_PATH="${fdb-packages.foundationdb}/lib:$${LD_LIBRARY_PATH:-}"
            export PKG_CONFIG_PATH="${fdb-packages.foundationdb}/lib/pkgconfig:$${PKG_CONFIG_PATH:-}"
            
            # Create local include directory for FoundationDB headers
            if command -v fdbcli >/dev/null 2>&1; then
              FDB_LOCAL_INCLUDE="$PWD/.fdb_include"
              if [ ! -d "$FDB_LOCAL_INCLUDE/foundationdb" ]; then
                echo "Setting up local FoundationDB headers..."
                mkdir -p "$FDB_LOCAL_INCLUDE/foundationdb"
                
                # Find and copy FDB headers from Nix store
                FDB_NIX_INCLUDE=$(find ${fdb-packages.foundationdb} -name "fdb_c.h" -type f 2>/dev/null | head -1 | xargs dirname 2>/dev/null || echo "")
                if [ -n "$FDB_NIX_INCLUDE" ] && [ -d "$FDB_NIX_INCLUDE" ]; then
                  cp -r "$FDB_NIX_INCLUDE"/* "$FDB_LOCAL_INCLUDE/foundationdb/" 2>/dev/null || true
                fi
                
                # Create stub fdb.options if missing
                if [ ! -f "$FDB_LOCAL_INCLUDE/foundationdb/fdb.options" ]; then
                  cat > "$FDB_LOCAL_INCLUDE/foundationdb/fdb.options" << 'EOFFDB'
# FoundationDB Options File (stub for compilation)
# Generated for Nix build environment
EOFFDB
                fi
              fi
              
              # Export paths for build
              export FDB_INCLUDE_DIR="$FDB_LOCAL_INCLUDE"
              export CPATH="$FDB_LOCAL_INCLUDE:$${CPATH:-}"
              export C_INCLUDE_PATH="$FDB_LOCAL_INCLUDE:$${C_INCLUDE_PATH:-}"
            fi
            
            # Set up blockchain client environment variables
            export ETH_RPC_URL="http://localhost:8545"
            export GETH_DATA_DIR="$PWD/data/ethereum/geth"
            export LIGHTHOUSE_DATA_DIR="$PWD/data/ethereum/lighthouse"
            export COSMOS_RPC_URL="http://localhost:26657"
            export SOLANA_RPC_URL="http://localhost:8899"
            
            echo "=== Almanac Cross-Chain Development Environment ==="
            echo "rust version: $(rustc --version)"
            echo ""
            echo "Available blockchain environments:"
            echo "  FoundationDB: fdb-manage start/stop/status"
            echo "  Ethereum: geth, lighthouse, forge"
            echo "  Cosmos: neutron"  
            echo "  Solana: solana, anchor"
            echo ""
            echo "API Documentation:"
            echo "  nix run .#swagger-ui    # Start Swagger UI server"
            echo "  nix run .#swagger-ui -- 8080  # Custom port"
            echo ""
            echo "Quick start:"
            echo "  fdb-manage start     # Start FoundationDB"
            echo "  geth --sepolia       # Start Ethereum testnet"
            echo "  neutron init mynode  # Initialize Cosmos node"
            echo "  solana-test-validator # Start Solana validator"
            echo "  nix run .#swagger-ui # Start API docs server"
            echo ""
          '';
          
          inherit (commonEnv) MACOSX_DEPLOYMENT_TARGET;
        };
        
      in {
        # Main packages - expose all subflake packages
        packages = {
          # FoundationDB packages
          foundationdb = fdb-packages.foundationdb;
          fdb-manage = fdb-packages.fdb-manage;
          fdb-init = fdb-packages.fdb-init;
          fdb-config-generator = fdb-packages.fdb-config-generator;
          
          # Ethereum packages
          geth = ethereum-packages.geth;
          lighthouse = ethereum-packages.lighthouse;
          forge = ethereum-packages.forge;
          
          # Cosmos packages
          neutron = cosmos-packages.neutron;
          
          # Solana packages
          solana-tools = solana-packages.solana-tools;
          solana-node = solana-packages.solana-node;
          anchor = solana-packages.anchor;
          
          # Simulation packages
          sim-cluster = simulation-packages.sim-cluster;
          sim-config = simulation-packages.sim-config;
          sim-ethereum = simulation-packages.sim-ethereum;
          sim-cosmos = simulation-packages.sim-cosmos;
          sim-solana = simulation-packages.sim-solana;
          
          # Almanac API with Swagger UI
          swagger-ui = pkgs.writeShellApplication {
            name = "almanac-swagger-ui";
            runtimeInputs = with pkgs; [ 
              rustc 
              cargo 
              lsof 
              procps 
              coreutils 
              bash 
            ];
            text = ''
              # Swagger UI Server Script for Almanac API
              set -euo pipefail
              
              # Configuration
              DEFAULT_PORT=8080
              DEFAULT_HOST="127.0.0.1"
              DEFAULT_CONFIG="config/dev.toml"
              
              # Parse command line arguments
              PORT="''${1:-$DEFAULT_PORT}"
              HOST="''${2:-$DEFAULT_HOST}"
              CONFIG="''${3:-$DEFAULT_CONFIG}"
              
              # Colors for output
              RED='\033[0;31m'
              GREEN='\033[0;32m'
              YELLOW='\033[1;33m'
              BLUE='\033[0;34m'
              NC='\033[0m' # No Color
              
              print_info() {
                  echo -e "''${BLUE}[INFO]''${NC} $1"
              }
              
              print_success() {
                  echo -e "''${GREEN}[SUCCESS]''${NC} $1"
              }
              
              print_warning() {
                  echo -e "''${YELLOW}[WARNING]''${NC} $1"
              }
              
              print_error() {
                  echo -e "''${RED}[ERROR]''${NC} $1"
              }
              
              # Check if port is available
              check_port() {
                  local port=$1
                  if lsof -Pi :"$port" -sTCP:LISTEN -t >/dev/null 2>&1; then
                      return 1
                  else
                      return 0
                  fi
              }
              
              # Find a free port
              find_free_port() {
                  local start_port=$1
                  local port=$start_port
                  
                  while ! check_port "$port"; do
                      port=$((port + 1))
                      if [ "$port" -gt 65535 ]; then
                          print_error "No available ports found"
                          exit 1
                      fi
                  done
                  
                  echo "$port"
              }
              
              # Cleanup on exit
              cleanup() {
                  if [ -n "''${API_PID:-}" ]; then
                      print_info "Stopping API server (PID: $API_PID)..."
                      kill "$API_PID" 2>/dev/null || true
                  fi
                  exit 0
              }
              
              trap cleanup SIGTERM SIGINT
              
              # Print banner will be shown after port determination
              
              # Check if we're in the right directory
              if [ ! -f "Cargo.toml" ] || [ ! -d "crates/almanac-api" ]; then
                  print_error "Please run this script from the almanac project root directory"
                  exit 1
              fi
              
              # Check if config file exists
              if [ ! -f "$CONFIG" ]; then
                  print_warning "Config file '$CONFIG' not found. Using default configuration."
                  CONFIG=""
              fi
              
              # Find a free port if needed
              if ! check_port "$PORT"; then
                  print_warning "Port $PORT is already in use"
                  NEW_PORT=$(find_free_port $((PORT + 1)))
                  print_info "Using port $NEW_PORT instead"
                  PORT=$NEW_PORT
              fi
              
              # Print banner with port information
              echo -e "''${BLUE}"
              echo "╔═══════════════════════════════════════════════════════════╗"
              echo "║                Almanac API Swagger UI Server              ║"
              echo "║           High-Performance Multi-Chain Indexer            ║"
              echo "║                                                           ║"
              PORT_TEXT="Serving on port $PORT"
              PORT_PADDING=$(( (59 - ''${#PORT_TEXT}) / 2 ))
              printf "║%*s%s%*s║\n" $PORT_PADDING "" "$PORT_TEXT" $((59 - ''${#PORT_TEXT} - PORT_PADDING)) ""
              echo "╚═══════════════════════════════════════════════════════════╝"
              echo -e "''${NC}"
              
              # Build the API
              print_info "Building Almanac API with Swagger UI integration..."
              if ! cargo build --release --bin almanac-api 2>/dev/null; then
                  print_warning "Release build failed, trying debug build..."
                  if ! cargo build --bin almanac-api; then
                      print_error "Failed to build almanac-api"
                      exit 1
                  fi
                  API_BINARY="target/debug/almanac-api"
              else
                  API_BINARY="target/release/almanac-api"
              fi
              
              print_success "API built successfully: $API_BINARY"
              
              # Set environment
              export RUST_LOG="''${RUST_LOG:-info}"
              export ALMANAC_HOST="$HOST"
              export ALMANAC_PORT="$PORT"
              
              # Start the API server
              print_info "Starting Almanac API server..."
              print_info "Host: $HOST"
              print_info "Port: $PORT"
              print_info "Config: ''${CONFIG:-default}"
              
              # Start server
              if [ -n "$CONFIG" ]; then
                  $API_BINARY --config "$CONFIG" --host "$HOST" --port "$PORT" &
              else
                  $API_BINARY --host "$HOST" --port "$PORT" &
              fi
              
              API_PID=$!
              
              # Wait for server to start
              print_info "Waiting for server to start..."
              sleep 3
              
              # Check if server is running
              if ! kill -0 "$API_PID" 2>/dev/null; then
                  print_error "API server failed to start"
                  exit 1
              fi
              
              # Print access information
              print_success "Almanac API server is running!"
              echo ""
              echo -e "''${GREEN}Documentation URLs:''${NC}"
              echo -e "  ''${BLUE}Swagger UI:''${NC}      http://$HOST:$PORT/swagger-ui"
              echo -e "  ''${BLUE}OpenAPI Spec:''${NC}    http://$HOST:$PORT/openapi.json"
              echo -e "  ''${BLUE}ReDoc:''${NC}           http://$HOST:$PORT/api/redoc"
              echo -e "  ''${BLUE}Health Check:''${NC}    http://$HOST:$PORT/health"
              echo ""
              echo -e "''${GREEN}API Endpoints:''${NC}"
              echo -e "  ''${BLUE}Events:''${NC}          http://$HOST:$PORT/api/v1/events"
              echo -e "  ''${BLUE}Chain Events:''${NC}    http://$HOST:$PORT/api/v1/chains/{chain}/events"
              echo -e "  ''${BLUE}Latest Block:''${NC}    http://$HOST:$PORT/api/v1/chains/{chain}/latest-block"
              echo ""
              echo -e "''${YELLOW}Pro Tips:''${NC}"
              echo -e "  • Use the Swagger UI to test endpoints interactively"
              echo -e "  • Check the OpenAPI spec for complete API documentation"
              echo -e "  • Press Ctrl+C to stop the server"
              echo ""
              
              # Open browser if available
              if command -v open &> /dev/null; then
                  print_info "Opening Swagger UI in browser..."
                  open "http://$HOST:$PORT/swagger-ui" &
              elif command -v xdg-open &> /dev/null; then
                  print_info "Opening Swagger UI in browser..."
                  xdg-open "http://$HOST:$PORT/swagger-ui" &
              fi
              
              print_info "Server is running. Press Ctrl+C to stop."
              wait "$API_PID"
            '';
          };
          
          # Default package
          default = fdb-packages.foundationdb;
        };
        
        # Development shells
        devShells = {
          default = combinedDevShell;
          
          # Individual blockchain development shells
          fdb = fdb.devShells.${system}.default;
          ethereum = ethereum.devShells.${system}.default;
          cosmos = cosmos.devShells.${system}.default;
          solana = solana.devShells.${system}.default;
          
          # Simulation environment with all tools
          simulation = pkgs.mkShell {
            name = "almanac-simulation";
            packages = [
              simulation-packages.sim-cluster
              simulation-packages.sim-config
              fdb-packages.fdb-manage
              ethereum-packages.forge
              cosmos-packages.neutron
            ] ++ lib.optionals (pkgs.stdenv.system != "aarch64-linux") [
              solana-packages.solana-tools
            ];
            
            shellHook = ''
              echo "=== Almanac Simulation Environment ==="
              echo "Available commands:"
              echo "  sim-cluster init [chains]    # Initialize simulation"
              echo "  sim-cluster start            # Start all services"
              echo "  sim-cluster stop             # Stop all services"
              echo "  sim-cluster status           # Check service status"
              echo "  sim-cluster logs [service]   # Show service logs"
              echo ""
              echo "Example workflows:"
              echo "  sim-cluster init ethereum,fdb"
              echo "  sim-cluster start"
              echo "  sim-cluster status"
              echo ""
            '';
          };
        };
        
        # Apps for easy access
        apps = {
          fdb-manage = {
            type = "app";
            program = "${fdb-packages.fdb-manage}/bin/fdb-manage";
          };
          
          run-e2e-tests = {
            type = "app";
            program = "${fdb-packages.run-e2e-tests}/bin/run-e2e-tests";
          };
          
          geth = {
            type = "app";
            program = "${ethereum-packages.geth}/bin/geth";
          };
          
          neutron = {
            type = "app";
            program = "${cosmos-packages.neutron}/bin/neutron";
          };
          
          solana = {
            type = "app";
            program = "${solana-packages.solana-node}/bin/solana";
          };
          
          sim-cluster = {
            type = "app";
            program = "${simulation-packages.sim-cluster}/bin/sim-cluster";
          };
          
          swagger-ui = {
            type = "app";
            program = "${self.packages.${system}.swagger-ui}/bin/almanac-swagger-ui";
          };
        };
        
        formatter = pkgs.nixpkgs-fmt;
      }
    );
}