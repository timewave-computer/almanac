/// Database indexing strategies for optimizing common query patterns
use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use async_trait::async_trait;

use crate::Result;

/// Index types supported by the indexing system
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum IndexType {
    /// B-tree index for range queries and ordering
    BTree,
    
    /// Hash index for exact match queries
    Hash,
    
    /// Full-text search index
    FullText,
    
    /// Composite index on multiple columns
    Composite { fields: Vec<String> },
    
    /// Partial index with WHERE clause
    Partial { condition: String },
    
    /// Unique constraint index
    Unique,
    
    /// Covering index that includes additional columns
    Covering { included_fields: Vec<String> },
}

/// Database index definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexDefinition {
    /// Index name
    pub name: String,
    
    /// Table or collection name
    pub table: String,
    
    /// Columns/fields to index
    pub fields: Vec<String>,
    
    /// Type of index
    pub index_type: IndexType,
    
    /// Whether this is a unique index
    pub unique: bool,
    
    /// Optional WHERE clause for partial indexes
    pub where_clause: Option<String>,
    
    /// Storage parameters
    pub storage_params: HashMap<String, String>,
}

/// Query pattern types for index optimization
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum QueryPattern {
    /// Exact match on single field
    ExactMatch { field: String },
    
    /// Range query on single field
    Range { field: String },
    
    /// Multiple exact matches (IN clause)
    InClause { field: String },
    
    /// Text search query
    TextSearch { fields: Vec<String> },
    
    /// Composite query on multiple fields
    Composite { fields: Vec<String> },
    
    /// Sorting/ordering query
    OrderBy { fields: Vec<String> },
    
    /// Aggregation query with grouping
    Aggregation { group_by: Vec<String>, aggregates: Vec<String> },
    
    /// Time-based range query
    TimeRange { time_field: String },
    
    /// Join query between tables
    Join { tables: Vec<String>, join_fields: Vec<String> },
}

/// Common query patterns for blockchain events
#[derive(Debug, Clone)]
pub struct EventQueryPatterns;

impl EventQueryPatterns {
    /// Get standard indexing strategy for blockchain events
    pub fn get_standard_indexes() -> Vec<IndexDefinition> {
        vec![
            // Primary key index (usually auto-created)
            IndexDefinition {
                name: "idx_events_pk".to_string(),
                table: "events".to_string(),
                fields: vec!["id".to_string()],
                index_type: IndexType::BTree,
                unique: true,
                where_clause: None,
                storage_params: HashMap::new(),
            },
            
            // Chain and block number composite index for block queries
            IndexDefinition {
                name: "idx_events_chain_block".to_string(),
                table: "events".to_string(),
                fields: vec!["chain_id".to_string(), "block_number".to_string()],
                index_type: IndexType::BTree,
                unique: false,
                where_clause: None,
                storage_params: HashMap::new(),
            },
            
            // Timestamp index for time-based queries
            IndexDefinition {
                name: "idx_events_timestamp".to_string(),
                table: "events".to_string(),
                fields: vec!["timestamp".to_string()],
                index_type: IndexType::BTree,
                unique: false,
                where_clause: None,
                storage_params: HashMap::new(),
            },
            
            // Transaction hash index for transaction lookups
            IndexDefinition {
                name: "idx_events_tx_hash".to_string(),
                table: "events".to_string(),
                fields: vec!["tx_hash".to_string()],
                index_type: IndexType::Hash,
                unique: false,
                where_clause: None,
                storage_params: HashMap::new(),
            },
            
            // Event type index for filtering by event type
            IndexDefinition {
                name: "idx_events_type".to_string(),
                table: "events".to_string(),
                fields: vec!["event_type".to_string()],
                index_type: IndexType::Hash,
                unique: false,
                where_clause: None,
                storage_params: HashMap::new(),
            },
            
            // Block hash index for block-based queries
            IndexDefinition {
                name: "idx_events_block_hash".to_string(),
                table: "events".to_string(),
                fields: vec!["block_hash".to_string()],
                index_type: IndexType::Hash,
                unique: false,
                where_clause: None,
                storage_params: HashMap::new(),
            },
            
            // Composite index for chain + event type queries
            IndexDefinition {
                name: "idx_events_chain_type".to_string(),
                table: "events".to_string(),
                fields: vec!["chain_id".to_string(), "event_type".to_string()],
                index_type: IndexType::Composite {
                    fields: vec!["chain_id".to_string(), "event_type".to_string()]
                },
                unique: false,
                where_clause: None,
                storage_params: HashMap::new(),
            },
            
            // Time-based partial index for recent events
            IndexDefinition {
                name: "idx_events_recent".to_string(),
                table: "events".to_string(),
                fields: vec!["timestamp".to_string(), "chain_id".to_string()],
                index_type: IndexType::Partial {
                    condition: "timestamp > NOW() - INTERVAL '30 days'".to_string()
                },
                unique: false,
                where_clause: Some("timestamp > NOW() - INTERVAL '30 days'".to_string()),
                storage_params: HashMap::new(),
            },
            
            // Full-text search index for event data
            IndexDefinition {
                name: "idx_events_fulltext".to_string(),
                table: "events".to_string(),
                fields: vec!["raw_data".to_string()],
                index_type: IndexType::FullText,
                unique: false,
                where_clause: None,
                storage_params: {
                    let mut params = HashMap::new();
                    params.insert("config".to_string(), "english".to_string());
                    params
                },
            },
            
            // Covering index for common event queries
            IndexDefinition {
                name: "idx_events_covering".to_string(),
                table: "events".to_string(),
                fields: vec!["chain_id".to_string(), "timestamp".to_string()],
                index_type: IndexType::Covering {
                    included_fields: vec![
                        "event_type".to_string(),
                        "block_number".to_string(),
                        "tx_hash".to_string()
                    ]
                },
                unique: false,
                where_clause: None,
                storage_params: HashMap::new(),
            },
        ]
    }
    
