// postgres_opt.rs - PostgreSQL optimization utilities
//
// Purpose: Provides tools for optimizing PostgreSQL performance through query analysis,
// index recommendations, and connection pool tuning

use std::collections::HashMap;
use std::time::Duration;
use sqlx::{Pool, Postgres, postgres::PgPoolOptions};
use super::{Measurement};
use indexer_core::Error;

/// Index type for PostgreSQL
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IndexType {
    /// B-tree index (default)
    BTree,
    
    /// Hash index
    Hash,
    
    /// GIN index (for jsonb, arrays, etc.)
    Gin,
    
    /// BRIN index (Block Range INdex - for large tables with natural ordering)
    Brin,
}

impl std::fmt::Display for IndexType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IndexType::BTree => write!(f, "BTREE"),
            IndexType::Hash => write!(f, "HASH"),
            IndexType::Gin => write!(f, "GIN"),
            IndexType::Brin => write!(f, "BRIN"),
        }
    }
}

/// Index definition
#[derive(Debug, Clone)]
pub struct IndexDefinition {
    /// Table name
    pub table: String,
    
    /// Columns to index
    pub columns: Vec<String>,
    
    /// Index type
    pub index_type: IndexType,
    
    /// Whether the index is unique
    pub unique: bool,
    
    /// Whether to create index concurrently
    pub concurrently: bool,
    
    /// Index name (optional - will be generated if not provided)
    pub name: Option<String>,
    
    /// Additional options (e.g., "fillfactor=70")
    pub options: Vec<String>,
}

impl IndexDefinition {
    /// Create a new index definition
    pub fn new(table: &str, columns: Vec<String>, index_type: IndexType) -> Self {
        Self {
            table: table.to_string(),
            columns,
            index_type,
            unique: false,
            concurrently: true,
            name: None,
            options: Vec::new(),
        }
    }
    
    /// Make the index unique
    pub fn unique(mut self) -> Self {
        self.unique = true;
        self
    }
    
    /// Add an option
    pub fn with_option(mut self, option: &str) -> Self {
        self.options.push(option.to_string());
        self
    }
    
    /// Generate the SQL to create the index
    pub fn to_sql(&self) -> String {
        let mut sql = String::new();
        
        sql.push_str("CREATE ");
        
        if self.unique {
            sql.push_str("UNIQUE ");
        }
        
        sql.push_str("INDEX ");
        
        if self.concurrently {
            sql.push_str("CONCURRENTLY ");
        }
        
        if let Some(name) = &self.name {
            sql.push_str(&format!("{} ", name));
        } else {
            let columns_str = self.columns.join("_");
            sql.push_str(&format!("idx_{}_{} ", self.table, columns_str));
        }
        
        sql.push_str(&format!("ON {} USING {} (", self.table, self.index_type));
        
        sql.push_str(&self.columns.join(", "));
        
        sql.push(')');
        
        if !self.options.is_empty() {
            sql.push_str(" WITH (");
            sql.push_str(&self.options.join(", "));
            sql.push(')');
        }
        
        sql
    }
}

/// Configuration for PostgreSQL connection pool
#[derive(Debug, Clone)]
pub struct ConnectionPoolConfig {
    /// Minimum connections to keep in the pool
    pub min_connections: u32,
    
    /// Maximum connections to allow in the pool
    pub max_connections: u32,
    
    /// Maximum lifetime of a connection
    pub max_lifetime: Duration,
    
    /// Idle timeout for connections
    pub idle_timeout: Duration,
    
    /// Connection timeout
    pub connect_timeout: Duration,
    
    /// Statement timeout
    pub statement_timeout: Duration,
}

impl Default for ConnectionPoolConfig {
    fn default() -> Self {
        Self {
            min_connections: 5,
            max_connections: 20,
            max_lifetime: Duration::from_secs(30 * 60), // 30 minutes
            idle_timeout: Duration::from_secs(10 * 60), // 10 minutes
            connect_timeout: Duration::from_secs(3),    // 3 seconds
            statement_timeout: Duration::from_secs(30), // 30 seconds
        }
    }
}

