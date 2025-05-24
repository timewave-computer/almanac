# This file defines a Nix flake for the Almanac project with CosmWasm support.
{
  description = "Almanac: Cross-chain event indexer and processor";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    flake-parts.url = "github:hercules-ci/flake-parts";
    rust-overlay.url = "github:oxalica/rust-overlay";
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    crane = {
      url = "github:ipetkov/crane";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    crate2nix = {
      url = "github:kolloch/crate2nix";
      flake = false;
    };
    foundry = {
      url = "github:shazow/foundry.nix/monthly"; # Use monthly for stability
      inputs.nixpkgs.follows = "nixpkgs";
    };
    # Add workflow environments
    workflows = {
      url = "path:./nix/environments";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = inputs@{ self, nixpkgs, flake-parts, fenix, crane, rust-overlay, foundry, workflows, crate2nix, ... }:
    # Create a simplified flake that directly specifies outputs without using modules
    flake-parts.lib.mkFlake { inherit self inputs; } {
      systems = [ "aarch64-darwin" "x86_64-linux" ]; # Add systems as needed
      
      # Include the database-module from the local nix directory
      imports = [];
      
      perSystem = { config, self', inputs', pkgs, system, ... }:
        let
          # Use pkgs.lib for convenience
          lib = pkgs.lib;
          
          # For macOS, set a deployment target
          darwinDeploymentTarget = "11.0";
          
          # Create a set of common environment variables
          commonEnv = {
            # Always set MACOSX_DEPLOYMENT_TARGET, it won't affect non-macOS systems
            MACOSX_DEPLOYMENT_TARGET = darwinDeploymentTarget;
            # Clear the DEVELOPER_DIR variable to fix linking issues on macOS
            DEVELOPER_DIR = "";
          };
          
          # Build the Rust project using crate2nix (conditionally)
          project = if builtins.pathExists ./Cargo.nix then import ./Cargo.nix {
            inherit pkgs;
            defaultCrateOverrides = pkgs.defaultCrateOverrides // {
              # Add specific overrides for our crates if needed
              indexer-storage = attrs: {
                buildInputs = with pkgs; [ 
                  postgresql_15
                  sqlx-cli
                ] ++ lib.optionals pkgs.stdenv.isDarwin [
                  pkgs.darwin.apple_sdk.frameworks.Security
                  pkgs.darwin.apple_sdk.frameworks.SystemConfiguration
                ];
                
                preBuild = ''
                  # Set environment for SQLx offline mode
                  export SQLX_OFFLINE=true
                '';
              };
              
              indexer-ethereum = attrs: {
                buildInputs = lib.optionals pkgs.stdenv.isDarwin [
                  pkgs.darwin.apple_sdk.frameworks.Security
                  pkgs.darwin.apple_sdk.frameworks.SystemConfiguration
                ];
              };
              
              indexer-cosmos = attrs: {
                buildInputs = lib.optionals pkgs.stdenv.isDarwin [
                  pkgs.darwin.apple_sdk.frameworks.Security
                  pkgs.darwin.apple_sdk.frameworks.SystemConfiguration
                ];
              };
              
              # RocksDB system dependencies
              librocksdb-sys = attrs: {
                nativeBuildInputs = with pkgs; [
                  pkg-config
                  cmake
                ];
                buildInputs = with pkgs; [
                  zlib
                  bzip2
                  lz4
                  zstd
                  snappy
                ] ++ lib.optionals pkgs.stdenv.isDarwin [
                  pkgs.darwin.apple_sdk.frameworks.Security
                  pkgs.darwin.apple_sdk.frameworks.SystemConfiguration
                ];
                
                # Set environment variables for RocksDB compilation
                preBuild = ''
                  export ZLIB_INCLUDE_DIR=${pkgs.zlib.dev}/include
                  export ZLIB_LIB_DIR=${pkgs.zlib}/lib
                  export BZIP2_INCLUDE_DIR=${pkgs.bzip2.dev}/include
                  export BZIP2_LIB_DIR=${pkgs.bzip2}/lib
                  export LZ4_INCLUDE_DIR=${pkgs.lz4.dev}/include
                  export LZ4_LIB_DIR=${pkgs.lz4}/lib
                  export ZSTD_INCLUDE_DIR=${pkgs.zstd.dev}/include
                  export ZSTD_LIB_DIR=${pkgs.zstd}/lib
                  export SNAPPY_INCLUDE_DIR=${pkgs.snappy}/include
                  export SNAPPY_LIB_DIR=${pkgs.snappy}/lib
                '';
              };
              
              # OpenSSL system dependencies
              openssl-sys = attrs: {
                nativeBuildInputs = with pkgs; [ pkg-config ];
                buildInputs = [ pkgs.openssl ];
                preBuild = ''
                  export OPENSSL_DIR=${pkgs.openssl.dev}
                  export OPENSSL_LIB_DIR=${pkgs.openssl.out}/lib
                  export OPENSSL_INCLUDE_DIR=${pkgs.openssl.dev}/include
                '';
              };
              
              # Protocol Buffers dependencies
              prost-build = attrs: {
                nativeBuildInputs = [ pkgs.protobuf ];
              };
              
              # Additional system crates that might need overrides
              ring = attrs: {
                buildInputs = lib.optionals pkgs.stdenv.isDarwin [
                  pkgs.darwin.apple_sdk.frameworks.Security
                ];
              };
            };
          } else null;
          
          # Create database packages directly (simplified version of what's in the module)
          initDatabasesScript = pkgs.writeShellApplication {
            name = "init-databases";
            runtimeInputs = with pkgs; [
              postgresql_15
              sqlx-cli
              git
            ];
            text = ''
              # Initialize and start databases for the Almanac project
              set -e

              echo "=== Almanac Database Initialization ==="

              # Create required data directories
              PROJECT_ROOT="$(pwd)"
              mkdir -p "$PROJECT_ROOT/data/rocksdb"
              mkdir -p "$PROJECT_ROOT/data/postgres"

              # === PostgreSQL initialization and startup ===
              echo "Initializing PostgreSQL..."

              # Configure PostgreSQL data directory
              export PGDATA="$PROJECT_ROOT/data/postgres"
              export PGUSER=postgres
              export PGPASSWORD=postgres
              export PGDATABASE=indexer
              export PGHOST=localhost
              export PGPORT=5432

              # Initialize PostgreSQL if not already done
              if [ ! -f "$PGDATA/PG_VERSION" ]; then
                echo "Creating new PostgreSQL database cluster..."
                initdb -D "$PGDATA" --auth=trust --no-locale --encoding=UTF8 --username=$PGUSER
                
                # Configure PostgreSQL to listen on localhost
                echo "listen_addresses = '127.0.0.1'" >> "$PGDATA/postgresql.conf"
                echo "port = 5432" >> "$PGDATA/postgresql.conf"
                
                echo "PostgreSQL initialized successfully."
              else
                echo "Using existing PostgreSQL database at $PGDATA"
              fi

              # Start PostgreSQL server if not running
              if ! pg_isready -q; then
                echo "Starting PostgreSQL server..."
                pg_ctl -D "$PGDATA" -o "-F -p $PGPORT" start -l "$PGDATA/postgres.log"
                
                # Wait for PostgreSQL to be ready
                attempt=0
                max_attempts=10
                until pg_isready -q || [ $attempt -eq $max_attempts ]; do
                  attempt=$((attempt+1))
                  echo "Waiting for PostgreSQL to be ready... (attempt $attempt/$max_attempts)"
                  sleep 1
                done

                if [ $attempt -eq $max_attempts ]; then
                  echo "Failed to connect to PostgreSQL after $max_attempts attempts."
                  exit 1
                fi
              else
                echo "PostgreSQL is already running."
              fi

              # Create development database if it doesn't exist
              if ! psql -lqt | cut -d \| -f 1 | grep -qw indexer; then
                echo "Creating development database 'indexer'..."
                createdb indexer
                
                # Set DATABASE_URL for applications
                export DATABASE_URL="postgresql://postgres:postgres@localhost:5432/indexer"
                echo "export DATABASE_URL=\"$DATABASE_URL\"" > "$PROJECT_ROOT/.db_env"
                
                # Run migrations if needed
                echo "Running PostgreSQL migrations..."
                cd "$PROJECT_ROOT/crates/storage"
                if command -v sqlx &> /dev/null; then
                  SQLX_OFFLINE=false sqlx migrate run
                else
                  echo "sqlx-cli not found, skipping automatic migrations."
                  echo "You may need to run migrations manually."
                fi
                cd "$PROJECT_ROOT"
              else
                echo "Development database 'indexer' already exists."
              fi

              # === RocksDB initialization ===
              echo -e "\nInitializing RocksDB storage..."

              # RocksDB doesn't need a server, but we'll pre-create the directory
              # and ensure permissions are correct
              ROCKS_PATH="$PROJECT_ROOT/data/rocksdb"
              mkdir -p "$ROCKS_PATH"
              echo "RocksDB storage directory prepared at $ROCKS_PATH"

              # === Finish ===
              echo -e "\n=== Database Initialization Complete ==="
              echo "PostgreSQL: Running on localhost:5432"
              echo "  • Database: indexer"
              echo "  • Connection URL: $DATABASE_URL"
              echo "  • Data directory: $PGDATA"
              echo "  • Log file: $PGDATA/postgres.log"
              echo ""
              echo "RocksDB:"
              echo "  • Data directory: $ROCKS_PATH"
              echo "  • Access through application configs pointing to this path"
              echo ""
              echo "Environment variables have been saved to .db_env"

              # To ensure these variables are available in the current shell
              echo "Run 'source .db_env' to load database environment variables in your current shell."

              # Create a stop-postgres.sh script for clean shutdown
              cat > "$PGDATA/stop-postgres.sh" << 'EOF'
#!/bin/bash
set -e

# Script to cleanly shut down PostgreSQL
if [ -f "$PGDATA/postmaster.pid" ]; then
  echo "Stopping PostgreSQL server..."
  pg_ctl -D "$PGDATA" stop -m fast
  echo "PostgreSQL server stopped successfully."
else
  echo "PostgreSQL is not running or PID file not found."
fi
EOF

              chmod +x "$PGDATA/stop-postgres.sh"
              echo "Created PostgreSQL stop script at $PGDATA/stop-postgres.sh"
            '';
          };

          stopDatabasesScript = pkgs.writeShellApplication {
            name = "stop-databases";
            runtimeInputs = with pkgs; [
              postgresql_15
              git
            ];
            text = ''
              # Gracefully stop running database services
              set -e

              PROJECT_ROOT="$(pwd)"
              echo "=== Gracefully Stopping Databases ==="

              # === PostgreSQL shutdown ===
              export PGDATA="$PROJECT_ROOT/data/postgres"
              
              if [ ! -d "$PGDATA" ]; then
                echo "PostgreSQL data directory not found at $PGDATA"
                echo "No PostgreSQL instance to stop."
              elif pg_isready -q; then
                echo "Stopping PostgreSQL server..."
                pg_ctl -D "$PGDATA" stop -m fast
                
                # Wait for PostgreSQL to stop
                attempt=0
                max_attempts=10
                while pg_isready -q && [ $attempt -lt $max_attempts ]; do
                  attempt=$((attempt+1))
                  echo "Waiting for PostgreSQL to stop... (attempt $attempt/$max_attempts)"
                  sleep 1
                done
                
                if pg_isready -q; then
                  echo "WARNING: PostgreSQL server did not stop gracefully."
                  echo "You may need to manually kill the process."
                else
                  echo "✓ PostgreSQL server stopped successfully."
                fi
              else
                echo "PostgreSQL server is not running."
              fi

              # RocksDB is not a server, so nothing to stop

              echo -e "\n=== Database Shutdown Complete ==="
            '';
          };
          
          # Use rust from nixpkgs for now
          rustToolchain = pkgs.rustc;
          
          # Define PostgreSQL configuration 
          pgPort = 5432;
          pgUser = "postgres";
          pgPassword = "postgres";
          pgDatabase = "indexer"; # Changed to match what's used in the scripts
          pgSchema = "public";
          
          # Load foundry package properly
          foundryPackage = inputs.foundry.defaultPackage.${system};
          
          # Add PostgreSQL to the basic development shell
          basicDevShell = pkgs.mkShell {
            packages = [ 
              rustToolchain 
              pkgs.cargo
              pkgs.pkg-config 
              pkgs.openssl
              pkgs.postgresql_15
              pkgs.sqlx-cli
              pkgs.curl
              pkgs.jq
              foundryPackage
              pkgs.crate2nix
            ];
            
            # Shell hook to set up PostgreSQL
            shellHook = ''
              # Setup PostgreSQL environment
              export PGHOST=localhost
              export PGPORT=${toString pgPort}
              export PGUSER=${pgUser}
              export PGPASSWORD=${pgPassword}
              export PGDATABASE=${pgDatabase}
              export PGDATA="$PWD/data/postgres"
              export DATABASE_URL="postgres://$PGUSER:$PGPASSWORD@$PGHOST:$PGPORT/$PGDATABASE?schema=$pgSchema"
              
              # Set macOS deployment target for Rust builds
              export MACOSX_DEPLOYMENT_TARGET="${darwinDeploymentTarget}"
              
              # Clear the DEVELOPER_DIR variable to fix macOS linking issues
              unset DEVELOPER_DIR
              unset DEVELOPER_DIR_aarch64_apple_darwin
              
              # Database commands - we're exposing the database scripts
              echo "Using simplified development environment"
              echo "rust version: $(rustc --version)"
              echo "crate2nix version: $(crate2nix --version)"
              echo ""
              echo "Database commands available:"
              echo "  init_databases       - Initialize and start PostgreSQL and RocksDB"
              echo "  stop_databases       - Stop PostgreSQL server"
              echo "  run_almanac_tests    - Run the Almanac test suite"
              echo ""
              echo "Nix build commands available:"
              echo "  generate_cargo_nix   - Generate Cargo.nix from Cargo.toml using crate2nix"
              echo "  update_cargo_nix     - Update existing Cargo.nix file"
              echo ""
              
              # Expose the database commands as shell functions
              function init_databases {
                ${initDatabasesScript}/bin/init-databases
              }
              
              function stop_databases {
                ${stopDatabasesScript}/bin/stop-databases
              }
              
              function run_almanac_tests {
                bash $PWD/scripts/almanac-test-suite.sh
              }
              
              # Expose crate2nix commands as shell functions
              function generate_cargo_nix {
                ${packages.generate-cargo-nix}/bin/generate-cargo-nix
              }
              
              function update_cargo_nix {
                ${packages.update-cargo-nix}/bin/update-cargo-nix
              }
              
              export -f init_databases
              export -f stop_databases
              export -f run_almanac_tests
              export -f generate_cargo_nix
              export -f update_cargo_nix
              
              # Check PostgreSQL status and start if needed
              if ! pg_isready -q; then
                echo "PostgreSQL is not running. Run 'init_databases' to start it."
              else 
                echo "PostgreSQL is running at $PGHOST:$PGPORT"
                echo "Database: $PGDATABASE"
                echo "Connection URL: $DATABASE_URL"
              fi
              
              # Check if Cargo.nix exists
              if [ ! -f "Cargo.nix" ]; then
                echo ""
                echo "Note: Cargo.nix not found. Run 'generate_cargo_nix' to create it for Nix builds."
              else
                echo ""
                echo "Cargo.nix found. You can use 'nix build' to build Rust packages."
              fi
            '';
            
            # Set environment variables for macOS compatibility
            inherit (commonEnv) MACOSX_DEPLOYMENT_TARGET;
          };
          
          # Use wasm-bindgen-cli from nixpkgs instead of building from source
          packages = {
            wasm-bindgen-pkg = pkgs.wasm-bindgen-cli;
            init-databases = initDatabasesScript;
            stop-databases = stopDatabasesScript;
            
            # Script to generate Cargo.nix using crate2nix
            generate-cargo-nix = pkgs.writeShellApplication {
              name = "generate-cargo-nix";
              runtimeInputs = with pkgs; [ 
                crate2nix
                nix 
              ];
              text = ''
                echo "Generating Cargo.nix using crate2nix..."
                
                # Check if we have a Cargo.toml file
                if [ ! -f "Cargo.toml" ]; then
                  echo "Error: Cargo.toml not found in current directory"
                  exit 1
                fi
                
                # Generate Cargo.nix using crate2nix from nixpkgs
                crate2nix generate \
                  --nixpkgs-path ${pkgs.path} \
                  --output ./Cargo.nix
                
                echo "Successfully generated Cargo.nix"
                echo "You can now use 'nix build .#almanac' to build the main binary"
                echo "Or 'nix build .#<crate-name>' to build specific workspace crates"
              '';
            };
            
            # Script to update the generated Cargo.nix
            update-cargo-nix = pkgs.writeShellApplication {
              name = "update-cargo-nix";
              runtimeInputs = with pkgs; [ 
                crate2nix
                nix 
              ];
              text = ''
                echo "Updating Cargo.nix..."
                
                # Backup existing Cargo.nix if it exists
                if [ -f "Cargo.nix" ]; then
                  cp Cargo.nix Cargo.nix.backup
                  echo "Backed up existing Cargo.nix to Cargo.nix.backup"
                fi
                
                # Regenerate Cargo.nix
                crate2nix generate \
                  --nixpkgs-path ${pkgs.path} \
                  --output ./Cargo.nix
                
                echo "Successfully updated Cargo.nix"
              '';
            };
            
            e2e-test = pkgs.writeShellApplication {
              name = "e2e-test";
              runtimeInputs = with pkgs; [
                foundryPackage
                bash
              ];
              text = ''
                # Set up environment
                export RPC_URL="http://localhost:8545"
                export PRIVATE_KEY="0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80"
                export TOKEN_NAME="Faucet Token"
                export TOKEN_SYMBOL="FCT"
                export TOKEN_DECIMALS="18"
                export FAUCET_AMOUNT="100000000000000000000" # 100 tokens
                
                # Create a temporary copy of the script
                TEMP_SCRIPT=$(mktemp)
                cp ${toString ./scripts/e2e-test.sh} $TEMP_SCRIPT
                chmod +x $TEMP_SCRIPT
                
                # Execute the script
                $TEMP_SCRIPT
                EXIT_CODE=$?
                
                # Clean up
                rm -f $TEMP_SCRIPT
                
                exit $EXIT_CODE
              '';
            };
            almanac-test-suite = pkgs.writeShellApplication {
              name = "almanac-test-suite";
              runtimeInputs = with pkgs; [
                bash
                foundryPackage
                curl
                jq
                postgresql_15
                rustc
                cargo
                sqlx-cli
              ];
              text = ''
                # Run the almanac test suite
                SCRIPT_PATH=${toString ./scripts/almanac-test-suite.sh}
                
                # Make sure the script is executable
                chmod +x $SCRIPT_PATH
                
                # Execute the script
                $SCRIPT_PATH
                exit $?
              '';
            };
            default = pkgs.wasm-bindgen-cli;
            
            # Add workflow packages
            workflow-menu = workflows.packages.${system}.workflow-menu;
            anvil-workflow = workflows.packages.${system}.anvil-workflow;
            reth-workflow = workflows.packages.${system}.reth-workflow;
            cosmwasm-workflow = workflows.packages.${system}.cosmwasm-workflow;
            all-workflows = workflows.packages.${system}.all-workflows;
          } // (if project != null then {
            # crate2nix-generated packages (only available when Cargo.nix exists)
            almanac = project.workspaceMembers.almanac.build;
            indexer-core = project.workspaceMembers.indexer-core.build;
            indexer-storage = project.workspaceMembers.indexer-storage.build;
            indexer-ethereum = project.workspaceMembers.indexer-ethereum.build;
            indexer-cosmos = project.workspaceMembers.indexer-cosmos.build;
            indexer-api = project.workspaceMembers.indexer-api.build;
            indexer-pipeline = project.workspaceMembers.indexer-pipeline.build;
            indexer-query = project.workspaceMembers.indexer-query.build;
            indexer-tools = project.workspaceMembers.indexer-tools.build;
            indexer-common = project.workspaceMembers.indexer-common.build;
            indexer-benchmarks = project.workspaceMembers.indexer-benchmarks.build;
          } else {});
          
          # Add apps for workflow environments
          apps = {
            default = workflows.apps.${system}.default; # Use workflow menu as the default
            workflow-menu = workflows.apps.${system}.workflow-menu;
            anvil-workflow = workflows.apps.${system}.anvil-workflow;
            reth-workflow = workflows.apps.${system}.reth-workflow;
            cosmwasm-workflow = workflows.apps.${system}.cosmwasm-workflow;
            all-workflows = workflows.apps.${system}.all-workflows;
          };
          
        in
        {
          # Define minimal outputs needed to get the shell working
          packages = packages;
          apps = apps;
          devShells.default = basicDevShell;
          formatter = pkgs.nixpkgs-fmt;
        };
    };
}