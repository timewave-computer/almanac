# This file defines a Nix flake for the Almanac project with CosmWasm support.
{
  description = "Almanac Project Root";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-parts.url = "github:hercules-ci/flake-parts";
    # No longer need direct reth input
    # reth.url = "github:paradigmxyz/reth/main";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    
    # Input needed for flake reference (not used directly anymore)
    wasmd-src = {
      url = "github:CosmWasm/wasmd/v0.31.0";
      flake = false;
    };
    foundry = {
      url = "github:foundry-rs/foundry";
    };
  };

  outputs = inputs@{ self, nixpkgs, flake-parts, wasmd-src, foundry, ... }:
    flake-parts.lib.mkFlake { inherit inputs; } {
      # Apply nixpkgs config and overlays here
      flake = { 
        nixpkgs.config = {
          allowUnfree = true;
          allowUnsupportedSystem = true;
        };
      };
      # Import our modules
      imports = [
        ./nix/cosmos-module.nix
        ./nix/database-module.nix
        ./nix/cross-chain-module.nix
      ];
      
      systems = ["aarch64-darwin" "x86_64-linux"];

      # Define perSystem configuration
      perSystem = { config, self', inputs', system, ... }:
        let 
          # Apply overlay to pkgs for this system
          pkgs = import inputs.nixpkgs {
            inherit system;
            overlays = [ 
              inputs.rust-overlay.overlays.default
            ];
            config = {
              allowUnfree = true;
              allowUnsupportedSystem = true;
            };
          };

          # Define foundry package from nixpkgs
          foundryPkg = pkgs.foundry;

          # Define Reth build logic based on their flake
          rethSrc = pkgs.fetchFromGitHub {
            owner = "paradigmxyz";
            repo = "reth";
            rev = "v1.3.7"; # Use tag name instead of hash
            hash = "sha256-nqahs6zGQG/qG6Qe/tKNIPGLIiQcng1zDZFKrUBpoiM="; # Correct hash
            fetchSubmodules = true;
          };
          cargoTOML = (builtins.fromTOML (builtins.readFile "${rethSrc}/Cargo.toml"));
          packageVersion = cargoTOML.workspace.package.version;
          # Use a specific version known to be available via rust-overlay
          rustVersion = cargoTOML.workspace.package.rust-version;
          rustPkg = pkgs.rust-bin.stable."1.85.0".default.override {
            extensions = [ "rust-src" "rust-analyzer" "clippy" "rustfmt" ];
          };
          macPackages = pkgs.lib.optionals pkgs.stdenv.isDarwin (with pkgs.darwin.apple_sdk.frameworks; [ Security CoreFoundation CoreServices ]);
          linuxPackages = pkgs.lib.optionals pkgs.stdenv.isLinux (with pkgs; [
            libclang.lib
            llvmPackages.libcxxClang
          ]);
          cargoDeps = pkgs.rustPlatform.importCargoLock {
            lockFile = "${rethSrc}/Cargo.lock";
          };
          rustPlatform = pkgs.makeRustPlatform {
            rustc = rustPkg;
            cargo = rustPkg;
          };

          # Define scripts for test apps
          testEthAnvilScript = pkgs.writeShellScript "test-eth-anvil-runner" ''
            export ETH_RPC_URL="http://127.0.0.1:8545"
            # Set target dir to a writable temporary location (use escaped $)
            export CARGO_TARGET_DIR="\$TMPDIR/cargo-target-anvil"
            exec "${self}/scripts/test-ethereum-adapter.sh" "$@"
          '';
          testEthRethScript = pkgs.writeShellScript "test-eth-reth-runner" ''
            export ETH_RPC_URL="http://127.0.0.1:8545" # Assuming default reth port
            # Set target dir to a writable temporary location (use escaped $)
            export CARGO_TARGET_DIR="\$TMPDIR/cargo-target-reth"
            exec "${self}/scripts/test-ethereum-adapter.sh" "$@"
          '';

        in
        {
          # Create the default development shell
          devShells.default = pkgs.mkShell {
            packages = [ 
              pkgs.git 
              # Include essential cosmos packages
              self'.packages.wasmd-node-fixed
              self'.packages.test-cosmos-adapter
              # Include Ethereum tools
              foundryPkg # Provides anvil
              self'.packages.reth-pkg # Use our manually built reth
              # General dev tools
              pkgs.jq
              pkgs.go
              pkgs.curl
              pkgs.gzip
              pkgs.sqlx-cli
              pkgs.postgresql # Add PostgreSQL server package
            ];
            
            shellHook = ''
              echo "=== Almanac Development Environment ===="
              echo "Available shell commands:"
              echo "  (Cosmos)"
              echo "  - wasmd-node: Start a local wasmd node for testing"
              echo "  - test-cosmos-adapter: Run cosmos adapter tests against local node"
              echo "  (Ethereum)"
              echo "  - anvil: Start local Ethereum test node"
              echo "  - reth node: Start Reth Ethereum node (requires config)"
              echo "  - test-ethereum-adapter-anvil: Run tests against anvil"
              echo "  - test-ethereum-adapter-reth: Run tests against reth"
              echo "  (Database)"
              echo "  - init-databases: Initialize PostgreSQL and RocksDB"
              echo "  - stop-databases: Gracefully stop running databases"
              echo "  - test-databases: Test database connectivity"
              echo "  - wipe-databases: Completely wipe all database data"
              echo ""
              echo "Available nix run commands:"
              echo "  (Cosmos)"
              echo "  - nix run .#wasmd-node"
              echo "  - nix run .#test-cosmos-adapter"
              echo "  (Ethereum)"
              echo "  - nix run .#start-anvil"
              echo "  - nix run .#start-reth"
              echo "  - nix run .#test-ethereum-adapter-anvil"
              echo "  - nix run .#test-ethereum-adapter-reth"
              echo "  (Database)"
              echo "  - nix run .#init-databases"
              echo "  - nix run .#stop-databases"
              echo "  - nix run .#test-databases"
              echo "  - nix run .#wipe-databases"
            '';
          };

          # Define packages needed for apps
          packages = {
            # Build reth manually
            reth-pkg = rustPlatform.buildRustPackage {
              pname = "reth";
              version = packageVersion;
              cargoLock = {
                lockFile = "${rethSrc}/Cargo.lock";
              };
              checkFlags = [
                #this test breaks Read Only FS sandbox
                "--skip=cli::tests::parse_env_filter_directives"
              ];
              LIBCLANG_PATH = "${pkgs.llvmPackages.libclang.lib}/lib";
              nativeBuildInputs = (with pkgs;[ libclang ]) ++ macPackages ++ linuxPackages;
              src = rethSrc;
            };

            # Simple wrapper for anvil
             start-anvil = pkgs.stdenv.mkDerivation {
               name = "start-anvil";
               src = pkgs.lib.cleanSource ./.;
               buildInputs = [ pkgs.makeWrapper foundryPkg ];
               installPhase = ''
                 mkdir -p $out/bin
                 makeWrapper ${foundryPkg}/bin/anvil $out/bin/start-anvil
               '';
             };
             # Simple wrapper for reth
             start-reth = pkgs.stdenv.mkDerivation {
               name = "start-reth";
               src = pkgs.lib.cleanSource ./.;
               buildInputs = [ pkgs.makeWrapper self'.packages.reth-pkg pkgs.openssl ]; 
               installPhase = ''
                 mkdir -p $out/bin
                 makeWrapper ${self'.packages.reth-pkg}/bin/reth $out/bin/start-reth --add-flags "node"
               '';
             };
             
             # Create a fixed wasmd-node wrapper
             wasmd-node-fixed = pkgs.writeShellScriptBin "wasmd-node" ''
               set -e

               WASMD_HOME="''${WASMD_HOME:-$HOME/.wasmd-test}"
               echo "Setting up wasmd node at $WASMD_HOME"

               # Stop any existing wasmd processes
               echo "Stopping any existing wasmd processes..."
               pkill -f "wasmd start" || true
               sleep 1

               # Check for wasmd binary
               if ! command -v wasmd &> /dev/null; then
                 echo "wasmd not found. Installing..."
                 sudo wget -O /usr/local/bin/wasmd https://github.com/CosmWasm/wasmd/releases/download/v0.30.0/wasmd-0.30.0-darwin-arm64
                 sudo chmod +x /usr/local/bin/wasmd
               fi

               # Optionally purge old data if requested
               if [ "$1" = "--purge" ]; then
                 echo "Purging old data..."
                 rm -rf "$WASMD_HOME"
               fi

               # Initialize wasmd chain if needed
               if [ ! -d "$WASMD_HOME" ] || [ ! -f "$WASMD_HOME/config/genesis.json" ]; then
                 echo "Initializing wasmd chain..."
                 mkdir -p "$WASMD_HOME"
                 wasmd init --chain-id=wasmchain testing --home="$WASMD_HOME"
                 
                 # Configure basic settings
                 wasmd config chain-id wasmchain --home="$WASMD_HOME"
                 wasmd config keyring-backend test --home="$WASMD_HOME"
                 wasmd config broadcast-mode block --home="$WASMD_HOME"
                 wasmd config node tcp://127.0.0.1:26657 --home="$WASMD_HOME"

                 # Create validator account
                 wasmd keys add validator --keyring-backend=test --home="$WASMD_HOME" || {
                   echo "Validator key may already exist, attempting to continue..."
                 }
                 
                 # Get validator address
                 VALIDATOR_ADDR=$(wasmd keys show validator -a --keyring-backend=test --home="$WASMD_HOME")
                 echo "Validator address: $VALIDATOR_ADDR"
                 
                 # Add genesis account
                 wasmd add-genesis-account "$VALIDATOR_ADDR" 1000000000stake,1000000000validatortoken --home="$WASMD_HOME"
                 
                 # Generate genesis transaction
                 wasmd gentx validator 1000000stake --chain-id=wasmchain --keyring-backend=test --home="$WASMD_HOME"
                 
                 # Collect genesis transactions
                 wasmd collect-gentxs --home="$WASMD_HOME"
               fi

               # Fix app.toml settings for local development
               APP_TOML="$WASMD_HOME/config/app.toml"
               echo "Updating app.toml settings..."
               
               # Configure API settings
               sed -i'.bak' 's|^enable = false|enable = true|g' "$APP_TOML"
               sed -i'.bak' 's|^swagger = false|swagger = true|g' "$APP_TOML"
               sed -i'.bak' 's|^enabled-unsafe-cors = false|enabled-unsafe-cors = true|g' "$APP_TOML"
               sed -i'.bak' 's|^address = "tcp://0.0.0.0:1317"|address = "tcp://0.0.0.0:1317"|g' "$APP_TOML"
               
               # Configure GRPC settings
               sed -i'.bak' 's|^address = "0.0.0.0:9090"|address = "0.0.0.0:9090"|g' "$APP_TOML"
               sed -i'.bak' 's|^enable = true|enable = true|g' "$APP_TOML"
               
               # Set minimum gas prices to avoid warnings
               sed -i'.bak' 's|^minimum-gas-prices = ""|minimum-gas-prices = "0.025stake"|g' "$APP_TOML"

               # Fix config.toml settings for local development
               CONFIG_TOML="$WASMD_HOME/config/config.toml"
               echo "Updating config.toml settings for improved local performance..."
               
               # Timeout settings to avoid validator timeout issues
               sed -i'.bak' 's|^timeout_commit = "5s"|timeout_commit = "1s"|g' "$CONFIG_TOML"
               sed -i'.bak' 's|^timeout_propose = "3s"|timeout_propose = "10s"|g' "$CONFIG_TOML"
               sed -i'.bak' 's|^timeout_precommit = "1s"|timeout_precommit = "10s"|g' "$CONFIG_TOML"
               sed -i'.bak' 's|^timeout_prevote = "1s"|timeout_prevote = "10s"|g' "$CONFIG_TOML"
               sed -i'.bak' 's|^skip_timeout_commit = false|skip_timeout_commit = true|g' "$CONFIG_TOML"
               sed -i'.bak' 's|^timeout_broadcast_tx_commit = "10s"|timeout_broadcast_tx_commit = "30s"|g' "$CONFIG_TOML"
               
               # Network settings to avoid validator connection issues
               sed -i'.bak' 's|^addr_book_strict = true|addr_book_strict = false|g' "$CONFIG_TOML"
               sed -i'.bak' 's|^allow_duplicate_ip = false|allow_duplicate_ip = true|g' "$CONFIG_TOML"
               sed -i'.bak' 's|^max_num_outbound_peers = 10|max_num_outbound_peers = 5|g' "$CONFIG_TOML"
               sed -i'.bak' 's|^max_num_inbound_peers = 40|max_num_inbound_peers = 5|g' "$CONFIG_TOML"
               sed -i'.bak' 's|^flush_throttle_timeout = "100ms"|flush_throttle_timeout = "10ms"|g' "$CONFIG_TOML"
               
               # Most importantly - disable private validator socket to prevent errors
               sed -i'.bak' 's|^priv_validator_laddr = ".*"|priv_validator_laddr = ""|g' "$CONFIG_TOML"

               # Check and ensure priv_validator_key.json and priv_validator_state.json exist and are set up correctly
               PRIV_VAL_KEY="$WASMD_HOME/config/priv_validator_key.json"
               PRIV_VAL_STATE="$WASMD_HOME/data/priv_validator_state.json"
               
               # Make sure the data directory exists
               mkdir -p "$WASMD_HOME/data"
               
               # Ensure priv_validator_state.json is in the correct location with correct permissions
               if [ ! -f "$PRIV_VAL_STATE" ]; then
                 echo "Creating priv_validator_state.json..."
                 echo '{
                   "height": "0",
                   "round": 0,
                   "step": 0
                 }' > "$PRIV_VAL_STATE"
                 chmod 600 "$PRIV_VAL_STATE"
               fi
               
               # Start the node with appropriate flags
               echo "Starting wasmd node..."
               wasmd start \
                 --home "$WASMD_HOME" \
                 --rpc.laddr "tcp://0.0.0.0:26657" \
                 --grpc.address "0.0.0.0:9090" \
                 --address "tcp://0.0.0.0:26656" \
                 --log_level debug \
                 > "$WASMD_HOME/node.log" 2>&1 &
               
               PID=$!
               echo $PID > "$WASMD_HOME/node.pid"
               echo "Node started with PID: $PID"
               echo "Logs available at: $WASMD_HOME/node.log"
             '';
          };

          # Define runnable applications
          apps = {
            # Cosmos Apps
            wasmd-node = {
              type = "app";
              program = "${self'.packages.wasmd-node-fixed}/bin/wasmd-node";
            };
            test-cosmos-adapter = {
              type = "app";
              program = "${self'.packages.test-cosmos-adapter}/bin/test-cosmos-adapter";
            };
            # Ethereum Apps
            start-anvil = {
              type = "app";
              program = "${self'.packages.start-anvil}/bin/start-anvil";
            };
            start-reth = {
              type = "app";
              program = "${self'.packages.start-reth}/bin/start-reth";
            };
             # Define test apps directly with inline script
             test-ethereum-adapter-anvil = {
              type = "app";
              program = "${testEthAnvilScript}"; # Reference the script derivation
            };
            test-ethereum-adapter-reth = {
              type = "app";
              program = "${testEthRethScript}"; # Reference the script derivation
            };
            test-valence-contracts = {
              type = "app";
              program = "${pkgs.writeShellScript "test-valence-contracts-runner" ''
                cd ${self}
                source ${self}/scripts/test-valence-contracts.sh "$@"
              ''}";
            };
            test-valence-real-contracts = {
              type = "app";
              program = "${pkgs.writeShellScript "test-valence-real-contracts-runner" ''
                cd ${self}
                source ${self}/scripts/test-valence-real-contracts.sh "$@"
              ''}";
            };
            cross-chain-e2e-test = {
              type = "app";
              program = "${pkgs.writeShellScript "cross-chain-e2e-test-runner" ''
                cd ${self}
                # Make sure Go is available
                export PATH="$HOME/go/bin:$PATH"
                # Install wasmd via Go if not already installed
                if ! command -v wasmd > /dev/null 2>&1; then
                  echo "Installing wasmd via Go..."
                  export GOBIN="$HOME/go/bin"
                  mkdir -p $GOBIN
                  go install github.com/CosmWasm/wasmd/cmd/wasmd@v0.31.0
                fi
                # Copy pre-compiled WASM files to test directory
                source ${self}/scripts/fix_e2e_test_wasm.sh
                # Run the test script
                source ${self}/scripts/cross_chain_e2e_test.sh "$@"
              ''}";
            };
          };
        };
    };
}