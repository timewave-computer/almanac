use async_trait::async_trait;
use sqlx::{Pool, Postgres};
use tracing::{instrument, debug};

use indexer_core::Result;

/// Repository for managing contract schemas
#[async_trait]
pub trait ContractSchemaRepository: Send + Sync + 'static {
    /// Store a contract schema
    async fn store_schema(&self, chain: &str, address: &str, schema_data: &[u8]) -> Result<()>;
    
    /// Get a contract schema
    async fn get_schema(&self, chain: &str, address: &str) -> Result<Option<Vec<u8>>>;
}

/// PostgreSQL implementation of contract schema repository
pub struct PostgresContractSchemaRepository {
    /// Database connection pool
    pool: Pool<Postgres>,
}

impl PostgresContractSchemaRepository {
    /// Create a new PostgreSQL contract schema repository
    pub fn new(pool: Pool<Postgres>) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl ContractSchemaRepository for PostgresContractSchemaRepository {
    #[instrument(skip(self, schema_data), fields(chain = %chain, address = %address))]
    async fn store_schema(&self, chain: &str, address: &str, schema_data: &[u8]) -> Result<()> {
        // Use basic SQLx query to store schema data as BYTEA
        sqlx::query(
            r#"
            INSERT INTO contract_schemas (chain, address, schema_data)
            VALUES ($1, $2, $3)
            ON CONFLICT (chain, address) 
            DO UPDATE SET 
                schema_data = EXCLUDED.schema_data,
                updated_at = NOW()
            "#
        )
        .bind(chain)
        .bind(address)
        .bind(schema_data)
        .execute(&self.pool)
        .await?;
        
        Ok(())
    }
    
    #[instrument(skip(self), fields(chain = %chain, address = %address))]
    async fn get_schema(&self, chain: &str, address: &str) -> Result<Option<Vec<u8>>> {
        use sqlx::Row;
        
        // Use manual row mapping to get schema data as BYTEA
        let row = sqlx::query(
            r#"
            SELECT schema_data
            FROM contract_schemas
            WHERE chain = $1 AND address = $2
            "#
        )
        .bind(chain)
        .bind(address)
        .fetch_optional(&self.pool)
        .await?;
        
        Ok(row.map(|r| r.get("schema_data")))
    }
} 