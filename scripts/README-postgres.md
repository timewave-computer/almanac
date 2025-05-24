# PostgreSQL Database Setup

This document describes how to set up and use PostgreSQL with the Almanac project.

## Setup Scripts

We have two main scripts for PostgreSQL setup:

1. **`setup-complete-pg.sh`** - Complete PostgreSQL setup
   - Initializes PostgreSQL
   - Creates both `indexer` and `indexer_test` databases
   - Creates all necessary tables in both databases
   - Updates SQLx metadata

2. **`create-test-db-tables.sh`** - Creates tables in the test database only
   - Use this if you only need to update the test database tables

## How to Use

### Initial Setup

For a fresh installation:

```bash
# Make the script executable
chmod +x scripts/setup-complete-pg.sh

# Run the script
./scripts/setup-complete-pg.sh
```

### Running Tests

To run tests with PostgreSQL:

```bash
# Make the test script executable
chmod +x scripts/run-test-with-db-fix.sh

# Run the tests
./scripts/run-test-with-db-fix.sh
```

### Test RocksDB Only

If you want to test only RocksDB functionality:

```bash
chmod +x scripts/test-rocks-only.sh
./scripts/test-rocks-only.sh
```

## Database Connection Information

- **PostgreSQL URL**: `postgres://postgres:postgres@localhost:5432/indexer`
- **Test Database URL**: `postgres://postgres:postgres@localhost:5432/indexer_test`

## Notes

- Always ensure you're in the Nix environment when running database commands
- The scripts use `nix develop --command bash -c "..."` to ensure correct environment
- Data is stored in the `data/postgres` directory 