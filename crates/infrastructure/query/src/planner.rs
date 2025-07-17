use almanac_infrastructure_core::{Result, Error};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;
use tracing::{debug, info, warn};
use async_trait::async_trait;

// Define local types to avoid circular dependencies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeInfo {
    pub node_id: String,
    pub capabilities: NodeCapabilities,
    pub locality: DataLocality,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeCapabilities {
    pub can_query_chains: Vec<almanac_infrastructure_core::ChainId>,
    pub can_aggregate: bool,
    pub can_stream: bool,
}

// Define query types directly to avoid circular dependencies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryFilter {
    pub chain_ids: Option<Vec<almanac_infrastructure_core::ChainId>>,
    pub contract_addresses: Option<Vec<String>>,
    pub contract_types: Option<Vec<String>>,
    pub entity_ids: Option<Vec<String>>,
    pub attributes: Option<HashMap<String, String>>,
    pub tags: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pagination {
    pub limit: usize,
    pub offset: usize,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum SortDirection {
    Ascending,
    Descending,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Sorting {
    pub field: String,
    pub direction: SortDirection,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaseQuery {
    pub filter: Option<QueryFilter>,
    pub pagination: Option<Pagination>,
    pub sorting: Option<Sorting>,
}

/// Query plan that describes how a query should be executed across nodes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryPlan {
    /// Unique identifier for this query plan
    pub plan_id: String,
    
    /// Original query
    pub query: DistributedQuery,
    
    /// Sub-queries to be executed on different nodes
    pub sub_queries: Vec<SubQuery>,
    
    /// Aggregation strategy for combining results
    pub aggregation: AggregationStrategy,
    
    /// Query routing metadata
    pub routing_metadata: QueryRoutingMetadata,
    
    /// Cache key for this query plan
    pub cache_key: Option<String>,
}

/// Sub-query to be executed on a specific node
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubQuery {
    /// Node that should execute this sub-query
    pub node_id: String,
    
    /// Query filter specific to this node
    pub filter: QueryFilter,
    
    /// Pagination for this sub-query
    pub pagination: Option<Pagination>,
    
    /// Estimated cost for this sub-query
    pub estimated_cost: QueryCost,
    
    /// Data locality information
    pub locality: DataLocality,
}

/// Distributed query that can span multiple nodes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistributedQuery {
    /// Base query parameters
    pub base: BaseQuery,
    
    /// Execution hints
    pub hints: QueryHints,
    
    /// Query timeout in milliseconds
    pub timeout_ms: u64,
    
    /// Whether to use cached results
    pub use_cache: bool,
}

/// Query execution hints
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct QueryHints {
    /// Preferred nodes for execution
    pub preferred_nodes: Option<Vec<String>>,
    
    /// Minimum required capabilities
    pub required_capabilities: Option<NodeCapabilities>,
    
    /// Maximum parallel executions
    pub max_parallel: Option<usize>,
    
    /// Whether to allow partial results
    pub allow_partial_results: bool,
}

/// Aggregation strategy for combining results from multiple nodes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AggregationStrategy {
    /// Simply merge all results
    MergeAll,
    
    /// Merge with deduplication
    MergeDistinct { key_fields: Vec<String> },
    
    /// Apply sorting after merge
    MergeAndSort { sorting: Sorting },
    
    /// Custom aggregation function
    Custom { function: String, parameters: HashMap<String, serde_json::Value> },
}

/// Query routing metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryRoutingMetadata {
    /// Total estimated cost
    pub total_cost: QueryCost,
    
    /// Data distribution across nodes
    pub data_distribution: HashMap<String, DataDistribution>,
    
    /// Routing decision reason
    pub routing_reason: String,
}

/// Query cost estimation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryCost {
    /// Estimated CPU cost (0-100)
    pub cpu_cost: f64,
    
    /// Estimated I/O cost (0-100)
    pub io_cost: f64,
    
    /// Estimated network cost (0-100)
    pub network_cost: f64,
    
    /// Estimated time in milliseconds
    pub estimated_time_ms: u64,
}

/// Data locality information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataLocality {
    /// Chains available on this node
    pub chains: Vec<almanac_infrastructure_core::ChainId>,
    
    /// Block ranges available
    pub block_ranges: HashMap<almanac_infrastructure_core::ChainId, Vec<(u64, u64)>>,
    
    /// Contract addresses indexed
    pub contracts: Option<Vec<String>>,
    
    /// Cached query results available
    pub cached_queries: Vec<String>,
}

