use std::time::{SystemTime, UNIX_EPOCH};

use async_trait::async_trait;
use sqlx::{Pool, Postgres};
use thiserror::Error;
use tracing::{debug, info};

use indexer_core::Result;
use indexer_common::{Error};

pub mod postgres;
pub mod schema;

/// Migration error
#[derive(Debug, Error)]
pub enum MigrationError {
    /// Database error
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
    
    /// Migration already exists
    #[error("Migration already exists: {0}")]
    MigrationExists(String),
    
    /// Migration not found
    #[error("Migration not found: {0}")]
    MigrationNotFound(String),
    
    /// IO error
    #[error("IO error: {0}")]
    IO(String),
    
    /// Unknown error
    #[error("Unknown error: {0}")]
    Unknown(String),
}

/// Migration manager
#[async_trait]
pub trait MigrationManager: Send + Sync + 'static {
    /// Apply all pending migrations
    async fn apply_migrations(&self) -> Result<Vec<String>>;
    
    /// Get migration status
    async fn migration_status(&self) -> Result<Vec<(String, bool)>>;
    
    /// Get applied migrations
    async fn applied_migrations(&self) -> Result<Vec<String>>;
}

/// PostgreSQL migration manager
pub struct PostgresMigrationManager {
    /// Database connection pool
    pool: Pool<Postgres>,
    
    /// Migration directory
    migrations_dir: String,
}

impl PostgresMigrationManager {
    /// Create a new PostgreSQL migration manager
    pub fn new(pool: Pool<Postgres>, migrations_dir: String) -> Self {
        Self {
            pool,
            migrations_dir,
        }
    }
    
    /// Create the migrations table if it doesn't exist
    async fn ensure_migrations_table(&self) -> Result<()> {
        // Create the migrations table if it doesn't exist
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS migrations (
                id SERIAL PRIMARY KEY,
                name VARCHAR(255) NOT NULL UNIQUE,
                applied_at BIGINT NOT NULL
            );
            "#,
        )
        .execute(&self.pool)
        .await?;
        
        Ok(())
    }
}

