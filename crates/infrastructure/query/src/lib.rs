//! Query planning and execution infrastructure for distributed Almanac deployments

pub mod planner;
pub mod executor;

use serde::{Deserialize, Serialize};
use almanac_infrastructure_core::types::ChainId;

// Re-export commonly used types
pub use planner::{
    DistributedQueryPlanner, QueryPlan, DistributedQuery, SubQuery,
    QueryResult, QueryExecutor, QueryRoutingService
};
pub use executor::{StreamingQueryExecutor, ParallelQueryOrchestrator};

/// Base query structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaseQuery {
    pub filter: Option<QueryFilter>,
    pub pagination: Option<Pagination>,
    pub sorting: Option<Sorting>,
}

/// Query filter
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryFilter {
    pub chain_ids: Option<Vec<ChainId>>,
    pub contract_addresses: Option<Vec<String>>,
    pub contract_types: Option<Vec<String>>,
    pub entity_ids: Option<Vec<String>>,
    pub attributes: Option<Vec<String>>,
    pub tags: Option<Vec<String>>,
}

/// Pagination parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pagination {
    pub limit: usize,
    pub offset: usize,
}

/// Sorting parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Sorting {
    pub field: String,
    pub direction: SortDirection,
}

/// Sort direction
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum SortDirection {
    Ascending,
    Descending,
}