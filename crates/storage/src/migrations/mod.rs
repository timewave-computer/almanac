/// Database migration framework
mod schema;

pub use self::schema::*;

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use async_trait::async_trait;
use sqlx::{Pool, Postgres};
use tracing::{debug, error, info};

use indexer_core::{Result, Error};

/// Migration status
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MigrationStatus {
    /// Migration has not been applied
    Pending,
    
    /// Migration is currently being applied
    InProgress,
    
    /// Migration has been successfully applied
    Complete,
    
    /// Migration failed
    Failed,
    
    /// Migration has been rolled back
    RolledBack,
}

/// Migration direction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MigrationDirection {
    /// Apply the migration (up)
    Up,
    
    /// Rollback the migration (down)
    Down,
}

/// Migration metadata
#[derive(Debug, Clone)]
pub struct MigrationMeta {
    /// Migration ID
    pub id: String,
    
    /// Migration description
    pub description: String,
    
    /// When the migration was created
    pub created_at: u64,
    
    /// When the migration was applied
    pub applied_at: Option<u64>,
    
    /// Migration status
    pub status: MigrationStatus,
    
    /// Error message if failed
    pub error: Option<String>,
}

/// Migration definition
#[async_trait]
pub trait Migration: Send + Sync {
    /// Get migration ID
    fn id(&self) -> &str;
    
    /// Get migration description
    fn description(&self) -> &str;
    
    /// Apply the migration (up)
    async fn up(&self) -> Result<()>;
    
    /// Rollback the migration (down)
    async fn down(&self) -> Result<()>;
    
    /// Get migration dependencies
    fn dependencies(&self) -> Vec<String> {
        Vec::new()
    }
    
    /// Get migration metadata
    fn metadata(&self) -> MigrationMeta {
        MigrationMeta {
            id: self.id().to_string(),
            description: self.description().to_string(),
            created_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            applied_at: None,
            status: MigrationStatus::Pending,
            error: None,
        }
    }
}

/// SQL migration
pub struct SqlMigration {
    /// Migration ID
    id: String,
    
    /// Migration description
    description: String,
    
    /// SQL to run for up migration
    up_sql: String,
    
    /// SQL to run for down migration
    down_sql: String,
    
    /// Database pool
    pool: Pool<Postgres>,
    
    /// Migration dependencies
    dependencies: Vec<String>,
}

impl SqlMigration {
    /// Create a new SQL migration
    pub fn new(
        id: impl Into<String>,
        description: impl Into<String>,
        up_sql: impl Into<String>,
        down_sql: impl Into<String>,
        pool: Pool<Postgres>,
    ) -> Self {
        Self {
            id: id.into(),
            description: description.into(),
            up_sql: up_sql.into(),
            down_sql: down_sql.into(),
            pool,
            dependencies: Vec::new(),
        }
    }
    
    /// Add dependencies
    pub fn with_dependencies(mut self, dependencies: Vec<String>) -> Self {
        self.dependencies = dependencies;
        self
    }
}

#[async_trait]
impl Migration for SqlMigration {
    fn id(&self) -> &str {
        &self.id
    }
    
    fn description(&self) -> &str {
        &self.description
    }
    
    async fn up(&self) -> Result<()> {
        sqlx::query(&self.up_sql)
            .execute(&self.pool)
            .await
            .map_err(|e| Error::Storage(format!("Failed to apply migration: {}", e)))?;
        
        Ok(())
    }
    
    async fn down(&self) -> Result<()> {
        sqlx::query(&self.down_sql)
            .execute(&self.pool)
            .await
            .map_err(|e| Error::Storage(format!("Failed to rollback migration: {}", e)))?;
        
        Ok(())
    }
    
    fn dependencies(&self) -> Vec<String> {
        self.dependencies.clone()
    }
}

/// RocksDB migration
pub struct RocksMigration {
    /// Migration ID
    id: String,
    
    /// Migration description
    description: String,
    
    /// Function to run for up migration
    up_fn: Box<dyn Fn() -> Result<()> + Send + Sync>,
    
    /// Function to run for down migration
    down_fn: Box<dyn Fn() -> Result<()> + Send + Sync>,
    
    /// Migration dependencies
    dependencies: Vec<String>,
}

impl RocksMigration {
    /// Create a new RocksDB migration
    pub fn new(
        id: impl Into<String>,
        description: impl Into<String>,
        up_fn: impl Fn() -> Result<()> + Send + Sync + 'static,
        down_fn: impl Fn() -> Result<()> + Send + Sync + 'static,
    ) -> Self {
        Self {
            id: id.into(),
            description: description.into(),
            up_fn: Box::new(up_fn),
            down_fn: Box::new(down_fn),
            dependencies: Vec::new(),
        }
    }
    
    /// Add dependencies
    pub fn with_dependencies(mut self, dependencies: Vec<String>) -> Self {
        self.dependencies = dependencies;
        self
    }
}

