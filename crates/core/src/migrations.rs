/// Database migration system for schema changes and data transformations
use std::collections::HashMap;
use std::time::{SystemTime, Duration};
use serde::{Deserialize, Serialize};
use async_trait::async_trait;
use chrono::{DateTime, Utc};

use crate::{Result, Error};

/// Migration configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationConfig {
    /// Table to store migration history
    pub migrations_table: String,
    
    /// Migration file directory
    pub migrations_dir: String,
    
    /// Whether to create migrations table if it doesn't exist
    pub auto_create_table: bool,
    
    /// Maximum time to wait for migration lock
    pub lock_timeout: Duration,
    
    /// Whether to run migrations in transactions
    pub use_transactions: bool,
    
    /// Whether to backup before running migrations
    pub backup_before_migration: bool,
    
    /// Rollback strategy
    pub rollback_strategy: RollbackStrategy,
}

impl Default for MigrationConfig {
    fn default() -> Self {
        Self {
            migrations_table: "schema_migrations".to_string(),
            migrations_dir: "migrations".to_string(),
            auto_create_table: true,
            lock_timeout: Duration::from_secs(60),
            use_transactions: true,
            backup_before_migration: false,
            rollback_strategy: RollbackStrategy::Manual,
        }
    }
}

/// Rollback strategy for failed migrations
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RollbackStrategy {
    /// No automatic rollback
    Manual,
    
    /// Rollback to previous migration
    Previous,
    
    /// Rollback to specific version
    ToVersion(String),
    
    /// Rollback all migrations
    All,
}

/// Migration direction
#[derive(Debug, Clone, PartialEq)]
pub enum MigrationDirection {
    Up,
    Down,
}

/// Migration status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MigrationStatus {
    Pending,
    Running,
    Completed,
    Failed,
    RolledBack,
}

/// Migration metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationInfo {
    /// Migration version/identifier
    pub version: String,
    
    /// Migration name
    pub name: String,
    
    /// Migration description
    pub description: Option<String>,
    
    /// Migration status
    pub status: MigrationStatus,
    
    /// When migration was applied
    pub applied_at: Option<SystemTime>,
    
    /// How long the migration took
    pub duration: Option<Duration>,
    
    /// Migration checksum
    pub checksum: Option<String>,
    
    /// Rollback information
    pub rollback_info: Option<String>,
    
    /// Migration dependencies
    pub dependencies: Vec<String>,
    
    /// Tags for categorizing migrations
    pub tags: Vec<String>,
}

/// A database migration definition
#[derive(Debug, Clone)]
pub struct Migration {
    /// Migration metadata
    pub info: MigrationInfo,
    
    /// SQL or operations to run when migrating up
    pub up: MigrationOperation,
    
    /// SQL or operations to run when migrating down
    pub down: MigrationOperation,
}

/// Migration operation definition
#[derive(Debug)]
pub enum MigrationOperation {
    /// Raw SQL statements
    Sql(Vec<String>),
    
    /// Rust function to execute
    Function(fn() -> Result<()>),
    
    /// Custom operation
    Custom(String), // Simplified to just store operation description
}

impl Clone for MigrationOperation {
    fn clone(&self) -> Self {
        match self {
            MigrationOperation::Sql(statements) => MigrationOperation::Sql(statements.clone()),
            MigrationOperation::Function(func) => MigrationOperation::Function(*func),
            MigrationOperation::Custom(desc) => MigrationOperation::Custom(desc.clone()),
        }
    }
}

/// Trait for custom migration executors
#[async_trait]
pub trait MigrationExecutor: Send + Sync {
    /// Execute the migration operation
    async fn execute(&self, direction: MigrationDirection) -> Result<()>;
    
    /// Validate the migration can be executed
    async fn validate(&self) -> Result<()>;
    
    /// Get operation description
    fn description(&self) -> String;
}

/// Migration runner trait
#[async_trait]
pub trait MigrationRunner: Send + Sync {
    /// Run migrations up to target version
    async fn migrate(&self, target: Option<String>) -> Result<Vec<MigrationInfo>>;
    
