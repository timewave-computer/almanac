#!/bin/bash
# Purpose: Run tests with the fixed database schema

set -e

echo "=== Running Tests With Fixed Database Schema ==="

# Make sure the PostgreSQL server is running
nix develop --command bash -c "
  # Check PostgreSQL status
  if ! pg_isready -h localhost -p 5432 -U postgres > /dev/null 2>&1; then
    echo 'PostgreSQL is not running. Starting PostgreSQL...'
    pg_ctl -D data/postgres start -l data/postgres/postgres.log
    sleep 2
  fi
  
  echo 'PostgreSQL is running'

  # Run your specific test command here
  cd crates/storage
  
  echo 'Running application with database fix...'
  SQLX_OFFLINE=false cargo run --example basic_storage_test
"

echo "=== Test completed ===" 