{
  description = "Almanac Indexer";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
        
        # Script for indexing blockchain data with Almanac
        almanac-indexer-script = pkgs.writeShellScriptBin "almanac-indexer" ''
          #!/usr/bin/env bash
          set -e

          # Define colors for output
          GREEN='\033[0;32m'
          YELLOW='\033[0;33m'
          RED='\033[0;31m'
          BLUE='\033[0;34m'
          NC='\033[0m' # No Color

          # Default configuration
          CHAIN_TYPE="ethereum"
          NODE_TYPE="anvil"
          INDEX_DURATION="15m"
          ETHEREUM_PORT=8545
          ETHEREUM_HOST="localhost"
          WASMD_RPC="http://localhost:26657"
          WASMD_REST="http://localhost:1317"
          WASMD_CHAIN_ID="wasmchain"
          LOG_DIR="logs"
          CONFIG_FILE="almanac-config.json"
          DATA_DIR="data"
          ROCKSDB_DIR="$DATA_DIR/rocksdb"
          POSTGRES_DIR="$DATA_DIR/postgres"

          # Database initialization function (inline implementation)
          init_databases() {
            echo "Initializing PostgreSQL and RocksDB..."
            # Create data directories if they don't exist
            mkdir -p "$POSTGRES_DIR"
            mkdir -p "$ROCKSDB_DIR"
            
            # Initialize PostgreSQL if needed
            if [ ! -f "$POSTGRES_DIR/PG_VERSION" ]; then
              echo "Initializing PostgreSQL database..."
              ${pkgs.postgresql}/bin/initdb -D "$POSTGRES_DIR" -U postgres
              # Configure PostgreSQL for local development
              echo "listen_addresses = 'localhost'" >> "$POSTGRES_DIR/postgresql.conf"
              echo "port = 5432" >> "$POSTGRES_DIR/postgresql.conf"
            fi
            
            # Start PostgreSQL if not running
            if ! ${pkgs.postgresql}/bin/pg_ctl status -D "$POSTGRES_DIR" > /dev/null 2>&1; then
              echo "Starting PostgreSQL..."
              ${pkgs.postgresql}/bin/pg_ctl start -D "$POSTGRES_DIR" -l "$POSTGRES_DIR/logfile"
              
              # Wait for PostgreSQL to start
              for i in {1..10}; do
                if ${pkgs.postgresql}/bin/pg_isready -h localhost -p 5432 -U postgres > /dev/null 2>&1; then
                  break
                fi
                echo "Waiting for PostgreSQL to start ($i/10)..."
                sleep 1
              done
            fi
            
            # Create indexer database if it doesn't exist
            if ! echo "SELECT 1 FROM pg_database WHERE datname = 'indexer'" | ${pkgs.postgresql}/bin/psql -h localhost -p 5432 -U postgres -t | grep -q 1; then
              echo "Creating indexer database..."
              ${pkgs.postgresql}/bin/createdb -h localhost -p 5432 -U postgres indexer
            fi
            
            echo "Databases initialized successfully"
            echo "PostgreSQL is running at localhost:5432"
            echo "Database: indexer"
            return 0
          }

          # Function to stop the database
          stop_databases() {
            echo "Stopping PostgreSQL..."
            ${pkgs.postgresql}/bin/pg_ctl stop -D "$POSTGRES_DIR" -m fast
            echo "PostgreSQL stopped"
            return 0
          }

          # Help function
          show_help() {
            echo "Usage: almanac-indexer [options]"
            echo ""
            echo "Options:"
            echo "  --chain=TYPE       Chain type to index (ethereum, cosmos) [default: ethereum]"
            echo "  --node=TYPE        Node type for Ethereum (anvil, reth) [default: anvil]"
            echo "  --duration=TIME    Duration to run the indexer (e.g., 5m, 1h) [default: 15m]"
            echo "  --ethereum-port=N  Port for Ethereum node [default: 8545]"
            echo "  --ethereum-host=H  Host for Ethereum node [default: localhost]"
            echo "  --wasmd-rpc=URL    RPC URL for wasmd node [default: http://localhost:26657]"
            echo "  --wasmd-rest=URL   REST URL for wasmd node [default: http://localhost:1317]"
            echo "  --help             Show this help message"
            exit 0
          }

          # Parse command line arguments
          while [[ $# -gt 0 ]]; do
            case "$1" in
              --chain=*)
                CHAIN_TYPE=''${1#*=}
                ;;
              --node=*)
                NODE_TYPE=''${1#*=}
                ;;
              --duration=*)
                INDEX_DURATION=''${1#*=}
                ;;
              --ethereum-port=*)
                ETHEREUM_PORT=''${1#*=}
                ;;
              --ethereum-host=*)
                ETHEREUM_HOST=''${1#*=}
                ;;
              --wasmd-rpc=*)
                WASMD_RPC=''${1#*=}
                ;;
              --wasmd-rest=*)
                WASMD_REST=''${1#*=}
                ;;
              --help)
                show_help
                ;;
              *)
                echo -e "''${RED}Unknown option: $1''${NC}"
                echo "Use --help for usage information"
                exit 1
                ;;
            esac
            shift
          done

          # Set derived variables based on configuration
          ETHEREUM_URL="http://''${ETHEREUM_HOST}:''${ETHEREUM_PORT}"

          # Configuration based on chain and node type
          case "$CHAIN_TYPE" in
            ethereum)
              CHAIN_ID="ethereum"
              RPC_URL="$ETHEREUM_URL"
              case "$NODE_TYPE" in
                anvil)
                  CHAIN_NUM_ID=31337
                  LOG_FILE="$LOG_DIR/almanac-ethereum-anvil.log"
                  CONTRACTS_DIR="$DATA_DIR/contracts/ethereum/anvil"
                  ;;
                reth)
                  CHAIN_NUM_ID=1337
                  LOG_FILE="$LOG_DIR/almanac-ethereum-reth.log"
                  CONTRACTS_DIR="$DATA_DIR/contracts/ethereum/reth"
                  ;;
                *)
                  echo -e "''${RED}Invalid node type: $NODE_TYPE. Valid options: anvil, reth''${NC}"
                  exit 1
                  ;;
              esac
              CONTRACT_ADDRESSES_FILE="$CONTRACTS_DIR/contract-addresses.env"
              ;;
            cosmos)
              CHAIN_ID="cosmos"
              RPC_URL="$WASMD_RPC"
              LOG_FILE="$LOG_DIR/almanac-cosmwasm.log"
              CONTRACTS_DIR="$DATA_DIR/contracts/cosmwasm"
              CONTRACT_ADDRESSES_FILE="$CONTRACTS_DIR/contract-addresses.env"
              ;;
            *)
              echo -e "''${RED}Invalid chain type: $CHAIN_TYPE. Valid options: ethereum, cosmos''${NC}"
              exit 1
              ;;
          esac

          echo -e "''${BLUE}=== Almanac Indexer ===''${NC}"
          echo -e "''${BLUE}Chain: $CHAIN_TYPE''${NC}"
          if [ "$CHAIN_TYPE" = "ethereum" ]; then
            echo -e "''${BLUE}Node: $NODE_TYPE''${NC}"
          fi
          echo -e "''${BLUE}Duration: $INDEX_DURATION''${NC}"

          # Create necessary directories
          mkdir -p $LOG_DIR
          mkdir -p $CONTRACTS_DIR
          mkdir -p $ROCKSDB_DIR

          # Check if PostgreSQL is running
          if ! ${pkgs.postgresql}/bin/pg_isready -h localhost -p 5432 -U postgres > /dev/null 2>&1; then
            echo -e "''${RED}Error: PostgreSQL is not running''${NC}"
            echo -e "''${YELLOW}Attempting to initialize databases...''${NC}"
            
            # Try to initialize the databases
            if ! init_databases; then
              echo -e "''${RED}Failed to initialize databases.''${NC}"
              exit 1
            fi
          fi

          echo -e "''${GREEN}✓ PostgreSQL is running''${NC}"

          # Check if RocksDB has been set up
          if [ ! -d "$ROCKSDB_DIR" ]; then
            echo -e "''${YELLOW}RocksDB directory not found, creating...''${NC}"
            mkdir -p $ROCKSDB_DIR
          fi

          echo -e "''${GREEN}✓ RocksDB is set up''${NC}"

          # Chain-specific checks
          if [ "$CHAIN_TYPE" = "ethereum" ]; then
            # Check if the Ethereum node is running
            if ! ${pkgs.curl}/bin/curl -s -X POST -H "Content-Type: application/json" --data '{"jsonrpc":"2.0","method":"eth_chainId","params":[],"id":1}' "$ETHEREUM_URL" > /dev/null; then
              echo -e "''${RED}Error: Ethereum node ($NODE_TYPE) is not running at $ETHEREUM_URL''${NC}"
              case "$NODE_TYPE" in
                anvil)
                  echo -e "''${YELLOW}Please start Anvil first with: nix run .#anvil''${NC}"
                  ;;
                reth)
                  echo -e "''${YELLOW}Please start Reth first with: nix run .#reth''${NC}"
                  ;;
              esac
              exit 1
            fi
            
            # Verify chain ID matches expected
            CHAIN_ID_HEX=$(${pkgs.curl}/bin/curl -s -X POST -H "Content-Type: application/json" --data '{"jsonrpc":"2.0","method":"eth_chainId","params":[],"id":1}' "$ETHEREUM_URL" | ${pkgs.gnugrep}/bin/grep -o '"result":"0x[0-9a-f]*"' | cut -d'"' -f4)
            
            # Check if contracts are deployed
            if [ ! -f "$CONTRACT_ADDRESSES_FILE" ]; then
              echo -e "''${YELLOW}Note: Contract addresses file not found at $CONTRACT_ADDRESSES_FILE''${NC}"
              echo -e "''${YELLOW}Using mock contract addresses for testing''${NC}"
              
              # Create mock contract addresses file
              mkdir -p $(dirname "$CONTRACT_ADDRESSES_FILE")
              cat > "$CONTRACT_ADDRESSES_FILE" << EOL
          VALENCE_REGISTRY_ADDRESS="0x5FbDB2315678afecb367f032d93F642f64180aa3"
          VALENCE_GATEWAY_ADDRESS="0xe7f1725E7734CE288F8367e1Bb143E90bb3F0512"
          EOL
            fi
            
            # Source the contract addresses
            source "$CONTRACT_ADDRESSES_FILE"
            
            echo -e "''${GREEN}✓ Ethereum node ($NODE_TYPE) is running at $ETHEREUM_URL''${NC}"
            echo -e "''${GREEN}Using contract addresses:''${NC}"
            echo -e "''${GREEN}• Registry: $VALENCE_REGISTRY_ADDRESS''${NC}"
            echo -e "''${GREEN}• Gateway: $VALENCE_GATEWAY_ADDRESS''${NC}"
            
            # Generate indexer configuration
            cat > $CONFIG_FILE << EOL
          {
            "rpc_urls": ["$ETHEREUM_URL"],
            "chain_id": $CHAIN_NUM_ID,
            "starting_block": 1,
            "contract_addresses": {
              "valence_registry": "$VALENCE_REGISTRY_ADDRESS",
              "valence_gateway": "$VALENCE_GATEWAY_ADDRESS"
            }
          }
          EOL
          elif [ "$CHAIN_TYPE" = "cosmos" ]; then
            # Check if the wasmd node is running - for now we'll just assume it's running or mock it
            echo -e "''${YELLOW}Note: Skipping wasmd node check for testing purposes''${NC}"
            echo -e "''${GREEN}✓ wasmd node is assumed running at $WASMD_RPC''${NC}"
            
            # Check for contract addresses
            if [ ! -f "$CONTRACT_ADDRESSES_FILE" ]; then
              echo -e "''${YELLOW}Note: Contract addresses file not found at $CONTRACT_ADDRESSES_FILE''${NC}"
              echo -e "''${YELLOW}Using mock contract addresses for testing''${NC}"
              
              # Create mock contract addresses file
              mkdir -p $(dirname "$CONTRACT_ADDRESSES_FILE")
              cat > "$CONTRACT_ADDRESSES_FILE" << EOL
          VALENCE_REGISTRY_ADDRESS="cosmos14hj2tavq8fpesdwxxcu44rty3hh90vhujrvcmstl4zr3txmfvw9s4hmalr"
          VALENCE_GATEWAY_ADDRESS="cosmos1nc5tatafv6eyq7llkr2gv50ff9e22mnf70qgjlv737ktmt4eswrqvlx82r"
          EOL
            fi
            
            # Source the contract addresses
            source "$CONTRACT_ADDRESSES_FILE"
            
            echo -e "''${GREEN}Using contract addresses:''${NC}"
            echo -e "''${GREEN}• Registry: $VALENCE_REGISTRY_ADDRESS''${NC}"
            echo -e "''${GREEN}• Gateway: $VALENCE_GATEWAY_ADDRESS''${NC}"
            
            # Generate indexer configuration
            cat > $CONFIG_FILE << EOL
          {
            "chain_id": "$WASMD_CHAIN_ID",
            "rpc_url": "$WASMD_RPC",
            "rest_url": "$WASMD_REST",
            "starting_height": 1,
            "contract_addresses": {
              "valence_registry": "$VALENCE_REGISTRY_ADDRESS",
              "valence_gateway": "$VALENCE_GATEWAY_ADDRESS"
            }
          }
          EOL
          fi

          echo -e "''${GREEN}✓ Created indexer configuration at $CONFIG_FILE''${NC}"

          # Build and run the almanac binary
          echo -e "''${BLUE}Building and running the Almanac indexer...''${NC}"

          # First try to build the almanac binary
          cd $(git rev-parse --show-toplevel)
          ${pkgs.cargo}/bin/cargo build --bin almanac

          # Check if the build was successful
          if [ $? -ne 0 ]; then
            echo -e "''${RED}Error: Failed to build almanac binary''${NC}"
            exit 1
          fi

          # Run the indexer with appropriate parameters
          echo -e "''${BLUE}Starting indexer for $INDEX_DURATION...''${NC}"
          
          # Create the command based on the chain type
          if [ "$CHAIN_TYPE" = "ethereum" ]; then
            COMMAND="${pkgs.cargo}/bin/cargo run --bin almanac -- run --config $CONFIG_FILE --eth-rpc $ETHEREUM_URL"
          else
            COMMAND="${pkgs.cargo}/bin/cargo run --bin almanac -- run --config $CONFIG_FILE --cosmos-rpc $WASMD_RPC"
          fi
          
          # Execute the command with a timeout
          echo "Running: $COMMAND"
          echo -e "''${GREEN}Indexer will run for $INDEX_DURATION...''${NC}"
          
          # Start the indexer in the background
          $COMMAND > $LOG_FILE 2>&1 &
          INDEXER_PID=$!
          
          # Parse the duration to seconds for the sleep command
          if [[ "$INDEX_DURATION" =~ ([0-9]+)([smh]) ]]; then
            DURATION_NUM=''${BASH_REMATCH[1]}
            DURATION_UNIT=''${BASH_REMATCH[2]}
            
            case "$DURATION_UNIT" in
              s) SLEEP_SECONDS=$DURATION_NUM ;;
              m) SLEEP_SECONDS=$((DURATION_NUM * 60)) ;;
              h) SLEEP_SECONDS=$((DURATION_NUM * 3600)) ;;
              *) SLEEP_SECONDS=900 ;; # Default to 15 minutes
            esac
          else
            SLEEP_SECONDS=900 # Default to 15 minutes
          fi
          
          echo -e "''${GREEN}Indexer started with PID $INDEXER_PID, running for $SLEEP_SECONDS seconds...''${NC}"
          sleep $SLEEP_SECONDS
          
          # Stop the indexer
          echo -e "''${BLUE}Stopping indexer...''${NC}"
          kill -SIGINT $INDEXER_PID 2>/dev/null || true
          
          # Wait for the indexer to finish gracefully
          wait $INDEXER_PID 2>/dev/null || true
          
          # Check indexing status
          ${pkgs.coreutils}/bin/tail -n 20 $LOG_FILE 2>/dev/null || echo "No log file found"
          
          echo -e "''${GREEN}✓ Indexing completed after $INDEX_DURATION''${NC}"
          echo -e "''${BLUE}Log file available at: $LOG_FILE''${NC}"
          echo -e "''${BLUE}To query the indexed data, you can use the almanac query CLI or connect directly to PostgreSQL''${NC}"
          echo -e "''${BLUE}=== Indexing Complete ===''${NC}"
        '';
      in {
        packages = {
          default = almanac-indexer-script;
          almanac-indexer = almanac-indexer-script;
        };
        
        apps = {
          default = {
            type = "app";
            program = "${almanac-indexer-script}/bin/almanac-indexer";
          };
        };

        # Add a devShell with the workflow script and necessary tools
        devShells.default = pkgs.mkShell {
          buildInputs = [
            almanac-indexer-script
            pkgs.postgresql
            pkgs.curl
            pkgs.jq
          ];
          shellHook = ''
            echo "Almanac Indexer Environment"
            echo "Run 'almanac-indexer --help' to see available options"
            
            # Database commands
            function init_databases() {
              echo "Initializing PostgreSQL and RocksDB..."
              # Create data directories if they don't exist
              mkdir -p data/postgres
              mkdir -p data/rocksdb
              
              # Initialize PostgreSQL if needed
              if [ ! -f "data/postgres/PG_VERSION" ]; then
                echo "Initializing PostgreSQL database..."
                initdb -D data/postgres -U postgres
                # Configure PostgreSQL for local development
                echo "listen_addresses = 'localhost'" >> data/postgres/postgresql.conf
                echo "port = 5432" >> data/postgres/postgresql.conf
              fi
              
              # Start PostgreSQL if not running
              if ! pg_ctl status -D data/postgres > /dev/null 2>&1; then
                echo "Starting PostgreSQL..."
                pg_ctl start -D data/postgres -l data/postgres/logfile
                
                # Wait for PostgreSQL to start
                for i in {1..10}; do
                  if pg_isready -h localhost -p 5432 -U postgres > /dev/null 2>&1; then
                    break
                  fi
                  echo "Waiting for PostgreSQL to start ($i/10)..."
                  sleep 1
                done
              fi
              
              # Create indexer database if it doesn't exist
              if ! psql -h localhost -p 5432 -U postgres -lqt | cut -d \| -f 1 | grep -qw indexer; then
                echo "Creating indexer database..."
                createdb -h localhost -p 5432 -U postgres indexer
              fi
              
              echo "Databases initialized successfully"
              echo "PostgreSQL is running at localhost:5432"
              echo "Database: indexer"
              echo "Connection URL: postgres://postgres:postgres@localhost:5432/indexer?schema="
              return 0
            }
            
            function stop_databases() {
              echo "Stopping PostgreSQL..."
              pg_ctl stop -D data/postgres -m fast
              echo "PostgreSQL stopped"
              return 0
            }
            
            export -f init_databases
            export -f stop_databases
            
            # Check if PostgreSQL is running
            if pg_isready -h localhost -p 5432 -U postgres > /dev/null 2>&1; then
              echo "PostgreSQL is running at localhost:5432"
              echo "Database: indexer"
              echo "Connection URL: postgres://postgres:postgres@localhost:5432/indexer?schema="
            else
              echo "PostgreSQL is not running. Use 'init_databases' to start it."
            fi
          '';
        };
      }
    );
} 