    /// Rollback migrations to target version
    async fn rollback(&self, target: Option<String>) -> Result<Vec<MigrationInfo>>;
    
    /// Get current migration status
    async fn status(&self) -> Result<Vec<MigrationInfo>>;
    
    /// Check if migrations are pending
    async fn pending(&self) -> Result<Vec<MigrationInfo>>;
    
    /// Validate all migrations
    async fn validate(&self) -> Result<Vec<String>>;
    
    /// Create new migration
    async fn create_migration(&self, name: &str, description: Option<&str>) -> Result<Migration>;
    
    /// Get migration history
    async fn history(&self) -> Result<Vec<MigrationInfo>>;
    
    /// Reset all migrations (danger!)
    async fn reset(&self) -> Result<()>;
}

/// Default migration runner implementation
pub struct DefaultMigrationRunner {
    config: MigrationConfig,
    migrations: Vec<Migration>,
    #[allow(dead_code)]
    db_executor: Box<dyn DatabaseExecutor>,
}

/// Database executor trait for running SQL
#[async_trait]
pub trait DatabaseExecutor: Send + Sync {
    /// Execute SQL statement
    async fn execute(&self, sql: &str) -> Result<u64>;
    
    /// Query and return results
    async fn query(&self, sql: &str) -> Result<Vec<serde_json::Value>>;
    
    /// Begin transaction
    async fn begin_transaction(&self) -> Result<()>;
    
    /// Commit transaction
    async fn commit(&self) -> Result<()>;
    
    /// Rollback transaction
    async fn rollback_transaction(&self) -> Result<()>;
    
    /// Check if table exists
    async fn table_exists(&self, table_name: &str) -> Result<bool>;
}

impl DefaultMigrationRunner {
    /// Create a new migration runner
    pub fn new(config: MigrationConfig, db_executor: Box<dyn DatabaseExecutor>) -> Self {
        Self {
            config,
            migrations: Vec::new(),
            db_executor,
        }
    }
    
    /// Add a migration to the runner
    pub fn add_migration(&mut self, migration: Migration) {
        self.migrations.push(migration);
    }
    
    /// Sort migrations by version
    #[allow(dead_code)]
    fn sort_migrations(&mut self) {
        self.migrations.sort_by(|a, b| a.info.version.cmp(&b.info.version));
    }
    
    /// Initialize migrations table
    async fn init_migrations_table(&self) -> Result<()> {
        if !self.config.auto_create_table {
            return Ok(());
        }
        
        let table_exists = self.db_executor.table_exists(&self.config.migrations_table).await?;
        if table_exists {
            return Ok(());
        }
        
        let create_table_sql = format!(
            r#"
            CREATE TABLE {} (
                version VARCHAR(255) PRIMARY KEY,
                name VARCHAR(255) NOT NULL,
                description TEXT,
                status VARCHAR(50) NOT NULL,
                applied_at TIMESTAMP,
                duration_ms BIGINT,
                checksum VARCHAR(255),
                rollback_info TEXT,
                dependencies TEXT,
                tags TEXT
            )
            "#,
            self.config.migrations_table
        );
        
        self.db_executor.execute(&create_table_sql).await?;
        
        tracing::info!("Created migrations table: {}", self.config.migrations_table);
        Ok(())
    }
    
