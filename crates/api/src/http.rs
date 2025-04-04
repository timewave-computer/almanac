/// HTTP API server implementation
use std::net::SocketAddr;
use std::collections::HashMap;
use std::time::UNIX_EPOCH;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Json},
    routing::{get, post},
    Router,
};
use base64::prelude::*;
use serde::{Deserialize, Serialize};
use tokio::net::TcpListener;
use tracing::info;

use indexer_common::{BlockStatus, Error, Result};
use indexer_core::event::Event;
use indexer_core::service::BoxedEventService;
use indexer_core::types::EventFilter;

/// State for the HTTP server
#[derive(Clone)]
pub struct HttpServerState {
    /// Event service
    event_service: BoxedEventService,
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
#[derive(Debug, Clone, Deserialize)]
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

/// Start the HTTP server
pub async fn start_http_server(
    addr: SocketAddr,
    event_service: BoxedEventService,
) -> Result<()> {
    let state = HttpServerState { event_service };
    
    // Create router with routes
    let app = Router::new()
        .route("/events", post(get_events))
        .route("/events/:id", get(get_event))
        .route("/blocks/latest", get(get_latest_block))
        .route("/blocks/latest/:status", get(get_latest_block_with_status))
        .route("/blocks/:number", get(get_block))
        .route("/blocks/:number/events", get(get_events_by_block))
        .with_state(state);
    
    info!("Starting HTTP server on {}", addr);
    
    // Bind to address
    let listener = TcpListener::bind(addr).await
        .map_err(|e| Error::api(format!("Failed to bind to address: {}", e)))?;
    
    // Start server
    axum::serve(listener, app).await
        .map_err(|e| Error::api(format!("Server error: {}", e)))?;
    
    Ok(())
}

/// Get events based on filters
async fn get_events(
    State(state): State<HttpServerState>,
    Json(filter_req): Json<EventFilterRequest>,
) -> impl IntoResponse {
    let mut filter = EventFilter::new();
    filter.chain = filter_req.chain;
    filter.block_range = filter_req.block_range;
    filter.time_range = filter_req.time_range;
    filter.event_types = filter_req.event_types;
    filter.limit = filter_req.limit;
    filter.offset = filter_req.offset;
    filter.custom_filters = filter_req.custom_filters;
    
    match state.event_service.get_events(vec![filter]).await {
        Ok(events) => {
            let responses: Vec<EventResponse> = events.into_iter()
                .map(|event| event_to_response(&event))
                .collect();
            (StatusCode::OK, Json(responses)).into_response()
        }
        Err(e) => {
            (error_to_status_code(&e), Json(e.to_string())).into_response()
        }
    }
}

/// Get a specific event by ID
async fn get_event(
    State(state): State<HttpServerState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    // Use a filter to find the event by ID
    let mut filter = EventFilter::new();
    filter.custom_filters.insert("id".to_string(), id);
    filter.limit = Some(1);
    
    match state.event_service.get_events(vec![filter]).await {
        Ok(events) if !events.is_empty() => {
            (StatusCode::OK, Json(event_to_response(&events[0]))).into_response()
        }
        Ok(_) => {
            (StatusCode::NOT_FOUND, Json("Event not found".to_string())).into_response()
        }
        Err(e) => {
            (error_to_status_code(&e), Json(e.to_string())).into_response()
        }
    }
}

/// Get the latest block number
async fn get_latest_block(
    State(state): State<HttpServerState>,
) -> impl IntoResponse {
    match state.event_service.get_latest_block().await {
        Ok(block) => (StatusCode::OK, Json(block)).into_response(),
        Err(e) => (error_to_status_code(&e), Json(e.to_string())).into_response(),
    }
}

/// Get the latest block number with a specific status
async fn get_latest_block_with_status(
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

/// Get a specific block
async fn get_block(Path(number): Path<u64>) -> impl IntoResponse {
    (StatusCode::OK, Json(number)).into_response()
}

/// Get events for a specific block
async fn get_events_by_block(
    State(state): State<HttpServerState>,
    Path(block_number): Path<u64>,
) -> impl IntoResponse {
    let mut filter = EventFilter::new();
    filter.block_range = Some((block_number, block_number));
    
    match state.event_service.get_events(vec![filter]).await {
        Ok(events) => {
            let responses: Vec<EventResponse> = events.into_iter()
                .map(|event| event_to_response(&event))
                .collect();
            (StatusCode::OK, Json(responses)).into_response()
        }
        Err(e) => {
            (error_to_status_code(&e), Json(e.to_string())).into_response()
        }
    }
}

/// Convert an error to an HTTP status code
fn error_to_status_code(error: &Error) -> StatusCode {
    match error {
        Error::Generic(_) => StatusCode::INTERNAL_SERVER_ERROR,
        Error::Storage(_) => StatusCode::INTERNAL_SERVER_ERROR,
        Error::Database(_) => StatusCode::INTERNAL_SERVER_ERROR,
        Error::MissingService(_) => StatusCode::NOT_FOUND,
        Error::InvalidEvent(_) => StatusCode::BAD_REQUEST,
        Error::Api(_) => StatusCode::INTERNAL_SERVER_ERROR,
        Error::RocksDB(_) => StatusCode::INTERNAL_SERVER_ERROR,
        Error::Serialization(_) => StatusCode::BAD_REQUEST,
    }
}

/// Convert an event to a response
fn event_to_response(event: &Box<dyn Event>) -> EventResponse {
    // Convert SystemTime to u64 timestamp
    let timestamp = event.timestamp()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
        
    EventResponse {
        id: event.id().to_string(),
        chain: event.chain().to_string(),
        block_number: event.block_number(),
        block_hash: event.block_hash().to_string(),
        tx_hash: event.tx_hash().to_string(),
        timestamp,
        event_type: event.event_type().to_string(),
        data: BASE64_STANDARD.encode(event.raw_data()),
    }
} 