/// Data distribution information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataDistribution {
    /// Number of events
    pub event_count: u64,
    
    /// Size in bytes
    pub size_bytes: u64,
    
    /// Last update timestamp
    pub last_update: u64,
}

/// Query result from a node
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResult<T> {
    /// Node that produced this result
    pub node_id: String,
    
    /// Actual results
    pub data: Vec<T>,
    
    /// Execution metadata
    pub metadata: QueryExecutionMetadata,
}

/// Query execution metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryExecutionMetadata {
    /// Actual execution time
    pub execution_time_ms: u64,
    
    /// Number of records scanned
    pub records_scanned: u64,
    
    /// Cache hit rate
    pub cache_hit_rate: f64,
    
    /// Any warnings during execution
    pub warnings: Vec<String>,
}

/// Distributed query planner
pub struct DistributedQueryPlanner {
    node_capabilities: Arc<RwLock<HashMap<String, NodeCapabilities>>>,
    data_locality: Arc<RwLock<HashMap<String, DataLocality>>>,
    query_cache: Arc<RwLock<QueryCache>>,
}

impl DistributedQueryPlanner {
    pub fn new() -> Self {
        Self {
            node_capabilities: Arc::new(RwLock::new(HashMap::new())),
            data_locality: Arc::new(RwLock::new(HashMap::new())),
            query_cache: Arc::new(RwLock::new(QueryCache::new(1000))), // 1000 entries
        }
    }
    
    /// Create a query plan for a distributed query
    pub async fn create_plan(&self, query: DistributedQuery) -> Result<QueryPlan> {
        info!("Creating query plan for distributed query");
        
        // Check cache first if enabled
        if query.use_cache {
            let cache_key = self.compute_cache_key(&query);
            let cache = self.query_cache.read().await;
            if let Some(cached_plan) = cache.get(&cache_key) {
                debug!("Found cached query plan: {}", cache_key);
                return Ok(cached_plan.clone());
            }
        }
        
        // Get active nodes
        let nodes = self.get_active_nodes().await?;
        if nodes.is_empty() {
            return Err(Error::Generic("No active nodes available".to_string()));
        }
        
        // Update node capabilities cache
        let _ = self.update_node_capabilities(&nodes);
        
        // Analyze query to determine routing
        let routing_analysis = self.analyze_query_routing(&query, &nodes).await?;
        
        // Create sub-queries based on data locality
        let sub_queries = self.create_sub_queries(&query, &routing_analysis).await?;
        
        // Determine aggregation strategy
        let aggregation = self.determine_aggregation_strategy(&query);
        
        // Create query plan
        let plan = QueryPlan {
            plan_id: Uuid::new_v4().to_string(),
            query: query.clone(),
            sub_queries,
            aggregation,
            routing_metadata: routing_analysis.metadata,
            cache_key: if query.use_cache {
                Some(self.compute_cache_key(&query))
            } else {
                None
            },
        };
        
        // Cache the plan if caching is enabled
        if query.use_cache {
            let mut cache = self.query_cache.write().await;
            cache.put(plan.cache_key.as_ref().unwrap().clone(), plan.clone());
        }
        
        Ok(plan)
    }
    
    /// Execute a query plan and aggregate results
    pub async fn execute_plan<T>(&self, plan: QueryPlan) -> Result<Vec<T>>
    where
        T: serde::de::DeserializeOwned + serde::Serialize + Send + Sync + 'static,
    {
        info!("Executing query plan: {}", plan.plan_id);
        
        // Execute sub-queries in parallel
        let mut handles = Vec::new();
        for sub_query in plan.sub_queries {
            let handle = tokio::spawn(async move {
                // In a real implementation, this would send the query to the node
                // and wait for results. For now, we'll return a placeholder.
                Ok::<QueryResult<T>, Error>(QueryResult {
                    node_id: sub_query.node_id,
                    data: Vec::new(),
                    metadata: QueryExecutionMetadata {
                        execution_time_ms: 0,
                        records_scanned: 0,
                        cache_hit_rate: 0.0,
                        warnings: Vec::new(),
                    },
                })
            });
            handles.push(handle);
        }
        
        // Collect results
        let mut results = Vec::new();
        for handle in handles {
            match handle.await {
                Ok(Ok(result)) => results.push(result),
                Ok(Err(e)) => warn!("Sub-query failed: {}", e),
                Err(e) => warn!("Task join error: {}", e),
            }
        }
        
        // Aggregate results according to strategy
        self.aggregate_results(results, &plan.aggregation).await
    }
    