    /// Get applied migrations from database
    async fn get_applied_migrations(&self) -> Result<HashMap<String, MigrationInfo>> {
        let sql = format!("SELECT * FROM {}", self.config.migrations_table);
        let rows = self.db_executor.query(&sql).await?;
        
        let mut applied = HashMap::new();
        for row in rows {
            let info = MigrationInfo {
                version: row.get("version")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string(),
                name: row.get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string(),
                description: row.get("description")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
                status: serde_json::from_value(
                    row.get("status").cloned().unwrap_or_default()
                ).unwrap_or(MigrationStatus::Pending),
                applied_at: row.get("applied_at")
                    .and_then(|v| v.as_str())
                    .and_then(|s| {
                        // Try to parse as chrono timestamp first, then fall back
                        DateTime::parse_from_rfc3339(s)
                            .map(|dt| SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(dt.timestamp() as u64))
                            .ok()
                    }),
                duration: row.get("duration_ms")
                    .and_then(|v| v.as_u64())
                    .map(|ms| Duration::from_millis(ms)),
                checksum: row.get("checksum")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
                rollback_info: row.get("rollback_info")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
                dependencies: row.get("dependencies")
                    .and_then(|v| v.as_str())
                    .map(|s| serde_json::from_str(s).unwrap_or_default())
                    .unwrap_or_default(),
                tags: row.get("tags")
                    .and_then(|v| v.as_str())
                    .map(|s| serde_json::from_str(s).unwrap_or_default())
                    .unwrap_or_default(),
            };
            applied.insert(info.version.clone(), info);
        }
        
        Ok(applied)
    }
    
    /// Record migration in database
    async fn record_migration(&self, info: &MigrationInfo) -> Result<()> {
        // Simplified parameter passing - in a real implementation you'd use proper parameter binding
        let values_sql = format!(
            r#"
            INSERT INTO {} 
            (version, name, description, status, applied_at, duration_ms, checksum, rollback_info, dependencies, tags)
            VALUES ('{}', '{}', '{}', '{}', '{}', {}, '{}', '{}', '{}', '{}')
            "#,
            self.config.migrations_table,
            info.version,
            info.name,
            info.description.as_deref().unwrap_or(""),
            serde_json::to_string(&info.status).unwrap_or_default().trim_matches('"'),
            info.applied_at.map(|t| format!("{:?}", t)).unwrap_or_default(),
            info.duration.map(|d| d.as_millis()).unwrap_or(0),
            info.checksum.as_deref().unwrap_or(""),
            info.rollback_info.as_deref().unwrap_or(""),
            serde_json::to_string(&info.dependencies).unwrap_or_default(),
            serde_json::to_string(&info.tags).unwrap_or_default()
        );
        
        self.db_executor.execute(&values_sql).await?;
        Ok(())
    }
    
    /// Execute migration operation
    async fn execute_operation(&self, operation: &MigrationOperation, _direction: MigrationDirection) -> Result<()> {
        match operation {
            MigrationOperation::Sql(statements) => {
                for sql in statements {
                    self.db_executor.execute(sql).await?;
                }
                Ok(())
            }
            MigrationOperation::Function(func) => {
                func()
            }
            MigrationOperation::Custom(description) => {
                tracing::info!("Executing custom migration: {}", description);
                // In a real implementation, custom operations would be handled differently
                Ok(())
            }
        }
    }
}

#[async_trait]
impl MigrationRunner for DefaultMigrationRunner {
    async fn migrate(&self, target: Option<String>) -> Result<Vec<MigrationInfo>> {
        self.init_migrations_table().await?;
        
        let applied = self.get_applied_migrations().await?;
        let mut completed = Vec::new();
        
        for migration in &self.migrations {
            // Check if we've reached the target
            if let Some(ref target_version) = target {
                if migration.info.version > *target_version {
                    break;
                }
            }
            
            // Skip if already applied
            if applied.contains_key(&migration.info.version) {
                continue;
            }
            
            // Check dependencies
            for dep in &migration.info.dependencies {
                if !applied.contains_key(dep) {
                    return Err(Error::Generic(format!(
                        "Migration {} depends on {} which is not applied",
                        migration.info.version, dep
                    )));
                }
            }
            
            // Execute migration
            let start_time = SystemTime::now();
            
            if self.config.use_transactions {
                self.db_executor.begin_transaction().await?;
            }
            
            match self.execute_operation(&migration.up, MigrationDirection::Up).await {
                Ok(()) => {
                    let duration = start_time.elapsed().ok();
                    
                    let mut info = migration.info.clone();
                    info.status = MigrationStatus::Completed;
                    info.applied_at = Some(SystemTime::now());
                    info.duration = duration;
                    
                    self.record_migration(&info).await?;
                    
                    if self.config.use_transactions {
                        self.db_executor.commit().await?;
                    }
                    
                    completed.push(info);
                    tracing::info!("Applied migration: {} - {}", migration.info.version, migration.info.name);
                }
                Err(e) => {
                    if self.config.use_transactions {
                        self.db_executor.rollback_transaction().await?;
                    }
                    
                    let mut info = migration.info.clone();
                    info.status = MigrationStatus::Failed;
                    
                    self.record_migration(&info).await.ok(); // Don't fail if we can't record
                    
                    return Err(Error::Generic(format!(
                        "Migration {} failed: {}",
                        migration.info.version, e
                    )));
                }
            }
        }
        
        Ok(completed)
    }
    
