/// REST API implementation for the indexer
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use axum::{
    extract::{Path, Query, State, ConnectInfo},
    http::{StatusCode, HeaderMap, header},
    middleware::{self, Next},
    response::{IntoResponse, Json, Response},
    routing::{get, post},
    Router,
};
use axum::http::Request;
use base64::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::net::TcpListener;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use indexer_core::{Error, Result, BlockStatus};
use indexer_core::event::Event;
use indexer_core::service::BoxedEventService;
use indexer_core::types::{ChainId, EventFilter as CoreEventFilter};
use indexer_core::security::{RateLimiter, ConnectionManager};
use crate::{ContractSchemaRegistry, auth::{AuthState, OptionalUser}};

/// HTTP server state
#[derive(Clone)]
pub struct HttpState {
    /// Event service
    pub event_service: BoxedEventService,
    /// Schema registry
    pub schema_registry: Arc<dyn ContractSchemaRegistry + Send + Sync>,
    /// Authentication state
    pub auth_state: AuthState,
    /// Rate limiter
    pub rate_limiter: Arc<RateLimiter>,
    /// Server start time for uptime calculation
    pub start_time: SystemTime,
}

impl AsRef<HttpState> for HttpState {
    fn as_ref(&self) -> &HttpState {
        self
    }
}

/// Chain status response
#[derive(Debug, Serialize)]
pub struct ChainStatusResponse {
    pub chain_id: String,
    pub latest_height: u64,
    pub finalized_height: Option<u64>,
    pub is_indexing: bool,
    pub last_indexed_at: u64,
    pub error: Option<String>,
}

/// Event response for REST API
#[derive(Debug, Serialize)]
pub struct EventResponse {
    pub chain_id: String,
    pub height: u64,
    pub tx_hash: String,
    pub index: u32,
    pub address: String,
    pub event_type: String,
    pub attributes: HashMap<String, Value>,
    pub raw_data: String, // Base64 encoded
    pub timestamp: u64,
}

/// Events list response with pagination
#[derive(Debug, Serialize)]
pub struct EventsResponse {
    pub events: Vec<EventResponse>,
    pub pagination: PaginationInfo,
}

/// Pagination information
#[derive(Debug, Serialize)]
pub struct PaginationInfo {
    pub total: Option<u64>,
    pub limit: usize,
    pub offset: usize,
    pub has_more: bool,
}

/// Block information response
#[derive(Debug, Serialize)]
pub struct BlockResponse {
    pub chain_id: String,
    pub number: u64,
    pub hash: String,
    pub timestamp: u64,
    pub finality_status: String,
}

/// Health check response
#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub healthy: bool,
    pub version: String,
    pub uptime_seconds: u64,
    pub latest_blocks: HashMap<String, BlockInfo>,
}

/// Block information for health check
#[derive(Debug, Serialize)]
pub struct BlockInfo {
    pub block_number: u64,
    pub block_hash: String,
    pub timestamp: u64,
    pub finality_status: String,
}

/// Event filter request for POST endpoints
#[derive(Debug, Deserialize)]
pub struct EventFilterRequest {
    pub chain_id: Option<String>,
    pub address: Option<String>,
    pub event_type: Option<String>,
    pub attributes: Option<HashMap<String, Value>>,
    pub from_height: Option<u64>,
    pub to_height: Option<u64>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

/// Query parameters for GET endpoints
#[derive(Debug, Deserialize)]
pub struct EventsQuery {
    pub limit: Option<usize>,
    pub offset: Option<usize>,
    pub ascending: Option<bool>,
    pub from_height: Option<u64>,
    pub to_height: Option<u64>,
    pub event_type: Option<String>,
}

/// REST API errors
#[derive(Debug)]
pub enum ApiError {
    NotFound,
    BadRequest(String),
    InternalError(String),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        let (status, error_message) = match &self {
            ApiError::NotFound => (StatusCode::NOT_FOUND, "Not found".to_string()),
            ApiError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg.clone()),
            ApiError::InternalError(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg.clone()),
        };

        let body = Json(json!({
            "error": error_message,
            "status": status.as_u16()
        }));

        (status, body).into_response()
    }
}

impl From<Error> for ApiError {
    fn from(err: Error) -> Self {
        match err {
            Error::Generic(msg) => ApiError::InternalError(msg),
            Error::Config(msg) => ApiError::BadRequest(msg),
            _ => ApiError::InternalError(format!("Internal error: {}", err)),
        }
    }
}

