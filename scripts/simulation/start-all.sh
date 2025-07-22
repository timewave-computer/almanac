#!/bin/bash
# Purpose: Start all services required for development and testing using Nix simulation environment

set -e

# Define colors for output
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
RED='\033[0;31m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}=== Starting All Services ===${NC}"

# Determine which chains to start
CHAINS="${1:-ethereum,cosmos,fdb}"

echo -e "${YELLOW}Starting simulation environment for chains: $CHAINS${NC}"

# Check if sim-cluster command is available
if command -v sim-cluster &> /dev/null; then
    SIM_CMD="sim-cluster"
else
    echo -e "${YELLOW}sim-cluster not found, using nix run${NC}"
    SIM_CMD="nix run .#sim-cluster --"
fi

# Initialize the simulation environment
echo -e "${BLUE}Initializing simulation environment...${NC}"
if $SIM_CMD init "$CHAINS"; then
    echo -e "${GREEN}Simulation environment initialized${NC}"
else
    echo -e "${RED}Failed to initialize simulation environment${NC}"
    exit 1
fi

# Start all services
echo -e "${BLUE}Starting all services...${NC}"
if $SIM_CMD start; then
    echo -e "${GREEN}All services started successfully${NC}"
    
    # Show status
    echo -e "${BLUE}Service status:${NC}"
    $SIM_CMD status
else
    echo -e "${RED}Failed to start services${NC}"
    exit 1
fi

echo -e "${GREEN}=== All Services Started Successfully ===${NC}"
echo ""
echo "To check service status: $SIM_CMD status"
echo "To view logs: $SIM_CMD logs"
echo "To stop all services: ./scripts/simulation/stop-all.sh"