    /// Update the data locality information for a node
    pub async fn update_data_locality(&self, node_id: String, locality: DataLocality) {
        let mut cache = self.data_locality.write().await;
        cache.insert(node_id, locality);
    }
    
    /// Analyze query to determine optimal routing
    async fn analyze_query_routing(
        &self,
        query: &DistributedQuery,
        nodes: &[NodeInfo],
    ) -> Result<RoutingAnalysis> {
        let mut analysis = RoutingAnalysis {
            selected_nodes: Vec::new(),
            metadata: QueryRoutingMetadata {
                total_cost: QueryCost {
                    cpu_cost: 0.0,
                    io_cost: 0.0,
                    network_cost: 0.0,
                    estimated_time_ms: 0,
                },
                data_distribution: HashMap::new(),
                routing_reason: String::new(),
            },
        };
        
        // Get data locality information
        let locality_cache = self.data_locality.read().await;
        
        // Analyze which nodes have relevant data
        if let Some(filter) = &query.base.filter {
            // Check chain filters
            if let Some(chain_ids) = &filter.chain_ids {
                for node in nodes {
                    if let Some(locality) = locality_cache.get(&node.node_id) {
                        let has_chains = chain_ids.iter().any(|chain_id| {
                            locality.chains.contains(chain_id)
                        });
                        
                        if has_chains {
                            analysis.selected_nodes.push(node.clone());
                        }
                    }
                }
            } else {
                // No specific chain filter, consider all active nodes
                analysis.selected_nodes = nodes.to_vec();
            }
        } else {
            // No filter, use all nodes
            analysis.selected_nodes = nodes.to_vec();
        }
        
        // Apply hints
        if let Some(preferred_nodes) = &query.hints.preferred_nodes {
            analysis.selected_nodes.retain(|n| preferred_nodes.contains(&n.node_id));
        }
        
        // Calculate costs
        for node in &analysis.selected_nodes {
            let node_cost = self.estimate_node_cost(node, query).await;
            analysis.metadata.total_cost.cpu_cost += node_cost.cpu_cost;
            analysis.metadata.total_cost.io_cost += node_cost.io_cost;
            analysis.metadata.total_cost.network_cost += node_cost.network_cost;
            analysis.metadata.total_cost.estimated_time_ms = 
                analysis.metadata.total_cost.estimated_time_ms.max(node_cost.estimated_time_ms);
        }
        
        analysis.metadata.routing_reason = format!(
            "Selected {} nodes based on data locality and query filters",
            analysis.selected_nodes.len()
        );
        
        Ok(analysis)
    }
    
    /// Create sub-queries for each selected node
    async fn create_sub_queries(
        &self,
        query: &DistributedQuery,
        routing: &RoutingAnalysis,
    ) -> Result<Vec<SubQuery>> {
        let mut sub_queries = Vec::new();
        let locality_cache = self.data_locality.read().await;
        
        for node in &routing.selected_nodes {
            let locality = locality_cache.get(&node.node_id)
                .cloned()
                .unwrap_or_else(|| DataLocality {
                    chains: Vec::new(),
                    block_ranges: HashMap::new(),
                    contracts: None,
                    cached_queries: Vec::new(),
                });
            
            // Create node-specific filter
            let mut node_filter = query.base.filter.clone().unwrap_or_else(|| QueryFilter {
                chain_ids: None,
                contract_addresses: None,
                contract_types: None,
                entity_ids: None,
                attributes: None,
                tags: None,
            });
            
            // Restrict to chains available on this node
            if !locality.chains.is_empty() {
                if let Some(chain_ids) = &mut node_filter.chain_ids {
                    chain_ids.retain(|id| locality.chains.contains(id));
                } else {
                    node_filter.chain_ids = Some(locality.chains.clone());
                }
            }
            
            let sub_query = SubQuery {
                node_id: node.node_id.clone(),
                filter: node_filter,
                pagination: query.base.pagination.clone(),
                estimated_cost: self.estimate_node_cost(node, query).await,
                locality,
            };
            
            sub_queries.push(sub_query);
        }
        
        Ok(sub_queries)
    }
    
    /// Determine the best aggregation strategy
    fn determine_aggregation_strategy(&self, query: &DistributedQuery) -> AggregationStrategy {
        if let Some(sorting) = &query.base.sorting {
            AggregationStrategy::MergeAndSort {
                sorting: sorting.clone(),
            }
        } else {
            // Default to merge with deduplication
            AggregationStrategy::MergeDistinct {
                key_fields: vec!["id".to_string()],
            }
        }
    }
    
