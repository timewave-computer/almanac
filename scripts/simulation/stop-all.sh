#!/usr/bin/env bash
# stop-all.sh - Script to stop all running services in the Almanac workflow

set -e

echo "Stopping all running services..."

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

# Stop wasmd node
if pgrep -f "wasmd start" > /dev/null; then
  echo "Stopping wasmd node..."
  pkill -f "wasmd start" || echo "Failed to stop wasmd"
  if [ -f "/tmp/wasmd-almanac.pid" ]; then
    WASMD_PID=$(cat /tmp/wasmd-almanac.pid)
    kill $WASMD_PID 2>/dev/null || echo "Failed to stop wasmd with PID $WASMD_PID"
    rm -f /tmp/wasmd-almanac.pid
  fi
else
  echo "No wasmd node found running"
fi

# Stop mock-wasmd if running
if pgrep -f "mock-wasmd" > /dev/null; then
  echo "Stopping mock-wasmd server..."
  pkill -f "mock-wasmd" || echo "Failed to stop mock-wasmd"
  if [ -f "/tmp/mock-wasmd.pid" ]; then
    MOCK_WASMD_PID=$(cat /tmp/mock-wasmd.pid)
    kill $MOCK_WASMD_PID 2>/dev/null || echo "Failed to stop mock-wasmd with PID $MOCK_WASMD_PID"
    rm -f /tmp/mock-wasmd.pid
  fi
fi

# Stop PostgreSQL
DATA_DIR="$HOME/.almanac"
if [ -f "$DATA_DIR/postgres/stop-postgres.sh" ]; then
  echo "Stopping PostgreSQL server..."
  "$DATA_DIR/postgres/stop-postgres.sh" 2>/dev/null || echo "PostgreSQL was not running or failed to stop"
else
  pg_ctl -D "$DATA_DIR/postgres" stop 2>/dev/null || echo "PostgreSQL was not running or failed to stop"
fi

# Stop any Almanac indexers
if pgrep -f "almanac-indexer" > /dev/null; then
  echo "Stopping Almanac indexers..."
  pkill -f "almanac-indexer" || echo "Failed to stop almanac-indexer"
fi

echo "All services stopped successfully" 