#!/bin/bash
# Initialize and start databases for the Almanac project
set -e

# Check if we're running in a Nix shell
if [ -z "$IN_NIX_SHELL" ]; then
  echo "This script must be run within a Nix shell. Please run 'nix develop' first."
  exit 1
fi

echo "=== Almanac Database Initialization ==="

# Create required data directories
mkdir -p data/rocksdb
mkdir -p data/postgres

# === PostgreSQL initialization and startup ===
echo "Initializing PostgreSQL..."

# Configure PostgreSQL data directory
export PGDATA="$(pwd)/data/postgres"

# Initialize PostgreSQL if not already done
if [ ! -f "$PGDATA/PG_VERSION" ]; then
  echo "Creating new PostgreSQL database cluster..."
  initdb -D "$PGDATA" --no-locale --encoding=UTF8
  
  # Configure PostgreSQL to listen on localhost
  echo "listen_addresses = '127.0.0.1'" >> "$PGDATA/postgresql.conf"
  echo "port = 5432" >> "$PGDATA/postgresql.conf"
  
  echo "PostgreSQL initialized successfully."
else
  echo "Using existing PostgreSQL database at $PGDATA"
fi

# Start PostgreSQL server if not running
if ! pg_isready -q; then
  echo "Starting PostgreSQL server..."
  pg_ctl -D "$PGDATA" start -l "$PGDATA/postgres.log"
  
  # Wait for PostgreSQL to be ready
  attempt=0
  max_attempts=10
  until pg_isready -q || [ $attempt -eq $max_attempts ]; do
    attempt=$((attempt+1))
    echo "Waiting for PostgreSQL to be ready... (attempt $attempt/$max_attempts)"
    sleep 1
  done

  if [ $attempt -eq $max_attempts ]; then
    echo "Failed to connect to PostgreSQL after $max_attempts attempts."
    exit 1
  fi
else
  echo "PostgreSQL is already running."
fi

# Create development database if it doesn't exist
if ! psql -lqt | cut -d \| -f 1 | grep -qw indexer; then
  echo "Creating development database 'indexer'..."
  createdb indexer
  
  # Set DATABASE_URL for applications
  export DATABASE_URL="postgresql://localhost/indexer"
  echo "export DATABASE_URL=\"$DATABASE_URL\"" > .db_env
  
  # Run migrations if needed
  echo "Running PostgreSQL migrations..."
  cd crates/storage
  if command -v sqlx &> /dev/null; then
    sqlx migrate run
  else
    echo "sqlx-cli not found, skipping automatic migrations."
    echo "You may need to run migrations manually."
  fi
  cd ../..
else
  echo "Development database 'indexer' already exists."
fi

# === RocksDB initialization ===
echo -e "\nInitializing RocksDB storage..."

# RocksDB doesn't need a server, but we'll pre-create the directory
# and ensure permissions are correct
ROCKS_PATH="$(pwd)/data/rocksdb"
mkdir -p "$ROCKS_PATH"
echo "RocksDB storage directory prepared at $ROCKS_PATH"

# === Finish ===
echo -e "\n=== Database Initialization Complete ==="
echo "PostgreSQL: Running on localhost:5432"
echo "  • Database: indexer"
echo "  • Connection URL: $DATABASE_URL"
echo "  • Data directory: $PGDATA"
echo "  • Log file: $PGDATA/postgres.log"
echo ""
echo "RocksDB:"
echo "  • Data directory: $ROCKS_PATH"
echo "  • Access through application configs pointing to this path"
echo ""
echo "Environment variables have been saved to .db_env"

# To ensure these variables are available in the current shell
echo "Run 'source .db_env' to load database environment variables in your current shell." 