/// Convert core event to API response
fn event_to_response(event: &dyn Event) -> EventResponse {
    let mut attributes = HashMap::new();
    
    // Extract basic attributes from the event
    // This is a simplified implementation - in practice, you'd want to 
    // properly parse event-specific attributes
    
    EventResponse {
        chain_id: event.chain().to_string(),
        height: event.block_number(),
        tx_hash: event.tx_hash().to_string(),
        index: 0, // Events don't currently expose index
        address: "0x0000000000000000000000000000000000000000".to_string(), // Placeholder
        event_type: event.event_type().to_string(),
        attributes,
        raw_data: BASE64_STANDARD.encode(event.raw_data()),
        timestamp: event.timestamp()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
    }
}

/// Convert API filter to core filter
impl From<EventFilterRequest> for CoreEventFilter {
    fn from(filter: EventFilterRequest) -> Self {
        let mut core_filter = CoreEventFilter::new();
        
        core_filter.chain_id = filter.chain_id.as_ref().map(|id| ChainId(id.clone()));
        core_filter.chain = filter.chain_id;
        
        if let (Some(from), Some(to)) = (filter.from_height, filter.to_height) {
            core_filter.block_range = Some((from, to));
        }
        
        if let Some(event_type) = filter.event_type {
            core_filter.event_types = Some(vec![event_type]);
        }
        
        // Convert attributes to custom_filters
        if let Some(attributes) = filter.attributes {
            for (key, value) in attributes {
                if let Some(str_value) = value.as_str() {
                    core_filter.custom_filters.insert(key, str_value.to_string());
                }
            }
        }
        
        // Add address as a custom filter if provided
        if let Some(address) = filter.address {
            core_filter.custom_filters.insert("address".to_string(), address);
        }
        
        core_filter.limit = filter.limit;
        core_filter.offset = filter.offset;
        
        core_filter
    }
}

/// Rate limiting middleware
async fn rate_limit_middleware<B>(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    State(state): State<HttpState>,
    request: Request<B>,
    next: Next<B>,
) -> std::result::Result<Response, StatusCode> {
    let client_ip = addr.ip().to_string();
    
    // Check rate limit
    if !state.rate_limiter.is_allowed(&client_ip).await {
        // Add rate limit headers
        let mut response = Json(json!({
            "error": "Rate limit exceeded",
            "status": 429
        })).into_response();
        
        let headers = response.headers_mut();
        headers.insert("X-RateLimit-Limit", "1000".parse().unwrap());
        headers.insert("X-RateLimit-Remaining", "0".parse().unwrap());
        
        *response.status_mut() = StatusCode::TOO_MANY_REQUESTS;
        return Ok(response);
    }
    
    // Process request
    let mut response = next.run(request).await;
    
    // Add rate limit headers to successful responses
    let headers = response.headers_mut();
    headers.insert("X-RateLimit-Limit", "1000".parse().unwrap());
    // TODO: Calculate remaining requests more accurately
    headers.insert("X-RateLimit-Remaining", "999".parse().unwrap());
    
    Ok(response)
}

/// Start the HTTP REST API server
pub async fn start_http_server(
    addr: SocketAddr,
    event_service: BoxedEventService,
    schema_registry: Arc<dyn ContractSchemaRegistry + Send + Sync>,
    auth_state: AuthState,
) -> Result<()> {
    let state = HttpState {
        event_service,
        schema_registry,
        auth_state,
        rate_limiter: Arc::new(RateLimiter::new(1000, std::time::Duration::from_secs(60))), // 1000 requests per minute
        start_time: SystemTime::now(),
    };

    let app = Router::new()
        // Chain endpoints
        .route("/api/v1/chains/:chain_id/status", get(get_chain_status))
        
        // Event endpoints
        .route("/api/v1/events/address/:chain_id/:address", get(get_events_by_address))
        .route("/api/v1/events/chain/:chain_id", get(get_events_by_chain))
        .route("/api/v1/events/filter", post(filter_events))
        .route("/api/v1/events/:event_id", get(get_event_by_id))
        
        // Block endpoints
        .route("/api/v1/blocks/:chain_id/latest", get(get_latest_block))
        .route("/api/v1/blocks/:chain_id/latest/:status", get(get_latest_block_with_status))
        .route("/api/v1/blocks/:chain_id/:block_number", get(get_block))
        
        // Authentication endpoints
        .route("/api/v1/auth/keys", post(crate::auth::endpoints::create_api_key))
        .route("/api/v1/auth/users", post(crate::auth::endpoints::create_user))
        .route("/api/v1/auth/users", get(crate::auth::endpoints::list_users))
        .route("/api/v1/auth/me", get(crate::auth::endpoints::get_current_user))
        
        // WebSocket endpoints
        .route("/api/v1/ws", get(crate::websocket::websocket_handler))
        .route("/api/v1/ws/stats", get(crate::websocket::websocket_stats))
        
        // Health and utility endpoints
        .route("/api/v1/health", get(health_check))
        .route("/api/v1/version", get(get_version))
        
        .layer(middleware::from_fn_with_state(state.clone(), rate_limit_middleware))
        .with_state(state);

    info!("Starting HTTP REST API server on {}", addr);
    
    axum::Server::bind(&addr)
        .serve(app.into_make_service_with_connect_info::<SocketAddr>())
        .await
        .map_err(|e| Error::generic(&format!("HTTP server error: {}", e)))?;

    Ok(())
}

