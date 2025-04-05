/// HTTP API server implementation
use std::net::SocketAddr;
use std::collections::HashMap;
use std::time::UNIX_EPOCH;
use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Json, Response},
    routing::{get, post},
    Router, Server,
};
use base64::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::net::TcpListener;
use tracing::{debug, error, info, warn};
use tokio::sync::Mutex;
use chrono::{DateTime, Utc};

use indexer_common::{Error, Result, BlockStatus};
use indexer_core::event::Event;
use indexer_core::service::BoxedEventService;
// Import the core EventFilter directly to avoid name conflicts
use indexer_core::types::EventFilter as CoreEventFilter;

// Import FinalityStatus for the API
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FinalityStatus {
    Confirmed,
    Safe, 
    Justified,
    Finalized,
}

/// State for the HTTP server
#[derive(Clone)]
pub struct HttpServerState {
    /// Event service
    event_service: BoxedEventService,
    /// RocksDB store from core
    rocks_store: CoreRocksDBStore,
    /// PostgreSQL store from core
    pg_store: CorePostgresStore,
}

// Mock storage types to fix imports
#[derive(Clone)]
pub struct CoreRocksDBStore;

#[derive(Clone)]
pub struct CorePostgresStore;

impl Default for CoreRocksDBStore {
    fn default() -> Self {
        Self
    }
}

impl Default for CorePostgresStore {
    fn default() -> Self {
        Self
    }
}

impl CoreRocksDBStore {
    pub async fn get_latest_block_number(&self, chain: &str) -> Result<Option<u64>> {
        // Mock implementation
        Ok(Some(1000))
    }
    
    pub async fn get_latest_block_with_status(&self, chain: &str, status: BlockStatus) -> Result<Option<u64>> {
        // Mock implementation
        Ok(Some(1000))
    }
    
    pub async fn get_block(&self, chain: &str, block_num: u64) -> Result<Option<Block>> {
        // Mock implementation
        Ok(Some(Block {
            number: block_num,
            hash: format!("0x{:x}", block_num),
            timestamp: chrono::Utc::now().timestamp(),
            finality_status: FinalityStatus::Confirmed,
        }))
    }
}

impl CorePostgresStore {
    pub async fn get_events(
        &self,
        chain: Option<&str>,
        block_range: Option<(u64, u64)>,
        event_types: Option<&[String]>,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<Box<dyn Event>>> {
        // Mock implementation
        Ok(Vec::new())
    }
    
    pub async fn get_event_by_id(&self, id: &str) -> Result<Option<Box<dyn Event>>> {
        // Mock implementation
        Ok(None)
    }
    
    pub async fn get_events_by_block(&self, chain: &str, block_number: u64) -> Result<Vec<Box<dyn Event>>> {
        // Mock implementation
        Ok(Vec::new())
    }
}

/// Block information
#[derive(Debug, Clone)]
pub struct Block {
    pub number: u64,
    pub hash: String,
    pub timestamp: i64,
    pub finality_status: FinalityStatus,
}

/// Response for event data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventResponse {
    /// Event ID
    pub id: String,
    
    /// Chain ID
    pub chain: String,
    
    /// Block number
    pub block_number: u64,
    
    /// Block hash
    pub block_hash: String,
    
    /// Transaction hash
    pub tx_hash: String,
    
    /// Timestamp
    pub timestamp: u64,
    
    /// Event type
    pub event_type: String,
    
    /// Event data as base64
    pub data: String,
}

/// Request for event filters
#[derive(Debug, Clone, Deserialize, Default)]
pub struct EventFilterRequest {
    /// Chain ID
    pub chain: Option<String>,
    
    /// Block range
    pub block_range: Option<(u64, u64)>,
    
    /// Time range
    pub time_range: Option<(u64, u64)>,
    
    /// Event types
    pub event_types: Option<Vec<String>>,
    
    /// Limit
    pub limit: Option<usize>,
    
    /// Offset
    pub offset: Option<usize>,
    
    /// Custom filters
    #[serde(default)]
    pub custom_filters: HashMap<String, String>,
}

/// API server state
#[derive(Clone)]
pub struct ApiState {
    pub rocks_store: CoreRocksDBStore,
    pub pg_store: CorePostgresStore,
}

/// HTTP server for the REST API
pub struct HttpServer {
    state: Arc<ApiState>,
}

impl HttpServer {
    /// Create a new HTTP server
    pub async fn new(rocks_store: CoreRocksDBStore, pg_store: CorePostgresStore) -> Self {
        Self {
            state: Arc::new(ApiState {
                rocks_store,
                pg_store,
            }),
        }
    }

