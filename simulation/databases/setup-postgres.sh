#!/bin/bash
# Purpose: Set up PostgreSQL database for development and testing

set -e

# Define colors for output
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
RED='\033[0;31m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}=== Setting up PostgreSQL Database ===${NC}"

# Check if PostgreSQL is available
if ! command -v psql >/dev/null 2>&1; then
    echo -e "${RED}Error: PostgreSQL client (psql) not found${NC}"
    echo -e "${YELLOW}Please ensure PostgreSQL is installed and in your PATH${NC}"
    exit 1
fi

echo -e "${GREEN}✓ Found PostgreSQL client${NC}"

# PostgreSQL configuration
PG_HOST="localhost"
PG_PORT="5432"
PG_USER="postgres"
PG_DB="almanac"

# Check if PostgreSQL server is running
if ! pg_isready -h $PG_HOST -p $PG_PORT -U $PG_USER > /dev/null 2>&1; then
    echo -e "${YELLOW}PostgreSQL server is not running. Attempting to start...${NC}"
    
    # Try to start PostgreSQL server using pg_ctl if available
    if command -v pg_ctl >/dev/null 2>&1; then
        # Get PostgreSQL data directory
        PG_DATA=$(pg_config --sharedir | sed 's/share$/data/')
        if [ -d "$PG_DATA" ]; then
            echo -e "${BLUE}Starting PostgreSQL server...${NC}"
            pg_ctl -D "$PG_DATA" start -l logs/postgres.log || true
            sleep 2
        else
            echo -e "${RED}Error: PostgreSQL data directory not found${NC}"
            echo -e "${YELLOW}Please initialize PostgreSQL data directory or start PostgreSQL manually${NC}"
            exit 1
        fi
    else
        echo -e "${RED}Error: Unable to start PostgreSQL server${NC}"
        echo -e "${YELLOW}Please start PostgreSQL server manually before running this script${NC}"
        exit 1
    fi
fi

# Verify PostgreSQL server is running
if ! pg_isready -h $PG_HOST -p $PG_PORT -U $PG_USER > /dev/null 2>&1; then
    echo -e "${RED}Error: Unable to connect to PostgreSQL server${NC}"
    echo -e "${YELLOW}Please ensure PostgreSQL server is running and accessible${NC}"
    exit 1
fi

echo -e "${GREEN}✓ PostgreSQL server is running${NC}"

# Create the database if it doesn't exist
echo -e "${BLUE}Checking if database '${PG_DB}' exists...${NC}"
if ! psql -h $PG_HOST -p $PG_PORT -U $PG_USER -lqt | cut -d \| -f 1 | grep -qw $PG_DB; then
    echo -e "${BLUE}Creating database '${PG_DB}'...${NC}"
    psql -h $PG_HOST -p $PG_PORT -U $PG_USER -c "CREATE DATABASE $PG_DB;" || true
    echo -e "${GREEN}✓ Database '${PG_DB}' created${NC}"
else
    echo -e "${GREEN}✓ Database '${PG_DB}' already exists${NC}"
fi

# Set up environmental variables
echo -e "${BLUE}Setting up environment variables...${NC}"
export PGHOST=$PG_HOST
export PGPORT=$PG_PORT
export PGUSER=$PG_USER
export PGDATABASE=$PG_DB

echo -e "${GREEN}=== PostgreSQL setup completed successfully! ===${NC}"
echo -e "${BLUE}PostgreSQL is running at: ${PG_HOST}:${PG_PORT}${NC}"
echo -e "${BLUE}Database: ${PG_DB}${NC}"
echo -e "${BLUE}User: ${PG_USER}${NC}" 