    async fn rollback(&self, target: Option<String>) -> Result<Vec<MigrationInfo>> {
        let applied = self.get_applied_migrations().await?;
        let mut rolled_back = Vec::new();
        
        // Find migrations to rollback (in reverse order)
        let mut to_rollback = Vec::new();
        for migration in self.migrations.iter().rev() {
            if !applied.contains_key(&migration.info.version) {
                continue;
            }
            
            to_rollback.push(migration);
            
            // Stop if we've reached the target
            if let Some(ref target_version) = target {
                if migration.info.version == *target_version {
                    break;
                }
            }
        }
        
        // Execute rollbacks
        for migration in to_rollback {
            if self.config.use_transactions {
                self.db_executor.begin_transaction().await?;
            }
            
            match self.execute_operation(&migration.down, MigrationDirection::Down).await {
                Ok(()) => {
                    // Remove from migrations table
                    let delete_sql = format!(
                        "DELETE FROM {} WHERE version = '{}'",
                        self.config.migrations_table, migration.info.version
                    );
                    self.db_executor.execute(&delete_sql).await?;
                    
                    if self.config.use_transactions {
                        self.db_executor.commit().await?;
                    }
                    
                    let mut info = migration.info.clone();
                    info.status = MigrationStatus::RolledBack;
                    rolled_back.push(info);
                    
                    tracing::info!("Rolled back migration: {} - {}", migration.info.version, migration.info.name);
                }
                Err(e) => {
                    if self.config.use_transactions {
                        self.db_executor.rollback_transaction().await?;
                    }
                    
                    return Err(Error::Generic(format!(
                        "Rollback of migration {} failed: {}",
                        migration.info.version, e
                    )));
                }
            }
        }
        
        Ok(rolled_back)
    }
    
    async fn status(&self) -> Result<Vec<MigrationInfo>> {
        self.init_migrations_table().await?;
        let applied = self.get_applied_migrations().await?;
        
        let mut status = Vec::new();
        for migration in &self.migrations {
            let info = applied.get(&migration.info.version)
                .cloned()
                .unwrap_or_else(|| {
                    let mut info = migration.info.clone();
                    info.status = MigrationStatus::Pending;
                    info
                });
            status.push(info);
        }
        
        Ok(status)
    }
    
    async fn pending(&self) -> Result<Vec<MigrationInfo>> {
        let status = self.status().await?;
        Ok(status.into_iter()
            .filter(|info| info.status == MigrationStatus::Pending)
            .collect())
    }
    
    async fn validate(&self) -> Result<Vec<String>> {
        let mut errors = Vec::new();
        
        // Check for duplicate versions
        let mut versions = std::collections::HashSet::new();
        for migration in &self.migrations {
            if !versions.insert(&migration.info.version) {
                errors.push(format!("Duplicate migration version: {}", migration.info.version));
            }
        }
        
        // Check dependencies
        for migration in &self.migrations {
            for dep in &migration.info.dependencies {
                if !self.migrations.iter().any(|m| m.info.version == *dep) {
                    errors.push(format!(
                        "Migration {} depends on {} which doesn't exist",
                        migration.info.version, dep
                    ));
                }
            }
        }
        
        Ok(errors)
    }
    
