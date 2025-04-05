# This module adds CosmWasm development tools to our flake
{ self, inputs, ... }:
{
  flake = {
    # Add overlay
    overlays.default = final: prev: {
      # Add our cosmos packages
      almanac-cosmos = self.packages.${prev.system};
    };
  };

  # Define per-system outputs
  perSystem = { config, self', inputs', pkgs, system, ... }: {
    # Define packages for this system
    packages = {
      # Shell script to install wasmd via Go
      wasmd-setup = pkgs.writeShellApplication {
        name = "wasmd-setup";
        runtimeInputs = with pkgs; [ go cacert jq ];
        text = ''
          #!/usr/bin/env bash
          set -euo pipefail
          
          echo "Installing wasmd v0.31.0 via Go..."
          
          # Install wasmd via Go
          export GOPATH="$HOME/go"
          export PATH="$GOPATH/bin:$PATH"
          
          go install github.com/CosmWasm/wasmd/cmd/wasmd@v0.31.0
          
          # Check installation
          if [ -f "$GOPATH/bin/wasmd" ]; then
            echo "✓ wasmd installed successfully at $GOPATH/bin/wasmd"
            wasmd version
          else
            echo "✗ Failed to install wasmd"
            exit 1
          fi
        '';
      };
      
      # Shell script to run a wasmd test node
      wasmd-node = pkgs.writeShellApplication {
        name = "wasmd-node";
        runtimeInputs = with pkgs; [ jq ];
        text = ''
          #!/usr/bin/env bash
          set -euo pipefail
          
          # Check if wasmd is installed
          if ! command -v wasmd >/dev/null; then
            echo "wasmd not found. Please run wasmd-setup first."
            exit 1
          fi
          
          # Set up wasmd test node
          TEST_DIR="$HOME/.wasmd-test"
          echo "Setting up wasmd test node at $TEST_DIR"
          
          # Create test directory if it doesn't exist
          mkdir -p "$TEST_DIR"
          
          # Initialize wasmd node config if it doesn't exist
          if [ ! -d "$TEST_DIR/config" ]; then
            wasmd init --chain-id=testing testing --home="$TEST_DIR"
            
            # Configure node
            wasmd config chain-id testing --home="$TEST_DIR"
            wasmd config keyring-backend test --home="$TEST_DIR"
            wasmd config broadcast-mode block --home="$TEST_DIR"
            
            # Create test accounts
            wasmd keys add validator --keyring-backend=test --home="$TEST_DIR"
            VALIDATOR_ADDR=$(wasmd keys show validator -a --keyring-backend=test --home="$TEST_DIR")
            wasmd add-genesis-account "$VALIDATOR_ADDR" 1000000000stake,1000000000validatortoken --home="$TEST_DIR"
            wasmd gentx validator 1000000stake --chain-id=testing --keyring-backend=test --home="$TEST_DIR"
            wasmd collect-gentxs --home="$TEST_DIR"
          fi
          
          # Check if a wasmd node is already running
          PID_FILE="$TEST_DIR/wasmd.pid"
          if [ -f "$PID_FILE" ]; then
            PID=$(cat "$PID_FILE")
            if ps -p "$PID" > /dev/null; then
              kill "$PID"
              echo "Stopped existing wasmd node (PID $PID)"
            fi
            rm -f "$PID_FILE"
          fi
          
          # Start the wasmd node
          echo "Starting wasmd node..."
          wasmd start --home="$TEST_DIR" &
          echo $! > "$PID_FILE"
          
          # Give node time to start up
          sleep 2
          
          # Show node status
          echo "Testing node connection..."
          wasmd status --node=tcp://localhost:26657 | jq '.node_info.network, .sync_info.latest_block_height'
          
          echo ""
          echo "wasmd node is running!"
          echo "RPC URL: http://localhost:26657"
          echo "REST URL: http://localhost:1317"
          echo "Chain ID: testing"
          echo ""
          echo "Press Ctrl+C to stop the node"
          echo ""
          
          # Wait for user to press Ctrl+C
          wait $!
        '';
      };
      
      # Script to run cosmos adapter tests against local node
      test-cosmos-adapter = pkgs.writeShellApplication {
        name = "test-cosmos-adapter";
        runtimeInputs = with pkgs; [ jq ];
        text = ''
          #!/usr/bin/env bash
          set -euo pipefail
          
          # Check if a wasmd node is running
          if ! curl -s http://localhost:26657/status > /dev/null; then
            echo "No wasmd node found at http://localhost:26657"
            echo "Please start a wasmd node first with: wasmd-node"
            exit 1
          fi
          
          echo "Running Cosmos adapter tests against local wasmd node..."
          
          # Set necessary environment variables
          export COSMOS_RPC_URL="http://localhost:26657"
          
          # TODO: Run the actual test command here
          echo "Running tests..."
          echo "(Test command placeholder - implement actual test command)"
          
          echo "Tests completed."
        '';
      };
    };
  };
}
