#!/bin/bash
# Initialize PostgreSQL database for development

set -e

echo "=== Initializing PostgreSQL Database for Development ==="

# Get the project root directory
PROJECT_ROOT="$(git rev-parse --show-toplevel)"
PG_DATA_DIR="$PROJECT_ROOT/data/postgres"

# Create data directory if it doesn't exist
mkdir -p "$PG_DATA_DIR"

# Initialize PostgreSQL if not already done
if [ ! -f "$PG_DATA_DIR/PG_VERSION" ]; then
    echo "Creating new PostgreSQL database cluster..."
    initdb -D "$PG_DATA_DIR" --no-locale --encoding=UTF8
    
    # Configure PostgreSQL to listen on localhost
    echo "listen_addresses = '127.0.0.1'" >> "$PG_DATA_DIR/postgresql.conf"
    echo "port = 5432" >> "$PG_DATA_DIR/postgresql.conf"
    
    echo "PostgreSQL initialized successfully."
else
    echo "Using existing PostgreSQL database at $PG_DATA_DIR"
fi

# Start PostgreSQL server if not running
if ! pg_isready -q; then
    echo "Starting PostgreSQL server..."
    pg_ctl -D "$PG_DATA_DIR" start -l "$PG_DATA_DIR/postgres.log"
    
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
    echo "export DATABASE_URL=\"$DATABASE_URL\"" > "$PROJECT_ROOT/.db_env"
    
    # Run migrations if needed
    echo "Running PostgreSQL migrations..."
    cd "$PROJECT_ROOT/crates/storage"
    if command -v sqlx &> /dev/null; then
        sqlx migrate run
    else
        echo "sqlx-cli not found, skipping automatic migrations."
        echo "You may need to run migrations manually."
    fi
    cd "$PROJECT_ROOT"
else
    echo "Development database 'indexer' already exists."
fi

echo "=== PostgreSQL Database Initialization Complete ==="
echo "Database: indexer"
echo "URL: postgresql://localhost/indexer"
echo "To use, run: source .db_env" 