#[async_trait]
impl Migration for RocksMigration {
    fn id(&self) -> &str {
        &self.id
    }
    
    fn description(&self) -> &str {
        &self.description
    }
    
    async fn up(&self) -> Result<()> {
        (self.up_fn)()
    }
    
    async fn down(&self) -> Result<()> {
        (self.down_fn)()
    }
    
    fn dependencies(&self) -> Vec<String> {
        self.dependencies.clone()
    }
}

/// Migration registry
pub struct MigrationRegistry {
    /// Registered migrations
    migrations: HashMap<String, Arc<dyn Migration>>,
    
    /// Migration metadata
    metadata: Mutex<HashMap<String, MigrationMeta>>,
    
    /// PostgreSQL pool for tracking migrations
    pg_pool: Option<Pool<Postgres>>,
}

impl MigrationRegistry {
    /// Create a new migration registry
    pub fn new() -> Self {
        Self {
            migrations: HashMap::new(),
            metadata: Mutex::new(HashMap::new()),
            pg_pool: None,
        }
    }
    
    /// Set PostgreSQL pool for tracking migrations
    pub fn with_postgres(mut self, pool: Pool<Postgres>) -> Self {
        self.pg_pool = Some(pool);
        self
    }
    
    /// Register a migration
    pub fn register(&mut self, migration: Arc<dyn Migration>) -> Result<()> {
        let id = migration.id().to_string();
        if self.migrations.contains_key(&id) {
            return Err(Error::Storage(format!("Migration with ID {} already registered", id)));
        }
        
        let metadata = migration.metadata();
        self.migrations.insert(id.clone(), migration);
        self.metadata.lock().unwrap().insert(id, metadata);
        
        Ok(())
    }
    
    /// Initialize migration tracking tables
    pub async fn initialize(&self) -> Result<()> {
        if let Some(pool) = &self.pg_pool {
            // Create migration tracking table
            sqlx::query(
                r#"
                CREATE TABLE IF NOT EXISTS migrations (
                    id TEXT PRIMARY KEY,
                    description TEXT NOT NULL,
                    created_at BIGINT NOT NULL,
                    applied_at BIGINT,
                    status TEXT NOT NULL,
                    error TEXT,
                    
                    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
                );
                "#
            )
            .execute(pool)
            .await
            .map_err(|e| Error::Storage(format!("Failed to create migrations table: {}", e)))?;
            
            // Create migration dependencies table
            sqlx::query(
                r#"
                CREATE TABLE IF NOT EXISTS migration_dependencies (
                    migration_id TEXT NOT NULL,
                    depends_on TEXT NOT NULL,
                    PRIMARY KEY (migration_id, depends_on),
                    FOREIGN KEY (migration_id) REFERENCES migrations(id)
                );
                "#
            )
            .execute(pool)
            .await
            .map_err(|e| Error::Storage(format!("Failed to create migration dependencies table: {}", e)))?;
            
            // Load existing migrations from database
            let rows = sqlx::query(
                r#"
                SELECT id, description, created_at, applied_at, status, error
                FROM migrations
                "#
            )
            .fetch_all(pool)
            .await
            .map_err(|e| Error::Storage(format!("Failed to load migrations: {}", e)))?;
            
            let mut metadata = self.metadata.lock().unwrap();
            for row in rows {
                let id: String = row.get("id");
                let status_str: String = row.get("status");
                let status = match status_str.as_str() {
                    "pending" => MigrationStatus::Pending,
                    "in_progress" => MigrationStatus::InProgress,
                    "complete" => MigrationStatus::Complete,
                    "failed" => MigrationStatus::Failed,
                    "rolled_back" => MigrationStatus::RolledBack,
                    _ => MigrationStatus::Pending,
                };
                
                let meta = MigrationMeta {
                    id: id.clone(),
                    description: row.get("description"),
                    created_at: row.get("created_at"),
                    applied_at: row.get("applied_at"),
                    status,
                    error: row.get("error"),
                };
                
                metadata.insert(id, meta);
            }
        }
        
        Ok(())
    }
    
    /// Get all migrations
    pub fn get_all(&self) -> Vec<MigrationMeta> {
        self.metadata.lock().unwrap().values().cloned().collect()
    }
    
    /// Get pending migrations
    pub fn get_pending(&self) -> Vec<MigrationMeta> {
        self.metadata
            .lock()
            .unwrap()
            .values()
            .filter(|meta| meta.status == MigrationStatus::Pending)
            .cloned()
            .collect()
    }
    
    /// Get migration by ID
    pub fn get(&self, id: &str) -> Option<MigrationMeta> {
        self.metadata.lock().unwrap().get(id).cloned()
    }
    
