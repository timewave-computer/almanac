/// Enhanced HTTP API server with distributed query support
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use axum::{middleware, Router};
use tower::ServiceBuilder;
use tower_http::cors::{CorsLayer, Any};
use tower_http::trace::TraceLayer;
use tracing::info;

use indexer_core::{Result, Error};
use indexer_core::service::BoxedEventService;
use indexer_core::distributed_query_planner::{DistributedQueryPlanner, QueryRoutingService};
use indexer_core::coordination::CoordinationService;

use crate::{ContractSchemaRegistry, AuthState};
use crate::http::{HttpState, start_http_server};
use crate::distributed_gateway::{
    DistributedGatewayState, NodeDiscoveryCache, DistributedRateLimiter,
    ResponseAggregator, GatewayMetrics, RateLimitConfig, create_distributed_gateway,
};
use crate::distributed_query_api::{DistributedQueryState, create_distributed_query_router};

/// Enhanced HTTP server configuration
#[derive(Debug, Clone)]
pub struct EnhancedHttpConfig {
    /// Base HTTP configuration
    pub http_addr: SocketAddr,
    /// Enable distributed query support
    pub enable_distributed: bool,
    /// Node discovery cache TTL
    pub node_cache_ttl: Duration,
    /// Rate limit configuration
    pub rate_limit: RateLimitConfig,
    /// Enable CORS
    pub enable_cors: bool,
    /// Enable request tracing
    pub enable_tracing: bool,
}

impl Default for EnhancedHttpConfig {
    fn default() -> Self {
        Self {
            http_addr: "0.0.0.0:8080".parse().unwrap(),
            enable_distributed: true,
            node_cache_ttl: Duration::from_secs(60),
            rate_limit: RateLimitConfig {
                requests_per_window: 1000,
                window_duration: Duration::from_secs(60),
                sync_threshold: 100,
            },
            enable_cors: true,
            enable_tracing: true,
        }
    }
}

/// Start the enhanced HTTP server with distributed query support
pub async fn start_enhanced_http_server(
    config: EnhancedHttpConfig,
    event_service: BoxedEventService,
    schema_registry: Arc<dyn ContractSchemaRegistry + Send + Sync>,
    auth_state: AuthState,
    coordination: Arc<dyn CoordinationService>,
) -> Result<()> {
    info!("Starting enhanced HTTP server with distributed query support on {}", config.http_addr);
    
    if config.enable_distributed {
        // Create distributed components
        let query_planner = Arc::new(DistributedQueryPlanner::new(coordination.clone()));
        let routing_service = Arc::new(QueryRoutingService::new(query_planner.clone()));
        
        // Create distributed gateway state
        let gateway_state = Arc::new(DistributedGatewayState {
            http_state: HttpState {
                event_service: event_service.clone(),
                schema_registry: schema_registry.clone(),
                auth_state: auth_state.clone(),
                rate_limiter: Arc::new(indexer_core::security::RateLimiter::new(
                    config.rate_limit.requests_per_window,
                    config.rate_limit.window_duration,
                )),
                start_time: std::time::SystemTime::now(),
            },
            query_planner: query_planner.clone(),
            routing_service,
            coordination: coordination.clone(),
            node_cache: Arc::new(tokio::sync::RwLock::new(
                NodeDiscoveryCache::new(config.node_cache_ttl)
            )),
            distributed_rate_limiter: Arc::new(DistributedRateLimiter::new(
                coordination.clone(),
                config.rate_limit.clone(),
            )),
            response_aggregator: Arc::new(ResponseAggregator::new()),
            metrics: Arc::new(tokio::sync::RwLock::new(GatewayMetrics::default())),
        });
        
        // Create distributed query state
        let query_state = Arc::new(DistributedQueryState {
            query_planner: query_planner.clone(),
            coordination: coordination.clone(),
        });
        
        // Build the complete router
        let app = build_enhanced_router(
            gateway_state.clone(),
            query_state,
            config.enable_cors,
            config.enable_tracing,
        );
        
        // Start background tasks
        start_background_tasks(gateway_state).await;
        
        // Start the server
        axum::Server::bind(&config.http_addr)
            .serve(app.into_make_service_with_connect_info::<SocketAddr>())
            .await
            .map_err(|e| Error::generic(format!("Enhanced HTTP server error: {}", e)))?;
    } else {
        // Fall back to standard HTTP server
        start_http_server(
            config.http_addr,
            event_service,
            schema_registry,
            auth_state,
        ).await?;
    }
    
    Ok(())
}

