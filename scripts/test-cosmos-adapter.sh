#!/usr/bin/env bash
set -e

# Script to run Cosmos adapter tests against local UFO node
echo "Running Cosmos adapter tests..."

# Make sure we're in the project root directory
cd "$(dirname "$0")/.."

# Define expected path for built UFO node
UFO_NODE_BINARY="./ufo-patched-osmosis/bin/ufo-node" # Adjust if build output path differs

# Check if the UFO node binary exists
if [ ! -f "$UFO_NODE_BINARY" ]; then
    echo "Error: UFO node binary not found at $UFO_NODE_BINARY"
    echo "Please build the UFO node first using:"
    echo "  nix run .#ufo:build-osmosis-ufo -- /path/to/osmosis/source"
    # Replace /path/to/osmosis/source with the actual path, e.g., from nix/ufo-module.nix or config
    # We might need to figure out the actual source path expected by the build command.
    # For now, using a placeholder. Let's assume it's /tmp/osmosis-source as per flake.nix example.
    echo "Example: nix run .#ufo:build-osmosis-ufo -- /tmp/osmosis-source"
    exit 1
fi

# Start local UFO node if it's not already running
if ! pgrep -f "$UFO_NODE_BINARY" > /dev/null; then
    echo "Starting local UFO node from $UFO_NODE_BINARY..."
    # Run the binary directly
    "$UFO_NODE_BINARY" --home ./data/ufo & # Add necessary flags like --home if needed
    UFO_PID=$!
    # Give it time to start
    sleep 5
    # Check if the process actually started
    if ! kill -0 $UFO_PID > /dev/null 2>&1; then
      echo "Error: Failed to start UFO node."
      exit 1
    fi
    echo "UFO node started with PID $UFO_PID"
    # Register cleanup function to kill UFO node on exit
    function cleanup {
        echo "Stopping UFO node..."
        kill $UFO_PID || true # Use || true to ignore error if already stopped
    }
    trap cleanup EXIT
else
    echo "Using already running UFO node"
fi

# Run the Cosmos adapter tests
# Ensure tests run from the correct directory if needed, assuming project root is fine
# or adjust cd command if tests expect a specific working dir like 'crates/storage'
echo "Running tests from directory: $(pwd)"
# If tests are in crates/storage and expect to be run from there:
# cd crates/storage || exit 1
# cargo run --bin test_cosmos_adapter
# Running from root for now, adjust if needed:
cargo run --bin test_cosmos_adapter --manifest-path crates/storage/Cargo.toml 