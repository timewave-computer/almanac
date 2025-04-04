/// The API crate provides HTTP and GraphQL API endpoints for the indexer service
use std::net::SocketAddr;

use indexer_common::{Result, Error};
use indexer_core::service::BoxedEventService;
use indexer_core::types::ApiConfig;
use tracing::{info, error};

pub mod http;
pub mod graphql;

/// API server implementation
pub struct ApiServer {
    /// API configuration
    config: ApiConfig,
    
    /// Event service
    event_service: BoxedEventService,
}

impl ApiServer {
    /// Create a new API server
    pub fn new(config: ApiConfig, event_service: BoxedEventService) -> Self {
        Self {
            config,
            event_service,
        }
    }
    
    /// Start the API server
    pub async fn start(&self) -> Result<()> {
        // Parse server address
        let addr = format!("{}:{}", self.config.host, self.config.port)
            .parse::<SocketAddr>()
            .map_err(|e| Error::api(format!("Invalid server address: {}", e)))?;

        info!("Starting API server on {}", addr);

        // Start servers based on configuration
        if self.config.enable_rest {
            info!("HTTP API enabled");
            // Using tokio::spawn to run HTTP server in background
            tokio::spawn({
                let event_service = self.event_service.clone();
                async move {
                    if let Err(e) = http::start_http_server(addr, event_service).await {
                        error!("HTTP server error: {}", e);
                    }
                }
            });
        }

        if self.config.enable_graphql {
            info!("GraphQL API enabled");
            // Using tokio::spawn to run GraphQL server in background
            tokio::spawn({
                let event_service = self.event_service.clone();
                let graphql_addr = SocketAddr::new(addr.ip(), addr.port() + 1);
                async move {
                    if let Err(e) = graphql::start_graphql_server(graphql_addr, event_service).await {
                        error!("GraphQL server error: {}", e);
                    }
                }
            });
        }

        Ok(())
    }
} 