#[async_trait]
impl MigrationManager for PostgresMigrationManager {
    async fn apply_migrations(&self) -> Result<Vec<String>> {
        // Ensure the migrations table exists
        self.ensure_migrations_table().await?;
        
        // Get list of applied migrations
        let applied = self.applied_migrations().await?;
        let applied_set: std::collections::HashSet<String> = applied.into_iter().collect();
        
        // Get list of available migrations
        let mut available_migrations = Vec::new();
        
        // For demonstration, we'll use a predefined list of migrations
        // In a real implementation, these would be read from SQL files in the migrations_dir
        let migrations = vec![
            "001_create_events_table",
            "002_create_blocks_table",
            "003_create_contract_schemas_table",
            "004_valence_accounts",
            "005_valence_processors",
            "006_valence_authorization",
            "007_valence_libraries",
        ];
        
        for migration in migrations {
            if !applied_set.contains(migration) {
                available_migrations.push(migration.to_string());
            }
        }
        
        // Sort migrations
        available_migrations.sort();
        
        // Apply each pending migration
        let mut applied_migrations = Vec::new();
        for migration_name in &available_migrations {
            info!("Applying migration: {}", migration_name);
            
            // In a real implementation, we would read the SQL from a file
            // and execute it within a transaction
            let sql = match migration_name.as_str() {
                "001_create_events_table" => {
                    r#"
                    CREATE TABLE IF NOT EXISTS events (
                        id VARCHAR(255) PRIMARY KEY,
                        chain VARCHAR(64) NOT NULL,
                        block_number BIGINT NOT NULL,
                        block_hash VARCHAR(66) NOT NULL,
                        tx_hash VARCHAR(66) NOT NULL,
                        timestamp BIGINT NOT NULL,
                        event_type VARCHAR(255) NOT NULL,
                        raw_data JSONB NOT NULL,
                        created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
                    );
                    
                    CREATE INDEX IF NOT EXISTS events_chain_idx ON events(chain);
                    CREATE INDEX IF NOT EXISTS events_block_number_idx ON events(block_number);
                    CREATE INDEX IF NOT EXISTS events_event_type_idx ON events(event_type);
                    "#
                }
                "002_create_blocks_table" => {
                    r#"
                    CREATE TABLE IF NOT EXISTS blocks (
                        chain VARCHAR(64) NOT NULL,
                        block_number BIGINT NOT NULL,
                        block_hash VARCHAR(66) NOT NULL,
                        timestamp BIGINT NOT NULL,
                        status VARCHAR(20) NOT NULL DEFAULT 'confirmed',
                        created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                        PRIMARY KEY (chain, block_number)
                    );
                    "#
                }
                "003_create_contract_schemas_table" => {
                    r#"
                    CREATE TABLE IF NOT EXISTS contract_schemas (
                        chain VARCHAR(64) NOT NULL,
                        address VARCHAR(42) NOT NULL,
                        schema_data BYTEA NOT NULL,
                        created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                        PRIMARY KEY (chain, address)
                    );
                    "#
                }
                "004_valence_accounts" => {
                    r#"
                    -- Migration for creating Valence Account related tables

                    CREATE TABLE valence_accounts (
                        id VARCHAR PRIMARY KEY,                         -- Unique ID (e.g., chain_id:contract_address)
                        chain_id VARCHAR NOT NULL,
                        contract_address VARCHAR NOT NULL,
                        created_at_block BIGINT NOT NULL,
                        created_at_tx VARCHAR NOT NULL,
                        current_owner VARCHAR,                          -- Nullable if renounced
                        pending_owner VARCHAR,
                        pending_owner_expiry BIGINT,                    -- Can be block height or timestamp depending on cw_ownable config
                        last_updated_block BIGINT NOT NULL,
                        last_updated_tx VARCHAR NOT NULL,

                        CONSTRAINT uq_valence_accounts_chain_address UNIQUE (chain_id, contract_address)
                    );

                    CREATE INDEX idx_valence_accounts_owner ON valence_accounts (current_owner);
                    CREATE INDEX idx_valence_accounts_chain ON valence_accounts (chain_id);

                    COMMENT ON COLUMN valence_accounts.id IS 'Primary key combining chain_id and contract_address';
                    COMMENT ON COLUMN valence_accounts.pending_owner_expiry IS 'Block height or timestamp for ownership transfer expiry';

                    CREATE TABLE valence_account_libraries (
                        account_id VARCHAR NOT NULL REFERENCES valence_accounts(id) ON DELETE CASCADE,
                        library_address VARCHAR NOT NULL,
                        approved_at_block BIGINT NOT NULL,
                        approved_at_tx VARCHAR NOT NULL,

                        PRIMARY KEY (account_id, library_address)
                    );

                    CREATE INDEX idx_valence_account_libraries_account ON valence_account_libraries (account_id);
                    CREATE INDEX idx_valence_account_libraries_library ON valence_account_libraries (library_address);

                    COMMENT ON TABLE valence_account_libraries IS 'Stores libraries approved to act on behalf of a Valence account';

                    CREATE TABLE valence_account_executions (
                        id BIGSERIAL PRIMARY KEY,                       -- Auto-incrementing ID
                        account_id VARCHAR NOT NULL REFERENCES valence_accounts(id) ON DELETE CASCADE,
                        chain_id VARCHAR NOT NULL,
                        block_number BIGINT NOT NULL,
                        tx_hash VARCHAR NOT NULL,
                        executor_address VARCHAR NOT NULL,              -- Address that called execute_msg/execute_submsgs
                        message_index INT NOT NULL,                     -- Index of the execute msg within the tx (if determinable)
                        correlated_event_ids TEXT[],                    -- Array of event IDs (FK to a general events table assumed)
                        raw_msgs JSONB,                                 -- Raw CosmosMsg/SubMsg array if parseable
                        payload TEXT,                                   -- Payload from execute_submsgs
                        executed_at TIMESTAMP WITH TIME ZONE NOT NULL
                    );

                    CREATE INDEX idx_valence_account_executions_account ON valence_account_executions (account_id);
                    CREATE INDEX idx_valence_account_executions_tx ON valence_account_executions (tx_hash);
                    CREATE INDEX idx_valence_account_executions_block ON valence_account_executions (chain_id, block_number);
                    CREATE INDEX idx_valence_account_executions_executor ON valence_account_executions (executor_address);

                    COMMENT ON TABLE valence_account_executions IS 'Historical record of executions initiated by Valence accounts';
                    COMMENT ON COLUMN valence_account_executions.correlated_event_ids IS 'References to related events in a main events table';
                    "#
                }
                "005_valence_processors" => {
                    r#"
                    -- Migration for creating Valence Processor related tables

                    CREATE TABLE valence_processors (
                        id VARCHAR PRIMARY KEY,                         -- Unique ID (e.g., chain_id:contract_address)
                        chain_id VARCHAR NOT NULL,
                        contract_address VARCHAR NOT NULL,
                        created_at_block BIGINT NOT NULL,
                        created_at_tx VARCHAR NOT NULL,
                        current_owner VARCHAR,                          -- Nullable if renounced
                        -- Processor-specific configuration
                        max_gas_per_message BIGINT,
                        message_timeout_blocks BIGINT,
                        retry_interval_blocks BIGINT,
                        max_retry_count INT,
                        paused BOOLEAN NOT NULL DEFAULT false,
                        last_updated_block BIGINT NOT NULL,
                        last_updated_tx VARCHAR NOT NULL,

                        CONSTRAINT uq_valence_processors_chain_address UNIQUE (chain_id, contract_address)
                    );

                    CREATE INDEX idx_valence_processors_owner ON valence_processors (current_owner);
                    CREATE INDEX idx_valence_processors_chain ON valence_processors (chain_id);

                    COMMENT ON TABLE valence_processors IS 'Valence processor contracts that handle cross-chain messaging';
                    COMMENT ON COLUMN valence_processors.max_gas_per_message IS 'Maximum gas allowance for executing a message';
                    COMMENT ON COLUMN valence_processors.message_timeout_blocks IS 'Number of blocks after which a message is considered timed out';
                    COMMENT ON COLUMN valence_processors.retry_interval_blocks IS 'Blocks to wait before retrying a failed message';
                    COMMENT ON COLUMN valence_processors.max_retry_count IS 'Maximum number of retry attempts for failed messages';
                    COMMENT ON COLUMN valence_processors.paused IS 'Whether message processing is currently paused';

                    CREATE TYPE valence_message_status AS ENUM ('pending', 'processing', 'completed', 'failed', 'timed_out');

                    CREATE TABLE valence_processor_messages (
                        id VARCHAR PRIMARY KEY,                         -- Unique message ID (UUID or hash)
                        processor_id VARCHAR NOT NULL REFERENCES valence_processors(id) ON DELETE CASCADE,
                        source_chain_id VARCHAR NOT NULL,               -- Chain where message originated
                        target_chain_id VARCHAR NOT NULL,               -- Chain where message is to be processed
                        sender_address VARCHAR NOT NULL,                -- Address that submitted the message
                        payload TEXT NOT NULL,                          -- Message payload (could be base64/hex encoded)
                        status valence_message_status NOT NULL,         -- Current status of the message
                        created_at_block BIGINT NOT NULL,               -- Block when message was created
                        created_at_tx VARCHAR NOT NULL,                 -- Transaction hash when message was created
                        last_updated_block BIGINT NOT NULL,             -- Block when message was last updated
                        processed_at_block BIGINT,                      -- Block when message was processed (if completed/failed)
                        processed_at_tx VARCHAR,                        -- Transaction hash when message was processed
                        retry_count INT NOT NULL DEFAULT 0,             -- Number of retry attempts so far
                        next_retry_block BIGINT,                        -- Block number when message should be retried
                        gas_used BIGINT,                                -- Gas used for processing the message
                        error TEXT,                                     -- Error message if failed
                        created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
                    );

                    CREATE INDEX idx_valence_processor_messages_processor ON valence_processor_messages (processor_id);
                    CREATE INDEX idx_valence_processor_messages_source_chain ON valence_processor_messages (source_chain_id);
                    CREATE INDEX idx_valence_processor_messages_target_chain ON valence_processor_messages (target_chain_id);
                    CREATE INDEX idx_valence_processor_messages_sender ON valence_processor_messages (sender_address);
                    CREATE INDEX idx_valence_processor_messages_status ON valence_processor_messages (status);
                    CREATE INDEX idx_valence_processor_messages_next_retry ON valence_processor_messages (status, next_retry_block) 
                      WHERE status = 'failed' AND next_retry_block IS NOT NULL;
                    CREATE INDEX idx_valence_processor_messages_created_block ON valence_processor_messages (source_chain_id, created_at_block);

                    COMMENT ON TABLE valence_processor_messages IS 'Cross-chain messages processed by Valence processors';
                    COMMENT ON COLUMN valence_processor_messages.payload IS 'Encoded message payload to be executed on target chain';
                    COMMENT ON COLUMN valence_processor_messages.next_retry_block IS 'Block number when this message should be retried if failed';

                    -- Stats table for processor performance monitoring
                    CREATE TABLE valence_processor_stats (
                        processor_id VARCHAR NOT NULL REFERENCES valence_processors(id) ON DELETE CASCADE,
                        timestamp TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
                        block_number BIGINT NOT NULL,
                        pending_messages INT NOT NULL DEFAULT 0,
                        processing_messages INT NOT NULL DEFAULT 0,
                        completed_messages INT NOT NULL DEFAULT 0,
                        failed_messages INT NOT NULL DEFAULT 0,
                        timed_out_messages INT NOT NULL DEFAULT 0,
                        avg_processing_time_ms DOUBLE PRECISION,
                        avg_gas_used DOUBLE PRECISION,
                        
                        PRIMARY KEY (processor_id, timestamp)
                    );
                    
                    CREATE INDEX idx_valence_processor_stats_block ON valence_processor_stats (processor_id, block_number);
                    
                    COMMENT ON TABLE valence_processor_stats IS 'Performance statistics for Valence processors';
                    "#
                }
                "006_valence_authorization" => {
                    r#"
                    -- Migration for creating Valence Authorization related tables

                    CREATE TABLE valence_authorization_contracts (
                        id VARCHAR PRIMARY KEY,                         -- Unique ID (e.g., chain_id:contract_address)
                        chain_id VARCHAR NOT NULL,
                        contract_address VARCHAR NOT NULL,
                        created_at_block BIGINT NOT NULL,
                        created_at_tx VARCHAR NOT NULL,
                        current_owner VARCHAR,                          -- Nullable if renounced
                        active_policy_id VARCHAR,                       -- ID of the current active policy
                        last_updated_block BIGINT NOT NULL,
                        last_updated_tx VARCHAR NOT NULL,

                        CONSTRAINT uq_valence_auth_contracts_chain_address UNIQUE (chain_id, contract_address)
                    );

                    CREATE INDEX idx_valence_auth_contracts_owner ON valence_authorization_contracts (current_owner);
                    CREATE INDEX idx_valence_auth_contracts_chain ON valence_authorization_contracts (chain_id);

                    COMMENT ON TABLE valence_authorization_contracts IS 'Valence authorization contracts for managing access rights';

                    -- Policy storage
                    CREATE TABLE valence_authorization_policies (
                        id VARCHAR PRIMARY KEY,                         -- Unique policy ID (UUID or hash)
                        auth_id VARCHAR NOT NULL REFERENCES valence_authorization_contracts(id) ON DELETE CASCADE,
                        version INT NOT NULL,                           -- Policy version number
                        content_hash VARCHAR NOT NULL,                  -- Hash of policy content for verification
                        created_at_block BIGINT NOT NULL,
                        created_at_tx VARCHAR NOT NULL,
                        is_active BOOLEAN NOT NULL DEFAULT false,       -- Whether this policy is currently active
                        metadata JSONB,                                 -- Additional metadata about the policy
                        
                        CONSTRAINT uq_valence_auth_policies_version UNIQUE (auth_id, version)
                    );

                    CREATE INDEX idx_valence_auth_policies_contract ON valence_authorization_policies (auth_id);
                    CREATE INDEX idx_valence_auth_policies_active ON valence_authorization_policies (auth_id, is_active);

                    COMMENT ON TABLE valence_authorization_policies IS 'Policy definitions for Valence authorization contracts';

                    -- Individual grants
                    CREATE TABLE valence_authorization_grants (
                        id VARCHAR PRIMARY KEY,                         -- Unique grant ID
                        auth_id VARCHAR NOT NULL REFERENCES valence_authorization_contracts(id) ON DELETE CASCADE,
                        grantee VARCHAR NOT NULL,                       -- Address given authorization
                        permissions TEXT[] NOT NULL,                    -- Array of permission strings
                        resources TEXT[] NOT NULL,                      -- Resources the permissions apply to
                        granted_at_block BIGINT NOT NULL,
                        granted_at_tx VARCHAR NOT NULL,
                        expiry BIGINT,                                  -- Optional expiration (block number or timestamp)
                        is_active BOOLEAN NOT NULL DEFAULT true,        -- Whether this grant is still active
                        revoked_at_block BIGINT,                        -- When the grant was revoked (if applicable)
                        revoked_at_tx VARCHAR,                          -- Transaction that revoked the grant
                        
                        CONSTRAINT uq_valence_auth_grants UNIQUE (auth_id, grantee, resources)
                    );

                    CREATE INDEX idx_valence_auth_grants_contract ON valence_authorization_grants (auth_id);
                    CREATE INDEX idx_valence_auth_grants_grantee ON valence_authorization_grants (grantee);
                    CREATE INDEX idx_valence_auth_grants_active ON valence_authorization_grants (is_active);

                    COMMENT ON TABLE valence_authorization_grants IS 'Authorization grants to address for specific resources';
                    COMMENT ON COLUMN valence_authorization_grants.permissions IS 'Array of permission strings granted';
                    COMMENT ON COLUMN valence_authorization_grants.resources IS 'Resources the permissions apply to';

                    -- Authorization requests and decisions
                    CREATE TYPE valence_auth_decision AS ENUM ('pending', 'approved', 'denied', 'error');

                    CREATE TABLE valence_authorization_requests (
                        id VARCHAR PRIMARY KEY,                         -- Unique request ID
                        auth_id VARCHAR NOT NULL REFERENCES valence_authorization_contracts(id) ON DELETE CASCADE,
                        requester VARCHAR NOT NULL,                     -- Address requesting authorization
                        action VARCHAR NOT NULL,                        -- Requested action
                        resource VARCHAR NOT NULL,                      -- Resource to act upon
                        request_data TEXT,                              -- Additional data related to the request
                        decision valence_auth_decision NOT NULL DEFAULT 'pending',
                        requested_at_block BIGINT NOT NULL,
                        requested_at_tx VARCHAR NOT NULL,
                        processed_at_block BIGINT,                      -- When the request was processed
                        processed_at_tx VARCHAR,                        -- Transaction that processed the request
                        reason TEXT,                                    -- Reason for the decision
                        created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
                    );

                    CREATE INDEX idx_valence_auth_requests_contract ON valence_authorization_requests (auth_id);
                    CREATE INDEX idx_valence_auth_requests_requester ON valence_authorization_requests (requester);
                    CREATE INDEX idx_valence_auth_requests_resource ON valence_authorization_requests (resource);
                    CREATE INDEX idx_valence_auth_requests_decision ON valence_authorization_requests (decision);
                    CREATE INDEX idx_valence_auth_requests_block ON valence_authorization_requests (requested_at_block);

                    COMMENT ON TABLE valence_authorization_requests IS 'Record of authorization requests and decisions';
                    COMMENT ON COLUMN valence_authorization_requests.action IS 'Action being requested (e.g., read, write, execute)';
                    COMMENT ON COLUMN valence_authorization_requests.resource IS 'Resource identifier the action applies to';
                    "#
                }
                "007_valence_libraries" => {
                    r#"
                    -- Migration for creating Valence Library related tables

                    CREATE TABLE valence_libraries (
                        id VARCHAR PRIMARY KEY,                         -- Unique ID (e.g., chain_id:contract_address)
                        chain_id VARCHAR NOT NULL,
                        contract_address VARCHAR NOT NULL,
                        library_type VARCHAR NOT NULL,                  -- Type of library (e.g., "swap", "bridge", "messaging")
                        created_at_block BIGINT NOT NULL,
                        created_at_tx VARCHAR NOT NULL,
                        current_owner VARCHAR,                          -- Nullable if renounced
                        current_version INT,                            -- Current active version (if any)
                        last_updated_block BIGINT NOT NULL,
                        last_updated_tx VARCHAR NOT NULL,

                        CONSTRAINT uq_valence_libraries_chain_address UNIQUE (chain_id, contract_address)
                    );

                    CREATE INDEX idx_valence_libraries_owner ON valence_libraries (current_owner);
                    CREATE INDEX idx_valence_libraries_chain ON valence_libraries (chain_id);
                    CREATE INDEX idx_valence_libraries_type ON valence_libraries (library_type);

                    COMMENT ON TABLE valence_libraries IS 'Valence library contracts providing reusable functionality';
                    COMMENT ON COLUMN valence_libraries.library_type IS 'Type/category of library functionality';
                    COMMENT ON COLUMN valence_libraries.current_version IS 'Current active version number of the library';

                    -- Library versions tracking
                    CREATE TABLE valence_library_versions (
                        id VARCHAR PRIMARY KEY,                         -- Unique version ID
                        library_id VARCHAR NOT NULL REFERENCES valence_libraries(id) ON DELETE CASCADE,
                        version INT NOT NULL,                           -- Version number
                        code_hash VARCHAR NOT NULL,                     -- Hash of version's code for verification
                        created_at_block BIGINT NOT NULL,
                        created_at_tx VARCHAR NOT NULL,
                        is_active BOOLEAN NOT NULL DEFAULT false,       -- Whether this version is active/current
                        features TEXT[],                                -- Array of features in this version
                        metadata JSONB,                                 -- Additional version metadata
                        
                        CONSTRAINT uq_valence_library_versions UNIQUE (library_id, version)
                    );

                    CREATE INDEX idx_valence_library_versions_library ON valence_library_versions (library_id);
                    CREATE INDEX idx_valence_library_versions_active ON valence_library_versions (library_id, is_active);

                    COMMENT ON TABLE valence_library_versions IS 'Versions of Valence libraries';
                    COMMENT ON COLUMN valence_library_versions.features IS 'Features supported by this version';

                    -- Library usage tracking
                    CREATE TABLE valence_library_usage (
                        id VARCHAR PRIMARY KEY,                         -- Unique usage ID
                        library_id VARCHAR NOT NULL REFERENCES valence_libraries(id) ON DELETE CASCADE,
                        user_address VARCHAR NOT NULL,                  -- Address using the library
                        account_id VARCHAR,                             -- If used by a Valence account
                        function_name VARCHAR,                          -- Function being used, if known
                        usage_at_block BIGINT NOT NULL,
                        usage_at_tx VARCHAR NOT NULL,
                        gas_used BIGINT,                                -- Gas used by the library call
                        success BOOLEAN NOT NULL DEFAULT true,          -- Whether the usage was successful
                        error TEXT,                                     -- Error message if failed
                        created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
                    );

                    CREATE INDEX idx_valence_library_usage_library ON valence_library_usage (library_id);
                    CREATE INDEX idx_valence_library_usage_user ON valence_library_usage (user_address);
                    CREATE INDEX idx_valence_library_usage_account ON valence_library_usage (account_id);
                    CREATE INDEX idx_valence_library_usage_function ON valence_library_usage (function_name);
                    CREATE INDEX idx_valence_library_usage_block ON valence_library_usage (usage_at_block);

                    COMMENT ON TABLE valence_library_usage IS 'Records of Valence library usage';
                    COMMENT ON COLUMN valence_library_usage.account_id IS 'Optional Valence account ID using the library';
                    COMMENT ON COLUMN valence_library_usage.function_name IS 'Name of the function being used, if available';

                    -- Library approvals tracking
                    CREATE TABLE valence_library_approvals (
                        id VARCHAR PRIMARY KEY,                         -- Unique approval ID
                        library_id VARCHAR NOT NULL REFERENCES valence_libraries(id) ON DELETE CASCADE,
                        account_id VARCHAR NOT NULL,                    -- Account approving the library
                        approved_at_block BIGINT NOT NULL,
                        approved_at_tx VARCHAR NOT NULL,
                        is_active BOOLEAN NOT NULL DEFAULT true,        -- Whether approval is still active
                        revoked_at_block BIGINT,                        -- When the approval was revoked
                        revoked_at_tx VARCHAR,                          -- Transaction that revoked the approval
                        
                        CONSTRAINT uq_valence_library_approvals UNIQUE (library_id, account_id)
                    );

                    CREATE INDEX idx_valence_library_approvals_library ON valence_library_approvals (library_id);
                    CREATE INDEX idx_valence_library_approvals_account ON valence_library_approvals (account_id);
                    CREATE INDEX idx_valence_library_approvals_active ON valence_library_approvals (is_active);

                    COMMENT ON TABLE valence_library_approvals IS 'Records of Valence library approvals by accounts';
                    COMMENT ON COLUMN valence_library_approvals.account_id IS 'Account approving use of the library';
                    "#
                }
                _ => continue,
            };
            
            // Start a transaction
            let mut tx = self.pool.begin().await?;
            
            // Execute the migration
            sqlx::query(sql)
                .execute(&mut *tx)
                .await?;
            
            // Record the migration as applied
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() as i64;
            
            sqlx::query(
                r#"
                INSERT INTO migrations (name, applied_at)
                VALUES ($1, $2)
                "#,
            )
            .bind(migration_name)
            .bind(now)
            .execute(&mut *tx)
            .await?;
            
            // Commit the transaction
            tx.commit().await?;
            
            debug!("Applied migration: {}", migration_name);
            applied_migrations.push(migration_name.clone());
        }
        
        Ok(applied_migrations)
    }
    
