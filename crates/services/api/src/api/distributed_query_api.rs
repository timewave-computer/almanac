use async_trait::async_trait;
use axum::{
    extract::{State, Path},
    response::Json,
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{info, debug};

use almanac_infrastructure_core::{Error, Result};
use almanac_infrastructure_query::planner::{
    DistributedQuery, DistributedQueryPlanner, QueryExecutor, QueryResult, 
    SubQuery, QueryExecutionMetadata, QueryHints, DataLocality,
    BaseQuery, QueryFilter, Pagination, Sorting, SortDirection
};
use almanac_infrastructure_coordination::CoordinationService;
use almanac_infrastructure_core::types::ChainId;

/// API state for distributed query endpoints
pub struct DistributedQueryState {
    pub query_planner: Arc<DistributedQueryPlanner>,
    pub coordination: Arc<dyn CoordinationService>,
}

/// Query request structure
#[derive(Debug, Deserialize)]
pub struct QueryRequest {
    /// Filter criteria
    pub filter: Option<QueryFilterRequest>,
    
    /// Pagination parameters
    pub pagination: Option<PaginationRequest>,
    
    /// Sorting parameters
    pub sorting: Option<SortingRequest>,
    
    /// Query hints
    pub hints: Option<QueryHints>,
    
    /// Timeout in milliseconds
    pub timeout_ms: Option<u64>,
    
    /// Whether to use cache
    pub use_cache: Option<bool>,
}

/// Filter request
#[derive(Debug, Deserialize)]
pub struct QueryFilterRequest {
    pub chain_ids: Option<Vec<String>>,
    pub contract_addresses: Option<Vec<String>>,
    pub contract_types: Option<Vec<String>>,
    pub block_range: Option<(u64, u64)>,
    pub time_range: Option<(String, String)>,
}

/// Pagination request
#[derive(Debug, Deserialize)]
pub struct PaginationRequest {
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

/// Sorting request
#[derive(Debug, Deserialize)]
pub struct SortingRequest {
    pub field: String,
    pub ascending: Option<bool>,
}

/// Query response
#[derive(Debug, Serialize)]
pub struct QueryResponse<T> {
    /// Query results
    pub data: Vec<T>,
    
    /// Execution metadata
    pub metadata: QueryResponseMetadata,
}

/// Query response metadata
#[derive(Debug, Serialize)]
pub struct QueryResponseMetadata {
    /// Query plan ID
    pub plan_id: String,
    
    /// Total execution time
    pub execution_time_ms: u64,
    
    /// Number of nodes queried
    pub nodes_queried: usize,
    
    /// Cache hit information
    pub cache_hits: usize,
    
    /// Total records scanned
    pub total_records_scanned: u64,
    
    /// Any warnings
    pub warnings: Vec<String>,
}

/// Data locality update request
#[derive(Debug, Deserialize)]
pub struct DataLocalityUpdate {
    pub node_id: String,
    pub chains: Vec<String>,
    pub block_ranges: Option<serde_json::Value>,
    pub contracts: Option<Vec<String>>,
}

/// Create distributed query router
pub fn create_distributed_query_router(state: Arc<DistributedQueryState>) -> Router {
    Router::new()
        .route("/query/events", post(query_events))
        .route("/query/plan", post(create_query_plan))
        .route("/query/plan/:plan_id", get(get_query_plan))
        .route("/locality/:node_id", post(update_data_locality))
        .route("/nodes/active", get(get_active_nodes))
        .with_state(state)
}

/// Execute a distributed query for events
async fn query_events(
    State(state): State<Arc<DistributedQueryState>>,
    Json(request): Json<QueryRequest>,
) -> Result<Json<QueryResponse<serde_json::Value>>> {
    info!("Executing distributed query for events");
    
    // Convert request to distributed query
    let distributed_query = create_distributed_query(request)?;
    
    // Create query plan
    let plan = state.query_planner.create_plan(distributed_query.clone()).await?;
    let plan_id = plan.plan_id.clone();
    let nodes_queried = plan.sub_queries.len();
    
    // Execute the query plan
    let start_time = std::time::Instant::now();
    let results = state.query_planner.execute_plan::<serde_json::Value>(plan).await?;
    let execution_time_ms = start_time.elapsed().as_millis() as u64;
    
    // Build response
    let response = QueryResponse {
        data: results,
        metadata: QueryResponseMetadata {
            plan_id,
            execution_time_ms,
            nodes_queried,
            cache_hits: 0, // Would be tracked in real implementation
            total_records_scanned: 0, // Would be aggregated from sub-queries
            warnings: Vec::new(),
        },
    };
    
    Ok(Json(response))
}

/// Create a query plan without executing it
async fn create_query_plan(
    State(state): State<Arc<DistributedQueryState>>,
    Json(request): Json<QueryRequest>,
) -> Result<Json<serde_json::Value>> {
    info!("Creating distributed query plan");
    
    // Convert request to distributed query
    let distributed_query = create_distributed_query(request)?;
    
    // Create query plan
    let plan = state.query_planner.create_plan(distributed_query).await?;
    
    // Serialize plan to JSON
    let plan_json = serde_json::to_value(plan)
        .map_err(|e| Error::Serialization(e.to_string()))?;
    
    Ok(Json(plan_json))
}

/// Get a cached query plan
async fn get_query_plan(
    State(_state): State<Arc<DistributedQueryState>>,
    Path(plan_id): Path<String>,
) -> Result<Json<serde_json::Value>> {
    debug!("Getting query plan: {}", plan_id);
    
    // In a real implementation, this would retrieve the plan from cache
    Err(Error::NotFound(format!("Query plan not found: {}", plan_id)))
}

/// Update data locality for a node
async fn update_data_locality(
    State(state): State<Arc<DistributedQueryState>>,
    Path(node_id): Path<String>,
    Json(update): Json<DataLocalityUpdate>,
) -> Result<Json<serde_json::Value>> {
    info!("Updating data locality for node: {}", node_id);
    
    // Convert to DataLocality
    let locality = DataLocality {
        chains: update.chains.into_iter().map(ChainId::from).collect(),
        block_ranges: Default::default(), // Would parse from JSON in real implementation
        contracts: update.contracts,
        cached_queries: Vec::new(),
    };
    
    // Update locality
    state.query_planner.update_data_locality(node_id.clone(), locality).await;
    
    Ok(Json(serde_json::json!({
        "status": "success",
        "node_id": node_id
    })))
}

/// Get active nodes in the cluster
async fn get_active_nodes(
    State(state): State<Arc<DistributedQueryState>>,
) -> Result<Json<Vec<serde_json::Value>>> {
    info!("Getting active nodes");
    
    let nodes = state.coordination.get_active_nodes().await?;
    
    // Convert to JSON
    let nodes_json: Vec<serde_json::Value> = nodes.into_iter()
        .map(|node| serde_json::json!({
            "node_id": node.node_id,
            "address": node.address,
            "status": format!("{:?}", node.status),
            "capabilities": {
                "chains": node.capabilities.supported_chains,
                "max_throughput": node.capabilities.max_throughput,
                "storage_backends": node.capabilities.storage_backends,
                "api_endpoints": node.capabilities.api_endpoints,
            }
        }))
        .collect();
    
    Ok(Json(nodes_json))
}

/// Convert API request to distributed query
fn create_distributed_query(request: QueryRequest) -> Result<DistributedQuery> {
    // Build query filter
    let filter = request.filter.map(|f| QueryFilter {
        chain_ids: f.chain_ids.map(|ids| ids.into_iter().map(ChainId::from).collect()),
        contract_addresses: f.contract_addresses,
        contract_types: f.contract_types,
        entity_ids: None,
        attributes: None,
        tags: None,
    });
    
    // Build pagination
    let pagination = request.pagination.map(|p| Pagination {
        limit: p.limit.unwrap_or(100),
        offset: p.offset.unwrap_or(0),
    });
    
    // Build sorting
    let sorting = request.sorting.map(|s| Sorting {
        field: s.field,
        direction: if s.ascending.unwrap_or(true) {
            SortDirection::Ascending
        } else {
            SortDirection::Descending
        },
    });
    
    // Create distributed query
    Ok(DistributedQuery {
        base: BaseQuery {
            filter,
            pagination,
            sorting,
        },
        hints: request.hints.unwrap_or_default(),
        timeout_ms: request.timeout_ms.unwrap_or(30000), // 30 seconds default
        use_cache: request.use_cache.unwrap_or(true),
    })
}

/// Local query executor implementation
pub struct LocalQueryExecutor {
    // In a real implementation, this would have access to local storage
}

#[async_trait]
impl QueryExecutor for LocalQueryExecutor {
    async fn execute_query(&self, query: SubQuery) -> Result<QueryResult<serde_json::Value>> {
        // Simplified implementation - in reality this would query local storage
        // and convert results to JSON values
        
        let metadata = QueryExecutionMetadata {
            execution_time_ms: 100,
            records_scanned: 1000,
            cache_hit_rate: 0.0,
            warnings: vec![],
        };
        
        Ok(QueryResult {
            node_id: query.node_id.clone(),
            data: vec![], // Empty for now
            metadata,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_create_distributed_query() {
        let request = QueryRequest {
            filter: Some(QueryFilterRequest {
                chain_ids: Some(vec!["ethereum".to_string()]),
                contract_addresses: None,
                contract_types: None,
                block_range: Some((100, 200)),
                time_range: None,
            }),
            pagination: Some(PaginationRequest {
                limit: Some(50),
                offset: Some(0),
            }),
            sorting: Some(SortingRequest {
                field: "block_number".to_string(),
                ascending: Some(false),
            }),
            hints: None,
            timeout_ms: Some(5000),
            use_cache: Some(true),
        };
        
        let query = create_distributed_query(request).unwrap();
        
        assert_eq!(query.timeout_ms, 5000);
        assert!(query.use_cache);
        assert_eq!(query.base.filter.as_ref().unwrap().chain_ids.as_ref().unwrap()[0], ChainId::from("ethereum"));
        assert_eq!(query.base.pagination.as_ref().unwrap().limit, 50);
        assert_eq!(query.base.sorting.as_ref().unwrap().field, "block_number");
    }
}