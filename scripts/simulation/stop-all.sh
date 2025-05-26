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

# Stop Reth node
if pgrep -f "reth" > /dev/null; then
  echo "Stopping Reth node..."
  pkill -f "reth" || echo "Failed to stop Reth"
  if [ -f "/tmp/reth-almanac.pid" ]; then
    RETH_PID=$(cat /tmp/reth-almanac.pid)
    kill $RETH_PID 2>/dev/null || echo "Failed to stop Reth with PID $RETH_PID"
    rm -f /tmp/reth-almanac.pid
  fi
else
  echo "No Reth node found running"
fi

# Stop mock-reth if running
if pgrep -f "mock-reth" > /dev/null; then
  echo "Stopping mock-reth server..."
  pkill -f "mock-reth" || echo "Failed to stop mock-reth"
  if [ -f "/tmp/mock-reth.pid" ]; then
    MOCK_RETH_PID=$(cat /tmp/mock-reth.pid)
    kill $MOCK_RETH_PID 2>/dev/null || echo "Failed to stop mock-reth with PID $MOCK_RETH_PID"
    rm -f /tmp/mock-reth.pid
  fi
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