/// GET /api/v1/chains/{chain_id}/status
async fn get_chain_status(
    State(state): State<HttpState>,
    Path(chain_id): Path<String>,
) -> std::result::Result<Json<ChainStatusResponse>, ApiError> {
    debug!("Getting status for chain: {}", chain_id);
    
    // Get latest block from the event service
    let latest_height = state.event_service.get_latest_block().await
        .map_err(|e| ApiError::InternalError(format!("Failed to get latest block: {}", e)))?;
    
    // Get finalized block (if supported)
    let finalized_height = state.event_service
        .get_latest_block_with_status(&chain_id, BlockStatus::Finalized).await
        .ok();
    
    let response = ChainStatusResponse {
        chain_id,
        latest_height,
        finalized_height,
        is_indexing: true, // Simplified - would check actual indexing status
        last_indexed_at: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
        error: None,
    };
    
    Ok(Json(response))
}

/// GET /api/v1/events/address/{chain_id}/{address}
async fn get_events_by_address(
    State(state): State<HttpState>,
    Path((chain_id, address)): Path<(String, String)>,
    Query(params): Query<EventsQuery>,
) -> std::result::Result<Json<EventsResponse>, ApiError> {
    debug!("Getting events for address {} on chain {}", address, chain_id);
    
    let limit = params.limit.unwrap_or(100).min(1000); // Cap at 1000
    let offset = params.offset.unwrap_or(0);
    
    // Create filter for this address
    let mut filter = CoreEventFilter::new();
    filter.chain_id = Some(ChainId(chain_id.clone()));
    filter.chain = Some(chain_id);
    filter.custom_filters.insert("address".to_string(), address);
    
    if let Some(event_type) = params.event_type {
        filter.event_types = Some(vec![event_type]);
    }
    
    if let (Some(from), Some(to)) = (params.from_height, params.to_height) {
        filter.block_range = Some((from, to));
    }
    
    // Get events from the service
    let events = state.event_service.get_events(vec![filter]).await
        .map_err(|e| ApiError::InternalError(format!("Failed to get events: {}", e)))?;
    
    // Convert to API responses
    let event_responses: Vec<EventResponse> = events.iter()
        .skip(offset)
        .take(limit)
        .map(|e| event_to_response(e.as_ref()))
        .collect();
    
    let has_more = events.len() > offset + limit;
    
    let response = EventsResponse {
        events: event_responses,
        pagination: PaginationInfo {
            total: Some(events.len() as u64),
            limit,
            offset,
            has_more,
        },
    };
    
    Ok(Json(response))
}

/// GET /api/v1/events/chain/{chain_id}
async fn get_events_by_chain(
    State(state): State<HttpState>,
    Path(chain_id): Path<String>,
    Query(params): Query<EventsQuery>,
) -> std::result::Result<Json<EventsResponse>, ApiError> {
    debug!("Getting events for chain: {}", chain_id);
    
    let limit = params.limit.unwrap_or(100).min(1000);
    let offset = params.offset.unwrap_or(0);
    
    // Create filter for this chain
    let mut filter = CoreEventFilter::new();
    filter.chain_id = Some(ChainId(chain_id.clone()));
    filter.chain = Some(chain_id);
    
    if let Some(event_type) = params.event_type {
        filter.event_types = Some(vec![event_type]);
    }
    
    if let (Some(from), Some(to)) = (params.from_height, params.to_height) {
        filter.block_range = Some((from, to));
    }
    
    // Get events from the service
    let events = state.event_service.get_events(vec![filter]).await
        .map_err(|e| ApiError::InternalError(format!("Failed to get events: {}", e)))?;
    
    // Convert to API responses
    let event_responses: Vec<EventResponse> = events.iter()
        .skip(offset)
        .take(limit)
        .map(|e| event_to_response(e.as_ref()))
        .collect();
    
    let has_more = events.len() > offset + limit;
    
    let response = EventsResponse {
        events: event_responses,
        pagination: PaginationInfo {
            total: Some(events.len() as u64),
            limit,
            offset,
            has_more,
        },
    };
    
    Ok(Json(response))
}