    /// Get indexing strategy for Valence account data
    pub fn get_valence_account_indexes() -> Vec<IndexDefinition> {
        vec![
            // Primary key for account states
            IndexDefinition {
                name: "idx_valence_accounts_pk".to_string(),
                table: "valence_account_states".to_string(),
                fields: vec!["account_id".to_string()],
                index_type: IndexType::BTree,
                unique: true,
                where_clause: None,
                storage_params: HashMap::new(),
            },
            
            // Chain ID index for chain-specific queries
            IndexDefinition {
                name: "idx_valence_accounts_chain".to_string(),
                table: "valence_account_states".to_string(),
                fields: vec!["chain_id".to_string()],
                index_type: IndexType::Hash,
                unique: false,
                where_clause: None,
                storage_params: HashMap::new(),
            },
            
            // Contract address index for address lookups
            IndexDefinition {
                name: "idx_valence_accounts_address".to_string(),
                table: "valence_account_states".to_string(),
                fields: vec!["address".to_string()],
                index_type: IndexType::Hash,
                unique: false,
                where_clause: None,
                storage_params: HashMap::new(),
            },
            
            // Owner index for ownership queries
            IndexDefinition {
                name: "idx_valence_accounts_owner".to_string(),
                table: "valence_account_states".to_string(),
                fields: vec!["current_owner".to_string()],
                index_type: IndexType::Hash,
                unique: false,
                where_clause: Some("current_owner IS NOT NULL".to_string()),
                storage_params: HashMap::new(),
            },
            
            // Historical states index
            IndexDefinition {
                name: "idx_valence_history".to_string(),
                table: "valence_account_history".to_string(),
                fields: vec!["account_id".to_string(), "block_number".to_string()],
                index_type: IndexType::BTree,
                unique: true,
                where_clause: None,
                storage_params: HashMap::new(),
            },
        ]
    }
}

/// Index management trait for different storage backends
#[async_trait]
pub trait IndexManager: Send + Sync {
    /// Create an index based on the definition
    async fn create_index(&self, index_def: &IndexDefinition) -> Result<()>;
    
    /// Drop an existing index
    async fn drop_index(&self, index_name: &str, table: &str) -> Result<()>;
    
    /// List all indexes for a table
    async fn list_indexes(&self, table: &str) -> Result<Vec<IndexDefinition>>;
    
    /// Analyze index usage and effectiveness
    async fn analyze_index_usage(&self, table: &str) -> Result<IndexUsageStats>;
    
