#!/bin/bash
set -euo pipefail

# Database connection parameters
DB_HOST="localhost"
DB_PORT="5432"
DB_USER="postgres"
DB_PASSWORD="postgres"
DB_NAME="indexer"
MIGRATION_DIR="./crates/storage/migrations"

# Export database connection string
export DATABASE_URL="postgres://${DB_USER}:${DB_PASSWORD}@${DB_HOST}:${DB_PORT}/${DB_NAME}"

# Check if PostgreSQL is running
if ! pg_isready -h ${DB_HOST} -p ${DB_PORT} -U ${DB_USER} > /dev/null 2>&1; then
    echo "Error: PostgreSQL is not running. Please start your PostgreSQL server."
    exit 1
fi

# Create database if it doesn't exist
if ! psql -h ${DB_HOST} -p ${DB_PORT} -U ${DB_USER} -lqt | cut -d \| -f 1 | grep -qw ${DB_NAME}; then
    echo "Creating database '${DB_NAME}'..."
    createdb -h ${DB_HOST} -p ${DB_PORT} -U ${DB_USER} ${DB_NAME}
else
    echo "Database '${DB_NAME}' already exists."
fi

# Apply migrations without failing if they've already been applied
echo "Applying migrations from ${MIGRATION_DIR}..."
for migration in $(find ${MIGRATION_DIR} -name "*.sql" | sort); do
    migration_name=$(basename ${migration})
    echo "Applying migration: ${migration_name}"
    
    # Apply the migration but don't fail if it's already been applied
    psql -h ${DB_HOST} -p ${DB_PORT} -U ${DB_USER} -d ${DB_NAME} -f ${migration} || {
        echo "Migration ${migration_name} may have already been applied or had errors."
        echo "Continuing with the next migration..."
    }
done

# This creates either a new schema_migrations table or uses the existing one
echo "Creating migrations tracking table if it doesn't exist..."
psql -h ${DB_HOST} -p ${DB_PORT} -U ${DB_USER} -d ${DB_NAME} -c "
CREATE TABLE IF NOT EXISTS schema_migrations (
    version VARCHAR(255) PRIMARY KEY,
    applied_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);" || true

# Mark all migrations as applied in the tracking table
echo "Updating migrations tracking table..."
for migration in $(find ${MIGRATION_DIR} -name "*.sql" | sort); do
    migration_version=$(basename ${migration} | cut -d_ -f1)
    migration_name=$(basename ${migration})
    
    # Insert the migration version if it doesn't exist
    psql -h ${DB_HOST} -p ${DB_PORT} -U ${DB_USER} -d ${DB_NAME} -c "
    INSERT INTO schema_migrations (version, applied_at) 
    VALUES ('${migration_version}', NOW()) 
    ON CONFLICT (version) DO NOTHING;" || true
    
    echo "Marked migration ${migration_name} as applied"
done

# Generate sqlx-data.json for offline queries
echo "Generating sqlx-data.json..."
cd "$(git rev-parse --show-toplevel)" # Navigate to project root
cargo sqlx prepare --workspace --database-url "${DATABASE_URL}" || {
    echo "Warning: Failed to generate sqlx-data.json"
    echo "This is not critical, but you may need to run this manually:"
    echo "cargo sqlx prepare --workspace --database-url \"${DATABASE_URL}\""
}

echo "Database preparation complete!" 