    async fn create_migration(&self, name: &str, description: Option<&str>) -> Result<Migration> {
        let version = format!("{}", Utc::now().format("%Y%m%d%H%M%S"));
        
        let info = MigrationInfo {
            version: version.clone(),
            name: name.to_string(),
            description: description.map(|s| s.to_string()),
            status: MigrationStatus::Pending,
            applied_at: None,
            duration: None,
            checksum: None,
            rollback_info: None,
            dependencies: Vec::new(),
            tags: Vec::new(),
        };
        
        let migration = Migration {
            info,
            up: MigrationOperation::Sql(vec!["-- Add your up migration here".to_string()]),
            down: MigrationOperation::Sql(vec!["-- Add your down migration here".to_string()]),
        };
        
        Ok(migration)
    }
    
    async fn history(&self) -> Result<Vec<MigrationInfo>> {
        self.init_migrations_table().await?;
        let applied = self.get_applied_migrations().await?;
        
        let mut history: Vec<_> = applied.into_values().collect();
        history.sort_by(|a, b| a.version.cmp(&b.version));
        
        Ok(history)
    }
    
    async fn reset(&self) -> Result<()> {
        // This is dangerous - only implement in development environments
        let delete_sql = format!("DELETE FROM {}", self.config.migrations_table);
        self.db_executor.execute(&delete_sql).await?;
        
        tracing::warn!("All migrations have been reset!");
        Ok(())
    }
}

/// Migration file generator
pub struct MigrationGenerator {
    #[allow(dead_code)]
    config: MigrationConfig,
}

impl MigrationGenerator {
    /// Create a new migration generator
    pub fn new(config: MigrationConfig) -> Self {
        Self { config }
    }
    
