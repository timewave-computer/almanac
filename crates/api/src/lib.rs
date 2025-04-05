/// The API crate provides HTTP and GraphQL API endpoints for the indexer service
use std::net::SocketAddr;
use std::sync::Arc;

use indexer_common::{Result, Error};
use indexer_core::service::BoxedEventService;
use indexer_core::types::ApiConfig;
use indexer_storage::migrations::schema::ContractSchemaRegistry;
use tracing::{info, error};
use tokio::sync::Mutex;

pub mod http;
pub mod graphql;
pub mod subscription;

/// API server configuration
pub struct ApiServerConfig {
    /// HTTP server address
    pub http_addr: SocketAddr,
    /// GraphQL server address
    pub graphql_addr: SocketAddr,
    /// WebSocket server address
    pub ws_addr: Option<SocketAddr>,
    /// Enable GraphQL playground
    pub enable_playground: bool,
}

/// API server
pub struct ApiServer {
    /// Event service
    event_service: BoxedEventService,
    /// Schema registry
    schema_registry: Arc<dyn ContractSchemaRegistry + Send + Sync>,
    /// API server configuration
    config: ApiServerConfig,
    /// Running state
    running: Arc<Mutex<bool>>,
}

impl ApiServer {
    /// Create a new API server
    pub fn new(
        event_service: BoxedEventService,
        schema_registry: Arc<dyn ContractSchemaRegistry + Send + Sync>,
        config: ApiServerConfig,
    ) -> Self {
        Self {
            event_service,
            schema_registry,
            config,
            running: Arc::new(Mutex::new(false)),
        }
    }

    /// Create a new API server from an API config
    pub fn from_config(
        config: &ApiConfig,
        event_service: BoxedEventService,
        schema_registry: Arc<dyn ContractSchemaRegistry + Send + Sync>,
    ) -> Result<Self> {
        // Parse host and port
        let host = &config.host;
        let port = config.port;
        
        // Create HTTP address
        let http_addr = format!("{}:{}", host, port)
            .parse()
            .map_err(|e| Error::Config(format!("Invalid HTTP address: {}", e)))?;
        
        // Create GraphQL address
        let graphql_port = port + 1;
        let graphql_addr = format!("{}:{}", host, graphql_port)
            .parse()
            .map_err(|e| Error::Config(format!("Invalid GraphQL address: {}", e)))?;
        
        // Create WebSocket address if enabled
        let ws_addr = if config.enable_websocket {
            let ws_port = port + 2;
            let addr = format!("{}:{}", host, ws_port)
                .parse()
                .map_err(|e| Error::Config(format!("Invalid WebSocket address: {}", e)))?;
            Some(addr)
        } else {
            None
        };
        
        // Create server config
        let server_config = ApiServerConfig {
            http_addr,
            graphql_addr,
            ws_addr,
            enable_playground: true, // Always enable for development
        };
        
        // Create API server
        Ok(Self::new(event_service, schema_registry, server_config))
    }

    /// Start the API server
    pub async fn start(&self) -> Result<()> {
        info!("Starting API server");
        
        // Set running status
        let mut running = self.running.lock().await;
        *running = true;
        drop(running);
        
        // Start HTTP server if enabled
        if self.config.http_addr.port() != 0 {
            let event_service = self.event_service.clone();
            let addr = self.config.http_addr;
            
            // Spawn HTTP server task
            tokio::spawn(async move {
                if let Err(e) = http::start_http_server(addr, event_service).await {
                    error!("HTTP server error: {}", e);
                }
            });
        }
        
        // Start GraphQL server if enabled
        if self.config.graphql_addr.port() != 0 {
            let event_service = self.event_service.clone();
            let schema_registry = self.schema_registry.clone();
            let addr = self.config.graphql_addr;
            let enable_playground = self.config.enable_playground;
            
            // Spawn GraphQL server task
            tokio::spawn(async move {
                if let Err(e) = graphql::start_graphql_server(addr, event_service, schema_registry, enable_playground).await {
                    error!("GraphQL server error: {}", e);
                }
            });
        }
        
        // Start WebSocket server if enabled
        if let Some(ws_addr) = self.config.ws_addr {
            let event_service = self.event_service.clone();
            
            // Spawn WebSocket server task
            tokio::spawn(async move {
                if let Err(e) = subscription::start_websocket_server(ws_addr, event_service).await {
                    error!("WebSocket server error: {}", e);
                }
            });
        }
        
        Ok(())
    }

    /// Stop the API server
    pub async fn stop(&self) -> Result<()> {
        let mut running = self.running.lock().await;
        *running = false;
        Ok(())
    }
} 