#!/usr/bin/env bash

set -euo pipefail
set -u # Exit on unset variables

# Default values
BUILD_MODE="patched"
VALIDATORS=1
BLOCK_TIME=1000
# OSMOSIS_SOURCE="/tmp/osmosis-source" # Replaced by argument parsing
UFO_OSMOSIS_SOURCE_PATH="" # Initialize variable

# Colors for terminal output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Function to show help text
show_help() {
    echo "Usage: $0 [OPTIONS]"
    echo ""
    echo "Run the UFO node with specified configuration."
    echo ""
    echo "Options:"
    echo "  --build-mode MODE     Set the build mode (patched, bridged, fauxmosis)"
    echo "                        Default: patched"
    echo "  --validators NUM      Set the number of validators"
    echo "                        Default: 1"
    echo "  --block-time MS       Set the block time in milliseconds"
    echo "                        Default: 1000"
    echo "  --osmosis-source PATH Set the path to Osmosis source code"
    echo "                        Default: /tmp/osmosis-source"
    echo "  --ufo-osmosis-source-path PATH Set the path to Osmosis source code"
    echo "                        Default: /tmp/osmosis-source"
    echo "  --help                Show this help message and exit"
    echo ""
    echo "Examples:"
    echo "  $0 --build-mode fauxmosis --block-time 100"
    echo "  $0 --validators 4 --block-time 500"
    echo ""
    exit 0
}

# Create pid file to track the background process
PID_FILE="/tmp/ufo-node.pid"

# Cleanup function to handle script exit
cleanup() {
    echo -e "${YELLOW}Shutting down UFO node...${NC}"
    if [ -f "$PID_FILE" ]; then
        PID=$(cat $PID_FILE)
        if ps -p $PID > /dev/null; then
            kill $PID
            echo -e "${GREEN}UFO node process $PID terminated${NC}"
        fi
        rm -f $PID_FILE
    fi
    echo -e "${GREEN}UFO node shutdown complete${NC}"
    exit 0
}

# Set up the trap to call cleanup when the script exits
trap cleanup SIGINT SIGTERM EXIT

# Parse command line arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --help)
            show_help
            ;;
        --build-mode)
            BUILD_MODE="$2"
            shift 2
            ;;
        --validators)
            VALIDATORS="$2"
            shift 2
            ;;
        --block-time)
            BLOCK_TIME="$2"
            shift 2
            ;;
        --osmosis-source) # Keep existing arg for direct runs if needed
            UFO_OSMOSIS_SOURCE_PATH="$2"
            shift 2
            ;;
        --ufo-osmosis-source-path) # Argument expected from Nix wrapper/app
            UFO_OSMOSIS_SOURCE_PATH="$2"
            shift 2
            ;;
        *)
            echo -e "${RED}Unknown option: $1${NC}"
            echo "Use --help to see available options"
            exit 1
            ;;
    esac
done

# Validate the path received from argument
if [ -z "$UFO_OSMOSIS_SOURCE_PATH" ]; then
    echo -e "${RED}Error: Osmosis source path was not provided.${NC}"
    echo -e "${YELLOW}Use --osmosis-source or --ufo-osmosis-source-path argument.${NC}"
    exit 1
fi

# Validate build mode
if [[ ! "$BUILD_MODE" =~ ^(patched|bridged|fauxmosis)$ ]]; then
    echo -e "${RED}Error: Invalid build mode '$BUILD_MODE'. Must be one of: patched, bridged, fauxmosis${NC}"
    exit 1
fi

# Validate validators count
if ! [[ "$VALIDATORS" =~ ^[0-9]+$ ]] || [ "$VALIDATORS" -lt 1 ]; then
    echo -e "${RED}Error: Invalid validators count '$VALIDATORS'. Must be a positive integer${NC}"
    exit 1
fi

# Validate block time
if ! [[ "$BLOCK_TIME" =~ ^[0-9]+$ ]] || [ "$BLOCK_TIME" -lt 1 ]; then
    echo -e "${RED}Error: Invalid block time '$BLOCK_TIME'. Must be a positive integer in milliseconds${NC}"
    exit 1
fi

echo -e "${BLUE}Starting UFO node with configuration:${NC}"
echo -e "  Build mode: ${GREEN}$BUILD_MODE${NC}"
echo -e "  Validators: ${GREEN}$VALIDATORS${NC}"
echo -e "  Block time: ${GREEN}$BLOCK_TIME ms${NC}"
echo -e "  Osmosis source: ${GREEN}$UFO_OSMOSIS_SOURCE_PATH${NC}"

