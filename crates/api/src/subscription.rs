// This module implements WebSocket-based subscriptions for the indexer API

use std::net::SocketAddr;
use std::sync::Arc;

use axum::{
    routing::get,
    Router,
};
use tokio::net::TcpListener;
use tracing::info;

use indexer_core::service::BoxedEventService;
use indexer_core::{Result, Error};
use crate::{
    auth::AuthState,
    http::HttpState,
    websocket::{websocket_handler, websocket_stats, ConnectionManager},
};

/// Start the WebSocket server for real-time event subscriptions
pub async fn start_websocket_server(
    addr: SocketAddr,
    event_service: BoxedEventService,
) -> Result<()> {
    info!("Starting WebSocket server on {}", addr);
    
    // Create authentication state
    let jwt_secret = b"your-256-bit-secret-key-here-please-change-this-in-production";
    let auth_state = AuthState::new(jwt_secret);
    
    // Create connection manager for WebSocket connections
    let _connection_manager = Arc::new(ConnectionManager::new(
        event_service.clone(),
        auth_state.clone(),
    ));
    
    // Create HTTP state for the WebSocket server
    let state = HttpState {
        event_service,
        schema_registry: Arc::new(crate::InMemorySchemaRegistry::new()),
        auth_state,
        rate_limiter: Arc::new(indexer_core::security::RateLimiter::new(
            1000,
            std::time::Duration::from_secs(60)
        )),
        start_time: std::time::SystemTime::now(),
    };
    
    // Create router with WebSocket endpoints
    let app = Router::new()
        .route("/ws", get(websocket_handler))
        .route("/ws/stats", get(websocket_stats))
        .route("/health", get(health_check))
        .with_state(state);
    
    // Create TCP listener
    let listener = TcpListener::bind(&addr).await
        .map_err(|e| Error::api(format!("Failed to bind to address {}: {}", addr, e)))?;
    
    info!("WebSocket server listening on {}", addr);
    
    // Start the server
    axum::Server::from_tcp(listener.into_std().unwrap())
        .map_err(|e| Error::api(format!("Failed to create server: {}", e)))?
        .serve(app.into_make_service_with_connect_info::<SocketAddr>())
        .await
        .map_err(|e| Error::api(format!("WebSocket server error: {}", e)))?;
    
    Ok(())
}

/// Health check endpoint for the WebSocket server
async fn health_check() -> axum::response::Json<serde_json::Value> {
    axum::response::Json(serde_json::json!({
        "status": "healthy",
        "service": "websocket",
        "timestamp": std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }))
} 