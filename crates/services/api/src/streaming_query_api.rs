use async_trait::async_trait;
use axum::{
    extract::{Query, State, Path},
    response::{Json, Response, IntoResponse, sse::Event},
    routing::{get, post},
    Router,
    body::Body,
};
use axum::response::sse::{KeepAlive, Sse};
use futures::{Stream, StreamExt, stream};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use std::convert::Infallible;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tracing::{info, debug, warn, error};

use indexer_core::{Error, Result};
use indexer_core::distributed_query_planner::{
    DistributedQuery, DistributedQueryPlanner, QueryPlan,
    QueryExecutionMetadata, QueryHints, DataLocality,
};
use indexer_core::streaming_query_executor::{
    StreamingQueryExecutor, ParallelQueryOrchestrator, ConnectionPool,
};
use indexer_core::distributed_cache::{DistributedCache, CacheConfig};
use indexer_core::optimized_serialization::{
    OptimizedSerializer, SerializationFormat, CompressionType, CompressionLevel,
};
use indexer_core::coordination::CoordinationService;
use indexer_query::{BaseQuery, QueryFilter, Pagination, Sorting};

/// Enhanced API state with streaming support
pub struct StreamingQueryState {
    /// Query planner
    pub query_planner: Arc<DistributedQueryPlanner>,
    /// Streaming query executor  
    pub query_executor: Arc<StreamingQueryExecutor>,
    /// Parallel query orchestrator
    pub query_orchestrator: Arc<ParallelQueryOrchestrator>,
    /// Distributed cache
    pub cache: Arc<DistributedCache>,
    /// Coordination service
    pub coordination: Arc<dyn CoordinationService>,
    /// Serializer for efficient data transfer
    pub serializer: Arc<OptimizedSerializer>,
}

/// Streaming query request
#[derive(Debug, Deserialize)]
pub struct StreamingQueryRequest {
    /// Base query parameters
    pub query: QueryRequest,
    
    /// Streaming options
    pub streaming: StreamingOptions,
    
    /// Serialization format
    pub format: Option<String>,
    