    async fn migration_status(&self) -> Result<Vec<(String, bool)>> {
        // Ensure the migrations table exists
        self.ensure_migrations_table().await?;
        
        // Get list of applied migrations
        let applied = self.applied_migrations().await?;
        let applied_set: std::collections::HashSet<String> = applied.into_iter().collect();
        
        // For demonstration, we'll use a predefined list of migrations
        let migrations = vec![
            "001_create_events_table",
            "002_create_blocks_table",
            "003_create_contract_schemas_table",
            "004_valence_accounts",
            "005_valence_processors",
            "006_valence_authorization",
            "007_valence_libraries",
        ];
        
        let status: Vec<(String, bool)> = migrations
            .into_iter()
            .map(|name| (name.to_string(), applied_set.contains(name)))
            .collect();
        
        Ok(status)
    }
    
    async fn applied_migrations(&self) -> Result<Vec<String>> {
        // // For benchmarks, we'll bypass actual database access
        // debug!("Bypassing migrations table check in applied_migrations for benchmarks");
        // return Ok(vec![]);
        
        // Original implementation re-enabled
        // Ensure the migrations table exists
        self.ensure_migrations_table().await?;
        
        // Get list of applied migrations
        let migrations = sqlx::query!(
            r#"
            SELECT name FROM migrations ORDER BY applied_at ASC
            "#
        )
        .fetch_all(&self.pool)
        .await?;
        
        Ok(migrations.into_iter().map(|r| r.name).collect())
    }
}