    /// Start the HTTP server
    pub async fn start(self, addr: &str) -> Result<()> {
        let addr: SocketAddr = addr.parse()
            .map_err(|e| Error::api(format!("Invalid server address: {}", e)))?;

        // Create a router with the API routes
        let app = Router::new()
            .route("/events", get(get_events).post(get_events))
            .route("/events/:id", get(get_event_by_id))
            .route("/blocks/latest", get(get_latest_block))
            .route("/blocks/latest/:status", get(get_latest_block_with_status))
            .route("/blocks/:number/events", get(get_events_by_block))
            .route("/health", get(health_check))
            .with_state(self.state);
        
        // Create server
        let listener = TcpListener::bind(&addr).await
            .map_err(|e| Error::api(format!("Failed to bind to address: {}", e)))?;
        
        info!("API server listening on {}", addr);
        
        // Start server
        axum::Server::from_tcp(listener.into_std().unwrap())
            .map_err(|e| Error::api(format!("Failed to create server: {}", e)))?
            .serve(app.into_make_service())
            .await
            .map_err(|e| Error::api(format!("Server error: {}", e)))?;
        
        Ok(())
    }
}

/// HTTP filter for querying events (API version)
#[derive(Debug, Deserialize)]
pub struct ApiEventFilter {
    pub chain: Option<String>,
    pub block_range: Option<[u64; 2]>,
    pub time_range: Option<[u64; 2]>,
    pub event_types: Option<Vec<String>>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
    #[serde(default)]
    pub custom_filters: HashMap<String, String>,
}

// Convert API EventFilter to core EventFilter
impl From<ApiEventFilter> for CoreEventFilter {
    fn from(filter: ApiEventFilter) -> Self {
        let mut core_filter = Self::new();
        core_filter.chain = filter.chain;
        
        // Convert block range from [u64; 2] to (u64, u64)
        if let Some([start, end]) = filter.block_range {
            core_filter.block_range = Some((start, end));
        }
        
        // Convert time range from [u64; 2] to (u64, u64)
        if let Some([start, end]) = filter.time_range {
            core_filter.time_range = Some((start, end));
        }
        
        core_filter.event_types = filter.event_types;
        core_filter.limit = filter.limit;
        core_filter.offset = filter.offset;
        core_filter.custom_filters = filter.custom_filters;
        
        core_filter
    }
}

/// Health response type
#[derive(Debug, Serialize)]
struct HealthResponse {
    healthy: bool,
    version: String,
    uptime_seconds: u64,
    latest_blocks: HashMap<String, BlockInfo>,
}

#[derive(Debug, Serialize)]
struct BlockInfo {
    block_number: u64,
    block_hash: String,
    timestamp: i64,
    finality_status: String,
}

/// API Error type
#[derive(Debug)]
enum ApiError {
    NotFound,
    BadRequest(String),
    InternalError(String),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            ApiError::NotFound => (StatusCode::NOT_FOUND, "Resource not found".to_string()),
            ApiError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg),
            ApiError::InternalError(msg) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Internal server error: {}", msg),
            ),
        };

        (status, Json(json!({ "error": message }))).into_response()
    }
}

/// Start the HTTP server
pub async fn start_http_server(
    addr: SocketAddr,
    event_service: BoxedEventService,
) -> Result<()> {
    // In a real implementation, you would initialize the stores here
    let rocks_store = CoreRocksDBStore::default();
    let pg_store = CorePostgresStore::default();
    
    let state = HttpServerState { 
        event_service,
        rocks_store,
        pg_store,
    };
    
    // Create router with routes
    let app = Router::new()
        .route("/events", post(get_old_events))
        .route("/events/:id", get(get_old_event))
        .route("/blocks/latest", get(get_old_latest_block))
        .route("/blocks/latest/:status", get(get_old_latest_block_with_status))
        .route("/blocks/:number/events", get(get_old_events_by_block))
        .with_state(state);

    info!("Starting HTTP server on {}", addr);
    
    // Bind to address
    let listener = TcpListener::bind(addr).await
        .map_err(|e| Error::api(format!("Failed to bind to address: {}", e)))?;
    
    // Create server from TcpListener
    let server = axum::Server::from_tcp(listener.into_std().unwrap())
        .map_err(|e| Error::api(format!("Failed to create server: {}", e)))?;
        
    // Start server
    server
        .serve(app.into_make_service())
        .await
        .map_err(|e| Error::api(format!("Server error: {}", e)))?;
    
    Ok(())
}

