#!/bin/bash

# Purpose: Run storage tests directly using environment variables instead of nix-shell

set -e
set -u

# Color definitions for output
GREEN='\033[0;32m'
RED='\033[0;31m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}=== Running Almanac Database Tests (Simple Mode) ===${NC}"

# Create a temporary directory for our test databases
TMP_DIR="${PWD}/tmp/db-tests-simple"
mkdir -p "${TMP_DIR}"
POSTGRES_DIR="${TMP_DIR}/postgres"
ROCKSDB_DIR="${TMP_DIR}/rocksdb"
mkdir -p "${POSTGRES_DIR}"
mkdir -p "${ROCKSDB_DIR}"

# Make sure we clean up on exit
function cleanup {
  echo "Stopping PostgreSQL..."
  pg_ctl -D "${POSTGRES_DIR}" stop || true
  echo "Done!"
}
trap cleanup EXIT

# Link migrations
echo "Setting up database migrations..."
./scripts/link-migrations.sh

# Initialize the PostgreSQL database
echo "Initializing PostgreSQL database for testing..."
initdb -D "${POSTGRES_DIR}" -U postgres || true

# Start PostgreSQL
pg_ctl -D "${POSTGRES_DIR}" -l "${POSTGRES_DIR}/logfile" -o "-k '${POSTGRES_DIR}'" start

# Create test database
createdb -h localhost -p 5432 -U postgres indexer_test || true
echo "Created test database: indexer_test"
echo "PostgreSQL is ready at: postgres://postgres:postgres@localhost:5432/indexer_test"

# Set environment variables for the tests
export DATABASE_URL="postgres://postgres:postgres@localhost:5432/indexer_test"
export TEST_ROCKSDB_PATH="${ROCKSDB_DIR}"
export SQLX_OFFLINE=false
export MACOSX_DEPLOYMENT_TARGET=11.0

# Run the storage tests
echo -e "${BLUE}Running storage sync tests...${NC}"
cargo test -p indexer-storage

# Check if the tests passed
EXIT_CODE=$?
if [ ${EXIT_CODE} -eq 0 ]; then
  echo -e "${GREEN}✓ All database tests passed!${NC}"
else
  echo -e "${RED}✗ Database tests failed${NC}"
fi

echo "Cleaning up..."
exit ${EXIT_CODE} 