#!/bin/bash
# Purpose: Wrapper script to run the Almanac indexer using Nix

set -e

# Define colors for output
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
RED='\033[0;31m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Help function
show_help() {
  echo "Usage: $0 [options]"
  echo ""
  echo "This script runs the Almanac indexer using Nix."
  echo ""
  echo "Options:"
  echo "  --chain=TYPE       Chain type to index (ethereum, cosmos) [default: ethereum]"
  echo "  --node=TYPE        Node type for Ethereum (anvil, reth) [default: anvil]"
  echo "  --duration=TIME    Duration to run the indexer (e.g., 5m, 1h) [default: 15m]"
  echo "  --ethereum-port=N  Port for Ethereum node [default: 8545]"
  echo "  --ethereum-host=H  Host for Ethereum node [default: localhost]"
  echo "  --wasmd-rpc=URL    RPC URL for wasmd node [default: http://localhost:26657]"
  echo "  --wasmd-rest=URL   REST URL for wasmd node [default: http://localhost:1317]"
  echo "  --help             Show this help message"
  exit 0
}

# Check if help was requested
for arg in "$@"; do
  if [ "$arg" == "--help" ] || [ "$arg" == "-h" ]; then
    show_help
  fi
done

echo -e "${BLUE}=== Almanac Indexer Runner ===${NC}"
echo -e "${YELLOW}Setting up Nix environment and running the indexer...${NC}"

# Make sure the script directory exists
mkdir -p nix

# Check if the Nix script exists
if [ ! -f "nix/almanac-indexer.nix" ]; then
  echo -e "${RED}Error: Nix script not found at nix/almanac-indexer.nix${NC}"
  echo -e "${YELLOW}Please create the script first.${NC}"
  exit 1
fi

# Build arguments string from all passed arguments
ARGS=""
for arg in "$@"; do
  ARGS="$ARGS $arg"
done

# Run the indexer using Nix
echo -e "${GREEN}Running: nix run ./nix/almanac-indexer.nix -- $ARGS${NC}"
nix run ./nix/almanac-indexer.nix -- $ARGS

# Check exit code
if [ $? -eq 0 ]; then
  echo -e "${GREEN}✓ Almanac indexer completed successfully${NC}"
else
  echo -e "${RED}✗ Almanac indexer encountered an error${NC}"
  exit 1
fi 