// aggregation.rs - Aggregation query functionality for the indexer
//
// Purpose: Provides aggregation capabilities for data analysis including
// counting, grouping, and statistical operations.

use indexer_core::Error;
use indexer_storage::BoxedStorage;
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use crate::{QueryFilter, Pagination, Sorting, BaseQuery, Queryable, QueryExecutor};

/// Aggregation function types
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum AggregationFunction {
    /// Count number of items
    Count,
    
    /// Sum of values
    Sum,
    
    /// Average of values
    Average,
    
    /// Minimum value
    Min,
    
    /// Maximum value
    Max,
    
    /// Count distinct values
    CountDistinct,
}

/// Aggregation query definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregationQuery {
    /// Base query components
    #[serde(flatten)]
    pub base: BaseQuery,
    
    /// List of fields to group by
    pub group_by: Option<Vec<String>>,
    
    /// Map of fields to aggregate functions
    pub aggregations: HashMap<String, AggregationFunction>,
    
    /// Having clause for filtering aggregation results
    pub having: Option<AggregationFilter>,
}

/// Filter for aggregation results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregationFilter {
    /// Field to filter on (must be an aggregated field)
    pub field: String,
    
    /// Comparison operator
    pub operator: ComparisonOperator,
    
    /// Value to compare with
    pub value: serde_json::Value,
}

/// Comparison operators for filters
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ComparisonOperator {
    /// Equal to
    Eq,
    
    /// Not equal to
    Ne,
    
    /// Greater than
    Gt,
    
    /// Greater than or equal to
    Ge,
    
    /// Less than
    Lt,
    
    /// Less than or equal to
    Le,
}

impl Queryable for AggregationQuery {
    fn with_filter(&mut self, filter: QueryFilter) -> &mut Self {
        self.base.filter = Some(filter);
        self
    }
    
    fn with_pagination(&mut self, pagination: Pagination) -> &mut Self {
        self.base.pagination = Some(pagination);
        self
    }
    
    fn with_sorting(&mut self, sorting: Sorting) -> &mut Self {
        self.base.sorting = Some(sorting);
        self
    }
}

/// Result of an aggregation query
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregationResult {
    /// Group by values, if any
    pub groups: HashMap<String, serde_json::Value>,
    
    /// Aggregated values
    pub values: HashMap<String, serde_json::Value>,
}

/// Implementation of materialized view for aggregation
pub struct MaterializedView {
    /// Name of the materialized view
    pub name: String,
    
    /// SQL query used to create the view
    pub sql_query: String,
    
    /// Refresh interval in seconds (0 means manual refresh)
    pub refresh_interval: u64,
    
    /// Whether the view is enabled
    pub enabled: bool,
}

/// Manager for aggregation views
pub struct AggregationManager {
    #[allow(dead_code)]
    storage: BoxedStorage,
    materialized_views: HashMap<String, MaterializedView>,
}

impl AggregationManager {
    /// Create a new aggregation manager
    pub fn new(storage: BoxedStorage) -> Self {
        Self {
            storage,
            materialized_views: HashMap::new(),
        }
    }
    
    /// Register a materialized view
    pub fn register_view(&mut self, view: MaterializedView) -> Result<(), Error> {
        // Check if view already exists
        if self.materialized_views.contains_key(&view.name) {
            return Err(Error::generic(format!("Materialized view '{}' already exists", view.name)));
        }
        
        // Store the view
        self.materialized_views.insert(view.name.clone(), view);
        
        Ok(())
    }
    
    /// Refresh a materialized view
    pub fn refresh_view(&self, name: &str) -> Result<(), Error> {
        // Check if view exists
        if !self.materialized_views.contains_key(name) {
            return Err(Error::generic(format!("Materialized view '{}' not found", name)));
        }
        
        // Note: We would need to implement this functionality in the storage trait
        // For now, just return success
        Ok(())
    }
    
    /// Execute an aggregation query using materialized views when possible
    pub fn execute_query(&self, query: &AggregationQuery) -> Result<Vec<AggregationResult>, Error> {
        // Check if we can use a materialized view
        if let Some(view_name) = self.find_matching_view(query) {
            // Query the view
            self.query_materialized_view(&view_name, query)
        } else {
            // Execute a dynamic query
            self.execute_dynamic_query(query)
        }
    }
    
    /// Find a materialized view that matches the query
    fn find_matching_view(&self, _query: &AggregationQuery) -> Option<String> {
        // This is a simple placeholder implementation
        // In a real system, we would analyze the query and find a matching view
        // based on the groups, aggregations, and filters
        
        None
    }
    
    /// Query a materialized view
    fn query_materialized_view(&self, _view_name: &str, _query: &AggregationQuery) -> Result<Vec<AggregationResult>, Error> {
        // This would query the materialized view with the given filters
        // For now, return an empty result
        Ok(Vec::new())
    }
    
    /// Execute a dynamic aggregation query
    fn execute_dynamic_query(&self, _query: &AggregationQuery) -> Result<Vec<AggregationResult>, Error> {
        // This would build and execute a dynamic aggregation query
        // For now, return an empty result
        Ok(Vec::new())
    }
}

impl QueryExecutor<AggregationResult, AggregationQuery> for AggregationManager {
    fn execute(&self, query: AggregationQuery) -> Result<Vec<AggregationResult>, Error> {
        self.execute_query(&query)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_create_aggregation_query() {
        // Create a filter with chain ID
        let filter = crate::QueryFilter {
            chain_ids: Some(vec![crate::ChainId::from("ethereum")]),
            contract_addresses: Some(vec!["0x123".to_string()]),
            contract_types: Some(vec!["ERC20".to_string()]),
            entity_ids: None,
            attributes: None,
            tags: None,
        };
        
        // Create pagination
        let pagination = crate::Pagination {
            limit: 10,
            offset: 0,
        };
        
        // Create a base query
        let base_query = crate::BaseQuery {
            filter: Some(filter),
            pagination: Some(pagination),
            sorting: None,
        };
        
        // Create an aggregation query
        let mut aggregations = HashMap::new();
        aggregations.insert("count".to_string(), AggregationFunction::Count);
        aggregations.insert("avg_price".to_string(), AggregationFunction::Average);
        
        let query = AggregationQuery {
            base: base_query,
            group_by: Some(vec!["token_symbol".to_string()]),
            aggregations,
            having: None,
        };
        
        // Verify the query is constructed correctly
        assert_eq!(query.base.filter.as_ref().unwrap().chain_ids.as_ref().unwrap()[0], crate::ChainId::from("ethereum"));
        assert_eq!(query.base.pagination.as_ref().unwrap().limit, 10);
        assert_eq!(query.group_by.as_ref().unwrap()[0], "token_symbol");
        assert_eq!(query.aggregations.len(), 2);
        assert_eq!(query.aggregations.get("count"), Some(&AggregationFunction::Count));
    }
} 