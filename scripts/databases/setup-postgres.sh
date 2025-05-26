#!/bin/bash
# Purpose: Set up PostgreSQL database server for development and testing

set -e

# Define colors for output
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
RED='\033[0;31m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}=== Setting up PostgreSQL Database Server ===${NC}"

# Stop any existing PostgreSQL instances
nix develop --command bash -c "stop_databases || true"
echo -e "${GREEN}✓ Stopped existing PostgreSQL instances${NC}"

# Remove old data directories
rm -rf data/postgres
echo -e "${GREEN}✓ Removed old data directories${NC}"

# Initialize PostgreSQL with nix develop
nix develop --command bash -c "init_databases"

echo -e "${GREEN}✓ PostgreSQL initialized${NC}"

# Check if PostgreSQL is running
nix develop --command bash -c "pg_isready -h localhost -p 5432" > /dev/null 2>&1
if [ $? -ne 0 ]; then
  echo -e "${RED}Error: PostgreSQL is not running. Please check the logs.${NC}"
  exit 1
fi

echo -e "${BLUE}=== PostgreSQL Setup Complete ===${NC}"
echo -e "${GREEN}PostgreSQL server is running at localhost:5432${NC}"
echo -e "${YELLOW}Use separate scripts to create and populate databases${NC}"
echo -e "${YELLOW}To stop PostgreSQL, run: nix develop --command bash -c \"stop_databases\"${NC}" 