    /// Generate migration file
    pub fn generate_file(&self, migration: &Migration) -> Result<String> {
        let up_sql = match &migration.up {
            MigrationOperation::Sql(statements) => statements.join(";\n"),
            _ => "-- Custom migration operation".to_string(),
        };
        
        let down_sql = match &migration.down {
            MigrationOperation::Sql(statements) => statements.join(";\n"),
            _ => "-- Custom migration operation".to_string(),
        };
        
        let content = format!(
            r#"-- Migration: {}
-- Name: {}
-- Description: {}
-- Created: {:?}

-- +migrate Up
{};

-- +migrate Down
{};
"#,
            migration.info.version,
            migration.info.name,
            migration.info.description.as_deref().unwrap_or(""),
            SystemTime::now(),
            up_sql,
            down_sql
        );
        
        Ok(content)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use tokio::sync::RwLock;
    
    // Mock database executor for testing
    struct MockDatabaseExecutor {
        tables: Arc<RwLock<std::collections::HashSet<String>>>,
        data: Arc<RwLock<HashMap<String, Vec<serde_json::Value>>>>,
    }
    
    impl MockDatabaseExecutor {
        fn new() -> Self {
            Self {
                tables: Arc::new(RwLock::new(std::collections::HashSet::new())),
                data: Arc::new(RwLock::new(HashMap::new())),
            }
        }
    }
    
    #[async_trait]
    impl DatabaseExecutor for MockDatabaseExecutor {
        async fn execute(&self, sql: &str) -> Result<u64> {
            if sql.contains("CREATE TABLE") {
                let table_name = "schema_migrations"; // Simplified
                let mut tables = self.tables.write().await;
                tables.insert(table_name.to_string());
                Ok(1)
            } else if sql.contains("INSERT INTO") {
                Ok(1)
            } else if sql.contains("DELETE FROM") {
                Ok(1)
            } else {
                Ok(0)
            }
        }
        
        async fn query(&self, _sql: &str) -> Result<Vec<serde_json::Value>> {
            Ok(vec![])
        }
        
        async fn begin_transaction(&self) -> Result<()> { Ok(()) }
        async fn commit(&self) -> Result<()> { Ok(()) }
        async fn rollback_transaction(&self) -> Result<()> { Ok(()) }
        
        async fn table_exists(&self, table_name: &str) -> Result<bool> {
            let tables = self.tables.read().await;
            Ok(tables.contains(table_name))
        }
    }
    
    #[test]
    fn test_migration_config_default() {
        let config = MigrationConfig::default();
        
        assert_eq!(config.migrations_table, "schema_migrations");
        assert_eq!(config.migrations_dir, "migrations");
        assert!(config.auto_create_table);
        assert_eq!(config.rollback_strategy, RollbackStrategy::Manual);
    }
    
    #[tokio::test]
    async fn test_migration_runner_creation() {
        let config = MigrationConfig::default();
        let db_executor = Box::new(MockDatabaseExecutor::new());
        let mut runner = DefaultMigrationRunner::new(config, db_executor);
        
        // Test adding a migration
        let migration = Migration {
            info: MigrationInfo {
                version: "001".to_string(),
                name: "test_migration".to_string(),
                description: Some("Test migration".to_string()),
                status: MigrationStatus::Pending,
                applied_at: None,
                duration: None,
                checksum: None,
                rollback_info: None,
                dependencies: Vec::new(),
                tags: vec!["test".to_string()],
            },
            up: MigrationOperation::Sql(vec!["CREATE TABLE test (id INT)".to_string()]),
            down: MigrationOperation::Sql(vec!["DROP TABLE test".to_string()]),
        };
        
        runner.add_migration(migration);
        assert_eq!(runner.migrations.len(), 1);
    }
    
    #[tokio::test]
    async fn test_migration_validation() {
        let config = MigrationConfig::default();
        let db_executor = Box::new(MockDatabaseExecutor::new());
        let mut runner = DefaultMigrationRunner::new(config, db_executor);
        
        // Add migration with dependency
        let migration = Migration {
            info: MigrationInfo {
                version: "002".to_string(),
                name: "dependent_migration".to_string(),
                description: None,
                status: MigrationStatus::Pending,
                applied_at: None,
                duration: None,
                checksum: None,
                rollback_info: None,
                dependencies: vec!["001".to_string()],
                tags: Vec::new(),
            },
            up: MigrationOperation::Sql(vec!["ALTER TABLE test ADD COLUMN name VARCHAR(255)".to_string()]),
            down: MigrationOperation::Sql(vec!["ALTER TABLE test DROP COLUMN name".to_string()]),
        };
        
        runner.add_migration(migration);
        
        let errors = runner.validate().await.unwrap();
        assert!(errors.len() > 0);
        assert!(errors[0].contains("depends on 001 which doesn't exist"));
    }
    
    #[test]
    fn test_migration_generator() {
        let config = MigrationConfig::default();
        let generator = MigrationGenerator::new(config);
        
        let migration = Migration {
            info: MigrationInfo {
                version: "20231201120000".to_string(),
                name: "create_users_table".to_string(),
                description: Some("Create users table".to_string()),
                status: MigrationStatus::Pending,
                applied_at: None,
                duration: None,
                checksum: None,
                rollback_info: None,
                dependencies: Vec::new(),
                tags: Vec::new(),
            },
            up: MigrationOperation::Sql(vec![
                "CREATE TABLE users (id SERIAL PRIMARY KEY, name VARCHAR(255))".to_string()
            ]),
            down: MigrationOperation::Sql(vec![
                "DROP TABLE users".to_string()
            ]),
        };
        
        let content = generator.generate_file(&migration).unwrap();
        
        assert!(content.contains("Migration: 20231201120000"));
        assert!(content.contains("Name: create_users_table"));
        assert!(content.contains("CREATE TABLE users"));
        assert!(content.contains("DROP TABLE users"));
        assert!(content.contains("-- +migrate Up"));
        assert!(content.contains("-- +migrate Down"));
    }
    
    #[test]
    fn test_rollback_strategies() {
        assert_eq!(RollbackStrategy::Manual, RollbackStrategy::Manual);
        assert_eq!(RollbackStrategy::Previous, RollbackStrategy::Previous);
        assert_eq!(RollbackStrategy::ToVersion("001".to_string()), RollbackStrategy::ToVersion("001".to_string()));
        assert_eq!(RollbackStrategy::All, RollbackStrategy::All);
    }
} 