impl ConnectionPoolConfig {
    /// Create a new connection pool with this configuration
    pub async fn create_pool(&self, database_url: &str) -> Result<Pool<Postgres>, Error> {
        let statement_timeout = self.statement_timeout;
        
        let pool = PgPoolOptions::new()
            .min_connections(self.min_connections)
            .max_connections(self.max_connections)
            .max_lifetime(self.max_lifetime)
            .idle_timeout(self.idle_timeout)
            .acquire_timeout(self.connect_timeout)
            .after_connect(move |conn, _meta| {
                let timeout_ms = statement_timeout.as_millis() as i64;
                Box::pin(async move {
                    // Set statement timeout
                    sqlx::query(&format!("SET statement_timeout = {}", timeout_ms))
                        .execute(conn)
                        .await?;
                    
                    // Set other session parameters if needed
                    Ok(())
                })
            })
            .connect(database_url)
            .await
            .map_err(|e| Error::generic(format!("Failed to create connection pool: {}", e)))?;
        
        Ok(pool)
    }
}

/// Information about a table
#[derive(Debug, Clone)]
pub struct TableInfo {
    /// Table name
    pub name: String,
    
    /// Approximate row count
    pub row_count: i64,
    
    /// Size in bytes
    pub size_bytes: i64,
    
    /// Existing indexes
    pub indexes: Vec<String>,
}

/// Query information for analysis
#[derive(Debug, Clone)]
pub struct QueryInfo {
    /// SQL query
    pub sql: String,
    
    /// Execution time in milliseconds
    pub execution_time_ms: f64,
    
    /// Query plan
    pub plan: Option<String>,
    
    /// Tables accessed
    pub tables: Vec<String>,
    
    /// Indexes used
    pub indexes: Vec<String>,
}

/// Query performance statistics
#[derive(Debug, Clone)]
pub struct QueryStats {
    /// Query information
    pub query: QueryInfo,
    
    /// Number of rows returned
    pub rows_returned: i64,
    
    /// Total planning time in milliseconds
    pub planning_time_ms: f64,
    
    /// Total execution time in milliseconds
    pub execution_time_ms: f64,
}

/// Get information about tables in the database
pub async fn get_table_info(pool: &Pool<Postgres>) -> Result<Vec<TableInfo>, Error> {
    #[derive(sqlx::FromRow)]
    struct TableRow {
        table_name: String,
        row_count: i64,
        size_bytes: i64,
    }
    
    // Query tables with their sizes and row counts
    let tables = sqlx::query_as::<_, TableRow>(
        r#"
        SELECT
            t.relname AS table_name,
            c.reltuples::bigint AS row_count,
            pg_total_relation_size(c.oid) AS size_bytes
        FROM
            pg_class c
        JOIN
            pg_namespace n ON n.oid = c.relnamespace
        JOIN
            information_schema.tables t ON t.table_schema = n.nspname AND t.table_name = c.relname
        WHERE
            n.nspname NOT IN ('pg_catalog', 'information_schema')
            AND c.relkind = 'r'
            AND t.table_type = 'BASE TABLE'
        ORDER BY
            pg_total_relation_size(c.oid) DESC
        "#
    )
    .fetch_all(pool)
    .await
    .map_err(|e| Error::generic(format!("Failed to get table info: {}", e)))?;
    
    // For each table, get its indexes
    let mut table_infos = Vec::with_capacity(tables.len());
    
    for table_row in tables {
        // Get indexes for this table
        let indexes: Vec<String> = sqlx::query_scalar::<_, String>(
            r#"
            SELECT
                indexname
            FROM
                pg_indexes
            WHERE
                tablename = $1
            ORDER BY
                indexname
            "#
        )
        .bind(&table_row.table_name)
        .fetch_all(pool)
        .await
        .map_err(|e| Error::generic(format!("Failed to get index info: {}", e)))?;
        
        table_infos.push(TableInfo {
            name: table_row.table_name,
            row_count: table_row.row_count,
            size_bytes: table_row.size_bytes,
            indexes,
        });
    }
    
    Ok(table_infos)
}