    /// Update migration status
    async fn update_status(
        &self,
        id: &str,
        status: MigrationStatus,
        error: Option<String>,
        applied_at: Option<u64>,
    ) -> Result<()> {
        // Update in-memory metadata
        {
            let mut metadata = self.metadata.lock().unwrap();
            if let Some(meta) = metadata.get_mut(id) {
                meta.status = status.clone();
                meta.error = error.clone();
                if let Some(applied) = applied_at {
                    meta.applied_at = Some(applied);
                }
            }
        }
        
        // Update in database if available
        if let Some(pool) = &self.pg_pool {
            let status_str = match status {
                MigrationStatus::Pending => "pending",
                MigrationStatus::InProgress => "in_progress",
                MigrationStatus::Complete => "complete",
                MigrationStatus::Failed => "failed",
                MigrationStatus::RolledBack => "rolled_back",
            };
            
            sqlx::query(
                r#"
                UPDATE migrations
                SET status = $1, error = $2, applied_at = $3, updated_at = NOW()
                WHERE id = $4
                "#
            )
            .bind(status_str)
            .bind(error)
            .bind(applied_at.map(|a| a as i64))
            .bind(id)
            .execute(pool)
            .await
            .map_err(|e| Error::Storage(format!("Failed to update migration status: {}", e)))?;
        }
        
        Ok(())
    }
    
    /// Apply migrations
    pub async fn apply(&self) -> Result<()> {
        let migrations = self.get_pending();
        if migrations.is_empty() {
            info!("No pending migrations to apply");
            return Ok(());
        }
        
        info!("Applying {} pending migrations", migrations.len());
        
        // Build dependency graph and calculate order
        let order = self.calculate_migration_order()?;
        
        for id in order {
            let migration = match self.migrations.get(&id) {
                Some(m) => m,
                None => {
                    error!("Migration {} not found", id);
                    continue;
                }
            };
            
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            
            info!("Applying migration {}: {}", id, migration.description());
            
            // Update status to in progress
            self.update_status(&id, MigrationStatus::InProgress, None, None).await?;
            
            // Apply migration
            match migration.up().await {
                Ok(()) => {
                    info!("Migration {} applied successfully", id);
                    self.update_status(&id, MigrationStatus::Complete, None, Some(now)).await?;
                }
                Err(e) => {
                    let error_msg = format!("Failed to apply migration {}: {}", id, e);
                    error!("{}", error_msg);
                    self.update_status(&id, MigrationStatus::Failed, Some(error_msg), None).await?;
                    return Err(e);
                }
            }
        }
        
        info!("All migrations applied successfully");
        
        Ok(())
    }
    
    /// Rollback a migration
    pub async fn rollback(&self, id: &str) -> Result<()> {
        let migration = match self.migrations.get(id) {
            Some(m) => m,
            None => {
                return Err(Error::Storage(format!("Migration {} not found", id)));
            }
        };
        
        let meta = match self.get(id) {
            Some(m) => m,
            None => {
                return Err(Error::Storage(format!("Migration metadata for {} not found", id)));
            }
        };
        
        if meta.status != MigrationStatus::Complete {
            return Err(Error::Storage(format!("Cannot rollback migration {} with status {:?}", id, meta.status)));
        }
        
        info!("Rolling back migration {}: {}", id, migration.description());
        
        // Apply rollback
        match migration.down().await {
            Ok(()) => {
                info!("Migration {} rolled back successfully", id);
                self.update_status(id, MigrationStatus::RolledBack, None, None).await?;
            }
            Err(e) => {
                let error_msg = format!("Failed to rollback migration {}: {}", id, e);
                error!("{}", error_msg);
                return Err(e);
            }
        }
        
        Ok(())
    }
    
    /// Calculate migration order based on dependencies
    fn calculate_migration_order(&self) -> Result<Vec<String>> {
        let mut result = Vec::new();
        let mut visited = HashSet::new();
        let mut temp = HashSet::new();
        
        let metadata = self.metadata.lock().unwrap();
        let pending: Vec<_> = metadata
            .values()
            .filter(|meta| meta.status == MigrationStatus::Pending)
            .collect();
        
        // Helper function for topological sort
        fn visit(
            id: &str,
            migrations: &HashMap<String, Arc<dyn Migration>>,
            visited: &mut HashSet<String>,
            temp: &mut HashSet<String>,
            result: &mut Vec<String>,
        ) -> Result<()> {
            if temp.contains(id) {
                return Err(Error::Storage(format!("Circular dependency detected involving migration {}", id)));
            }
            
            if visited.contains(id) {
                return Ok(());
            }
            
            temp.insert(id.to_string());
            
            if let Some(migration) = migrations.get(id) {
                for dep in migration.dependencies() {
                    visit(&dep, migrations, visited, temp, result)?;
                }
            }
            
            temp.remove(id);
            visited.insert(id.to_string());
            result.push(id.to_string());
            
            Ok(())
        }
        
        // Perform topological sort
        for meta in pending {
            if !visited.contains(&meta.id) {
                visit(
                    &meta.id,
                    &self.migrations,
                    &mut visited,
                    &mut temp,
                    &mut result,
                )?;
            }
        }
        
        Ok(result)
    }
}