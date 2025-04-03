{
  description = "Cross-Chain Indexer";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    foundry = {
      url = "github:shazow/foundry.nix/monthly";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay, foundry }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ 
          (import rust-overlay)
          foundry.overlay
        ];
        pkgs = import nixpkgs { inherit system overlays; };
        
        # Define macOS deployment target
        darwinDeploymentTarget = "11.0";
        
        # Rust package with stable toolchain
        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          extensions = [ "rust-src" "rustfmt" "clippy" ];
        };

        # Common script environment
        commonScriptEnv = {
          MACOSX_DEPLOYMENT_TARGET = darwinDeploymentTarget;
          SOURCE_DATE_EPOCH = "1672531200";
          DATABASE_URL = "postgresql://postgres:postgres@localhost:5432/indexer";
          ROCKSDB_PATH = "./data/rocksdb";
          ETH_RPC_URL = "http://localhost:8545";
          RETH_DATA_DIR = "./data/reth";
          COSMOS_RPC_URL = "http://localhost:26657";
        };
      in
      {
        # Development shell
        devShells.default = pkgs.mkShell {
          buildInputs = with pkgs; [
            # Rust toolchain
            rustToolchain
            
            # Build dependencies
            rocksdb
            postgresql_15
            
            # Database utilities
            postgresql_15.lib
            pgcli
            
            # Ethereum dependencies
            foundry-bin  # Includes Anvil, Forge, Cast
            
            # Development tools
            jq
            curl
          ];
          
          # Set environment variables
          shellHook = ''
            # Set environment variables
            export MACOSX_DEPLOYMENT_TARGET="${darwinDeploymentTarget}"
            export SOURCE_DATE_EPOCH="1672531200"
            
            # Set database environment variables
            export DATABASE_URL="postgresql://postgres:postgres@localhost:5432/indexer"
            export ROCKSDB_PATH="./data/rocksdb"
            export ETH_RPC_URL="http://localhost:8545"
            export RETH_DATA_DIR="./data/reth"
            export COSMOS_RPC_URL="http://localhost:26657"
            
            # Ensure directories exist
            mkdir -p ./data/rocksdb
            mkdir -p ./data/reth
            mkdir -p ./data/postgres
            mkdir -p ./logs
            
            echo "=== Cross-Chain Indexer Development Shell ==="
            echo "Available commands:"
            echo "  nix run .#start-postgres     - Start PostgreSQL server"
            echo "  nix run .#start-anvil        - Start Ethereum test node (Anvil)"
            echo "  nix run .#run-ufo-node       - Start UFO node"
            echo "  nix run .#deploy-contracts   - Deploy test contracts to Anvil"
            echo "  nix run .#mint-tokens        - Mint tokens to an Ethereum address"
            echo "  nix run .#ufo-mint-tokens    - Mint tokens to a Cosmos address"
            echo "  nix run .#e2e-test           - Run Ethereum end-to-end test"
            echo "  nix run .#ufo-e2e-test       - Run UFO end-to-end test"
            echo "  nix run .#prepare-sqlx       - Prepare SQL migrations for sqlx"
            echo "  nix run .#run-all-nodes      - Start all nodes for development"
          '';
        };
        
        # Packages
        packages = {
          default = pkgs.writeScriptBin "cargo-check" ''
            #!/usr/bin/env bash
            export MACOSX_DEPLOYMENT_TARGET="${darwinDeploymentTarget}"
            export SOURCE_DATE_EPOCH="1672531200"
            ${pkgs.cargo}/bin/cargo check "$@"
          '';
          
          # Postgres management
          start-postgres = pkgs.writeShellScriptBin "start-postgres" ''
            #!/usr/bin/env bash
            set -euo pipefail
            
            echo "=== Starting PostgreSQL Server ==="
            
            # Setup variables
            PG_DATA_DIR="./data/postgres/pgdata"
            PG_LISTEN_ADDR="127.0.0.1"
            PG_PORT=5432
            PG_LOG_FILE="./data/postgres/logfile"
            
            # Check if PostgreSQL is already running
            if ${pkgs.postgresql_15}/bin/pg_isready -h $PG_LISTEN_ADDR -p $PG_PORT > /dev/null 2>&1; then
              echo "PostgreSQL server is already running on $PG_LISTEN_ADDR:$PG_PORT"
              
              # Create database if it doesn't exist
              export PGPASSWORD=postgres
              ${pkgs.postgresql_15}/bin/createdb -h $PG_LISTEN_ADDR -p $PG_PORT -U postgres indexer 2>/dev/null || true
              
              echo "Database URL: $DATABASE_URL"
              exit 0
            fi
            
            # Initialize PostgreSQL if needed
            if [ ! -d "$PG_DATA_DIR" ]; then
              echo "Initializing PostgreSQL database cluster..."
              mkdir -p "$PG_DATA_DIR"
              ${pkgs.postgresql_15}/bin/initdb -D "$PG_DATA_DIR" --username=postgres --pwfile=<(echo "postgres") --auth=trust
              
              # Configure PostgreSQL
              echo "listen_addresses = '$PG_LISTEN_ADDR'" >> "$PG_DATA_DIR/postgresql.conf"
              echo "port = $PG_PORT" >> "$PG_DATA_DIR/postgresql.conf"
              
              echo "PostgreSQL database cluster initialized"
            fi
            
            # Check for and remove stale lock files
            if [ -f "$PG_DATA_DIR/postmaster.pid" ]; then
              echo "Found stale lock file, removing..."
              rm -f "$PG_DATA_DIR/postmaster.pid"
            fi
            
            # Start PostgreSQL
            echo "Starting PostgreSQL server..."
            mkdir -p "$(dirname "$PG_LOG_FILE")"
            ${pkgs.postgresql_15}/bin/pg_ctl -D "$PG_DATA_DIR" -l "$PG_LOG_FILE" start
            
            # Wait for PostgreSQL to start
            echo "Waiting for PostgreSQL to start..."
            for i in {1..30}; do
              if ${pkgs.postgresql_15}/bin/pg_isready -h $PG_LISTEN_ADDR -p $PG_PORT > /dev/null 2>&1; then
                echo "PostgreSQL server started"
                break
              fi
              sleep 1
              if [ $i -eq 30 ]; then
                echo "Failed to start PostgreSQL server"
                echo "Check the log file: $PG_LOG_FILE"
                exit 1
              fi
            done
            
            # Create database if it doesn't exist
            export PGPASSWORD=postgres
            ${pkgs.postgresql_15}/bin/createdb -h $PG_LISTEN_ADDR -p $PG_PORT -U postgres indexer 2>/dev/null || true
            
            echo "PostgreSQL server started"
            echo "Database URL: $DATABASE_URL"
          '';

          # Ethereum node management 
          start-anvil = pkgs.writeShellScriptBin "start-anvil" ''
            #!/usr/bin/env bash
            set -euo pipefail
            
            echo "Starting Ethereum node (Anvil)..."
            ${pkgs.foundry-bin}/bin/anvil \
              --host 0.0.0.0 \
              --accounts 10 \
              --balance 10000 \
              --gas-limit 30000000 \
              --block-time 1
          '';
          
          # UFO node scripts
          run-ufo-node = pkgs.writeShellScriptBin "run-ufo-node" ''
            #!/usr/bin/env bash
            set -euo pipefail
            
            cd "$PWD"
            ./scripts/run-ufo-node.sh "$@"
          '';
          
          # Contract scripts
          deploy-contracts = pkgs.writeShellScriptBin "deploy-contracts" ''
            #!/usr/bin/env bash
            set -euo pipefail
            
            cd "$PWD"
            ./scripts/deploy-contracts.sh "$@"
          '';
          
          # Token management scripts
          mint-tokens = pkgs.writeShellScriptBin "mint-tokens" ''
            #!/usr/bin/env bash
            set -euo pipefail
            
            cd "$PWD"
            ./scripts/mint-tokens.sh "$@"
          '';
          
          ufo-mint-tokens = pkgs.writeShellScriptBin "ufo-mint-tokens" ''
            #!/usr/bin/env bash
            set -euo pipefail
            
            cd "$PWD"
            ./scripts/ufo-mint-tokens.sh "$@"
          '';
          
          # Test scripts
          e2e-test = pkgs.writeShellScriptBin "e2e-test" ''
            #!/usr/bin/env bash
            set -euo pipefail
            
            cd "$PWD"
            ./scripts/e2e-test.sh "$@"
          '';
          
          ufo-e2e-test = pkgs.writeShellScriptBin "ufo-e2e-test" ''
            #!/usr/bin/env bash
            set -euo pipefail
            
            cd "$PWD"
            ./scripts/ufo-e2e-test.sh "$@"
          '';

          # SQL preparation script
          prepare-sqlx = pkgs.writeShellScriptBin "prepare-sqlx" ''
            #!/usr/bin/env bash
            set -euo pipefail
            
            cd "$PWD"
            ./scripts/prepare-sqlx.sh "$@"
          '';
          
          # Run all nodes in development
          run-all-nodes = pkgs.writeShellScriptBin "run-all-nodes" ''
            #!/usr/bin/env bash
            set -euo pipefail
            
            echo "Starting all nodes for development..."
            
            # Start PostgreSQL
            ${self.packages.${system}.start-postgres}/bin/start-postgres &
            PG_PID=$!
            echo "PostgreSQL started with PID: $PG_PID"
            
            # Wait for PostgreSQL to be ready
            sleep 3
            
            # Start Ethereum node
            ${pkgs.foundry-bin}/bin/anvil \
              --host 0.0.0.0 \
              --accounts 10 \
              --balance 10000 \
              --gas-limit 30000000 \
              --block-time 1 > ./logs/anvil.log 2>&1 &
            ANVIL_PID=$!
            echo "Anvil started with PID: $ANVIL_PID"
            
            # Wait for Anvil to be ready
            sleep 3
            
            # Deploy contracts
            ./scripts/deploy-contracts.sh > ./logs/deploy.log 2>&1
            
            # Start UFO node
            ./scripts/run-ufo-node.sh --block-time 100 > ./logs/ufo.log 2>&1 &
            UFO_PID=$!
            echo "UFO node started with PID: $UFO_PID"
            
            echo "All nodes started"
            echo "Press Ctrl+C to stop all nodes"
            
            # Setup trap to kill all processes
            function cleanup {
              echo "Stopping all nodes..."
              kill $PG_PID || true
              kill $ANVIL_PID || true
              kill $UFO_PID || true
              wait
              echo "All nodes stopped"
            }
            
            trap cleanup EXIT
            
            # Wait for Ctrl+C
            while true; do
              sleep 1
            done
          '';
        };
        
        # Apps
        apps = {
          default = self.apps.${system}.start-postgres;
          
          start-postgres = {
            type = "app";
            program = "${self.packages.${system}.start-postgres}/bin/start-postgres";
          };
          
          start-anvil = {
            type = "app";
            program = "${self.packages.${system}.start-anvil}/bin/start-anvil";
          };
          
          run-ufo-node = {
            type = "app";
            program = "${self.packages.${system}.run-ufo-node}/bin/run-ufo-node";
          };
          
          deploy-contracts = {
            type = "app";
            program = "${self.packages.${system}.deploy-contracts}/bin/deploy-contracts";
          };
          
          mint-tokens = {
            type = "app";
            program = "${self.packages.${system}.mint-tokens}/bin/mint-tokens";
          };
          
          ufo-mint-tokens = {
            type = "app";
            program = "${self.packages.${system}.ufo-mint-tokens}/bin/ufo-mint-tokens";
          };
          
          e2e-test = {
            type = "app";
            program = "${self.packages.${system}.e2e-test}/bin/e2e-test";
          };
          
          ufo-e2e-test = {
            type = "app";
            program = "${self.packages.${system}.ufo-e2e-test}/bin/ufo-e2e-test";
          };
          
          prepare-sqlx = {
            type = "app";
            program = "${self.packages.${system}.prepare-sqlx}/bin/prepare-sqlx";
          };
          
          run-all-nodes = {
            type = "app";
            program = "${self.packages.${system}.run-all-nodes}/bin/run-all-nodes";
          };
        };
      }
    );
}