/// Analyze a query and return its execution plan
pub async fn analyze_query(pool: &Pool<Postgres>, sql: &str) -> Result<QueryInfo, Error> {
    // Extract query plan
    let plan: String = sqlx::query_scalar::<_, String>(
        &format!("EXPLAIN (FORMAT JSON, ANALYZE, TIMING, BUFFERS) {}", sql)
    )
    .fetch_one(pool)
    .await
    .map_err(|e| Error::generic(format!("Failed to analyze query: {}", e)))?;
    
    // Execute query to get execution time
    let start = std::time::Instant::now();
    let _ = sqlx::query(sql)
        .execute(pool)
        .await
        .map_err(|e| Error::generic(format!("Failed to execute query: {}", e)))?;
    let execution_time_ms = start.elapsed().as_secs_f64() * 1000.0;
    
    // Parse plan to extract tables and indexes
    let plan_json: serde_json::Value = serde_json::from_str(&plan)
        .map_err(|e| Error::generic(format!("Failed to parse query plan: {}", e)))?;
    
    let mut tables = Vec::new();
    let mut indexes = Vec::new();
    
    // Extract tables and indexes from the plan
    if let Some(plans) = plan_json.as_array() {
        if let Some(first_plan) = plans.first() {
            if let Some(plan_obj) = first_plan.get("Plan") {
                // Recursively extract tables and indexes
                extract_tables_and_indexes(plan_obj, &mut tables, &mut indexes);
            }
        }
    }
    
    Ok(QueryInfo {
        sql: sql.to_string(),
        execution_time_ms,
        plan: Some(plan),
        tables,
        indexes,
    })
}

/// Recursively extract tables and indexes from a query plan
fn extract_tables_and_indexes(
    plan: &serde_json::Value,
    tables: &mut Vec<String>,
    indexes: &mut Vec<String>,
) {
    if let Some(node_type) = plan.get("Node Type").and_then(|v| v.as_str()) {
        // Extract table
        if let Some(relation_name) = plan.get("Relation Name").and_then(|v| v.as_str()) {
            if !tables.contains(&relation_name.to_string()) {
                tables.push(relation_name.to_string());
            }
        }
        
        // Extract index
        if node_type.contains("Index") {
            if let Some(index_name) = plan.get("Index Name").and_then(|v| v.as_str()) {
                if !indexes.contains(&index_name.to_string()) {
                    indexes.push(index_name.to_string());
                }
            }
        }
    }
    
    // Recurse into plans
    if let Some(plans) = plan.get("Plans").and_then(|v| v.as_array()) {
        for sub_plan in plans {
            extract_tables_and_indexes(sub_plan, tables, indexes);
        }
    }
}

/// Run a query and collect statistics
pub async fn benchmark_query(pool: &Pool<Postgres>, sql: &str, iterations: usize) -> Result<Measurement, Error> {
    let mut total_duration = Duration::from_secs(0);
    let mut total_rows = 0;
    
    for _ in 0..iterations {
        let start = std::time::Instant::now();
        let result = sqlx::query(sql)
            .execute(pool)
            .await
            .map_err(|e| Error::generic(format!("Failed to execute query: {}", e)))?;
        
        let duration = start.elapsed();
        total_duration += duration;
        total_rows += result.rows_affected() as u64;
    }
    
    // Create measurement
    let avg_duration = total_duration / iterations as u32;
    let measurement = Measurement::new(
        &format!("query_benchmark_{}", sql.chars().take(20).collect::<String>()),
        avg_duration,
        total_rows,
        0, // We don't know the byte size
    );
    
    Ok(measurement)
}

/// Suggest indexes based on query analysis
pub fn suggest_indexes(query_infos: &[QueryInfo], table_infos: &[TableInfo]) -> Vec<IndexDefinition> {
    let mut suggested_indexes = Vec::new();
    
    // Find tables used across multiple queries
    let mut table_usage = HashMap::new();
    for query in query_infos {
        for table in &query.tables {
            *table_usage.entry(table.clone()).or_insert(0) += 1;
        }
    }
    
    // Look for tables that are frequently accessed but don't have indexes used
    for (table, usage_count) in table_usage {
        if usage_count > 1 {
            let table_info = table_infos.iter().find(|t| t.name == table);
            
            if let Some(table_info) = table_info {
                // Skip tables with small row counts
                if table_info.row_count < 1000 {
                    continue;
                }
                
                // Check if any query is using this table without using an index
                for query in query_infos {
                    if query.tables.contains(&table) && !query.indexes.iter().any(|idx| idx.contains(&table)) {
                        // This query is using the table but not an index
                        // Suggest an index based on the query
                        if let Some(index) = suggest_index_for_query(&table, query) {
                            // Check if this index is already suggested
                            if !suggested_indexes.iter().any(|idx: &IndexDefinition| 
                                idx.table == index.table && idx.columns == index.columns
                            ) {
                                suggested_indexes.push(index);
                            }
                        }
                    }
                }
            }
        }
    }
    
    suggested_indexes
}

