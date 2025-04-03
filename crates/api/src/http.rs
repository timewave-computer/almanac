/// HTTP API server implementation
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use axum::{
    routing::{get, post},
    extract::{Path, Query, Json, State},
    Router, http::StatusCode,
};
use serde::{Deserialize, Serialize};
use tower_http::trace::TraceLayer;
use base64::{encode, decode};

use indexer_core::Result;
use indexer_core::event::Event;
use indexer_core::types::EventFilter as CoreEventFilter;
use indexer_storage::{Storage, EventFilter};

use crate::ApiConfig;

/// HTTP server state
#[derive(Clone)]
pub struct HttpServerState {
    /// Storage backend
    pub storage: Arc<dyn Storage>,
}

/// Start the HTTP server
pub async fn start_http_server(
    config: &ApiConfig, 
    storage: Arc<dyn Storage>
) -> Result<()> {
    // Create the server state
    let state = HttpServerState {
        storage,
    };
    
    // Create the router
    let app = Router::new()
        .route("/health", get(health_check))
        .route("/events", get(get_events))
        .route("/events/:chain/:id", get(get_event_by_id))
        .route("/events", post(store_event))
        .layer(TraceLayer::new_for_http())
        .with_state(state);
    
    // Parse the address
    let addr: SocketAddr = format!("{}:{}", config.host, config.port)
        .parse()
        .map_err(|e| indexer_core::Error::generic(format!("Failed to parse address: {}", e)))?;
    
    // Start the server
    tracing::info!("Starting HTTP server on {}", addr);
    
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .map_err(|e| indexer_core::Error::generic(format!("Failed to start HTTP server: {}", e)))?;
    
    Ok(())
}

/// Health check endpoint
async fn health_check() -> &'static str {
    "OK"
}

/// Event filter query parameters
#[derive(Debug, Deserialize)]
struct EventsQuery {
    /// Chain ID
    chain: Option<String>,
    
    /// Block range
    block_start: Option<u64>,
    block_end: Option<u64>,
    
    /// Timestamp range
    time_start: Option<u64>,
    time_end: Option<u64>,
    
    /// Event types
    event_types: Option<String>,
    
    /// Pagination
    limit: Option<usize>,
    offset: Option<usize>,
}

impl From<EventsQuery> for EventFilter {
    fn from(query: EventsQuery) -> Self {
        let block_range = if query.block_start.is_some() || query.block_end.is_some() {
            Some((
                query.block_start.unwrap_or(0),
                query.block_end.unwrap_or(u64::MAX),
            ))
        } else {
            None
        };
        
        let time_range = if query.time_start.is_some() || query.time_end.is_some() {
            Some((
                query.time_start.unwrap_or(0),
                query.time_end.unwrap_or(u64::MAX),
            ))
        } else {
            None
        };
        
        let event_types = query.event_types.map(|s| {
            s.split(',')
                .map(|s| s.trim().to_string())
                .collect()
        });
        
        Self {
            chain: query.chain,
            block_range,
            time_range,
            event_types,
            limit: query.limit,
            offset: query.offset,
        }
    }
}

/// Event response
#[derive(Debug, Serialize)]
struct EventResponse {
    /// Event ID
    id: String,
    
    /// Chain ID
    chain: String,
    
    /// Block number
    block_number: u64,
    
    /// Block hash
    block_hash: String,
    
    /// Transaction hash
    tx_hash: String,
    
    /// Timestamp
    timestamp: u64,
    
    /// Event type
    event_type: String,
    
    /// Raw data as base64
    data: String,
}

impl EventResponse {
    /// Create a new event response from an event
    fn from_event(event: &Box<dyn Event>) -> Self {
        Self {
            id: event.id().to_string(),
            chain: event.chain().to_string(),
            block_number: event.block_number(),
            block_hash: event.block_hash().to_string(),
            tx_hash: event.tx_hash().to_string(),
            timestamp: event.timestamp().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs(),
            event_type: event.event_type().to_string(),
            data: base64::encode(event.raw_data().to_vec()),
        }
    }
}

/// Get events endpoint
async fn get_events(
    State(state): State<HttpServerState>,
    Query(query): Query<EventsQuery>,
) -> Result<Json<Vec<EventResponse>>, StatusCode> {
    let filter = EventFilter::from(query);
    
    let events = state.storage.get_events(vec![filter])
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    let responses = events.iter()
        .map(|event| EventResponse::from_event(event))
        .collect();
    
    Ok(Json(responses))
}

/// Get event by ID endpoint
async fn get_event_by_id(
    State(state): State<HttpServerState>,
    Path((chain, id)): Path<(String, String)>,
) -> Result<Json<EventResponse>, StatusCode> {
    let filter = EventFilter {
        chain: Some(chain),
        block_range: None,
        time_range: None,
        event_types: None,
        limit: Some(1),
        offset: None,
    };
    
    let events = state.storage.get_events(vec![filter])
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    let event = events.first()
        .ok_or(StatusCode::NOT_FOUND)?;
    
    Ok(Json(EventResponse::from_event(event)))
}

/// Event creation request
#[derive(Debug, Deserialize)]
struct CreateEventRequest {
    /// Chain ID
    chain: String,
    
    /// Block number
    block_number: u64,
    
    /// Block hash
    block_hash: String,
    
    /// Transaction hash
    tx_hash: String,
    
    /// Timestamp
    timestamp: u64,
    
    /// Event type
    event_type: String,
    
    /// Raw data as base64
    data: String,
}

/// Store event endpoint
async fn store_event(
    State(state): State<HttpServerState>,
    Json(request): Json<CreateEventRequest>,
) -> Result<StatusCode, StatusCode> {
    // In a real implementation, we would create an event from the request
    // This is a placeholder implementation
    
    // Return success
    Ok(StatusCode::CREATED)
} 