    /// Aggregate results from multiple nodes
    async fn aggregate_results<T>(
        &self,
        results: Vec<QueryResult<T>>,
        strategy: &AggregationStrategy,
    ) -> Result<Vec<T>>
    where
        T: serde::de::DeserializeOwned + serde::Serialize + Send + Sync,
    {
        let mut aggregated = Vec::new();
        
        match strategy {
            AggregationStrategy::MergeAll => {
                for result in results {
                    aggregated.extend(result.data);
                }
            }
            AggregationStrategy::MergeDistinct { key_fields: _ } => {
                // In a real implementation, this would deduplicate based on key fields
                let mut seen = HashSet::new();
                for result in results {
                    for item in result.data {
                        // Simplified: use serialized form as key
                        let key = serde_json::to_string(&item).unwrap_or_default();
                        if seen.insert(key) {
                            aggregated.push(item);
                        }
                    }
                }
            }
            AggregationStrategy::MergeAndSort { sorting: _ } => {
                // First merge all results
                for result in results {
                    aggregated.extend(result.data);
                }
                // In a real implementation, sort based on the sorting parameter
            }
            AggregationStrategy::Custom { function, parameters: _ } => {
                return Err(Error::Generic(format!("Custom aggregation '{}' not implemented", function)));
            }
        }
        
        Ok(aggregated)
    }
    
    /// Estimate the cost of executing a query on a specific node
    async fn estimate_node_cost(&self, _node: &NodeInfo, query: &DistributedQuery) -> QueryCost {
        // Simplified cost estimation
        let base_cost = 10.0;
        let filter_cost = if query.base.filter.is_some() { 5.0 } else { 20.0 };
        let sort_cost = if query.base.sorting.is_some() { 10.0 } else { 0.0 };
        
        QueryCost {
            cpu_cost: base_cost + filter_cost + sort_cost,
            io_cost: base_cost * 2.0,
            network_cost: 5.0,
            estimated_time_ms: ((base_cost + filter_cost + sort_cost) * 10.0) as u64,
        }
    }
    
    /// Update node capabilities cache
    async fn update_node_capabilities(&self, nodes: &[NodeInfo]) {
        let mut cache = self.node_capabilities.write().await;
        for node in nodes {
            cache.insert(node.node_id.clone(), node.capabilities.clone());
        }
    }

    /// Get active nodes (placeholder - in a real system, this would query a coordination service)
    async fn get_active_nodes(&self) -> Result<Vec<NodeInfo>> {
        // In a real system, this would query a coordination service to get active nodes
        // For now, return a dummy list
        Ok(vec![
            NodeInfo {
                node_id: "node1".to_string(),
                capabilities: NodeCapabilities {
                    can_query_chains: vec![almanac_infrastructure_core::ChainId::from("ethereum")],
                    can_aggregate: true,
                    can_stream: false,
                },
                locality: DataLocality {
                    chains: vec![almanac_infrastructure_core::ChainId::from("ethereum")],
                    block_ranges: HashMap::new(),
                    contracts: None,
                    cached_queries: Vec::new(),
                },
            },
            NodeInfo {
                node_id: "node2".to_string(),
                capabilities: NodeCapabilities {
                    can_query_chains: vec![almanac_infrastructure_core::ChainId::from("cosmos")],
                    can_aggregate: true,
                    can_stream: false,
                },
                locality: DataLocality {
                    chains: vec![almanac_infrastructure_core::ChainId::from("cosmos")],
                    block_ranges: HashMap::new(),
                    contracts: None,
                    cached_queries: Vec::new(),
                },
            },
        ])
    }
    
    /// Compute cache key for a query
    fn compute_cache_key(&self, query: &DistributedQuery) -> String {
        // Simplified: use serialized query as cache key
        let key_data = serde_json::to_string(query).unwrap_or_default();
        // Use a simple hash instead of sha256
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        key_data.hash(&mut hasher);
        format!("query_{:x}", hasher.finish())
    }
}

/// Internal routing analysis result
struct RoutingAnalysis {
    selected_nodes: Vec<NodeInfo>,
    metadata: QueryRoutingMetadata,
}

/// Query cache implementation
struct QueryCache {
    capacity: usize,
    entries: HashMap<String, QueryPlan>,
    access_order: Vec<String>,
}