/// Suggest an index for a query on a specific table
fn suggest_index_for_query(table: &str, query: &QueryInfo) -> Option<IndexDefinition> {
    // Extract column names from the query for this table
    // This is a simplified approach and might not work for complex queries
    let columns = extract_columns_from_query(table, &query.sql);
    
    if columns.is_empty() {
        return None;
    }
    
    // Create index definition
    let index_type = if query.sql.to_lowercase().contains("like") {
        IndexType::Gin
    } else {
        IndexType::BTree
    };
    
    Some(IndexDefinition::new(table, columns, index_type))
}

/// Extract column names for a table from a query
/// This is a simplified implementation that may not work for complex queries
fn extract_columns_from_query(table: &str, sql: &str) -> Vec<String> {
    let mut columns = Vec::new();
    
    // Look for WHERE conditions
    if let Some(where_pos) = sql.to_lowercase().find("where") {
        let where_clause = &sql[where_pos + 5..];
        
        // Look for table.column or just column references
        let table_prefix = format!("{}.", table);
        
        for part in where_clause.split_whitespace() {
            if part.starts_with(&table_prefix) {
                // Extract column name from table.column
                let column = part[table_prefix.len()..].trim_end_matches([',', ')', '(', ';']);
                if !column.is_empty() && !columns.contains(&column.to_string()) {
                    columns.push(column.to_string());
                }
            } else if let Some(pos) = part.find('=') {
                // Simple column = value pattern
                let column = part[0..pos].trim_end_matches([' ', '\t']);
                
                // Check if this column belongs to the table by looking for references
                // This is a simplified approach and might not always work
                if !column.contains('.') && sql.contains(&format!("{}.{}", table, column)) && !columns.contains(&column.to_string()) {
                    columns.push(column.to_string());
                }
            }
        }
    }
    
    // Also look for JOIN conditions
    if let Some(join_pos) = sql.to_lowercase().find("join") {
        let join_clause = &sql[join_pos..];
        
        // Look for ON conditions
        if let Some(on_pos) = join_clause.to_lowercase().find(" on ") {
            let on_clause = &join_clause[on_pos + 4..];
            
            // Extract column names from JOIN ... ON table.column = other_table.column
            let table_prefix = format!("{}.", table);
            
            for part in on_clause.split_whitespace() {
                if part.starts_with(&table_prefix) {
                    let column = part[table_prefix.len()..].trim_end_matches([',', ')', '(', ';', '=']);
                    if !column.is_empty() && !columns.contains(&column.to_string()) {
                        columns.push(column.to_string());
                    }
                }
            }
        }
    }
    
    columns
}

/// Optimize a connection pool based on load testing
pub async fn optimize_connection_pool(
    database_url: &str,
    query: &str,
    max_concurrency: usize,
) -> Result<ConnectionPoolConfig, Error> {
    let mut optimal_config = ConnectionPoolConfig::default();
    let mut best_throughput = 0.0;
    
    // Test different connection pool sizes
    for &max_connections in &[5, 10, 20, 50, 100] {
        // Skip if max_connections > max_concurrency
        if max_connections as usize > max_concurrency {
            continue;
        }
        
        let min_connections = max_connections / 5;
        
        let config = ConnectionPoolConfig {
            min_connections,
            max_connections,
            ..ConnectionPoolConfig::default()
        };
        
        // Create pool with this configuration
        let pool = config.create_pool(database_url).await?;
        
        // Run load test
        let throughput = run_load_test(&pool, query, max_concurrency, Duration::from_secs(10)).await?;
        
        // Check if this is better
        if throughput > best_throughput {
            best_throughput = throughput;
            optimal_config = config;
        }
        
        // Close pool
        pool.close().await;
    }
    
    Ok(optimal_config)
}