    /// Suggest indexes based on query patterns
    async fn suggest_indexes(&self, query_patterns: &[QueryPattern]) -> Result<Vec<IndexDefinition>>;
    
    /// Rebuild an index
    async fn rebuild_index(&self, index_name: &str, table: &str) -> Result<()>;
    
    /// Get index size and statistics
    async fn get_index_stats(&self, index_name: &str, table: &str) -> Result<IndexStats>;
}

/// Index usage statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexUsageStats {
    /// Index name
    pub index_name: String,
    
    /// Number of times index was used
    pub usage_count: u64,
    
    /// Number of scans performed
    pub scan_count: u64,
    
    /// Number of tuples read
    pub tuples_read: u64,
    
    /// Number of tuples fetched
    pub tuples_fetched: u64,
    
    /// Index effectiveness ratio (0.0 to 1.0)
    pub effectiveness: f64,
    
    /// Last used timestamp
    pub last_used: Option<std::time::SystemTime>,
}

/// Index statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexStats {
    /// Index name
    pub index_name: String,
    
    /// Table name
    pub table_name: String,
    
    /// Index size in bytes
    pub size_bytes: u64,
    
    /// Number of pages/blocks
    pub page_count: u64,
    
    /// Number of tuples/rows
    pub tuple_count: u64,
    
    /// Index depth (for B-tree indexes)
    pub depth: Option<u32>,
    
    /// Fragmentation percentage
    pub fragmentation: Option<f64>,
    
    /// Last maintenance timestamp
    pub last_maintenance: Option<std::time::SystemTime>,
}

/// Query optimizer that uses index information
pub struct QueryOptimizer {
    /// Available indexes
    indexes: HashMap<String, Vec<IndexDefinition>>,
}

impl QueryOptimizer {
    /// Create a new query optimizer
    pub fn new() -> Self {
        Self {
            indexes: HashMap::new(),
        }
    }
    
    /// Register indexes for a table
    pub fn register_indexes(&mut self, table: &str, indexes: Vec<IndexDefinition>) {
        self.indexes.insert(table.to_string(), indexes);
    }
    
    /// Suggest the best index for a query pattern
    pub fn suggest_index_for_pattern(&self, table: &str, pattern: &QueryPattern) -> Option<&IndexDefinition> {
        let table_indexes = self.indexes.get(table)?;
        
        match pattern {
            QueryPattern::ExactMatch { field } => {
                // Prefer hash indexes for exact matches
                table_indexes.iter()
                    .find(|idx| {
                        idx.fields.len() == 1 && 
                        idx.fields[0] == *field &&
                        matches!(idx.index_type, IndexType::Hash)
                    })
                    .or_else(|| {
                        // Fallback to any index containing the field
                        table_indexes.iter()
                            .find(|idx| idx.fields.contains(field))
                    })
            }
            
            QueryPattern::Range { field } => {
                // Prefer B-tree indexes for range queries
                table_indexes.iter()
                    .find(|idx| {
                        idx.fields.len() == 1 && 
                        idx.fields[0] == *field &&
                        matches!(idx.index_type, IndexType::BTree)
                    })
            }
            
            QueryPattern::TextSearch { fields: _ } => {
                // Look for full-text search indexes
                table_indexes.iter()
                    .find(|idx| matches!(idx.index_type, IndexType::FullText))
            }
            
            QueryPattern::Composite { fields } => {
                // Look for composite indexes matching the field set
                table_indexes.iter()
                    .find(|idx| {
                        matches!(idx.index_type, IndexType::Composite { .. }) &&
                        idx.fields.len() >= fields.len() &&
                        fields.iter().all(|f| idx.fields.contains(f))
                    })
            }
            
            QueryPattern::OrderBy { fields } => {
                // Prefer B-tree indexes for ordering
                table_indexes.iter()
                    .find(|idx| {
                        matches!(idx.index_type, IndexType::BTree) &&
                        !idx.fields.is_empty() &&
                        fields.iter().take(idx.fields.len()).eq(idx.fields.iter())
                    })
            }
            
            QueryPattern::TimeRange { time_field } => {
                // Look for time-based indexes
                table_indexes.iter()
                    .find(|idx| {
                        idx.fields.contains(time_field) &&
                        matches!(idx.index_type, IndexType::BTree | IndexType::Partial { .. })
                    })
            }
            
            _ => {
                // Generic fallback - find any relevant index
                table_indexes.iter()
                    .find(|idx| !idx.fields.is_empty())
            }
        }
    }
    
