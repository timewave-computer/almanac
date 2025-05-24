#!/bin/bash
# Purpose: Stop all services that were started for development and testing

set -e

# Define colors for output
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
RED='\033[0;31m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}=== Stopping All Services ===${NC}"

# Stop Anvil Ethereum node
echo -e "${BLUE}Stopping Anvil Ethereum node...${NC}"
if [ -f "/tmp/anvil-almanac.pid" ]; then
    pkill -F /tmp/anvil-almanac.pid 2>/dev/null || true
    rm -f /tmp/anvil-almanac.pid
    echo -e "${GREEN}✓ Anvil stopped${NC}"
else
    echo -e "${YELLOW}Anvil not running or PID file not found${NC}"
fi

# Stop Reth Ethereum node
echo -e "${BLUE}Stopping Reth Ethereum node...${NC}"
if [ -f "/tmp/reth-almanac.pid" ]; then
    pkill -F /tmp/reth-almanac.pid 2>/dev/null || true
    rm -f /tmp/reth-almanac.pid
    echo -e "${GREEN}✓ Reth stopped${NC}"
else
    echo -e "${YELLOW}Reth not running or PID file not found${NC}"
fi

# Stop wasmd CosmWasm node
echo -e "${BLUE}Stopping wasmd CosmWasm node...${NC}"
if [ -f "/tmp/wasmd-almanac.pid" ]; then
    pkill -F /tmp/wasmd-almanac.pid 2>/dev/null || true
    rm -f /tmp/wasmd-almanac.pid
    if [ -d "/tmp/wasmd-status" ]; then
        rm -rf /tmp/wasmd-status
    fi
    echo -e "${GREEN}✓ wasmd stopped${NC}"
else
    echo -e "${YELLOW}wasmd not running or PID file not found${NC}"
fi

# Stop PostgreSQL (if started by our scripts)
echo -e "${BLUE}Stopping PostgreSQL server...${NC}"
if command -v pg_ctl >/dev/null 2>&1 && pg_isready -h localhost -p 5432 -U postgres > /dev/null 2>&1; then
    PG_DATA=$(pg_config --sharedir | sed 's/share$/data/')
    if [ -d "$PG_DATA" ]; then
        pg_ctl -D "$PG_DATA" stop -m fast || true
        echo -e "${GREEN}✓ PostgreSQL stopped${NC}"
    else
        echo -e "${YELLOW}PostgreSQL running, but not started by our scripts${NC}"
    fi
else
    echo -e "${YELLOW}PostgreSQL not running or pg_ctl not found${NC}"
fi

# Stop any Almanac indexer processes
echo -e "${BLUE}Stopping Almanac indexers...${NC}"
pkill -f "almanac-indexer" 2>/dev/null || true
echo -e "${GREEN}✓ Almanac indexers stopped${NC}"

echo -e "${GREEN}=== All Services Stopped Successfully ===${NC}" 