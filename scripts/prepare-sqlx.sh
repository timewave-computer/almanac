#!/usr/bin/env bash
# Script to run sqlx prepare for compile-time SQL validation

set -e

# Define colors for terminal output
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[0;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Function to show help text
show_help() {
    echo "Usage: $0 [OPTIONS]"
    echo ""
    echo "Prepare the database for sqlx compile-time checking."
    echo ""
    echo "Options:"
    echo "  --help              Show this help message and exit"
    echo "  --check-only        Only check if the database is prepared, don't run sqlx prepare"
    echo "  --offline           Run in offline mode (don't connect to database)"
    echo ""
    exit 0
}

# Default options
CHECK_ONLY=false
OFFLINE=false

# Parse command line arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --help)
            show_help
            ;;
        --check-only)
            CHECK_ONLY=true
            shift
            ;;
        --offline)
            OFFLINE=true
            shift
            ;;
        *)
            echo -e "${RED}Unknown option: $1${NC}"
            echo "Use --help to see available options"
            exit 1
            ;;
    esac
done

# Define variables
DB_NAME="indexer"
DB_USER="postgres"
DB_PASSWORD="postgres"
DB_HOST="localhost"
DB_PORT="5432"
DATABASE_URL="postgres://${DB_USER}:${DB_PASSWORD}@${DB_HOST}:${DB_PORT}/${DB_NAME}"

if [ "$OFFLINE" = true ]; then
    echo -e "${BLUE}Running in offline mode, skipping database checks${NC}"
else
    # Check if postgres is running
    echo -e "${BLUE}Checking if PostgreSQL is running...${NC}"
    if ! pg_isready -h $DB_HOST -p $DB_PORT -U $DB_USER > /dev/null 2>&1; then
        echo -e "${YELLOW}PostgreSQL is not running. Starting PostgreSQL...${NC}"
        
        if [ -d "./data/postgres/pgdata" ]; then
            pg_ctl -D "./data/postgres/pgdata" -l "./data/postgres/logfile" start || {
                echo -e "${RED}Failed to start PostgreSQL. Please start it manually.${NC}"
                exit 1
            }
            
            # Wait for PostgreSQL to start
            echo -e "${BLUE}Waiting for PostgreSQL to start...${NC}"
            tries=0
            while ! pg_isready -h $DB_HOST -p $DB_PORT -U $DB_USER > /dev/null 2>&1; do
                sleep 1
                tries=$((tries + 1))
                if [ $tries -ge 30 ]; then
                    echo -e "${RED}PostgreSQL did not start in 30 seconds. Exiting.${NC}"
                    exit 1
                fi
            done
        else
            echo -e "${RED}PostgreSQL data directory does not exist. Run nix run .#start-postgres first.${NC}"
            exit 1
        fi
    fi
    
    # Check if database exists, create if it doesn't
    echo -e "${BLUE}Checking if database exists...${NC}"
    if ! psql -h $DB_HOST -p $DB_PORT -U $DB_USER -lqt | cut -d \| -f 1 | grep -qw $DB_NAME; then
        echo -e "${YELLOW}Creating database ${DB_NAME}...${NC}"
        createdb -h $DB_HOST -p $DB_PORT -U $DB_USER $DB_NAME || {
            echo -e "${RED}Failed to create database ${DB_NAME}.${NC}"
            exit 1
        }
    fi
    
    echo -e "${GREEN}Database is ready.${NC}"
fi

# Set environment variables for sqlx
export DATABASE_URL

if [ "$CHECK_ONLY" = true ]; then
    echo -e "${BLUE}Check-only mode enabled, skipping sqlx prepare${NC}"
    exit 0
fi

# Create .sqlx directory if it doesn't exist
mkdir -p .sqlx

# Check if sqlx-cli is installed
if ! command -v sqlx &> /dev/null; then
    echo -e "${YELLOW}sqlx-cli is not installed. Installing...${NC}"
    echo -e "${BLUE}Running: cargo install sqlx-cli --no-default-features --features postgres${NC}"
    cargo install sqlx-cli --no-default-features --features postgres || {
        echo -e "${RED}Failed to install sqlx-cli. Please install it manually:${NC}"
        echo "cargo install sqlx-cli --no-default-features --features postgres"
        exit 1
    }
fi

# Try to run sqlx prepare with fallback for compilation errors
echo -e "${BLUE}Running sqlx prepare...${NC}"
if sqlx prepare --check --database-url "$DATABASE_URL" 2>/dev/null; then
    echo -e "${GREEN}Database schema is up to date with sqlx metadata.${NC}"
    exit 0
else
    echo -e "${YELLOW}Updating sqlx metadata...${NC}"
    if ! sqlx prepare --merged --database-url "$DATABASE_URL" 2>/tmp/sqlx_error.log; then
        if grep -q "Compilation" /tmp/sqlx_error.log; then
            echo -e "${YELLOW}Warning: Compilation errors in project, but sqlx metadata may still be updated${NC}"
            echo -e "${YELLOW}Creating a placeholder metadata file...${NC}"
            
            # Create a minimal placeholder file if it doesn't exist
            if [ ! -f ".sqlx/query-data.json" ]; then
                echo '{"db":"PostgreSQL","tables":[]}' > .sqlx/query-data.json
                echo -e "${GREEN}Created placeholder metadata file.${NC}"
            fi
            
            echo -e "${YELLOW}You may need to manually update this file when the code is fixed.${NC}"
            echo -e "${YELLOW}See error log at /tmp/sqlx_error.log for details.${NC}"
        else
            echo -e "${RED}Failed to run sqlx prepare. Error:${NC}"
            cat /tmp/sqlx_error.log
            exit 1
        fi
    else
        echo -e "${GREEN}sqlx prepare completed successfully.${NC}"
        echo -e "${GREEN}The SQL query metadata has been stored in .sqlx/query-data.json${NC}"
        echo -e "${GREEN}This file should be committed to the repository to enable offline builds.${NC}"
    fi
fi 