    /// Enable compression
    pub compression: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct StreamingOptions {
    /// Chunk size for streaming
    pub chunk_size: Option<usize>,
    
    /// Maximum duration for the stream
    pub max_duration_seconds: Option<u64>,
    
    /// Whether to use server-sent events
    pub use_sse: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct QueryRequest {
    pub filter: Option<QueryFilterRequest>,
    pub pagination: Option<PaginationRequest>,
    pub sorting: Option<SortingRequest>,
    pub hints: Option<QueryHints>,
    pub timeout_ms: Option<u64>,
    pub use_cache: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct QueryFilterRequest {
    pub chain_ids: Option<Vec<String>>,
    pub contract_addresses: Option<Vec<String>>,
    pub block_range: Option<(u64, u64)>,
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

/// Create the streaming query router
pub fn create_streaming_query_router(state: Arc<StreamingQueryState>) -> Router {
    Router::new()
        // Streaming endpoints
        .route("/api/v1/query/stream", post(stream_query))
        .route("/api/v1/query/stream/sse", post(stream_query_sse))
        
        // Batch query endpoints
        .route("/api/v1/query/batch", post(batch_query))
        
        // Cache management
        .route("/api/v1/cache/stats", get(get_cache_stats))
        .route("/api/v1/cache/invalidate", post(invalidate_cache))
        
        // Performance monitoring
        .route("/api/v1/performance/metrics", get(get_performance_metrics))
        
        .with_state(state)
}

/// Stream query results using chunked transfer encoding
async fn stream_query(
    State(state): State<Arc<StreamingQueryState>>,
    Json(request): Json<StreamingQueryRequest>,
) -> Result<Response> {
    info!("Starting streaming query");
    
    // Create distributed query
    let query = create_distributed_query(request.query)?;
    
    // Create query plan
    let plan = state.query_planner.create_plan(query).await?;
    
    // Execute with streaming
    let stream = state.query_orchestrator
        .execute_plan_streaming::<serde_json::Value>(plan)
        .await;
    
    // Create response stream with proper serialization
    let format = request.format.as_deref().unwrap_or("json");
    let enable_compression = request.compression.unwrap_or(true);
    
    let body_stream = create_response_stream(
        stream,
        state.serializer.clone(),
        format,
        enable_compression,
    );
    
    Ok(Response::builder()
        .header("Content-Type", get_content_type(format))
        .header("Transfer-Encoding", "chunked")
        .header("Cache-Control", "no-cache")
        .body(Body::from_stream(body_stream))
        .unwrap())
}

/// Stream query results using Server-Sent Events
async fn stream_query_sse(
    State(state): State<Arc<StreamingQueryState>>,
    Json(request): Json<StreamingQueryRequest>,
) -> Result<Sse<impl Stream<Item = std::result::Result<Event, Infallible>>>> {
    info!("Starting SSE streaming query");
    
    // Create distributed query
    let query = create_distributed_query(request.query)?;
    
    // Create query plan
    let plan = state.query_planner.create_plan(query).await?;
    let plan_id = plan.plan_id.clone();
    
    // Execute with streaming
    let result_stream = state.query_orchestrator
        .execute_plan_streaming::<serde_json::Value>(plan)
        .await;
    
    // Convert to SSE events
    let event_stream = stream::unfold(
        (result_stream, 0usize),
        move |(mut stream, counter)| async move {
            match stream.next().await {
                Some(Ok(chunk)) => {
                    let event = Event::default()
                        .event("data")
                        .id(format!("{}:{}", plan_id, counter))
                        .json_data(json!({
                            "chunk": counter,
                            "data": chunk,
                            "timestamp": chrono::Utc::now().timestamp_millis(),
                        }))
                        .unwrap();
                    
                    Some((Ok(event), (stream, counter + 1)))
                }
                Some(Err(e)) => {
                    let event = Event::default()
                        .event("error")
                        .data(format!("Query error: {}", e));
                    
                    Some((Ok(event), (stream, counter)))
                }
                None => {
                    let event = Event::default()
                        .event("complete")
                        .json_data(json!({
                            "total_chunks": counter,
                            "plan_id": plan_id,
                        }))
                        .unwrap();
                    
                    Some((Ok(event), (stream, counter)))
                }
            }
        },
    );
    
    Ok(Sse::new(event_stream).keep_alive(
        KeepAlive::new()
            .interval(Duration::from_secs(30))
            .text("keep-alive"),
    ))
}

/// Execute batch queries with optimized serialization
async fn batch_query(
    State(state): State<Arc<StreamingQueryState>>,
    Json(requests): Json<Vec<QueryRequest>>,
) -> Result<Json<BatchQueryResponse>> {
    info!("Executing batch query with {} requests", requests.len());
    
    let mut results = Vec::new();
    let mut total_time_ms = 0u64;
    
    // Process queries in parallel batches
    let batch_size = 10;
    for batch in requests.chunks(batch_size) {
        let mut batch_futures = Vec::new();
        
        for request in batch {
            let query = create_distributed_query(request.clone())?;
            let planner = state.query_planner.clone();
            let orchestrator = state.query_orchestrator.clone();
            
            batch_futures.push(async move {
                let start = std::time::Instant::now();
                let plan = planner.create_plan(query).await?;
                
                // Collect all results
                let mut all_data = Vec::new();
                let mut stream = orchestrator
                    .execute_plan_streaming::<serde_json::Value>(plan.clone())
                    .await;
                    
                while let Some(chunk) = stream.next().await {
                    match chunk {
                        Ok(data) => all_data.extend(data),
                        Err(e) => return Err(e),
                    }
                }
                
                let elapsed = start.elapsed().as_millis() as u64;
                
                Ok(BatchQueryResult {
                    plan_id: plan.plan_id,
                    data: all_data,
                    execution_time_ms: elapsed,
                })
            });
        }
        
        // Wait for batch to complete
        let batch_results = futures::future::join_all(batch_futures).await;
        
        for result in batch_results {
            match result {
                Ok(query_result) => {
                    total_time_ms += query_result.execution_time_ms;
                    results.push(query_result);
                }
                Err(e) => {
                    error!("Batch query failed: {}", e);
                    results.push(BatchQueryResult {
                        plan_id: "error".to_string(),
                        data: vec![json!({"error": e.to_string()})],
                        execution_time_ms: 0,
                    });
                }
            }
        }
    }
    
    Ok(Json(BatchQueryResponse {
        results,
        total_execution_time_ms: total_time_ms,
        query_count: requests.len(),
    }))
}

/// Get cache statistics
async fn get_cache_stats(
    State(state): State<Arc<StreamingQueryState>>,
) -> Result<Json<serde_json::Value>> {
    let stats = state.cache.stats().await;
    
    Ok(Json(json!({
        "total_requests": stats.total_requests,
        "local_hits": stats.local_hits,
        "remote_hits": stats.remote_hits,
        "misses": stats.misses,
        "hit_rate": stats.hit_rate(),
        "local_hit_rate": stats.local_hit_rate(),
        "total_sets": stats.total_sets,
        "invalidations": stats.invalidations,
    })))
}

/// Invalidate cache entries
async fn invalidate_cache(
    State(state): State<Arc<StreamingQueryState>>,
    Json(request): Json<CacheInvalidationRequest>,
) -> Result<Json<CacheInvalidationResponse>> {
    let count = if let Some(pattern) = request.pattern {
        state.cache.invalidate_pattern(&pattern).await?
    } else if let Some(key) = request.key {
        state.cache.invalidate(&key).await?;
        1
    } else {
        state.cache.clear().await?;
        0
    };
    
    Ok(Json(CacheInvalidationResponse {
        invalidated_count: count,
        success: true,
    }))
}

/// Get performance metrics
async fn get_performance_metrics(
    State(state): State<Arc<StreamingQueryState>>,
) -> Result<Json<PerformanceMetrics>> {
    // In a real implementation, these would be tracked metrics
    Ok(Json(PerformanceMetrics {
        queries_per_second: 0.0,
        average_query_time_ms: 0.0,
        active_connections: 0,
        connection_pool_size: 0,
        cache_memory_usage_bytes: 0,
        streaming_queries_active: 0,
    }))
}

/// Create response stream with proper serialization
fn create_response_stream<T>(
    mut result_stream: impl Stream<Item = Result<Vec<T>>> + Send + Unpin + 'static,
    serializer: Arc<OptimizedSerializer>,
    format: &str,
    enable_compression: bool,
) -> impl Stream<Item = std::result::Result<bytes::Bytes, std::io::Error>>
where
    T: serde::Serialize + Send + 'static,
{
    stream::unfold(
        (result_stream, serializer),
        move |(mut stream, serializer)| async move {
            match stream.next().await {
                Some(Ok(chunk)) => {
                    // Serialize chunk
                    match serializer.serialize(&chunk) {
                        Ok(bytes) => Some((Ok(bytes), (stream, serializer))),
                        Err(e) => Some((
                            Err(std::io::Error::new(
                                std::io::ErrorKind::Other,
                                e.to_string(),
                            )),
                            (stream, serializer),
                        )),
                    }
                }
                Some(Err(e)) => Some((
                    Err(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        e.to_string(),
                    )),
                    (stream, serializer),
                )),
                None => None,
            }
        },
    )
}

/// Helper to convert request to distributed query
fn create_distributed_query(request: QueryRequest) -> Result<DistributedQuery> {
    let filter = request.filter.map(|f| QueryFilter {
        chain_ids: f.chain_ids.map(|ids| {
            ids.into_iter().map(indexer_core::types::ChainId::from).collect()
        }),
        contract_addresses: f.contract_addresses,
        contract_types: None,
        entity_ids: None,
        attributes: None,
        tags: None,
    });
    
    let pagination = request.pagination.map(|p| Pagination {
        limit: p.limit.unwrap_or(100),
        offset: p.offset.unwrap_or(0),
    });
    
    let sorting = request.sorting.map(|s| Sorting {
        field: s.field,
        direction: if s.ascending.unwrap_or(true) {
            indexer_query::SortDirection::Ascending
        } else {
            indexer_query::SortDirection::Descending
        },
    });
    
    Ok(DistributedQuery {
        base: BaseQuery {
            filter,
            pagination,
            sorting,
        },
        hints: request.hints.unwrap_or_default(),
        timeout_ms: request.timeout_ms.unwrap_or(30000),
        use_cache: request.use_cache.unwrap_or(true),
    })
}

/// Get content type for format
fn get_content_type(format: &str) -> &'static str {
    match format {
        "json" => "application/json",
        "msgpack" => "application/msgpack",
        "protobuf" => "application/x-protobuf",
        _ => "application/octet-stream",
    }
}

// Request/Response types

#[derive(Debug, Serialize)]
struct BatchQueryResponse {
    results: Vec<BatchQueryResult>,
    total_execution_time_ms: u64,
    query_count: usize,
}

#[derive(Debug, Serialize)]
struct BatchQueryResult {
    plan_id: String,
    data: Vec<serde_json::Value>,
    execution_time_ms: u64,
}

#[derive(Debug, Deserialize)]
struct CacheInvalidationRequest {
    key: Option<String>,
    pattern: Option<String>,
}

#[derive(Debug, Serialize)]
struct CacheInvalidationResponse {
    invalidated_count: usize,
    success: bool,
}

#[derive(Debug, Serialize)]
struct PerformanceMetrics {
    queries_per_second: f64,
    average_query_time_ms: f64,
    active_connections: usize,
    connection_pool_size: usize,
    cache_memory_usage_bytes: usize,
    streaming_queries_active: usize,
}

use serde_json::json;

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_create_distributed_query() {
        let request = QueryRequest {
            filter: Some(QueryFilterRequest {
                chain_ids: Some(vec!["ethereum".to_string()]),
                contract_addresses: None,
                block_range: Some((100, 200)),
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
        assert_eq!(
            query.base.filter.as_ref().unwrap().chain_ids.as_ref().unwrap()[0],
            indexer_core::types::ChainId::from("ethereum")
        );
        assert_eq!(query.base.pagination.as_ref().unwrap().limit, 50);
        assert_eq!(query.base.sorting.as_ref().unwrap().field, "block_number");
    }
}