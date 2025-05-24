#!/bin/bash
# Purpose: Run PostgreSQL database tests in the Nix environment with the proper configuration

set -e

# Color definitions
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[0;33m'
BLUE='\033[0;36m'
NC='\033[0m' # No Color

echo -e "${BLUE}=== Running Almanac Database Tests in Nix Environment ===${NC}"

# Create a temporary directory for RocksDB
TMP_DIR="${PWD}/tmp/db-tests-nix"
ROCKSDB_DIR="${TMP_DIR}/rocksdb"
mkdir -p "${ROCKSDB_DIR}"

# Function to clean up on exit
function cleanup() {
  echo -e "\n${YELLOW}Cleaning up temporary files...${NC}"
  rm -rf "$TMP_DIR"
}
trap cleanup EXIT

# Check if PostgreSQL is properly set up
if ! nix develop --command bash -c 'pg_isready -q'; then
  echo -e "${RED}PostgreSQL is not running.${NC}"
  echo "Please run ./scripts/setup-postgres-nix.sh first to set up PostgreSQL."
  exit 1
fi

# Check if postgres role exists
POSTGRES_ROLE_EXISTS=$(nix develop --command bash -c 'psql -d postgres -tc "SELECT 1 FROM pg_roles WHERE rolname = '\''postgres'\''" | grep -c 1 || echo 0')
if [ "$POSTGRES_ROLE_EXISTS" -eq "0" ]; then
  echo -e "${RED}The 'postgres' role does not exist.${NC}"
  echo "Please run ./scripts/setup-postgres-nix.sh first to set up PostgreSQL."
  exit 1
fi

# Check if indexer_test database exists
INDEXER_TEST_DB_EXISTS=$(nix develop --command bash -c 'psql -lqt | cut -d \| -f 1 | grep -cw "indexer_test" || echo 0')
if [ "$INDEXER_TEST_DB_EXISTS" -eq "0" ]; then
  echo -e "${RED}The 'indexer_test' database does not exist.${NC}"
  echo "Please run ./scripts/setup-postgres-nix.sh first to set up PostgreSQL."
  exit 1
fi

# Extract PostgreSQL port from the Nix environment
PGPORT=$(nix develop --command bash -c 'echo $PGPORT')

# Run database tests
echo -e "\n${YELLOW}Running database tests...${NC}"
nix develop --command bash -c "
  set -e
  
  # Set environment variables for testing
  export TEST_ROCKSDB_PATH=\"${ROCKSDB_DIR}\"
  export DATABASE_URL=\"postgres://postgres:postgres@localhost:$PGPORT/indexer_test\"
  export SQLX_OFFLINE=false
  
  # Link migrations (if needed)
  if [ ! -d 'crates/storage/migrations' ] || [ -z \"\$(ls -A crates/storage/migrations)\" ]; then
    echo \"Linking migration files...\"
    ./scripts/link-migrations.sh
  fi
  
  # Run the tests
  echo \"Running storage sync tests...\"
  cd crates/storage
  RUST_BACKTRACE=1 cargo test --test storage_sync -- --nocapture
  
  # Run other tests that depend on PostgreSQL
  echo \"Running other database-dependent tests...\"
  RUST_BACKTRACE=1 cargo test --lib
"

# Check exit status
EXIT_CODE=$?
if [ $EXIT_CODE -eq 0 ]; then
  echo -e "\n${GREEN}✓ All database tests passed!${NC}"
else
  echo -e "\n${RED}✗ Database tests failed with exit code: $EXIT_CODE${NC}"
fi

exit $EXIT_CODE 