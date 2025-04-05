# This module adds CosmWasm development tools to the flake
{ inputs, ... }:
{
  flake = {
    # Define flake-level outputs if needed 
  };

  perSystem = { config, self', pkgs, system, ... }:
  let
    # --- Build wasmd from source correctly ---
    wasmd = pkgs.buildGoModule {
      pname = "wasmd";
      version = "0.31.0";
      src = inputs.wasmd-src;
      vendorHash = "sha256-sQWTbr/blbdK1MFGCgpDhyBi67LnBh/H9VVVRAJQJBA=";
      subPackages = [ "cmd/wasmd" ];
      
      # Download and extract wasmvm library for Apple Silicon
      postPatch = ''
        mkdir -p $out/lib
        
        # Download the prebuilt library for Apple Silicon
        ${pkgs.curl}/bin/curl --cacert ${pkgs.cacert}/etc/ssl/certs/ca-bundle.crt -L \
          -o libwasmvm_darwin.tar.gz \
          https://github.com/CosmWasm/wasmvm/releases/download/v2.0.0/libwasmvm_darwin_arm64.tar.gz
          
        ${pkgs.gnutar}/bin/tar -xzf libwasmvm_darwin.tar.gz
        
        # Copy to output lib directory
        cp libwasmvm.dylib $out/lib/
        
        # Make sure the Go code can find the library
        sed -i -e 's@".*libwasmvm.dylib"@"libwasmvm.dylib"@g' vendor/github.com/CosmWasm/wasmvm/v*/libwasmvm.go
      '';
      
      # Required for CosmWasm CGo components
      env = {
        CGO_ENABLED = "1";
        CGO_LDFLAGS = "-L$out/lib";
        CGO_CFLAGS = "-I$src/vendor/github.com/CosmWasm/wasmvm/v*/internal/api";
      };
      
      nativeBuildInputs = with pkgs; [
        pkg-config
        cacert
      ];
      
      # Fix the rpath in the final executable
      postInstall = ''
        for bin in $out/bin/*; do
          chmod +w $bin
          ${pkgs.darwin.cctools}/bin/install_name_tool -add_rpath $out/lib $bin
          ${pkgs.darwin.cctools}/bin/install_name_tool -change @rpath/libwasmvm.dylib $out/lib/libwasmvm.dylib $bin
        done
      '';
    };

    # --- Create a script to run a wasmd test node --- 
    run-wasmd-node = pkgs.writeShellScriptBin "run-wasmd-node" ''
      set -euo pipefail

      NODE_HOME="''${WASMD_HOME:-$HOME/.wasmd-test}"
      CHAIN_ID="wasmd-test-1"
      MONIKER="almanac-test-node"
      KEY_NAME="validator"
      KEY_MNEMONIC="clock post desk civil pottery foster expand merit dash seminar song memory figure uniform spice circle try happy obvious trash crime hybrid hood cushion"
      
      # Colors for terminal output
      GREEN='\033[0;32m'
      YELLOW='\033[0;33m'
      BLUE='\033[0;34m'
      RED='\033[0;31m'
      NC='\033[0m' # No Color
      
      echo -e "''${BLUE}Setting up wasmd test node at $NODE_HOME''${NC}"
      
      # Create pid file to track the background process
      PID_FILE="/tmp/wasmd-node.pid"

      # Cleanup function to handle script exit
      cleanup() {
          echo -e "''${YELLOW}Shutting down wasmd node...''${NC}"
          if [ -f "$PID_FILE" ]; then
              PID=$(cat $PID_FILE)
              if ps -p $PID > /dev/null; then
                  kill $PID
                  echo -e "''${GREEN}wasmd node process $PID terminated''${NC}"
              fi
              rm -f $PID_FILE
          fi
          echo -e "''${GREEN}wasmd node shutdown complete''${NC}"
          exit 0
      }

      # Set up the trap to call cleanup when the script exits
      trap cleanup SIGINT SIGTERM EXIT

      # Initialize node if needed
      if [ ! -d "$NODE_HOME/config" ]; then
        echo "Initializing wasmd node configuration in $NODE_HOME..."
        mkdir -p "$NODE_HOME"
        ${wasmd}/bin/wasmd init "$MONIKER" --chain-id "$CHAIN_ID" --home "$NODE_HOME"
        
        # Modify config.toml for test environment
        sed -i.bak 's/allow_duplicate_ip = false/allow_duplicate_ip = true/' "$NODE_HOME/config/config.toml"
        sed -i.bak 's/cors_allowed_origins = \[\]/cors_allowed_origins = ["*"]/' "$NODE_HOME/config/config.toml"
        sed -i.bak 's/^laddr = "tcp:\/\/127.0.0.1:26657"/laddr = "tcp:\/\/0.0.0.0:26657"/' "$NODE_HOME/config/config.toml"
        sed -i.bak 's/^timeout_commit = .*$/timeout_commit = "1000ms"/' "$NODE_HOME/config/config.toml"

        # Add validator key
        echo -e "$KEY_MNEMONIC" | ${wasmd}/bin/wasmd keys add "$KEY_NAME" --recover --home "$NODE_HOME"
        
        # Get validator address
        VALIDATOR_ADDR=$(${wasmd}/bin/wasmd keys show "$KEY_NAME" -a --home "$NODE_HOME")
        
        # Add genesis account
        ${wasmd}/bin/wasmd add-genesis-account "$VALIDATOR_ADDR" 10000000000stake --home "$NODE_HOME"
        
        # Create validator transaction
        ${wasmd}/bin/wasmd gentx "$KEY_NAME" 1000000stake --chain-id "$CHAIN_ID" --home "$NODE_HOME" 
        
        # Collect genesis transactions
        ${wasmd}/bin/wasmd collect-gentxs --home "$NODE_HOME"
        
        # Validate genesis
        ${wasmd}/bin/wasmd validate-genesis --home "$NODE_HOME"
      fi

      # Define log file path
      LOG_FILE="$NODE_HOME/wasmd-node.log"

      # Start the node
      echo "Starting wasmd node... Logs at $LOG_FILE"
      ${wasmd}/bin/wasmd start \
        --home "$NODE_HOME" \
        --rpc.laddr tcp://0.0.0.0:26657 \
        --grpc.address 0.0.0.0:9090 \
        --address tcp://0.0.0.0:26655 \
        --p2p.laddr tcp://0.0.0.0:26656 \
        --log_level info > "$LOG_FILE" 2>&1 &

      # Save the background process PID to the file
      NODE_PID=$!
      echo $NODE_PID > "$PID_FILE"

      echo -e "''${GREEN}wasmd node started in the background (PID: $NODE_PID)''${NC}"
      echo -e "''${YELLOW}Logs available at: $LOG_FILE''${NC}"
      echo -e "''${YELLOW}RPC Endpoint: http://localhost:26657''${NC}"
      echo -e "''${YELLOW}GRPC Endpoint: localhost:9090''${NC}"
      echo -e "''${YELLOW}Press Ctrl+C to stop the node''${NC}"

      # Wait indefinitely (until SIGINT/SIGTERM is received via trap)
      while true; do
          # Check if the process is still running
          if ! ps -p $NODE_PID > /dev/null; then
              echo -e "''${RED}wasmd node process (PID: $NODE_PID) stopped unexpectedly.''${NC}"
              echo "Check logs: $LOG_FILE"
              exit 1
          fi
          sleep 30 
      done
    '';

    # --- Create a script to run Cosmos adapter tests as a regular shell script ---
    test-cosmos-adapter = pkgs.writeShellScriptBin "test-cosmos-adapter" ''
      #!/usr/bin/env bash
      set -e

      # Script to run Cosmos adapter tests against local wasmd node
      echo "Running Cosmos adapter tests..."

      # Set environment variables for the tests
      export RUN_COSMOS_TESTS=1
      export COSMOS_TEST_ENDPOINT=http://localhost:26657

      # Check for running wasmd node
      if ! pgrep -f "wasmd start" > /dev/null; then
        echo "Warning: No running wasmd node detected!"
        echo "You should start one with 'run-wasmd-node' before running tests."
        echo "Press Ctrl+C to cancel, or Enter to continue anyway..."
        read -r
      fi

      # Run from the current directory
      echo "Running tests from: $(pwd)"
      
      # Run the tests
      cargo test -p indexer-cosmos -- --nocapture
      
      echo "All Cosmos adapter tests completed!"
    '';
  in {
    # Expose packages
    packages = {
      wasmd = wasmd;
      run-wasmd-node = run-wasmd-node;
      test-cosmos-adapter = test-cosmos-adapter;
    };
    
    # Create a combined shell for CosmWasm development
    devShells.cosmos = pkgs.mkShell {
      packages = [
        wasmd
        run-wasmd-node
        test-cosmos-adapter
        pkgs.jq
      ];
      
      shellHook = ''
        echo "=== Almanac CosmWasm Development Shell ==="
        echo "Available commands:"
        echo "  - run-wasmd-node: Start a local wasmd node for testing"
        echo "  - test-cosmos-adapter: Run cosmos adapter tests against local node"
      '';
    };
  };
}
