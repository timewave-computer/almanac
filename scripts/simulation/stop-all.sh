#!/usr/bin/env bash
# stop-all.sh - Script to stop all running services in the Almanac workflow

set -e

# Define colors for output
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
RED='\033[0;31m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}=== Stopping All Services ===${NC}"

# Check if sim-cluster command is available
if command -v sim-cluster &> /dev/null; then
    SIM_CMD="sim-cluster"
else
    echo -e "${YELLOW}sim-cluster not found, using nix run${NC}"
    SIM_CMD="nix run .#sim-cluster --"
fi

# Try to stop using sim-cluster first
if command -v sim-cluster &> /dev/null || [ -f "flake.nix" ]; then
    echo -e "${BLUE}Stopping services using sim-cluster...${NC}"
    $SIM_CMD stop 2>/dev/null || echo -e "${YELLOW}sim-cluster stop failed, falling back to manual cleanup${NC}"
fi

# Manual cleanup of individual services
echo -e "${BLUE}Cleaning up individual services...${NC}"

# Stop Anvil node
if pgrep -f "anvil.*--port 8545" > /dev/null; then
  echo "Stopping Anvil node..."
  pkill -f "anvil.*--port 8545" || echo "Failed to stop Anvil"
  if [ -f "/tmp/anvil.pid" ]; then
    ANVIL_PID=$(cat /tmp/anvil.pid)
    kill $ANVIL_PID 2>/dev/null || echo "Failed to stop Anvil with PID $ANVIL_PID"
    rm -f /tmp/anvil.pid
  fi
else
  echo "No Anvil node found running"
fi

# Stop Geth node
if pgrep -f "geth" > /dev/null; then
  echo "Stopping Geth node..."
  pkill -f "geth" || echo "Failed to stop Geth"
  if [ -f "/tmp/geth-almanac.pid" ]; then
    GETH_PID=$(cat /tmp/geth-almanac.pid)
    kill $GETH_PID 2>/dev/null || echo "Failed to stop Geth with PID $GETH_PID"
    rm -f /tmp/geth-almanac.pid
  fi
else
  echo "No Geth node found running"
fi

# Stop wasmd/neutrond node
if pgrep -f "wasmd start\|neutrond start" > /dev/null; then
  echo "Stopping Cosmos node..."
  pkill -f "wasmd start|neutrond start" || echo "Failed to stop Cosmos node"
  if [ -f "/tmp/wasmd-almanac.pid" ]; then
    WASMD_PID=$(cat /tmp/wasmd-almanac.pid)
    kill $WASMD_PID 2>/dev/null || echo "Failed to stop wasmd with PID $WASMD_PID"
    rm -f /tmp/wasmd-almanac.pid
  fi
else
  echo "No Cosmos node found running"
fi

# Stop Solana node
if pgrep -f "solana-test-validator" > /dev/null; then
  echo "Stopping Solana test validator..."
  pkill -f "solana-test-validator" || echo "Failed to stop Solana"
else
  echo "No Solana node found running"
fi

# Stop FoundationDB
if pgrep -f "fdbserver" > /dev/null; then
  echo "Stopping FoundationDB..."
  pkill -f "fdbserver" || echo "Failed to stop FoundationDB"
fi

# Stop PostgreSQL
DATA_DIR="$HOME/.almanac"
if [ -f "$DATA_DIR/postgres/stop-postgres.sh" ]; then
  echo "Stopping PostgreSQL server..."
  "$DATA_DIR/postgres/stop-postgres.sh" 2>/dev/null || echo "PostgreSQL was not running or failed to stop"
else
  pg_ctl -D "$DATA_DIR/postgres" stop 2>/dev/null || echo "PostgreSQL was not running or failed to stop"
fi

# Stop any Almanac services
if pgrep -f "almanac-indexing" > /dev/null; then
  echo "Stopping Almanac indexing service..."
  pkill -f "almanac-indexing" || echo "Failed to stop almanac-indexing"
fi

if pgrep -f "almanac-api" > /dev/null; then
  echo "Stopping Almanac API service..."
  pkill -f "almanac-api" || echo "Failed to stop almanac-api"
fi

echo -e "${GREEN}=== All Services Stopped Successfully ===${NC}"