/// Run a load test on a database connection pool
async fn run_load_test(
    pool: &Pool<Postgres>,
    query: &str,
    concurrency: usize,
    duration: Duration,
) -> Result<f64, Error> {
    use tokio::sync::Semaphore;
    use tokio::time::Instant;
    
    // Create a semaphore to limit concurrency
    let semaphore = std::sync::Arc::new(Semaphore::new(concurrency));
    
    // Create a counter for completed queries
    let query_count = std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0));
    
    // Start time
    let start_time = Instant::now();
    
    // Create futures
    let mut handles = Vec::with_capacity(concurrency);
    
    for _ in 0..concurrency {
        let semaphore = semaphore.clone();
        let query_count = query_count.clone();
        let pool = pool.clone();
        let query = query.to_string();
        
        let handle = tokio::spawn(async move {
            let _permit = semaphore.acquire().await.unwrap();
            
            // Execute queries until time is up
            loop {
                let _ = sqlx::query(&query)
                    .execute(&pool)
                    .await
                    .map_err(|e| {
                        eprintln!("Query error: {}", e);
                        e
                    });
                
                // Increment counter
                query_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            }
        });
        
        handles.push(handle);
    }
    
    // Wait for the test duration
    tokio::time::sleep(duration).await;
    
    // Calculate throughput
    let elapsed = start_time.elapsed();
    let query_count = query_count.load(std::sync::atomic::Ordering::Relaxed);
    let throughput = query_count as f64 / elapsed.as_secs_f64();
    
    // Abort all handles
    for handle in handles {
        handle.abort();
    }
    
    Ok(throughput)
}

/// Run a comprehensive PostgreSQL optimization
pub async fn optimize_postgres_database(
    database_url: &str,
    sample_queries: &[String],
) -> Result<(Vec<IndexDefinition>, ConnectionPoolConfig), Error> {
    // Create a default connection pool for analysis
    let config = ConnectionPoolConfig::default();
    let pool = config.create_pool(database_url).await?;
    
    // Get table information
    let table_infos = get_table_info(&pool).await?;
    
    // Analyze sample queries
    let mut query_infos = Vec::new();
    
    for sql in sample_queries {
        let query_info = analyze_query(&pool, sql).await?;
        query_infos.push(query_info);
    }
    
    // Suggest indexes
    let suggested_indexes = suggest_indexes(&query_infos, &table_infos);
    
    // Optimize connection pool if we have at least one query
    let optimal_pool_config = if !sample_queries.is_empty() {
        optimize_connection_pool(database_url, &sample_queries[0], 20).await?
    } else {
        config
    };
    
    // Close pool
    pool.close().await;
    
    Ok((suggested_indexes, optimal_pool_config))
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_index_definition() {
        let index = IndexDefinition::new("users", vec!["email".to_string()], IndexType::BTree)
            .unique()
            .with_option("fillfactor=70");
        
        let sql = index.to_sql();
        assert!(sql.contains("CREATE UNIQUE INDEX"));
        assert!(sql.contains("ON users USING BTREE"));
        assert!(sql.contains("(email)"));
        assert!(sql.contains("WITH (fillfactor=70)"));
    }
    
    #[test]
    fn test_extract_columns_from_query() {
        let query = "SELECT * FROM users WHERE users.email = 'test@example.com' AND users.active = true";
        let columns = extract_columns_from_query("users", query);
        
        assert!(columns.contains(&"email".to_string()));
        assert!(columns.contains(&"active".to_string()));
    }
    
    #[tokio::test]
    #[ignore] // This test requires a real database
    async fn test_analyze_query() {
        // This test would require a real database connection
        // For demonstration purposes only
        /*
        let pool = PgPoolOptions::new()
            .connect("postgres://postgres:postgres@localhost/test_db")
            .await
            .unwrap();
        
        let query_info = analyze_query(&pool, "SELECT * FROM users WHERE email = 'test@example.com'")
            .await
            .unwrap();
        
        assert!(query_info.tables.contains(&"users".to_string()));
        */
    }
} 