/// Get events based on filters using the old API
async fn get_old_events(
    State(state): State<HttpServerState>,
    Json(filter_req): Json<EventFilterRequest>,
) -> impl IntoResponse {
    let mut filter = CoreEventFilter::new();
    filter.chain = filter_req.chain;
    
    // Convert tuple to tuple
    filter.block_range = filter_req.block_range;
    filter.time_range = filter_req.time_range;
    
    filter.event_types = filter_req.event_types;
    filter.limit = filter_req.limit;
    filter.offset = filter_req.offset;
    filter.custom_filters = filter_req.custom_filters;
    
    match state.event_service.get_events(vec![filter]).await {
        Ok(events) => {
            let responses: Vec<EventResponse> = events.into_iter()
                .map(|event| event_to_response(&*event))
                .collect();
            (StatusCode::OK, Json(responses)).into_response()
        }
        Err(e) => {
            (error_to_status_code(&e), Json(e.to_string())).into_response()
        }
    }
}

/// Get a specific event by ID using the old API
async fn get_old_event(
    State(state): State<HttpServerState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    // Use a filter to find the event by ID
    let mut filter = CoreEventFilter::new();
    filter.custom_filters.insert("id".to_string(), id);
    filter.limit = Some(1);
    
    match state.event_service.get_events(vec![filter]).await {
        Ok(events) if !events.is_empty() => {
            (StatusCode::OK, Json(event_to_response(&*events[0]))).into_response()
        }
        Ok(_) => {
            (StatusCode::NOT_FOUND, Json("Event not found".to_string())).into_response()
        }
        Err(e) => {
            (error_to_status_code(&e), Json(e.to_string())).into_response()
        }
    }
}

/// Get the latest block number using the old API
async fn get_old_latest_block(
    State(state): State<HttpServerState>,
) -> impl IntoResponse {
    match state.event_service.get_latest_block().await {
        Ok(block) => (StatusCode::OK, Json(block)).into_response(),
        Err(e) => (error_to_status_code(&e), Json(e.to_string())).into_response(),
    }
}

/// Get the latest block number with a specific status using the old API
async fn get_old_latest_block_with_status(
    State(state): State<HttpServerState>,
    Path(status_str): Path<String>,
) -> impl IntoResponse {
    let status = match status_str.as_str() {
        "confirmed" => BlockStatus::Confirmed,
        "safe" => BlockStatus::Safe,
        "justified" => BlockStatus::Justified,
        "finalized" => BlockStatus::Finalized,
        _ => return (StatusCode::BAD_REQUEST, Json("Invalid block status".to_string())).into_response(),
    };
    
    match state.event_service.get_latest_block_with_status("", status).await {
        Ok(block) => (StatusCode::OK, Json(block)).into_response(),
        Err(e) => (error_to_status_code(&e), Json(e.to_string())).into_response(),
    }
}

/// Get events for a specific block using the old API
async fn get_old_events_by_block(
    State(state): State<HttpServerState>,
    Path(block_number): Path<u64>,
) -> impl IntoResponse {
    let mut filter = CoreEventFilter::new();
    filter.block_range = Some((block_number, block_number));
    
    match state.event_service.get_events(vec![filter]).await {
        Ok(events) => {
            let responses: Vec<EventResponse> = events.into_iter()
                .map(|event| event_to_response(&*event))
                .collect();
            (StatusCode::OK, Json(responses)).into_response()
        }
        Err(e) => {
            (error_to_status_code(&e), Json(e.to_string())).into_response()
        }
    }
}

/// Get events matching the specified filters
async fn get_events(
    State(state): State<Arc<ApiState>>,
    Query(params): Query<HashMap<String, String>>,
    Json(filter): Json<Option<EventFilterRequest>>,
) -> std::result::Result<impl IntoResponse, ApiError> {
    let filter = filter.unwrap_or_default();
    let limit = filter.limit.unwrap_or(100);
    let offset = filter.offset.unwrap_or(0);
    
    debug!("Getting events with filters: {:?}", filter);
    
    // Perform query with pagination
    match state.pg_store.get_events(
        filter.chain.as_deref(),
        filter.block_range,
        filter.event_types.as_deref(),
        limit,
        offset,
    ).await {
        Ok(events) => {
            // Convert events to response format
            let responses: Vec<EventResponse> = events.into_iter()
                .map(|event| event_to_response(&*event))
                .collect();
                
            Ok((StatusCode::OK, Json(responses)))
        },
        Err(e) => Err(ApiError::InternalError(e.to_string())),
    }
}

/// Get a specific event by ID
async fn get_event_by_id(
    State(state): State<Arc<ApiState>>,
    Path(id): Path<String>,
) -> std::result::Result<impl IntoResponse, ApiError> {
    debug!("Getting event with ID: {}", id);
    
    // Query the event by ID
    match state.pg_store.get_event_by_id(&id).await {
        Ok(Some(event)) => {
            // Convert to response format
            let response = event_to_response(&*event);
            Ok((StatusCode::OK, Json(response)))
        },
        Ok(None) => Err(ApiError::NotFound),
        Err(e) => Err(ApiError::InternalError(e.to_string())),
    }
}

