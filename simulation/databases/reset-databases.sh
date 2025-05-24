#!/bin/bash
# Purpose: Reset all databases for Almanac (PostgreSQL and RocksDB)

set -e

# Define colors for output
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
RED='\033[0;31m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}=== Resetting Almanac Databases ===${NC}"

# Check if PostgreSQL is available
if ! command -v psql >/dev/null 2>&1; then
    echo -e "${RED}Error: PostgreSQL client (psql) not found${NC}"
    echo -e "${YELLOW}Please ensure PostgreSQL is installed and in your PATH${NC}"
    exit 1
fi

# PostgreSQL configuration
PG_HOST="localhost"
PG_PORT="5432"
PG_USER="postgres"
PG_DB="almanac"

# RocksDB configuration
ROCKSDB_DIR="data/rocksdb"

# Check if PostgreSQL server is running
if ! pg_isready -h $PG_HOST -p $PG_PORT -U $PG_USER > /dev/null 2>&1; then
    echo -e "${YELLOW}PostgreSQL server is not running. No need to reset PostgreSQL database.${NC}"
else
    echo -e "${GREEN}✓ PostgreSQL server is running${NC}"
    echo -e "${BLUE}Dropping and recreating PostgreSQL database '${PG_DB}'...${NC}"
    
    # Drop and recreate the database
    psql -h $PG_HOST -p $PG_PORT -U $PG_USER -c "DROP DATABASE IF EXISTS $PG_DB;" || true
    psql -h $PG_HOST -p $PG_PORT -U $PG_USER -c "CREATE DATABASE $PG_DB;" || true
    
    echo -e "${GREEN}✓ PostgreSQL database reset successfully${NC}"
    
    # Recreate tables
    echo -e "${BLUE}Recreating tables...${NC}"
    ./simulation/databases/create-postgres-tables.sh
fi

# Reset RocksDB
echo -e "${BLUE}Resetting RocksDB directories...${NC}"
if [ -d "$ROCKSDB_DIR" ]; then
    echo -e "${YELLOW}Removing existing RocksDB data...${NC}"
    rm -rf ${ROCKSDB_DIR}/*
    echo -e "${GREEN}✓ RocksDB data removed${NC}"
fi

# Recreate RocksDB directories
echo -e "${BLUE}Recreating RocksDB directories...${NC}"
./simulation/databases/setup-rocksdb.sh

echo -e "${GREEN}=== All databases reset successfully! ===${NC}" 