impl QueryCache {
    fn new(capacity: usize) -> Self {
        Self {
            capacity,
            entries: HashMap::new(),
            access_order: Vec::new(),
        }
    }
    
    fn get(&self, key: &str) -> Option<&QueryPlan> {
        self.entries.get(key)
    }
    
    fn put(&mut self, key: String, plan: QueryPlan) {
        // Simple LRU eviction
        if self.entries.len() >= self.capacity && !self.entries.contains_key(&key) {
            if let Some(oldest) = self.access_order.first() {
                let oldest_key = oldest.clone();
                self.entries.remove(&oldest_key);
                self.access_order.remove(0);
            }
        }
        
        self.entries.insert(key.clone(), plan);
        self.access_order.push(key);
    }
}

/// Trait for query execution on nodes
#[async_trait]
pub trait QueryExecutor: Send + Sync {
    /// Execute a query and return results as JSON values
    async fn execute_query(&self, query: SubQuery) -> Result<QueryResult<serde_json::Value>>;
}

/// Query routing service that handles query distribution
pub struct QueryRoutingService {
    planner: Arc<DistributedQueryPlanner>,
    executors: Arc<RwLock<HashMap<String, Arc<dyn QueryExecutor>>>>,
}

impl QueryRoutingService {
    pub fn new(planner: Arc<DistributedQueryPlanner>) -> Self {
        Self {
            planner,
            executors: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// Register a query executor for a node
    pub async fn register_executor(&self, node_id: String, executor: Arc<dyn QueryExecutor>) {
        let mut executors = self.executors.write().await;
        executors.insert(node_id, executor);
    }
    
    /// Execute a distributed query
    pub async fn execute_query<T>(&self, query: DistributedQuery) -> Result<Vec<T>>
    where
        T: serde::de::DeserializeOwned + serde::Serialize + Send + Sync + 'static,
    {
        // Create query plan
        let plan = self.planner.create_plan(query).await?;
        
        // Execute sub-queries
        let mut handles = Vec::new();
        let executors = self.executors.read().await;
        
        for sub_query in plan.sub_queries.clone() {
            if let Some(executor) = executors.get(&sub_query.node_id) {
                let executor = Arc::clone(executor);
                let handle = tokio::spawn(async move {
                    executor.execute_query(sub_query).await
                });
                handles.push(handle);
            } else {
                warn!("No executor registered for node: {}", sub_query.node_id);
            }
        }
        
        // Collect results
        let mut results = Vec::new();
        for handle in handles {
            match handle.await {
                Ok(Ok(result)) => results.push(result),
                Ok(Err(e)) => warn!("Query execution failed: {}", e),
                Err(e) => warn!("Task join error: {}", e),
            }
        }
        
        // Convert JSON results to target type and aggregate
        let converted_results: Result<Vec<QueryResult<T>>> = results
            .into_iter()
            .map(|result| {
                let converted_data: Result<Vec<T>> = result.data
                    .into_iter()
                    .map(|value| serde_json::from_value(value).map_err(|e| Error::generic(e.to_string())))
                    .collect();
                
                converted_data.map(|data| QueryResult {
                    node_id: result.node_id,
                    data,
                    metadata: result.metadata,
                })
            })
            .collect();
            
        let typed_results = converted_results?;
        self.planner.aggregate_results(typed_results, &plan.aggregation).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    
    #[tokio::test]
    async fn test_query_plan_creation() {
        let planner = DistributedQueryPlanner::new();
        
        // Update data locality
        planner.update_data_locality(
            "node1".to_string(),
            DataLocality {
                chains: vec![almanac_infrastructure_core::ChainId::from("ethereum")],
                block_ranges: HashMap::new(),
                contracts: None,
                cached_queries: Vec::new(),
            },
        ).await;
        
        // Create a test query
        let query = DistributedQuery {
            base: BaseQuery {
                filter: Some(QueryFilter {
                    chain_ids: Some(vec![almanac_infrastructure_core::ChainId::from("ethereum")]),
                    contract_addresses: None,
                    contract_types: None,
                    entity_ids: None,
                    attributes: None,
                    tags: None,
                }),
                pagination: None,
                sorting: None,
            },
            hints: QueryHints::default(),
            timeout_ms: 5000,
            use_cache: true,
        };
        
        // Create plan
        let plan = planner.create_plan(query).await.unwrap();
        
        // Verify plan
        assert!(!plan.plan_id.is_empty());
        assert!(!plan.sub_queries.is_empty());
        assert_eq!(plan.sub_queries[0].node_id, "node1");
    }
}