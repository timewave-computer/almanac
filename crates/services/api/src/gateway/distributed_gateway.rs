/// Enhanced API gateway with distributed query support, smart routing, and response aggregation
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use axum::{
    extract::{Path, State, ConnectInfo, WebSocketUpgrade},
    http::StatusCode,
    middleware::{self, Next},
    response::{IntoResponse, Json, Response},
    routing::{get, post},
    Router,
    http::Request,
    body::Body,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::sync::RwLock;
use tokio::time::timeout;
use tracing::{info, error};
// futures imports removed as they're unused

use almanac_infrastructure_core::{Error, Result};
use almanac_infrastructure_query::planner::{
    DistributedQuery, DistributedQueryPlanner, QueryResult,
    QueryHints, DataLocality, QueryRoutingService,
    AggregationStrategy, BaseQuery, QueryFilter,
    Pagination, Sorting, SortDirection,
};
use almanac_infrastructure_coordination::{CoordinationService, NodeInfo, NodeStatus, NodeCapabilities, WorkAssignment, LeaderInfo};
// BoxedEventService import removed as it's unused
use almanac_infrastructure_core::types::ChainId;

use crate::{HttpState, EventResponse, EventsResponse, PaginationInfo};
// DistributedQueryState import removed as it's unused

/// Enhanced gateway state with distributed capabilities
#[derive(Clone)]
pub struct DistributedGatewayState {
    /// Base HTTP state
    pub http_state: HttpState,
    /// Distributed query planner
    pub query_planner: Arc<DistributedQueryPlanner>,
    /// Query routing service
    pub routing_service: Arc<QueryRoutingService>,
    /// Coordination service
    pub coordination: Arc<dyn CoordinationService>,
    /// Node discovery cache
    pub node_cache: Arc<RwLock<NodeDiscoveryCache>>,
    /// Distributed rate limiter
    pub distributed_rate_limiter: Arc<DistributedRateLimiter>,
    /// Response aggregator
    pub response_aggregator: Arc<ResponseAggregator>,
    /// Gateway metrics
    pub metrics: Arc<RwLock<GatewayMetrics>>,
}

/// Node discovery cache for efficient routing
#[derive(Debug, Clone)]
pub struct NodeDiscoveryCache {
    /// Active nodes by ID
    pub nodes: HashMap<String, NodeInfo>,
    /// Nodes by chain support
    pub nodes_by_chain: HashMap<ChainId, Vec<String>>,
    /// Node health scores
    pub health_scores: HashMap<String, f64>,
    /// Last update time
    pub last_update: Instant,
    /// Cache TTL
    pub ttl: Duration,
}

impl NodeDiscoveryCache {
    pub fn new(ttl: Duration) -> Self {
        Self {
            nodes: HashMap::new(),
            nodes_by_chain: HashMap::new(),
            health_scores: HashMap::new(),
            last_update: Instant::now(),
            ttl,
        }
    }

    /// Update cache with fresh node information
    pub fn update(&mut self, nodes: Vec<NodeInfo>) {
        self.nodes.clear();
        self.nodes_by_chain.clear();
        
        for node in nodes {
            // Update node info
            self.nodes.insert(node.node_id.clone(), node.clone());
            
            // Update chain mapping
            for chain in &node.capabilities.supported_chains {
                self.nodes_by_chain
                    .entry(ChainId(chain.clone()))
                    .or_insert_with(Vec::new)
                    .push(node.node_id.clone());
            }
            
            // Calculate initial health score
            let health_score = calculate_node_health(&node);
            self.health_scores.insert(node.node_id.clone(), health_score);
        }
        
        self.last_update = Instant::now();
    }

    /// Check if cache needs refresh
    pub fn needs_refresh(&self) -> bool {
        self.last_update.elapsed() > self.ttl
    }

    /// Get nodes supporting a specific chain
    pub fn get_nodes_for_chain(&self, chain_id: &ChainId) -> Vec<&NodeInfo> {
        self.nodes_by_chain
            .get(chain_id)
            .map(|node_ids| {
                node_ids.iter()
                    .filter_map(|id| self.nodes.get(id))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get node with best health score for a chain
    pub fn get_best_node_for_chain(&self, chain_id: &ChainId) -> Option<&NodeInfo> {
        self.get_nodes_for_chain(chain_id)
            .into_iter()
            .max_by(|a, b| {
                let score_a = self.health_scores.get(&a.node_id).unwrap_or(&0.0);
                let score_b = self.health_scores.get(&b.node_id).unwrap_or(&0.0);
                score_a.partial_cmp(score_b).unwrap_or(std::cmp::Ordering::Equal)
            })
    }
}

/// Calculate node health score (0.0 to 1.0)
fn calculate_node_health(node: &NodeInfo) -> f64 {
    let mut score = 1.0;
    
    // Penalize based on status
    match node.status {
        NodeStatus::Active => {},
        NodeStatus::Degraded => score *= 0.7,
        NodeStatus::Starting => score *= 0.3,
        _ => score = 0.0,
    }
    
    // Consider last heartbeat (assuming heartbeat within last 60 seconds is healthy)
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let time_since_heartbeat = now.saturating_sub(node.last_heartbeat);
    
    if time_since_heartbeat > 60 {
        score *= 0.5;
    } else if time_since_heartbeat > 30 {
        score *= 0.8;
    }
    
    score
}

/// Distributed rate limiter that coordinates across nodes
pub struct DistributedRateLimiter {
    /// Local rate limit cache
    local_cache: Arc<RwLock<HashMap<String, RateLimitEntry>>>,
    /// Coordination service for distributed state
    coordination: Arc<dyn CoordinationService>,
    /// Rate limit configuration
    config: RateLimitConfig,
    /// Sync interval
    sync_interval: Duration,
}

#[derive(Debug, Clone)]
struct RateLimitEntry {
    /// Number of requests
    count: u64,
    /// Window start time
    window_start: Instant,
    /// Last sync time
    last_sync: Instant,
}

#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    /// Requests per window
    pub requests_per_window: u64,
    /// Time window duration
    pub window_duration: Duration,
    /// Sync threshold (sync when this many requests accumulated)
    pub sync_threshold: u64,
}

impl DistributedRateLimiter {
    pub fn new(
        coordination: Arc<dyn CoordinationService>,
        config: RateLimitConfig,
    ) -> Self {
        Self {
            local_cache: Arc::new(RwLock::new(HashMap::new())),
            coordination,
            config,
            sync_interval: Duration::from_secs(5),
        }
    }

    /// Check if request is allowed for a client
    pub async fn check_rate_limit(&self, client_id: &str) -> Result<bool> {
        let mut cache = self.local_cache.write().await;
        let now = Instant::now();
        
        let entry = cache.entry(client_id.to_string()).or_insert_with(|| {
            RateLimitEntry {
                count: 0,
                window_start: now,
                last_sync: now,
            }
        });
        
        // Check if window has expired
        if now.duration_since(entry.window_start) > self.config.window_duration {
            // Reset window
            entry.count = 0;
            entry.window_start = now;
        }
        
        // Check rate limit
        if entry.count >= self.config.requests_per_window {
            return Ok(false);
        }
        
        // Increment counter
        entry.count += 1;
        
        // Sync with distributed state if threshold reached
        if entry.count % self.config.sync_threshold == 0 {
            // In a real implementation, this would sync with other nodes
            entry.last_sync = now;
        }
        
        Ok(true)
    }

    /// Get remaining requests for a client
    pub async fn get_remaining(&self, client_id: &str) -> u64 {
        let cache = self.local_cache.read().await;
        
        if let Some(entry) = cache.get(client_id) {
            let now = Instant::now();
            if now.duration_since(entry.window_start) > self.config.window_duration {
                self.config.requests_per_window
            } else {
                self.config.requests_per_window.saturating_sub(entry.count)
            }
        } else {
            self.config.requests_per_window
        }
    }
}

/// Response aggregator for distributed query results
pub struct ResponseAggregator {
    /// Custom aggregation strategies
    custom_strategies: HashMap<String, CustomAggregator>,
}

impl ResponseAggregator {
    pub fn new() -> Self {
        Self {
            custom_strategies: HashMap::new(),
        }
    }

    /// Aggregate responses based on strategy
    pub async fn aggregate<T>(
        &self,
        results: Vec<QueryResult<T>>,
        strategy: &AggregationStrategy,
    ) -> Result<Vec<T>>
    where
        T: serde::de::DeserializeOwned + serde::Serialize + Send + Sync + 'static,
    {
        match strategy {
            AggregationStrategy::MergeAll => {
                Self::merge_all(results).await
            }
            AggregationStrategy::MergeDistinct { key_fields } => {
                Self::merge_distinct(results, key_fields).await
            }
            AggregationStrategy::MergeAndSort { sorting } => {
                Self::merge_and_sort(results, sorting).await
            }
            AggregationStrategy::Custom { function, parameters } => {
                if let Some(custom) = self.custom_strategies.get(function) {
                    // Convert results to JSON values for the custom function
                    let json_results: Vec<QueryResult<serde_json::Value>> = results
                        .into_iter()
                        .map(|r| QueryResult {
                            node_id: r.node_id,
                            data: r.data.into_iter().map(|item| {
                                serde_json::to_value(item).unwrap_or(serde_json::Value::Null)
                            }).collect(),
                            metadata: r.metadata,
                        })
                        .collect();
                    
                    // Call the custom function
                    let json_result = custom(json_results, parameters).await?;
                    
                    // Convert back to T
                    let result: Vec<T> = json_result.into_iter()
                        .filter_map(|v| serde_json::from_value(v).ok())
                        .collect();
                    Ok(result)
                } else {
                    Err(Error::Generic(format!("Custom strategy '{}' not found", function)))
                }
            }
        }
    }

    /// Register a custom aggregation strategy
    pub fn register_strategy(&mut self, name: String, strategy: CustomAggregator) {
        self.custom_strategies.insert(name, strategy);
    }

    /// Simple merge all strategy
    async fn merge_all<T>(results: Vec<QueryResult<T>>) -> Result<Vec<T>>
    where
        T: serde::de::DeserializeOwned + serde::Serialize + Send + Sync + 'static,
    {
        let mut merged = Vec::new();
        for result in results {
            merged.extend(result.data);
        }
        Ok(merged)
    }

    /// Merge with deduplication strategy
    async fn merge_distinct<T>(
        results: Vec<QueryResult<T>>,
        _key_fields: &[String],
    ) -> Result<Vec<T>>
    where
        T: serde::de::DeserializeOwned + serde::Serialize + Send + Sync + 'static,
    {
        // For now, just do simple deduplication based on serialized form
        // In a real implementation, you'd use the key_fields to build a proper deduplication key
        let mut merged = Vec::new();
        let mut seen = std::collections::HashSet::new();
        
        for result in results {
            for item in result.data {
                if let Ok(serialized) = serde_json::to_string(&item) {
                    if seen.insert(serialized) {
                        merged.push(item);
                    }
                }
            }
        }
        Ok(merged)
    }

    /// Merge and sort strategy
    async fn merge_and_sort<T>(
        results: Vec<QueryResult<T>>,
        _sorting: &almanac_infrastructure_query::planner::Sorting,
    ) -> Result<Vec<T>>
    where
        T: serde::de::DeserializeOwned + serde::Serialize + Send + Sync + 'static,
    {
        // For now, just merge without sorting
        // In a real implementation, you'd implement proper sorting based on the sorting parameter
        let mut merged = Vec::new();
        for result in results {
            merged.extend(result.data);
        }
        Ok(merged)
    }
}

/// Custom aggregation strategy function
pub type CustomAggregator = fn(Vec<QueryResult<serde_json::Value>>, &HashMap<String, serde_json::Value>) -> futures::future::BoxFuture<'static, Result<Vec<serde_json::Value>>>;



/// Gateway metrics for monitoring
#[derive(Debug, Clone, Default, Serialize)]
pub struct GatewayMetrics {
    /// Total requests
    pub total_requests: u64,
    /// Distributed queries executed
    pub distributed_queries: u64,
    /// Cache hits
    pub cache_hits: u64,
    /// Cache misses
    pub cache_misses: u64,
    /// Rate limit rejections
    pub rate_limit_rejections: u64,
    /// Average query time
    pub avg_query_time_ms: f64,
    /// Node health checks
    pub health_checks: u64,
}

/// Create the enhanced distributed gateway router
pub fn create_distributed_gateway(state: Arc<DistributedGatewayState>) -> Router {
    Router::new()
        // Node discovery endpoints
        .route("/api/v1/nodes", get(list_nodes))
        .route("/api/v1/nodes/:node_id", get(get_node_info))
        .route("/api/v1/nodes/:node_id/health", get(get_node_health))
        .route("/api/v1/nodes/:node_id/locality", post(update_node_locality))
        
        // Distributed query endpoints
        .route("/api/v1/query/distributed", post(execute_distributed_query))
        .route("/api/v1/query/plan", post(create_query_plan))
        .route("/api/v1/query/analyze", post(analyze_query))
        
        // Enhanced event endpoints with smart routing
        .route("/api/v1/events/smart", post(query_events_smart))
        .route("/api/v1/events/aggregate", post(aggregate_events))
        
        // Metrics and monitoring
        .route("/api/v1/gateway/metrics", get(get_gateway_metrics))
        .route("/api/v1/gateway/health", get(gateway_health_check))
        
        // WebSocket for real-time updates
        .route("/api/v1/ws/distributed", get(distributed_websocket_handler))
        
        .layer(middleware::from_fn_with_state(
            state.clone(),
            distributed_rate_limit_middleware
        ))
        .with_state(state)
}

/// Enhanced rate limiting middleware with distributed coordination
async fn distributed_rate_limit_middleware(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    State(state): State<Arc<DistributedGatewayState>>,
    request: Request<Body>,
    next: Next,
) -> std::result::Result<Response, StatusCode> {
    let client_ip = addr.ip().to_string();
    
    // Check distributed rate limit
    match state.distributed_rate_limiter.check_rate_limit(&client_ip).await {
        Ok(true) => {
            // Request allowed
            let mut response = next.run(request).await;
            
            // Add rate limit headers
            let remaining = state.distributed_rate_limiter.get_remaining(&client_ip).await;
            let headers = response.headers_mut();
            headers.insert(
                "X-RateLimit-Limit",
                state.distributed_rate_limiter.config.requests_per_window.to_string().parse().unwrap()
            );
            headers.insert(
                "X-RateLimit-Remaining",
                remaining.to_string().parse().unwrap()
            );
            headers.insert(
                "X-RateLimit-Reset",
                (SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs() + 
                    state.distributed_rate_limiter.config.window_duration.as_secs())
                    .to_string()
                    .parse()
                    .unwrap()
            );
            
            Ok(response)
        }
        Ok(false) => {
            // Rate limit exceeded
            let mut metrics = state.metrics.write().await;
            metrics.rate_limit_rejections += 1;
            
            let response = Json(json!({
                "error": "Rate limit exceeded",
                "retry_after": state.distributed_rate_limiter.config.window_duration.as_secs()
            })).into_response();
            
            Ok((StatusCode::TOO_MANY_REQUESTS, response).into_response())
        }
        Err(e) => {
            error!("Rate limiter error: {}", e);
            // Allow request on error
            Ok(next.run(request).await)
        }
    }
}

/// List all active nodes in the cluster
async fn list_nodes(
    State(state): State<Arc<DistributedGatewayState>>,
) -> Result<Json<Vec<NodeInfoResponse>>> {
    // Refresh cache if needed
    let mut cache = state.node_cache.write().await;
    if cache.needs_refresh() {
        let nodes = state.coordination.get_active_nodes().await?;
        cache.update(nodes);
    }
    drop(cache);
    
    // Get nodes from cache
    let cache = state.node_cache.read().await;
    let nodes: Vec<NodeInfoResponse> = cache.nodes.values()
        .map(|node| NodeInfoResponse {
            node_id: node.node_id.clone(),
            address: node.address.clone(),
            status: format!("{:?}", node.status),
            health_score: cache.health_scores.get(&node.node_id).copied().unwrap_or(0.0),
            capabilities: NodeCapabilitiesResponse {
                supported_chains: node.capabilities.supported_chains.clone(),
                max_throughput: node.capabilities.max_throughput,
                storage_backends: node.capabilities.storage_backends.clone(),
                api_endpoints: node.capabilities.api_endpoints.clone(),
            },
            last_heartbeat: node.last_heartbeat,
        })
        .collect();
    
    Ok(Json(nodes))
}

/// Get information about a specific node
async fn get_node_info(
    State(state): State<Arc<DistributedGatewayState>>,
    Path(node_id): Path<String>,
) -> Result<Json<NodeInfoResponse>> {
    let cache = state.node_cache.read().await;
    
    let node = cache.nodes.get(&node_id)
        .ok_or_else(|| Error::NotFound(format!("Node not found: {}", node_id)))?;
    
    let response = NodeInfoResponse {
        node_id: node.node_id.clone(),
        address: node.address.clone(),
        status: format!("{:?}", node.status),
        health_score: cache.health_scores.get(&node.node_id).copied().unwrap_or(0.0),
        capabilities: NodeCapabilitiesResponse {
            supported_chains: node.capabilities.supported_chains.clone(),
            max_throughput: node.capabilities.max_throughput,
            storage_backends: node.capabilities.storage_backends.clone(),
            api_endpoints: node.capabilities.api_endpoints.clone(),
        },
        last_heartbeat: node.last_heartbeat,
    };
    
    Ok(Json(response))
}

/// Get health status of a specific node
async fn get_node_health(
    State(state): State<Arc<DistributedGatewayState>>,
    Path(node_id): Path<String>,
) -> Result<Json<NodeHealthResponse>> {
    let cache = state.node_cache.read().await;
    
    let node = cache.nodes.get(&node_id)
        .ok_or_else(|| Error::NotFound(format!("Node not found: {}", node_id)))?;
    
    let health_score = cache.health_scores.get(&node_id).copied().unwrap_or(0.0);
    
    let response = NodeHealthResponse {
        node_id: node_id.clone(),
        healthy: health_score > 0.5,
        health_score,
        status: format!("{:?}", node.status),
        last_heartbeat: node.last_heartbeat,
        time_since_heartbeat: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
            .saturating_sub(node.last_heartbeat),
    };
    
    // Increment health check metric
    let mut metrics = state.metrics.write().await;
    metrics.health_checks += 1;
    
    Ok(Json(response))
}

/// Update node locality information
async fn update_node_locality(
    State(state): State<Arc<DistributedGatewayState>>,
    Path(node_id): Path<String>,
    Json(locality): Json<DataLocalityRequest>,
) -> Result<Json<Value>> {
    info!("Updating locality for node: {}", node_id);
    
    let data_locality = DataLocality {
        chains: locality.chains.into_iter().map(ChainId).collect(),
        block_ranges: locality.block_ranges.unwrap_or_default(),
        contracts: locality.contracts,
        cached_queries: locality.cached_queries.unwrap_or_default(),
    };
    
    state.query_planner.update_data_locality(node_id.clone(), data_locality).await;
    
    Ok(Json(json!({
        "status": "success",
        "node_id": node_id,
        "message": "Locality information updated"
    })))
}

/// Execute a distributed query with smart routing
async fn execute_distributed_query(
    State(state): State<Arc<DistributedGatewayState>>,
    Json(request): Json<DistributedQueryRequest>,
) -> Result<Json<DistributedQueryResponse>> {
    let start_time = Instant::now();
    
    // Update metrics
    {
        let mut metrics = state.metrics.write().await;
        metrics.total_requests += 1;
        metrics.distributed_queries += 1;
    }
    
    // Create distributed query
    let query = create_distributed_query_from_request(request)?;
    
    // Create and execute query plan
    let plan = state.query_planner.create_plan(query.clone()).await?;
    let plan_id = plan.plan_id.clone();
    let nodes_queried = plan.sub_queries.len();
    
    // Check cache hit
    let cache_hit = plan.cache_key.is_some();
    if cache_hit {
        let mut metrics = state.metrics.write().await;
        metrics.cache_hits += 1;
    } else {
        let mut metrics = state.metrics.write().await;
        metrics.cache_misses += 1;
    }
    
    // Execute with timeout
    let results = match timeout(
        Duration::from_millis(query.timeout_ms),
        state.routing_service.execute_query::<Value>(query)
    ).await {
        Ok(Ok(results)) => results,
        Ok(Err(e)) => return Err(Error::generic(format!("Query execution failed: {}", e))),
        Err(_) => return Err(Error::generic("Query timeout")),
    };
    
    let execution_time = start_time.elapsed();
    
    // Update average query time
    {
        let mut metrics = state.metrics.write().await;
        let total_time = metrics.avg_query_time_ms * metrics.distributed_queries as f64;
        metrics.avg_query_time_ms = (total_time + execution_time.as_millis() as f64) / (metrics.distributed_queries as f64);
    }
    
    let response = DistributedQueryResponse {
        plan_id,
        data: results,
        metadata: QueryMetadata {
            execution_time_ms: execution_time.as_millis() as u64,
            nodes_queried,
            cache_hit,
            total_results: 0, // Would be calculated from results
            warnings: Vec::new(),
        },
    };
    
    Ok(Json(response))
}

/// Create a query plan without executing
async fn create_query_plan(
    State(state): State<Arc<DistributedGatewayState>>,
    Json(request): Json<DistributedQueryRequest>,
) -> Result<Json<Value>> {
    let query = create_distributed_query_from_request(request)?;
    let plan = state.query_planner.create_plan(query).await?;
    
    Ok(Json(serde_json::to_value(plan)?))
}

/// Analyze a query and return routing information
async fn analyze_query(
    State(state): State<Arc<DistributedGatewayState>>,
    Json(request): Json<DistributedQueryRequest>,
) -> Result<Json<QueryAnalysis>> {
    let query = create_distributed_query_from_request(request)?;
    let plan = state.query_planner.create_plan(query).await?;
    
    let analysis = QueryAnalysis {
        plan_id: plan.plan_id,
        estimated_cost: QueryCost {
            cpu_units: plan.routing_metadata.total_cost.estimated_time_ms,
            memory_mb: (plan.routing_metadata.total_cost.cpu_cost * 100.0) as u64,
            io_operations: (plan.routing_metadata.total_cost.io_cost * 100.0) as u64,
            network_bytes: (plan.routing_metadata.total_cost.network_cost * 1000.0) as u64,
        },
        selected_nodes: plan.sub_queries.iter()
            .map(|sq| sq.node_id.clone())
            .collect(),
        data_distribution: plan.routing_metadata.data_distribution
            .into_iter()
            .map(|(node_id, dist)| (node_id.clone(), DataDistribution {
                node_id,
                data_size_bytes: dist.size_bytes,
                row_count: dist.event_count,
                shard_keys: vec![], // Default empty shard keys
            }))
            .collect(),
        routing_reason: plan.routing_metadata.routing_reason,
        cache_key: plan.cache_key,
    };
    
    Ok(Json(analysis))
}

/// Smart event query with automatic routing
async fn query_events_smart(
    State(state): State<Arc<DistributedGatewayState>>,
    Json(request): Json<SmartQueryRequest>,
) -> Result<Json<EventsResponse>> {
    info!("Executing smart event query");
    
    // Analyze query to determine best routing
    let cache = state.node_cache.read().await;
    
    // Determine target nodes based on query parameters
    let target_nodes = if let Some(chain_id) = &request.chain_id {
        cache.get_nodes_for_chain(&ChainId(chain_id.clone()))
    } else {
        cache.nodes.values().collect()
    };
    
    if target_nodes.is_empty() {
        return Err(Error::not_found("No suitable nodes found for query"));
    }
    
    // For simple queries, route to single best node
    if target_nodes.len() == 1 || request.limit.unwrap_or(100) <= 100 {
        // Route to single node
        let best_node = if let Some(chain_id) = &request.chain_id {
            cache.get_best_node_for_chain(&ChainId(chain_id.clone()))
        } else {
            target_nodes.first().copied()
        };
        
        if let Some(_node) = best_node {
            // Execute query on single node
            // In real implementation, would forward to node
            return Ok(Json(EventsResponse {
                events: Vec::new(),
                pagination: PaginationInfo {
                    total: Some(0),
                    limit: request.limit.unwrap_or(100),
                    offset: request.offset.unwrap_or(0),
                    has_more: false,
                },
            }));
        }
    }
    
    // For complex queries, use distributed query planner
    let distributed_request = DistributedQueryRequest {
        filter: Some(QueryFilterRequest {
            chain_ids: request.chain_id.map(|c| vec![c]),
            addresses: request.addresses,
            event_types: request.event_types,
            from_height: request.from_height,
            to_height: request.to_height,
        }),
        pagination: Some(PaginationRequest {
            limit: request.limit,
            offset: request.offset,
        }),
        sorting: request.sort_by.map(|field| SortingRequest {
            field,
            ascending: request.ascending,
        }),
        hints: Some(QueryHints {
            preferred_nodes: None,
            required_capabilities: None,
            max_parallel: Some(target_nodes.len()),
            allow_partial_results: request.allow_partial.unwrap_or(false),
        }),
        timeout_ms: request.timeout_ms.unwrap_or(30000),
        use_cache: request.use_cache.unwrap_or(true),
    };
    
    let result = execute_distributed_query(State(state.clone()), Json(distributed_request)).await?;
    
    // Convert to EventsResponse
    let events: Vec<EventResponse> = result.data.iter()
        .filter_map(|v| serde_json::from_value(v.clone()).ok())
        .collect();
    
    Ok(Json(EventsResponse {
        events,
        pagination: PaginationInfo {
            total: Some(result.metadata.total_results as u64),
            limit: request.limit.unwrap_or(100),
            offset: request.offset.unwrap_or(0),
            has_more: false, // Would be calculated properly
        },
    }))
}

/// Aggregate events across nodes
async fn aggregate_events(
    State(state): State<Arc<DistributedGatewayState>>,
    Json(request): Json<AggregationRequest>,
) -> Result<Json<AggregationResponse>> {
    info!("Executing event aggregation");
    
    // Create distributed query for aggregation
    let distributed_query = DistributedQuery {
        base: BaseQuery {
            filter: Some(QueryFilter {
                chain_ids: request.chain_ids.map(|ids| ids.into_iter().map(ChainId).collect()),
                contract_addresses: request.contract_addresses,
                contract_types: None,
                entity_ids: None,
                attributes: None,
                tags: None,
            }),
            pagination: None,
            sorting: None,
        },
        hints: QueryHints::default(),
        timeout_ms: request.timeout_ms.unwrap_or(60000),
        use_cache: true,
    };
    
    // Execute aggregation
    let plan = state.query_planner.create_plan(distributed_query).await?;
    
    // For now, return empty results
    // In real implementation, would execute aggregation logic
    Ok(Json(AggregationResponse {
        aggregations: HashMap::new(),
        metadata: AggregationMetadata {
            total_buckets: 0,
            execution_time_ms: 0,
            nodes_queried: plan.sub_queries.len(),
        },
    }))
}

/// Get gateway metrics
async fn get_gateway_metrics(
    State(state): State<Arc<DistributedGatewayState>>,
) -> impl IntoResponse {
    let metrics = state.metrics.read().await;
    Json(metrics.clone())
}

/// Gateway health check
async fn gateway_health_check(
    State(state): State<Arc<DistributedGatewayState>>,
) -> Result<Json<GatewayHealth>> {
    let cache = state.node_cache.read().await;
    let healthy_nodes = cache.nodes.values()
        .filter(|n| matches!(n.status, NodeStatus::Active))
        .count();
    
    let health = GatewayHealth {
        healthy: healthy_nodes > 0,
        total_nodes: cache.nodes.len(),
        healthy_nodes,
        cache_age_seconds: cache.last_update.elapsed().as_secs(),
        uptime_seconds: state.http_state.start_time.elapsed().unwrap_or_default().as_secs(),
    };
    
    Ok(Json(health))
}

/// WebSocket handler for distributed real-time updates
async fn distributed_websocket_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<DistributedGatewayState>>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_distributed_websocket(socket, state))
}

async fn handle_distributed_websocket(
    socket: axum::extract::ws::WebSocket,
    state: Arc<DistributedGatewayState>,
) {
    // Implementation would handle distributed WebSocket connections
    // For now, just close the connection
    let _ = socket;
    let _ = state;
}

/// Helper function to create distributed query from request
fn create_distributed_query_from_request(request: DistributedQueryRequest) -> Result<DistributedQuery> {
    Ok(DistributedQuery {
        base: BaseQuery {
            filter: request.filter.map(|f| QueryFilter {
                chain_ids: f.chain_ids.map(|ids| ids.into_iter().map(ChainId).collect()),
                contract_addresses: f.addresses,
                contract_types: f.event_types,
                entity_ids: None,
                attributes: None,
                tags: None,
            }),
            pagination: request.pagination.map(|p| Pagination {
                limit: p.limit.unwrap_or(100),
                offset: p.offset.unwrap_or(0),
            }),
            sorting: request.sorting.map(|s| Sorting {
                field: s.field,
                direction: if s.ascending.unwrap_or(true) {
                    SortDirection::Ascending
                } else {
                    SortDirection::Descending
                },
            }),
        },
        hints: request.hints.unwrap_or_default(),
        timeout_ms: request.timeout_ms,
        use_cache: request.use_cache,
    })
}

// Request/Response types

#[derive(Debug, Deserialize)]
pub struct DistributedQueryRequest {
    pub filter: Option<QueryFilterRequest>,
    pub pagination: Option<PaginationRequest>,
    pub sorting: Option<SortingRequest>,
    pub hints: Option<QueryHints>,
    pub timeout_ms: u64,
    pub use_cache: bool,
}

#[derive(Debug, Deserialize)]
pub struct QueryFilterRequest {
    pub chain_ids: Option<Vec<String>>,
    pub addresses: Option<Vec<String>>,
    pub event_types: Option<Vec<String>>,
    pub from_height: Option<u64>,
    pub to_height: Option<u64>,
}

#[derive(Debug, Deserialize)]
pub struct PaginationRequest {
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

#[derive(Debug, Deserialize)]
pub struct SortingRequest {
    pub field: String,
    pub ascending: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct DistributedQueryResponse {
    pub plan_id: String,
    pub data: Vec<Value>,
    pub metadata: QueryMetadata,
}

#[derive(Debug, Serialize)]
pub struct QueryMetadata {
    pub execution_time_ms: u64,
    pub nodes_queried: usize,
    pub cache_hit: bool,
    pub total_results: usize,
    pub warnings: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct NodeInfoResponse {
    pub node_id: String,
    pub address: String,
    pub status: String,
    pub health_score: f64,
    pub capabilities: NodeCapabilitiesResponse,
    pub last_heartbeat: u64,
}

#[derive(Debug, Serialize)]
pub struct NodeCapabilitiesResponse {
    pub supported_chains: Vec<String>,
    pub max_throughput: u64,
    pub storage_backends: Vec<String>,
    pub api_endpoints: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct NodeHealthResponse {
    pub node_id: String,
    pub healthy: bool,
    pub health_score: f64,
    pub status: String,
    pub last_heartbeat: u64,
    pub time_since_heartbeat: u64,
}

#[derive(Debug, Deserialize)]
pub struct DataLocalityRequest {
    pub chains: Vec<String>,
    pub block_ranges: Option<HashMap<ChainId, Vec<(u64, u64)>>>,
    pub contracts: Option<Vec<String>>,
    pub cached_queries: Option<Vec<String>>,
}

// Local types for query analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryCost {
    pub cpu_units: u64,
    pub memory_mb: u64,
    pub io_operations: u64,
    pub network_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataDistribution {
    pub node_id: String,
    pub data_size_bytes: u64,
    pub row_count: u64,
    pub shard_keys: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct QueryAnalysis {
    pub plan_id: String,
    pub estimated_cost: QueryCost,
    pub selected_nodes: Vec<String>,
    pub data_distribution: HashMap<String, DataDistribution>,
    pub routing_reason: String,
    pub cache_key: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SmartQueryRequest {
    pub chain_id: Option<String>,
    pub addresses: Option<Vec<String>>,
    pub event_types: Option<Vec<String>>,
    pub from_height: Option<u64>,
    pub to_height: Option<u64>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
    pub sort_by: Option<String>,
    pub ascending: Option<bool>,
    pub allow_partial: Option<bool>,
    pub timeout_ms: Option<u64>,
    pub use_cache: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct AggregationRequest {
    pub chain_ids: Option<Vec<String>>,
    pub contract_addresses: Option<Vec<String>>,
    pub aggregation_type: String,
    pub group_by: Option<Vec<String>>,
    pub time_bucket: Option<String>,
    pub timeout_ms: Option<u64>,
}

#[derive(Debug, Serialize)]
pub struct AggregationResponse {
    pub aggregations: HashMap<String, Value>,
    pub metadata: AggregationMetadata,
}

#[derive(Debug, Serialize)]
pub struct AggregationMetadata {
    pub total_buckets: usize,
    pub execution_time_ms: u64,
    pub nodes_queried: usize,
}

#[derive(Debug, Serialize)]
pub struct GatewayHealth {
    pub healthy: bool,
    pub total_nodes: usize,
    pub healthy_nodes: usize,
    pub cache_age_seconds: u64,
    pub uptime_seconds: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_node_health_calculation() {
        let node = NodeInfo {
            node_id: "test".to_string(),
            address: "127.0.0.1:8080".to_string(),
            capabilities: NodeCapabilities {
                supported_chains: vec!["ethereum".to_string()],
                max_throughput: 1000,
                storage_backends: vec!["postgres".to_string()],
                api_endpoints: vec!["rest".to_string()],
            },
            status: NodeStatus::Active,
            last_heartbeat: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            metadata: HashMap::new(),
        };
        
        let health = calculate_node_health(&node);
        assert!(health > 0.9); // Should be healthy
    }
    
    #[tokio::test]
    async fn test_rate_limiter() {
        let coordination = Arc::new(MockCoordination);
        let config = RateLimitConfig {
            requests_per_window: 10,
            window_duration: Duration::from_secs(60),
            sync_threshold: 5,
        };
        
        let limiter = DistributedRateLimiter::new(coordination, config);
        
        // Should allow first request
        assert!(limiter.check_rate_limit("test_client").await.unwrap());
        
        // Should have 9 remaining
        assert_eq!(limiter.get_remaining("test_client").await, 9);
    }
    
    // Mock coordination service for tests
    struct MockCoordination;
    
    #[async_trait::async_trait]
    impl CoordinationService for MockCoordination {
        async fn register_node(&self, _info: NodeInfo) -> Result<()> {
            Ok(())
        }
        
        async fn update_node_status(&self, _node_id: &str, _status: NodeStatus) -> Result<()> {
            Ok(())
        }
        
        async fn get_active_nodes(&self) -> Result<Vec<NodeInfo>> {
            Ok(Vec::new())
        }
        
        async fn assign_work(&self, _assignment: WorkAssignment) -> Result<()> {
            Ok(())
        }
        
        async fn get_node_assignments(&self, _node_id: &str) -> Result<Vec<WorkAssignment>> {
            Ok(Vec::new())
        }
        
        async fn complete_assignment(&self, _assignment_id: &str) -> Result<()> {
            Ok(())
        }
        
        async fn elect_leader(&self, _candidate_id: &str) -> Result<bool> {
            Ok(false)
        }
        
        async fn get_leader(&self) -> Result<Option<LeaderInfo>> {
            Ok(None)
        }
        
        async fn release_assignment(&self, _assignment_id: &str) -> Result<()> {
            Ok(())
        }
    }
}