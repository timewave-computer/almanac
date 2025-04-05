#!/bin/bash
# This script sets up a temporary PostgreSQL instance, applies migrations,
# and runs `cargo sqlx prepare` to generate offline query data.
# It ensures PostgreSQL is available from the Nix environment.

set -euo pipefail

# --- Configuration ---
# Use a temporary directory for PG data within the project's preferred ignored dir
PG_DATA_DIR="$(git rev-parse --show-toplevel)/data/pgsql_temp_sqlx_prepare"
PG_LOG_FILE="$PG_DATA_DIR/pgsql.log"
PG_SOCKET_DIR="$PG_DATA_DIR" # Keep socket in data dir for simplicity
DB_PORT="5433" # Use a non-default port to avoid conflicts
DB_USER="sqlx_prep_user"
DB_NAME="sqlx_prep_db"
MIGRATION_DIR="./crates/storage/migrations"

# Database connection string (points to the temporary instance)
export DATABASE_URL="postgres://${DB_USER}@${PG_SOCKET_DIR}:${DB_PORT}/${DB_NAME}"

# --- Cleanup Function ---
cleanup() {
    echo "Cleaning up temporary PostgreSQL instance..."
    # Check if server is running before trying to stop
    if pg_ctl status -D "$PG_DATA_DIR" > /dev/null; then
        echo "Stopping PostgreSQL server..."
        pg_ctl stop -D "$PG_DATA_DIR" -m fast >> "$PG_LOG_FILE" 2>&1 || echo "Failed to stop server gracefully."
    else
        echo "PostgreSQL server not running or already stopped."
    fi
    
    echo "Removing temporary data directory: $PG_DATA_DIR"
    rm -rf "$PG_DATA_DIR"
    echo "Cleanup complete."
}

# Ensure cleanup runs on exit or interrupt
trap cleanup EXIT SIGINT SIGTERM

# --- Check Dependencies ---
command -v initdb >/dev/null 2>&1 || { echo >&2 "Error: initdb not found. Is postgresql in your Nix shell?"; exit 1; }
command -v pg_ctl >/dev/null 2>&1 || { echo >&2 "Error: pg_ctl not found. Is postgresql in your Nix shell?"; exit 1; }
command -v psql >/dev/null 2>&1 || { echo >&2 "Error: psql not found. Is postgresql in your Nix shell?"; exit 1; }
command -v createdb >/dev/null 2>&1 || { echo >&2 "Error: createdb not found. Is postgresql in your Nix shell?"; exit 1; }
command -v cargo >/dev/null 2>&1 || { echo >&2 "Error: cargo not found. Is Rust installed in your Nix shell?"; exit 1; }
command -v sqlx >/dev/null 2>&1 || { echo >&2 "Error: sqlx not found. Is sqlx-cli in your Nix shell?"; exit 1; }


# --- Setup ---
echo "Setting up temporary PostgreSQL instance in $PG_DATA_DIR"
mkdir -p "$PG_DATA_DIR"
# Initialize the database cluster
echo "Initializing database cluster (forcing C locale)..."
export LC_ALL=C # Avoid locale issues
initdb -D "$PG_DATA_DIR" --username="$DB_USER" --no-locale --encoding=UTF8 || {
    echo "Error: initdb failed. See output above."
    exit 1
}

# Configure PostgreSQL to listen on the specified port and socket directory
echo "port = $DB_PORT" >> "$PG_DATA_DIR/postgresql.conf"
echo "unix_socket_directories = '$PG_SOCKET_DIR'" >> "$PG_DATA_DIR/postgresql.conf"
echo "listen_addresses = ''" >> "$PG_DATA_DIR/postgresql.conf" # Listen only on Unix socket

# Start the PostgreSQL server
echo "Starting PostgreSQL server (logging to $PG_LOG_FILE)..."
pg_ctl start -D "$PG_DATA_DIR" -l "$PG_LOG_FILE" -o "-k $PG_SOCKET_DIR"

# Wait for the server to be ready
echo -n "Waiting for database server to start..."
retries=10
while ! pg_isready -h "$PG_SOCKET_DIR" -p "$DB_PORT" -U "$DB_USER" -q; do
    sleep 1
    retries=$((retries - 1))
    if [ $retries -eq 0 ]; then
        echo " FAILED!"
        echo "Error: PostgreSQL server failed to start. Check logs: $PG_LOG_FILE"
        exit 1
    fi
    echo -n "."
done
echo " OK"

# --- Database Operations ---
# Create the target database
echo "Creating database '$DB_NAME'..."
createdb -h "$PG_SOCKET_DIR" -p "$DB_PORT" -U "$DB_USER" "$DB_NAME" || {
    echo "Error: Failed to create database '$DB_NAME'. Check logs: $PG_LOG_FILE"
    exit 1
}

# Apply migrations (using sqlx migrate directly is often simpler)
echo "Applying migrations from ${MIGRATION_DIR}..."
sqlx migrate run --database-url "${DATABASE_URL}" --source "${MIGRATION_DIR}" || {
    echo "Error: Failed to apply migrations. Check logs: $PG_LOG_FILE"
    exit 1
}

# --- Generate SQLx Data ---
echo "Generating sqlx-data.json..."
# Navigate to project root (important for sqlx prepare finding .sqlx dir)
cd "$(git rev-parse --show-toplevel)" 
cargo sqlx prepare --workspace --database-url "${DATABASE_URL}" || {
    echo "Error: Failed to generate sqlx-data.json. Check logs: $PG_LOG_FILE"
    # Don't exit immediately, cleanup will still run
    exit 1 
}

echo "SQLx data preparation successful!"
echo "Database preparation complete! Temporary PostgreSQL instance will be stopped and cleaned up automatically."

# Cleanup is handled by the trap EXIT

exit 0 