/// Build the enhanced router with all endpoints
fn build_enhanced_router(
    gateway_state: Arc<DistributedGatewayState>,
    query_state: Arc<DistributedQueryState>,
    enable_cors: bool,
    enable_tracing: bool,
) -> Router {
    // Create base HTTP state for legacy endpoints
    let http_state = gateway_state.http_state.clone();
    
    let mut app = Router::new()
        // Mount distributed gateway endpoints
        .merge(create_distributed_gateway(gateway_state))
        
        // Mount distributed query endpoints under /api/v1/distributed
        .nest("/api/v1/distributed", create_distributed_query_router(query_state))
        
        // Legacy chain endpoints
        .route("/api/v1/chains/:chain_id/status", axum::routing::get(crate::http::get_chain_status))
        
        // Legacy event endpoints
        .route("/api/v1/events/address/:chain_id/:address", axum::routing::get(crate::http::get_events_by_address))
        .route("/api/v1/events/chain/:chain_id", axum::routing::get(crate::http::get_events_by_chain))
        .route("/api/v1/events/filter", axum::routing::post(crate::http::filter_events))
        .route("/api/v1/events/:event_id", axum::routing::get(crate::http::get_event_by_id))
        
        // Legacy block endpoints
        .route("/api/v1/blocks/:chain_id/latest", axum::routing::get(crate::http::get_latest_block))
        .route("/api/v1/blocks/:chain_id/latest/:status", axum::routing::get(crate::http::get_latest_block_with_status))
        .route("/api/v1/blocks/:chain_id/:block_number", axum::routing::get(crate::http::get_block))
        
        // Authentication endpoints
        .route("/api/v1/auth/keys", axum::routing::post(crate::auth::endpoints::create_api_key))
        .route("/api/v1/auth/users", axum::routing::post(crate::auth::endpoints::create_user))
        .route("/api/v1/auth/users", axum::routing::get(crate::auth::endpoints::list_users))
        .route("/api/v1/auth/me", axum::routing::get(crate::auth::endpoints::get_current_user))
        
        // Storage proof endpoints
        .route("/api/v1/proof/generate", axum::routing::post(crate::storage_proof_api::generate_proof))
        .route("/api/v1/proof/verify", axum::routing::post(crate::storage_proof_api::verify_proof))
        .route("/api/v1/proof/batch", axum::routing::post(crate::storage_proof_api::generate_batch_proofs))
        .route("/api/v1/proof/:proof_id", axum::routing::get(crate::storage_proof_api::get_proof))
        
        // WebSocket endpoints
        .route("/api/v1/ws", axum::routing::get(crate::websocket::websocket_handler))
        .route("/api/v1/ws/stats", axum::routing::get(crate::websocket::websocket_stats))
        
        // Health and utility endpoints
        .route("/api/v1/health", axum::routing::get(crate::http::health_check))
        .route("/api/v1/version", axum::routing::get(crate::http::get_version))
        
        // Apply rate limiting middleware
        .layer(middleware::from_fn_with_state(http_state.clone(), crate::http::rate_limit_middleware))
        .with_state(http_state);
    
    // Apply additional middleware
    let mut service_builder = ServiceBuilder::new();
    
    if enable_tracing {
        service_builder = service_builder.layer(TraceLayer::new_for_http());
    }
    
    if enable_cors {
        service_builder = service_builder.layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any)
        );
    }
    
    app.layer(service_builder)
}

/// Start background tasks for the distributed gateway
async fn start_background_tasks(gateway_state: Arc<DistributedGatewayState>) {
    // Start node discovery refresh task
    let state = gateway_state.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(30));
        loop {
            interval.tick().await;
            if let Err(e) = refresh_node_cache(&state).await {
                tracing::warn!("Failed to refresh node cache: {}", e);
            }
        }
    });
    
    // Start metrics aggregation task
    let state = gateway_state.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(60));
        loop {
            interval.tick().await;
            if let Err(e) = aggregate_metrics(&state).await {
                tracing::warn!("Failed to aggregate metrics: {}", e);
            }
        }
    });
    
    // Start health check task
    let state = gateway_state.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(15));
        loop {
            interval.tick().await;
            if let Err(e) = perform_health_checks(&state).await {
                tracing::warn!("Health check failed: {}", e);
            }
        }
    });
}

/// Refresh the node discovery cache
async fn refresh_node_cache(state: &DistributedGatewayState) -> Result<()> {
    let nodes = state.coordination.get_active_nodes().await?;
    let mut cache = state.node_cache.write().await;
    cache.update(nodes);
    Ok(())
}

/// Aggregate metrics from all nodes
async fn aggregate_metrics(state: &DistributedGatewayState) -> Result<()> {
    // In a real implementation, this would collect metrics from all nodes
    let mut metrics = state.metrics.write().await;
    
    // Log current metrics
    tracing::info!(
        "Gateway metrics - Requests: {}, Distributed queries: {}, Cache hit rate: {:.2}%, Avg query time: {:.2}ms",
        metrics.total_requests,
        metrics.distributed_queries,
        if metrics.cache_hits + metrics.cache_misses > 0 {
            (metrics.cache_hits as f64 / (metrics.cache_hits + metrics.cache_misses) as f64) * 100.0
        } else {
            0.0
        },
        metrics.avg_query_time_ms
    );
    
    Ok(())
}

/// Perform health checks on all nodes
async fn perform_health_checks(state: &DistributedGatewayState) -> Result<()> {
    let cache = state.node_cache.read().await;
    let mut unhealthy_nodes = Vec::new();
    
    for (node_id, node) in &cache.nodes {
        let health_score = cache.health_scores.get(node_id).copied().unwrap_or(0.0);
        if health_score < 0.5 {
            unhealthy_nodes.push((node_id.clone(), health_score));
        }
    }
    
    if !unhealthy_nodes.is_empty() {
        tracing::warn!(
            "Unhealthy nodes detected: {:?}",
            unhealthy_nodes
        );
    }
    
    Ok(())
}

/// Create a middleware stack for distributed queries
pub fn create_distributed_middleware() -> tower::ServiceBuilder<
    tower::layer::util::Stack<
        TraceLayer,
        tower::layer::util::Stack<
            CorsLayer,
            tower::layer::util::Identity,
        >,
    >,
> {
    ServiceBuilder::new()
        .layer(TraceLayer::new_for_http())
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any)
        )
}

#[cfg(test)]
mod tests {
    use super::*;
    use indexer_core::coordination::NodeStatus;
    
    #[test]
    fn test_enhanced_config_default() {
        let config = EnhancedHttpConfig::default();
        assert!(config.enable_distributed);
        assert_eq!(config.node_cache_ttl, Duration::from_secs(60));
        assert_eq!(config.rate_limit.requests_per_window, 1000);
    }
    
    #[tokio::test]
    async fn test_node_cache_refresh() {
        // Would test node cache refresh logic
    }
}