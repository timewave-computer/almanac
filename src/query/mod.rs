// mod.rs - Query module for the indexer
//
// Purpose: Provides a unified interface for querying indexed data with
// advanced capabilities for filtering, sorting, and aggregation

use indexer_core::{Error, ChainId};
use serde::{Serialize, Deserialize};
use std::collections::HashMap;

// Export submodules
pub mod historical;
pub mod aggregation;
pub mod performance;

/// Filter for queries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryFilter {
    /// Filter by chain IDs
    pub chain_ids: Option<Vec<ChainId>>,
    
    /// Filter by contract addresses
    pub contract_addresses: Option<Vec<String>>,
    
    /// Filter by contract types
    pub contract_types: Option<Vec<String>>,
    
    /// Filter by entity IDs
    pub entity_ids: Option<Vec<String>>,
    
    /// Filter by attributes (key-value pairs)
    pub attributes: Option<HashMap<String, String>>,
    
    /// Filter by tags
    pub tags: Option<Vec<String>>,
}

/// Pagination parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pagination {
    /// Maximum number of results to return
    pub limit: usize,
    
    /// Offset for pagination
    pub offset: usize,
}

impl Default for Pagination {
    fn default() -> Self {
        Self {
            limit: 100,
            offset: 0,
        }
    }
}

/// Sort direction
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum SortDirection {
    /// Ascending order (A to Z, 0 to 9)
    Ascending,
    
    /// Descending order (Z to A, 9 to 0)
    Descending,
}

impl Default for SortDirection {
    fn default() -> Self {
        Self::Ascending
    }
}

/// Sorting parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Sorting {
    /// Field to sort by
    pub field: String,
    
    /// Sort direction
    pub direction: SortDirection,
}

/// Trait for queryable structures
pub trait Queryable {
    /// Add a filter to the query
    fn with_filter(&mut self, filter: QueryFilter) -> &mut Self;
    
    /// Add pagination to the query
    fn with_pagination(&mut self, pagination: Pagination) -> &mut Self;
    
    /// Add sorting to the query
    fn with_sorting(&mut self, sorting: Sorting) -> &mut Self;
}

/// Base query with common parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaseQuery {
    /// Query filter
    pub filter: Option<QueryFilter>,
    
    /// Pagination
    pub pagination: Option<Pagination>,
    
    /// Sorting
    pub sorting: Option<Sorting>,
}

impl Queryable for BaseQuery {
    fn with_filter(&mut self, filter: QueryFilter) -> &mut Self {
        self.filter = Some(filter);
        self
    }
    
    fn with_pagination(&mut self, pagination: Pagination) -> &mut Self {
        self.pagination = Some(pagination);
        self
    }
    
    fn with_sorting(&mut self, sorting: Sorting) -> &mut Self {
        self.sorting = Some(sorting);
        self
    }
}

/// Trait for executing queries
pub trait QueryExecutor<T, Q> {
    /// Execute a query and return results
    fn execute(&self, query: Q) -> Result<Vec<T>, Error>;
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_query_filter() {
        // Create a filter
        let filter = QueryFilter {
            chain_ids: Some(vec![ChainId::from("ethereum")]),
            contract_addresses: Some(vec!["0x123".to_string()]),
            contract_types: Some(vec!["ERC20".to_string()]),
            entity_ids: Some(vec!["token1".to_string()]),
            attributes: {
                let mut map = HashMap::new();
                map.insert("symbol".to_string(), "ETH".to_string());
                Some(map)
            },
            tags: Some(vec!["stablecoin".to_string()]),
        };
        
        // Verify the filter is constructed correctly
        assert_eq!(filter.chain_ids.unwrap()[0], ChainId::from("ethereum"));
        assert_eq!(filter.contract_addresses.unwrap()[0], "0x123");
        assert_eq!(filter.contract_types.unwrap()[0], "ERC20");
        assert_eq!(filter.entity_ids.unwrap()[0], "token1");
        assert_eq!(filter.attributes.unwrap()["symbol"], "ETH");
        assert_eq!(filter.tags.unwrap()[0], "stablecoin");
    }
    
    #[test]
    fn test_base_query() {
        // Create a filter
        let filter = QueryFilter {
            chain_ids: Some(vec![ChainId::from("ethereum")]),
            contract_addresses: None,
            contract_types: None,
            entity_ids: None,
            attributes: None,
            tags: None,
        };
        
        // Create pagination
        let pagination = Pagination {
            limit: 10,
            offset: 5,
        };
        
        // Create sorting
        let sorting = Sorting {
            field: "timestamp".to_string(),
            direction: SortDirection::Descending,
        };
        
        // Create a base query
        let mut query = BaseQuery {
            filter: None,
            pagination: None,
            sorting: None,
        };
        
        // Apply filters
        query.with_filter(filter)
            .with_pagination(pagination)
            .with_sorting(sorting);
        
        // Verify the query is constructed correctly
        assert_eq!(query.filter.unwrap().chain_ids.unwrap()[0], ChainId::from("ethereum"));
        assert_eq!(query.pagination.unwrap().limit, 10);
        assert_eq!(query.pagination.unwrap().offset, 5);
        assert_eq!(query.sorting.unwrap().field, "timestamp");
        assert_eq!(query.sorting.unwrap().direction, SortDirection::Descending);
    }
} 