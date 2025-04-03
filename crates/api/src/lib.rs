/// The API crate provides HTTP and GraphQL API endpoints for the indexer service
use std::sync::Arc;

use indexer_core::event::{Event, EventService};
use indexer_storage::Storage;

pub mod http;
pub mod graphql;

use http::start_http_server;
use graphql::start_graphql_server;

/// Configuration for the API server
#[derive(Clone)]
pub struct ApiConfig {
    /// Host to bind to
    pub host: String,
    
    /// Port to listen on
    pub port: u16,
    
    /// Enable GraphQL API
    pub enable_graphql: bool,
    
    /// Enable HTTP API
    pub enable_http: bool,
}

impl Default for ApiConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 8080,
            enable_graphql: true,
            enable_http: true,
        }
    }
}

/// API Server
pub struct ApiServer {
    /// Configuration
    config: ApiConfig,
    
    /// Event services
    event_services: Vec<Arc<dyn EventService>>,
    
    /// Storage backend
    storage: Arc<dyn Storage>,
}

impl ApiServer {
    /// Create a new API server
    pub fn new(
        config: ApiConfig,
        event_services: Vec<Arc<dyn EventService>>,
        storage: Arc<dyn Storage>,
    ) -> Self {
        Self {
            config,
            event_services,
            storage,
        }
    }
    
    /// Start the API server
    pub async fn start(&self) -> indexer_core::Result<()> {
        if self.config.enable_http {
            // Initialize HTTP server
            tokio::spawn({
                let config = self.config.clone();
                let storage = self.storage.clone();
                async move {
                    if let Err(e) = start_http_server(&config, storage).await {
                        tracing::error!("HTTP server error: {}", e);
                    }
                }
            });
        }
        
        if self.config.enable_graphql {
            // Initialize GraphQL server
            tokio::spawn({
                let config = self.config.clone();
                let storage = self.storage.clone();
                async move {
                    if let Err(e) = start_graphql_server(&config, storage).await {
                        tracing::error!("GraphQL server error: {}", e);
                    }
                }
            });
        }
        
        Ok(())
    }
}