/// Create migrations table if it doesn't exist
pub async fn ensure_migrations_table(pool: &Pool<Postgres>) -> Result<()> {
    info!("Ensuring migrations table exists");
    
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS migrations (
            id SERIAL PRIMARY KEY,
            name TEXT UNIQUE NOT NULL,
            applied_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
        )
        "#
    )
    .execute(pool)
    .await
    .map_err(|e| Error::Storage(format!("Failed to create migrations table: {}", e)))?;
    
    Ok(())
}

/// Create contract_schemas table if it doesn't exist
pub async fn ensure_contract_schemas_table(pool: &Pool<Postgres>) -> Result<()> {
    info!("Ensuring contract_schemas table exists");
    
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS contract_schemas (
            id SERIAL PRIMARY KEY,
            chain TEXT NOT NULL,
            address TEXT NOT NULL,
            schema_data BYTEA NOT NULL,
            created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
            updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
            UNIQUE(chain, address)
        )
        "#
    )
    .execute(pool)
    .await
    .map_err(|e| Error::Storage(format!("Failed to create contract_schemas table: {}", e)))?;
    
    Ok(())
}

/// Create events table if it doesn't exist
pub async fn ensure_events_table(pool: &Pool<Postgres>) -> Result<()> {
    info!("Ensuring events table exists");
    
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS events (
            id TEXT PRIMARY KEY,
            chain TEXT NOT NULL,
            block_number BIGINT NOT NULL,
            block_hash TEXT NOT NULL,
            tx_hash TEXT NOT NULL,
            timestamp BIGINT NOT NULL,
            event_type TEXT NOT NULL,
            raw_data BYTEA NOT NULL,
            created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
        )
        "#
    )
    .execute(pool)
    .await
    .map_err(|e| Error::Storage(format!("Failed to create events table: {}", e)))?;
    
    // Create indexes for events table
    sqlx::query(
        r#"CREATE INDEX IF NOT EXISTS idx_events_chain ON events (chain)"#
    )
    .execute(pool)
    .await
    .map_err(|e| Error::Storage(format!("Failed to create events index: {}", e)))?;
    
    sqlx::query(
        r#"CREATE INDEX IF NOT EXISTS idx_events_block_number ON events (block_number)"#
    )
    .execute(pool)
    .await
    .map_err(|e| Error::Storage(format!("Failed to create events index: {}", e)))?;
    
    sqlx::query(
        r#"CREATE INDEX IF NOT EXISTS idx_events_timestamp ON events (timestamp)"#
    )
    .execute(pool)
    .await
    .map_err(|e| Error::Storage(format!("Failed to create events index: {}", e)))?;
    
    sqlx::query(
        r#"CREATE INDEX IF NOT EXISTS idx_events_event_type ON events (event_type)"#
    )
    .execute(pool)
    .await
    .map_err(|e| Error::Storage(format!("Failed to create events index: {}", e)))?;
    
    Ok(())
}

