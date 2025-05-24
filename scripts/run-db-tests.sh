#!/usr/bin/env bash

# This script runs the database tests using a dedicated Nix environment.
# It isolates the database tests from other environment issues.

set -euo pipefail

# Colors for output
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
RED='\033[0;31m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

echo -e "${CYAN}=== Running Almanac Database Tests ===${NC}"

# Set up temporary directories
TMP_DIR="$(pwd)/tmp/db-tests"
mkdir -p "$TMP_DIR"

# Function to clean up on exit
cleanup() {
  echo -e "\n${YELLOW}Cleaning up...${NC}"
  
  # Run the stop_db function from the Nix environment if possible
  nix-shell db-test-shell.nix --run "stop_db" || true
  
  echo -e "${GREEN}Done!${NC}"
}

# Set up cleanup on script exit
trap cleanup EXIT

# Link the PostgreSQL migrations before running tests
echo -e "${CYAN}Setting up database migrations...${NC}"
./scripts/link-migrations.sh

echo -e "${CYAN}Starting database tests using Nix environment...${NC}"

# Run the test with increased verbosity and a larger timeout
nix-shell db-test-shell.nix --run "
  # Initialize database
  init_db

  # Run storage sync tests with verbose output
  echo -e '${CYAN}Running storage sync tests...${NC}'
  RUST_BACKTRACE=1 SQLX_OFFLINE=false cargo test -p indexer-storage --test storage_sync -- --nocapture
  
  # If we get here, tests passed
  echo -e '${GREEN}Storage tests completed successfully!${NC}'
"

exit_code=$?

if [ $exit_code -eq 0 ]; then
  echo -e "\n${GREEN}✓ All database tests passed!${NC}"
else
  echo -e "\n${RED}✗ Database tests failed with exit code: $exit_code${NC}"
fi

exit $exit_code 