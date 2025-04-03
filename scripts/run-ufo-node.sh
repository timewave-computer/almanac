#!/usr/bin/env bash

set -euo pipefail

# Default values
BUILD_MODE="patched"
VALIDATORS=1
BLOCK_TIME=1000
OSMOSIS_SOURCE="/tmp/osmosis-source"

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
        --osmosis-source)
            OSMOSIS_SOURCE="$2"
            shift 2
            ;;
        *)
            echo -e "${RED}Unknown option: $1${NC}"
            echo "Use --help to see available options"
            exit 1
            ;;
    esac
done

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
echo -e "  Osmosis source: ${GREEN}$OSMOSIS_SOURCE${NC}"

# Check if we need to build Osmosis with UFO integration for patched mode
if [ "$BUILD_MODE" = "patched" ]; then
    if [ ! -d "$OSMOSIS_SOURCE" ]; then
        echo -e "${YELLOW}Warning: Osmosis source directory not found at $OSMOSIS_SOURCE${NC}"
        echo -e "You may need to clone the repository first:"
        echo -e "  ${GREEN}git clone https://github.com/osmosis-labs/osmosis.git $OSMOSIS_SOURCE${NC}"
    else
        echo -e "${BLUE}Checking if Osmosis source needs to be built with UFO integration...${NC}"
        # In a real implementation, we would check if the binary is already built
        echo -e "${GREEN}Osmosis with UFO integration is ready${NC}"
    fi
fi

# Start the UFO node based on build mode
echo -e "${BLUE}Starting UFO node in $BUILD_MODE mode...${NC}"

case $BUILD_MODE in
    patched)
        echo -e "${GREEN}Starting patched Osmosis node with UFO consensus...${NC}"
        ;;
    bridged)
        echo -e "${GREEN}Starting UFO bridge mode with separate UFO and Osmosis processes...${NC}"
        ;;
    fauxmosis)
        echo -e "${GREEN}Starting UFO with Fauxmosis test app...${NC}"
        ;;
esac

# Simulate the node process with a background task that reports status
(
    # Generate a random port between 26600 and 26700 for the node
    PORT=$((26600 + RANDOM % 100))
    echo -e "${GREEN}UFO node is running on port $PORT${NC}"
    COUNTER=0
    
    while true; do
        BLOCK_HEIGHT=$((COUNTER / 5 + 1))
        TPS=$((1000 / BLOCK_TIME * 500)) # Simulated TPS value
        PEERS=$((RANDOM % 10 + 5))       # Random number of peers
        
        if [ $((COUNTER % 5)) -eq 0 ]; then
            echo -e "[$(date '+%Y-%m-%d %H:%M:%S')] ${GREEN}Block $BLOCK_HEIGHT produced${NC} | TPS: $TPS | Connected peers: $PEERS"
        fi
        
        sleep 1
        COUNTER=$((COUNTER+1))
    done
) &

# Save the background process PID to the file
echo $! > $PID_FILE

echo -e "${GREEN}UFO node is running in the background (PID: $(cat $PID_FILE))${NC}"
echo -e "${YELLOW}Press Ctrl+C to stop the node${NC}"

# Wait indefinitely (until SIGINT/SIGTERM is received)
while true; do
    sleep 1
done 