    /// Estimate query cost with and without indexes
    pub fn estimate_query_cost(&self, table: &str, pattern: &QueryPattern, table_size: u64) -> QueryCostEstimate {
        let with_index = self.suggest_index_for_pattern(table, pattern);
        
        let without_index_cost = table_size; // Full table scan
        let with_index_cost = if with_index.is_some() {
            // Rough estimate based on index type and selectivity
            match pattern {
                QueryPattern::ExactMatch { .. } => (table_size as f64 * 0.001).max(1.0) as u64,
                QueryPattern::Range { .. } => (table_size as f64 * 0.1).max(1.0) as u64,
                QueryPattern::TextSearch { .. } => (table_size as f64 * 0.05).max(1.0) as u64,
                _ => (table_size as f64 * 0.1).max(1.0) as u64,
            }
        } else {
            without_index_cost
        };
        
        QueryCostEstimate {
            without_index: without_index_cost,
            with_index: with_index_cost,
            improvement_ratio: without_index_cost as f64 / with_index_cost as f64,
            recommended_index: with_index.cloned(),
        }
    }
}

impl Default for QueryOptimizer {
    fn default() -> Self {
        Self::new()
    }
}

/// Query cost estimation result
#[derive(Debug, Clone)]
pub struct QueryCostEstimate {
    /// Estimated cost without index
    pub without_index: u64,
    
    /// Estimated cost with best available index
    pub with_index: u64,
    
    /// Performance improvement ratio
    pub improvement_ratio: f64,
    
    /// Recommended index for this query
    pub recommended_index: Option<IndexDefinition>,
}

/// Index maintenance scheduler
pub struct IndexMaintenanceScheduler {
    /// Maintenance intervals by index type
    maintenance_intervals: HashMap<IndexType, std::time::Duration>,
}

impl IndexMaintenanceScheduler {
    /// Create a new maintenance scheduler
    pub fn new() -> Self {
        let mut intervals = HashMap::new();
        intervals.insert(IndexType::BTree, std::time::Duration::from_secs(24 * 3600)); // Daily
        intervals.insert(IndexType::Hash, std::time::Duration::from_secs(7 * 24 * 3600)); // Weekly
        intervals.insert(IndexType::FullText, std::time::Duration::from_secs(3 * 24 * 3600)); // Every 3 days
        
        Self {
            maintenance_intervals: intervals,
        }
    }
    
    /// Check if an index needs maintenance
    pub fn needs_maintenance(&self, stats: &IndexStats) -> bool {
        if let Some(last_maintenance) = stats.last_maintenance {
            if let Some(interval) = self.get_maintenance_interval(&IndexType::BTree) {
                return last_maintenance.elapsed().unwrap_or_default() > interval;
            }
        }
        
        // Also check fragmentation
        if let Some(fragmentation) = stats.fragmentation {
            return fragmentation > 20.0; // 20% fragmentation threshold
        }
        
        false
    }
    
    /// Get maintenance interval for index type
    pub fn get_maintenance_interval(&self, index_type: &IndexType) -> Option<std::time::Duration> {
        // For composite types, use the base interval
        match index_type {
            IndexType::Composite { .. } => self.maintenance_intervals.get(&IndexType::BTree).copied(),
            IndexType::Partial { .. } => self.maintenance_intervals.get(&IndexType::BTree).copied(),
            IndexType::Covering { .. } => self.maintenance_intervals.get(&IndexType::BTree).copied(),
            other => self.maintenance_intervals.get(other).copied(),
        }
    }
    
    /// Schedule maintenance for all indexes
    pub async fn schedule_maintenance<T: IndexManager>(&self, manager: &T, tables: &[String]) -> Result<Vec<String>> {
        let mut maintenance_tasks = Vec::new();
        
        for table in tables {
            let indexes = manager.list_indexes(table).await?;
            
            for index in indexes {
                let stats = manager.get_index_stats(&index.name, table).await?;
                
                if self.needs_maintenance(&stats) {
                    maintenance_tasks.push(format!("{}:{}", table, index.name));
                }
            }
        }
        
        Ok(maintenance_tasks)
    }
}

