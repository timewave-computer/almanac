#!/usr/bin/env bash
set -e

# Script to run Cosmos adapter tests against local UFO node
echo "Running Cosmos adapter tests..."

# Make sure we're in the project root directory
cd "$(dirname "$0")/.."

# Start local UFO node if it's not already running
if ! pgrep -x "ufo-node" > /dev/null; then
    echo "Starting local UFO node..."
    # Run the node setup script from the flake
    nix run .#run-ufo-node &
    UFO_PID=$!
    # Give it time to start
    sleep 5
    echo "UFO node started with PID $UFO_PID"
    # Register cleanup function to kill UFO node on exit
    function cleanup {
        echo "Stopping UFO node..."
        kill $UFO_PID
    }
    trap cleanup EXIT
else
    echo "Using already running UFO node"
fi

# Run the Cosmos adapter tests
cd crates/storage && cargo run --bin test_cosmos_adapter 