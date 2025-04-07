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
      inputs.rust-overlay.follows = "rust-overlay";
    };
    foundry = {
      url = "github:shazow/foundry.nix/monthly"; # Use monthly for stability
      inputs.nixpkgs.follows = "nixpkgs";
      inputs.flake-parts.follows = "flake-parts";
    };
  };

  outputs = inputs@{ self, nixpkgs, flake-parts, fenix, crane, rust-overlay, foundry, ... }:
    # Create a simplified flake that directly specifies outputs without using modules
    flake-parts.lib.mkFlake { inherit self inputs; } {
      systems = [ "aarch64-darwin" "x86_64-linux" ]; # Add systems as needed
      
      # Include the database-module from the local nix directory
      imports = [];
      
      perSystem = { config, self', inputs', pkgs, system, ... }:
        let
          # Use pkgs.lib for convenience
          lib = pkgs.lib;
          
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
          
          # Add PostgreSQL to the basic development shell
          basicDevShell = pkgs.mkShell {
            packages = [ 
              rustToolchain 
              pkgs.pkg-config 
              pkgs.openssl
              pkgs.postgresql_15
              pkgs.sqlx-cli
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
              
              # Database commands - we're exposing the database scripts
              echo "Using simplified development environment"
              echo "rust version: $(rustc --version)"
              echo ""
              echo "Database commands available:"
              echo "  init_databases       - Initialize and start PostgreSQL and RocksDB"
              echo "  stop_databases       - Stop PostgreSQL server"
              echo ""
              
              # Expose the database commands as shell functions
              function init_databases {
                ${initDatabasesScript}/bin/init-databases
              }
              
              function stop_databases {
                ${stopDatabasesScript}/bin/stop-databases
              }
              
              export -f init_databases
              export -f stop_databases
              
              # Check PostgreSQL status and start if needed
              if ! pg_isready -q; then
                echo "PostgreSQL is not running. Run 'init_databases' to start it."
              else 
                echo "PostgreSQL is running at $PGHOST:$PGPORT"
                echo "Database: $PGDATABASE"
                echo "Connection URL: $DATABASE_URL"
              fi
            '';
          };
          
          # Use wasm-bindgen-cli from nixpkgs instead of building from source
          packages = {
            wasm-bindgen-pkg = pkgs.wasm-bindgen-cli;
            init-databases = initDatabasesScript;
            stop-databases = stopDatabasesScript;
            default = pkgs.wasm-bindgen-cli;
          };
          
        in
        {
          # Define minimal outputs needed to get the shell working
          packages = packages;
          devShells.default = basicDevShell;
          formatter = pkgs.nixpkgs-fmt;
        };
    };
}