impl Default for IndexMaintenanceScheduler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_standard_event_indexes() {
        let indexes = EventQueryPatterns::get_standard_indexes();
        
        assert!(!indexes.is_empty());
        assert!(indexes.iter().any(|idx| idx.name == "idx_events_pk"));
        assert!(indexes.iter().any(|idx| idx.name == "idx_events_chain_block"));
        assert!(indexes.iter().any(|idx| idx.name == "idx_events_timestamp"));
    }
    
    #[test]
    fn test_valence_account_indexes() {
        let indexes = EventQueryPatterns::get_valence_account_indexes();
        
        assert!(!indexes.is_empty());
        assert!(indexes.iter().any(|idx| idx.name == "idx_valence_accounts_pk"));
        assert!(indexes.iter().any(|idx| idx.name == "idx_valence_accounts_chain"));
    }
    
    #[test]
    fn test_query_optimizer() {
        let mut optimizer = QueryOptimizer::new();
        let indexes = EventQueryPatterns::get_standard_indexes();
        optimizer.register_indexes("events", indexes);
        
        // Test exact match pattern
        let pattern = QueryPattern::ExactMatch { field: "tx_hash".to_string() };
        let suggestion = optimizer.suggest_index_for_pattern("events", &pattern);
        assert!(suggestion.is_some());
        assert_eq!(suggestion.unwrap().name, "idx_events_tx_hash");
        
        // Test range pattern
        let pattern = QueryPattern::Range { field: "timestamp".to_string() };
        let suggestion = optimizer.suggest_index_for_pattern("events", &pattern);
        assert!(suggestion.is_some());
        assert_eq!(suggestion.unwrap().name, "idx_events_timestamp");
    }
    
    #[test]
    fn test_query_cost_estimation() {
        let mut optimizer = QueryOptimizer::new();
        let indexes = EventQueryPatterns::get_standard_indexes();
        optimizer.register_indexes("events", indexes);
        
        let pattern = QueryPattern::ExactMatch { field: "tx_hash".to_string() };
        let estimate = optimizer.estimate_query_cost("events", &pattern, 1_000_000);
        
        assert!(estimate.with_index < estimate.without_index);
        assert!(estimate.improvement_ratio > 1.0);
        assert!(estimate.recommended_index.is_some());
    }
    
    #[test]
    fn test_maintenance_scheduler() {
        let scheduler = IndexMaintenanceScheduler::new();
        
        let stats = IndexStats {
            index_name: "test_index".to_string(),
            table_name: "test_table".to_string(),
            size_bytes: 1024,
            page_count: 10,
            tuple_count: 100,
            depth: Some(3),
            fragmentation: Some(25.0), // High fragmentation
            last_maintenance: Some(std::time::SystemTime::now() - std::time::Duration::from_secs(48 * 3600)),
        };
        
        assert!(scheduler.needs_maintenance(&stats));
    }
    
    #[test]
    fn test_index_types() {
        let btree_index = IndexDefinition {
            name: "test_btree".to_string(),
            table: "test_table".to_string(),
            fields: vec!["timestamp".to_string()],
            index_type: IndexType::BTree,
            unique: false,
            where_clause: None,
            storage_params: HashMap::new(),
        };
        
        assert_eq!(btree_index.index_type, IndexType::BTree);
        assert!(!btree_index.unique);
        
        let composite_index = IndexDefinition {
            name: "test_composite".to_string(),
            table: "test_table".to_string(),
            fields: vec!["chain_id".to_string(), "block_number".to_string()],
            index_type: IndexType::Composite {
                fields: vec!["chain_id".to_string(), "block_number".to_string()]
            },
            unique: false,
            where_clause: None,
            storage_params: HashMap::new(),
        };
        
        if let IndexType::Composite { fields } = &composite_index.index_type {
            assert_eq!(fields.len(), 2);
            assert_eq!(fields[0], "chain_id");
            assert_eq!(fields[1], "block_number");
        } else {
            panic!("Expected composite index type");
        }
    }
} 