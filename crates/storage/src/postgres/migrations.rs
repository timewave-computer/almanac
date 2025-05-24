/// PostgreSQL migrations using sqlx
use std::path::Path;
use std::time::SystemTime;

use sqlx::{Pool, Postgres};
use sqlx::migrate::{MigrateDatabase, Migrator};
use sqlx::Row;
use tracing::{info, warn};

use indexer_core::{Result, Error};

/// Schema module with contract schema types
pub mod schema {
    use std::collections::HashMap;

    /// Registry for contract schemas
    pub trait ContractSchemaRegistry {
        /// Get schema for a contract
        fn get_schema(&self, chain: &str, address: &str) -> Option<&ContractSchema>;
        
        /// Store schema for a contract
        fn store_schema(&mut self, chain: &str, address: &str, schema: ContractSchema);
    }

    /// Contract ABI schema
    #[derive(Debug, Clone)]
    pub struct ContractSchema {
        /// The contract chain
        pub chain: String,
        
        /// The contract address
        pub address: String,
        
        /// Raw schema data
        pub schema_data: Vec<u8>,
    }

    /// In-memory schema registry implementation
    pub struct InMemorySchemaRegistry {
        schemas: HashMap<String, ContractSchema>,
    }

    impl InMemorySchemaRegistry {
        /// Create a new in-memory schema registry
        pub fn new() -> Self {
            Self {
                schemas: HashMap::new(),
            }
        }
    }

    impl ContractSchemaRegistry for InMemorySchemaRegistry {
        fn get_schema(&self, chain: &str, address: &str) -> Option<&ContractSchema> {
            let key = format!("{}:{}", chain, address);
            self.schemas.get(&key)
        }
        
        fn store_schema(&mut self, chain: &str, address: &str, schema: ContractSchema) {
            let key = format!("{}:{}", chain, address);
            self.schemas.insert(key, schema);
        }
    }
}

/// Migration manager for PostgreSQL
pub struct PostgresMigrationManager {
    /// Database connection string
    pub connection_string: String,
    
    /// Path to migration files
    pub migrations_path: String,
}

impl PostgresMigrationManager {
    /// Create a new migration manager
    pub fn new(connection_string: impl Into<String>, migrations_path: impl Into<String>) -> Self {
        Self {
            connection_string: connection_string.into(),
            migrations_path: migrations_path.into(),
        }
    }
    
    /// Check if database exists, create if it doesn't
    pub async fn ensure_database_exists(&self) -> Result<()> {
        let database_url = &self.connection_string;
        
        if !Postgres::database_exists(database_url).await? {
            info!("Database does not exist, creating it");
            Postgres::create_database(database_url).await?;
        }
        
        Ok(())
    }
    
    /// Migrate the database to the latest version
    pub async fn migrate(&self) -> Result<()> {
        // Ensure database exists
        self.ensure_database_exists().await?;
        
        // Load migrations from the file system
        let migrations_path = Path::new(&self.migrations_path);
        if !migrations_path.exists() {
            warn!("Migrations directory not found at {}", self.migrations_path);
            return Ok(());
        }
        
        let migrator = match Migrator::new(migrations_path).await {
            Ok(m) => m,
            Err(e) => {
                warn!("Failed to load migrations: {}", e);
                return Err(Error::Storage(format!("Failed to load migrations: {}", e)));
            }
        };
        
        // Connect to the database
        let pool = Pool::<Postgres>::connect(&self.connection_string).await?;
        
        // Apply migrations
        let start = SystemTime::now();
        info!("Applying migrations from {}", self.migrations_path);
        
        migrator.run(&pool).await?;
        
        let elapsed = SystemTime::now().duration_since(start).unwrap_or_default();
        info!("Migrations applied successfully in {:.2}s", elapsed.as_secs_f32());
        
        Ok(())
    }
}

/// Initialize the database with migrations
pub async fn initialize_database(connection_string: &str, migrations_path: &str) -> Result<()> {
    let manager = PostgresMigrationManager::new(connection_string, migrations_path);
    manager.migrate().await
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;
    
    #[tokio::test]
    async fn test_migrations() -> Result<()> {
        // Create a temporary directory for migrations
        let dir = tempdir()?;
        let migrations_dir = dir.path().join("migrations");
        fs::create_dir_all(&migrations_dir)?;
        
        // Create a test migration
        let migration_file = migrations_dir.join("20220101000000_create_test_table.sql");
        fs::write(migration_file, r#"
-- Create a test table
CREATE TABLE IF NOT EXISTS test_table (
    id SERIAL PRIMARY KEY,
    name TEXT NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Add an index
CREATE INDEX IF NOT EXISTS idx_test_table_name ON test_table(name);
        "#)?;
        
        // Create a connection string for a test database
        let connection_string = "postgres://postgres:postgres@localhost/indexer_test";
        
        // Run migrations
        let manager = PostgresMigrationManager::new(connection_string, migrations_dir.to_string_lossy());
        
        // Skip the actual test if we can't connect to the database
        // This allows the tests to pass in CI environments without a database
        if !Postgres::database_exists(connection_string).await.unwrap_or(false) {
            warn!("Test database not available, skipping migration test");
            return Ok(());
        }
        
        // Run migrations
        manager.migrate().await?;
        
        // Verify that migrations worked by querying the database
        let pool = Pool::<Postgres>::connect(connection_string).await?;
        
        // Check if the table exists
        let result = sqlx::query("SELECT EXISTS (SELECT 1 FROM pg_tables WHERE tablename = 'test_table') as exists")
            .fetch_one(&pool)
            .await?;
        
        let exists: bool = result.try_get::<bool, _>("exists")?;
        assert!(exists);
        
        // Clean up
        sqlx::query("DROP TABLE IF EXISTS test_table")
            .execute(&pool)
            .await?;
        
        // Drop the test database
        drop(pool);
        Postgres::drop_database(connection_string).await?;
        
        Ok(())
    }
} 