/// POST /api/v1/events/filter
async fn filter_events(
    State(state): State<HttpState>,
    Json(filter_req): Json<EventFilterRequest>,
) -> std::result::Result<Json<EventsResponse>, ApiError> {
    debug!("Filtering events with custom filter");
    
    let limit = filter_req.limit.unwrap_or(100).min(1000);
    let offset = filter_req.offset.unwrap_or(0);
    
    // Convert to core filter
    let filter: CoreEventFilter = filter_req.into();
    
    // Get events from the service
    let events = state.event_service.get_events(vec![filter]).await
        .map_err(|e| ApiError::InternalError(format!("Failed to get events: {}", e)))?;
    
    // Convert to API responses
    let event_responses: Vec<EventResponse> = events.iter()
        .skip(offset)
        .take(limit)
        .map(|e| event_to_response(e.as_ref()))
        .collect();
    
    let has_more = events.len() > offset + limit;
    
    let response = EventsResponse {
        events: event_responses,
        pagination: PaginationInfo {
            total: Some(events.len() as u64),
            limit,
            offset,
            has_more,
        },
    };
    
    Ok(Json(response))
}

/// GET /api/v1/events/{event_id}
async fn get_event_by_id(
    State(_state): State<HttpState>,
    Path(_event_id): Path<String>,
) -> std::result::Result<Json<EventResponse>, ApiError> {
    // This would require additional indexing by event ID
    // For now, return not found
    Err(ApiError::NotFound)
}

/// GET /api/v1/blocks/{chain_id}/latest
async fn get_latest_block(
    State(state): State<HttpState>,
    Path(chain_id): Path<String>,
) -> std::result::Result<Json<BlockResponse>, ApiError> {
    debug!("Getting latest block for chain: {}", chain_id);
    
    let block_number = state.event_service.get_latest_block().await
        .map_err(|e| ApiError::InternalError(format!("Failed to get latest block: {}", e)))?;
    
    let response = BlockResponse {
        chain_id,
        number: block_number,
        hash: format!("0x{:x}", block_number), // Placeholder
        timestamp: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
        finality_status: "confirmed".to_string(),
    };
    
    Ok(Json(response))
}

/// GET /api/v1/blocks/{chain_id}/latest/{status}
async fn get_latest_block_with_status(
    State(state): State<HttpState>,
    Path((chain_id, status_str)): Path<(String, String)>,
) -> std::result::Result<Json<BlockResponse>, ApiError> {
    debug!("Getting latest {} block for chain: {}", status_str, chain_id);
    
    let status = match status_str.as_str() {
        "confirmed" => BlockStatus::Confirmed,
        "safe" => BlockStatus::Safe,
        "justified" => BlockStatus::Justified,
        "finalized" => BlockStatus::Finalized,
        _ => return Err(ApiError::BadRequest("Invalid block status".to_string())),
    };
    
    let block_number = state.event_service
        .get_latest_block_with_status(&chain_id, status).await
        .map_err(|e| ApiError::InternalError(format!("Failed to get latest block: {}", e)))?;
    
    let response = BlockResponse {
        chain_id,
        number: block_number,
        hash: format!("0x{:x}", block_number), // Placeholder
        timestamp: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
        finality_status: status_str,
    };
    
    Ok(Json(response))
}

/// GET /api/v1/blocks/{chain_id}/{block_number}
async fn get_block(
    State(_state): State<HttpState>,
    Path((_chain_id, _block_number)): Path<(String, u64)>,
) -> std::result::Result<Json<BlockResponse>, ApiError> {
    // This would require block-specific storage
    // For now, return not found
    Err(ApiError::NotFound)
}

/// GET /api/v1/health
async fn health_check(
    State(state): State<HttpState>,
) -> std::result::Result<Json<HealthResponse>, ApiError> {
    let uptime = state.start_time
        .elapsed()
        .unwrap_or_default()
        .as_secs();
    
    // Get latest block info for health check
    let mut latest_blocks = HashMap::new();
    
    if let Ok(latest_height) = state.event_service.get_latest_block().await {
        let chain_id = state.event_service.chain_id().0.clone();
        latest_blocks.insert(chain_id, BlockInfo {
            block_number: latest_height,
            block_hash: format!("0x{:x}", latest_height),
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            finality_status: "confirmed".to_string(),
        });
    }
    
    let response = HealthResponse {
        healthy: true,
        version: env!("CARGO_PKG_VERSION").to_string(),
        uptime_seconds: uptime,
        latest_blocks,
    };
    
    Ok(Json(response))
}

/// GET /api/v1/version
async fn get_version() -> Json<Value> {
    Json(json!({
        "version": env!("CARGO_PKG_VERSION"),
        "git_commit": option_env!("GIT_COMMIT").unwrap_or("unknown"),
        "build_time": option_env!("BUILD_TIME").unwrap_or("unknown")
    }))
} 