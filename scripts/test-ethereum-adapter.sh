#!/usr/bin/env bash
set -e

# Ensure the CARGO_TARGET_DIR (set by the Nix wrapper) exists
if [ -z "$CARGO_TARGET_DIR" ]; then
  echo "Error: CARGO_TARGET_DIR environment variable is not set." >&2
  exit 1
fi
mkdir -p "$CARGO_TARGET_DIR"

# Script to run Ethereum adapter tests against local Anvil node
echo "Running Ethereum adapter tests..."

# Make sure we're in the project root directory
# cd "$(dirname "$0")/.."

# Start local Anvil node if it's not already running
if ! pgrep -x "anvil" > /dev/null; then
    echo "Starting local Anvil node..."
    anvil --port 8545 &
    ANVIL_PID=$!
    # Give it time to start
    sleep 2
    echo "Anvil node started with PID $ANVIL_PID"
    # Register cleanup function to kill Anvil on exit
    function cleanup {
        echo "Stopping Anvil node..."
        kill $ANVIL_PID
    }
    trap cleanup EXIT
else
    echo "Using already running Anvil node"
fi

# Enable sqlx offline mode for compile-time checks
export SQLX_OFFLINE=true

# Remove cd commands
# PROJECT_ROOT=$(pwd)

# Run the Ethereum adapter tests using --manifest-path
# Assume nix run is executed from project root
cargo run --manifest-path crates/storage/Cargo.toml --bin test_ethereum_adapter 