/// Get the latest block number
async fn get_latest_block(
    State(state): State<Arc<ApiState>>,
    Query(params): Query<HashMap<String, String>>,
) -> std::result::Result<impl IntoResponse, ApiError> {
    let chain = params.get("chain").cloned().unwrap_or_else(|| "ethereum".to_string());
    debug!("Getting latest block for chain: {}", chain);

    // Query the latest block from RocksDB for better performance
    match state.rocks_store.get_latest_block_number(&chain).await {
        Ok(Some(latest_block)) => Ok((StatusCode::OK, Json(latest_block))),
        Ok(None) => Err(ApiError::NotFound),
        Err(e) => Err(ApiError::InternalError(e.to_string())),
    }
}

/// Get the latest block number with a specific finality status
async fn get_latest_block_with_status(
    State(state): State<Arc<ApiState>>,
    Path(status): Path<String>,
    Query(params): Query<HashMap<String, String>>,
) -> std::result::Result<impl IntoResponse, ApiError> {
    let chain = params.get("chain").cloned().unwrap_or_else(|| "ethereum".to_string());
    
    // Parse the finality status
    let block_status = match status.as_str() {
        "confirmed" => BlockStatus::Confirmed,
        "safe" => BlockStatus::Safe,
        "justified" => BlockStatus::Justified,
        "finalized" => BlockStatus::Finalized,
        _ => return Err(ApiError::BadRequest(format!("Invalid finality status: {}", status))),
    };
    
    debug!("Getting latest {} block for chain: {}", status, chain);

    // Query the latest block with the given finality status
    match state.rocks_store.get_latest_block_with_status(&chain, block_status).await {
        Ok(Some(latest_block)) => Ok((StatusCode::OK, Json(latest_block))),
        Ok(None) => Err(ApiError::NotFound),
        Err(e) => Err(ApiError::InternalError(e.to_string())),
    }
}

/// Get events for a specific block
async fn get_events_by_block(
    State(state): State<Arc<ApiState>>,
    Path(number): Path<u64>,
    Query(params): Query<HashMap<String, String>>,
) -> std::result::Result<impl IntoResponse, ApiError> {
    let chain = params.get("chain").cloned().unwrap_or_else(|| "ethereum".to_string());
    debug!("Getting events for block {} on chain {}", number, chain);

    // Query events for the specified block
    match state.pg_store.get_events_by_block(&chain, number).await {
        Ok(events) => {
            // Convert events to response format
            let responses: Vec<EventResponse> = events.into_iter()
                .map(|event| event_to_response(&*event))
                .collect();
                
            Ok((StatusCode::OK, Json(responses)))
        },
        Err(e) => Err(ApiError::InternalError(e.to_string())),
    }
}

/// Handler for health check
async fn health_check(
    State(state): State<Arc<ApiState>>,
) -> std::result::Result<impl IntoResponse, ApiError> {
    // Get the latest blocks for each chain
    let chains = vec!["ethereum", "cosmos"];
    let mut latest_blocks = HashMap::new();

    for chain in chains {
        if let Ok(Some(block_num)) = state.rocks_store.get_latest_block_number(chain).await {
            if let Ok(Some(block)) = state.rocks_store.get_block(chain, block_num).await {
                latest_blocks.insert(chain.to_string(), BlockInfo {
                    block_number: block.number,
                    block_hash: block.hash.clone(),
                    timestamp: block.timestamp,
                    finality_status: format!("{:?}", block.finality_status),
                });
            }
        }
    }

    // Get the uptime
    // In a real implementation, this would track the actual uptime
    let uptime_seconds = 0;

    Ok((StatusCode::OK, Json(HealthResponse {
        healthy: true,
        version: env!("CARGO_PKG_VERSION").to_string(),
        uptime_seconds,
        latest_blocks,
    })))
}

/// Convert an error to a status code
fn error_to_status_code(error: &Error) -> StatusCode {
    match error {
        Error::Generic(msg) if msg.contains("not found") => StatusCode::NOT_FOUND,
        Error::Storage(msg) if msg.contains("already exists") => StatusCode::CONFLICT,
        Error::InvalidEvent(_) => StatusCode::BAD_REQUEST,
        _ => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

/// Convert an event to a response
fn event_to_response(event: &dyn Event) -> EventResponse {
    EventResponse {
        id: event.id().to_string(),
        chain: event.chain().to_string(),
        block_number: event.block_number(),
        block_hash: event.block_hash().to_string(),
        tx_hash: event.tx_hash().to_string(),
        timestamp: event.timestamp().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs(),
        event_type: event.event_type().to_string(),
        data: BASE64_STANDARD.encode(event.raw_data()),
    }
} 