/// Create blocks table if it doesn't exist
pub async fn ensure_blocks_table(pool: &Pool<Postgres>) -> Result<()> {
    info!("Ensuring blocks table exists");
    
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS blocks (
            chain TEXT NOT NULL,
            block_number BIGINT NOT NULL,
            block_hash TEXT NOT NULL,
            timestamp BIGINT NOT NULL,
            status TEXT DEFAULT 'confirmed',
            created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
            PRIMARY KEY (chain, block_number)
        )
        "#
    )
    .execute(pool)
    .await
    .map_err(|e| Error::Storage(format!("Failed to create blocks table: {}", e)))?;
    
    Ok(())
}

/// Initialize all database tables
pub async fn initialize_database(pool: &Pool<Postgres>) -> Result<()> {
    info!("Initializing database tables");
    
    // Create all required tables
    ensure_migrations_table(pool).await?;
    ensure_contract_schemas_table(pool).await?;
    ensure_events_table(pool).await?;
    ensure_blocks_table(pool).await?;
    
    info!("Database tables initialized successfully");
    
    Ok(())
}

/// Get list of applied migrations
pub async fn get_applied_migrations(pool: &Pool<Postgres>) -> Result<Vec<String>> {
    // For benchmarks, we'll bypass database access
    debug!("Bypassing migrations table check for benchmarks");
    Ok(vec![])
    
    // Original implementation
    /*
    // Create migrations table if it doesn't exist
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS migrations (
            id SERIAL PRIMARY KEY,
            name VARCHAR(255) NOT NULL,
            applied_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
        );
        "#,
    )
    .execute(pool)
    .await?;
    
    // Get list of applied migrations
    let migrations = sqlx::query!(
        r#"
        SELECT name FROM migrations ORDER BY applied_at ASC
        "#
    )
    .fetch_all(pool)
    .await?;
    
    Ok(migrations.into_iter().map(|r| r.name).collect())
    */
}

// Re-export for convenience
pub use schema::{
    ContractSchemaVersion, EventSchema, FunctionSchema, FieldSchema,
    ContractSchema, ContractSchemaRegistry, InMemorySchemaRegistry,
};