# Remove build checks if not needed for running the node binary
# if [ "$BUILD_MODE" = "patched" ]; then ... fi

# Start the actual UFO node process
echo -e "${BLUE}Starting actual UFO node process...${NC}"

# Use a clean home directory for testing
# Define TEST_DIR or default if not passed via environment/args
TEST_DIR="${TEST_DIR:-/tmp/ufo-test-run}"
mkdir -p "$TEST_DIR"
UFO_HOME="$TEST_DIR/.ufo-node-home-$(date +%s)"
mkdir -p "$UFO_HOME"
echo "Using UFO home directory: $UFO_HOME"

# Define log file path
LOG_FILE="${LOG_FILE:-$TEST_DIR/ufo-node.log}"

# Get the expected binary path using the path argument
OSMOSIS_UFO_BINARY="${UFO_OSMOSIS_SOURCE_PATH}/osmosisd-ufo"

# Check if the binary exists
if [ ! -x "$OSMOSIS_UFO_BINARY" ]; then
  echo -e "${RED}Error: UFO-patched Osmosis binary not found at '$OSMOSIS_UFO_BINARY'${NC}"
  echo -e "${YELLOW}Please ensure you have cloned Osmosis to '$UFO_OSMOSIS_SOURCE_PATH' and run the build command:${NC}"
  echo -e "${YELLOW}  nix run <your_flake>#ufo:build-osmosis -- \"$UFO_OSMOSIS_SOURCE_PATH\"${NC}"
  exit 1
fi

# Assuming 'osmosisd-ufo' is the binary name in the Nix environment
# Initialize the node first if needed (common for Cosmos SDK)
if [ ! -d "$UFO_HOME/config" ]; then
  echo "Initializing UFO node configuration in $UFO_HOME..."
  "$OSMOSIS_UFO_BINARY" init "test-validator" --chain-id "ufo-test-1" --home "$UFO_HOME" || {
    echo -e "${RED}Failed to initialize UFO node${NC}"
    exit 1
  }
  # Modify config.toml for test environment
  sed -i '' "s/allow_duplicate_ip = false/allow_duplicate_ip = true/" "$UFO_HOME/config/config.toml"
  sed -i '' "s/cors_allowed_origins = \[\]/cors_allowed_origins = [\"*\"]/" "$UFO_HOME/config/config.toml"
  sed -i '' "s/^laddr = "tcp:\/\/127.0.0.1:26657"/laddr = "tcp:\/\/0.0.0.0:26657"/" "$UFO_HOME/config/config.toml"
  sed -i '' "s/^timeout_commit = .*$/timeout_commit = \"${BLOCK_TIME}ms\"/" "$UFO_HOME/config/config.toml"

  # Add genesis account if faucet enabled (example, adjust based on actual faucet mechanism)
  if [ "${FAUCET_ENABLED:-true}" == "true" ]; then
      echo "Adding genesis account..." 
      # Example: Add genesis account command (replace with actual ufo-node command)
      # ufo-node add-genesis-account ufo1... 1000000ufo --home "$UFO_HOME"
  fi
fi

# Start the node
echo "Starting UFO node... Logs at $LOG_FILE"
"$OSMOSIS_UFO_BINARY" start \
  --home "$UFO_HOME" \
  --rpc.laddr tcp://0.0.0.0:26657 \
  --grpc.address 0.0.0.0:9090 \
  --address tcp://0.0.0.0:26655 \
  --p2p.laddr tcp://0.0.0.0:26656 \
  --log_level info \
  --trace > "$LOG_FILE" 2>&1 &

# Save the background process PID to the file
NODE_PID=$!
echo $NODE_PID > "$PID_FILE"

echo -e "${GREEN}UFO node started in the background (PID: $(cat $PID_FILE))${NC}"
echo -e "${YELLOW}Logs available at: $LOG_FILE${NC}"
echo -e "${YELLOW}Press Ctrl+C to stop the node${NC}"

# Wait indefinitely (until SIGINT/SIGTERM is received via trap)
while true; do
    # Check if the process is still running
    if ! ps -p $NODE_PID > /dev/null; then
        echo -e "${RED}UFO node process (PID: $NODE_PID) stopped unexpectedly.${NC}"
        echo "Check logs: $LOG_FILE"
        exit 1
    fi
    sleep 30 # Check less frequently
done 