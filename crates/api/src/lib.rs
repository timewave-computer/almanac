/// The API crate provides HTTP and GraphQL API endpoints for the indexer service
use std::net::SocketAddr;
use std::sync::Arc;
use std::collections::HashMap;

use indexer_core::{Error, Result, BlockStatus};
use indexer_core::service::BoxedEventService;
use indexer_core::types::ApiConfig;
use tracing::{info, error};
use tokio::sync::Mutex;
use async_graphql::SimpleObject;

pub mod http;
pub mod graphql;
pub mod subscription;
pub mod auth;
pub mod websocket;

/// Registry for contract schemas
pub trait ContractSchemaRegistry: Send + Sync {
    /// Get a contract schema by chain and address
    fn get_schema(&self, chain: &str, address: &str, version: &str) -> Result<Option<ContractSchema>>;
    
    /// Get the latest schema version for a contract
    fn get_latest_schema(&self, chain: &str, address: &str) -> Result<Option<ContractSchema>>;
    
    /// Register a new schema
    fn register_schema(&self, schema: ContractSchemaVersion) -> Result<()>;
}

/// Contract schema version
#[derive(Clone, Debug, SimpleObject)]
pub struct ContractSchemaVersion {
    /// Version string (e.g. "1.0.0")
    pub version: String,
    /// The contract schema
    pub schema: ContractSchema,
}

/// Contract schema
#[derive(Clone, Debug, SimpleObject)]
pub struct ContractSchema {
    /// Chain ID (e.g. "ethereum", "polygon")
    pub chain: String,
    /// Contract address
    pub address: String,
    /// Contract name
    pub name: String,
    /// Event schemas
    pub events: Vec<EventSchema>,
    /// Function schemas
    pub functions: Vec<FunctionSchema>,
}

/// Event schema
#[derive(Clone, Debug, SimpleObject)]
pub struct EventSchema {
    /// Event name
    pub name: String,
    /// Event fields
    pub fields: Vec<FieldSchema>,
}

/// Function schema
#[derive(Clone, Debug, SimpleObject)]
pub struct FunctionSchema {
    /// Function name
    pub name: String,
    /// Function inputs
    pub inputs: Vec<FieldSchema>,
    /// Function outputs
    pub outputs: Vec<FieldSchema>,
}

/// Field schema
#[derive(Clone, Debug, SimpleObject)]
pub struct FieldSchema {
    /// Field name
    pub name: String,
    /// Field type (e.g. "uint256", "address")
    pub type_name: String,
    /// Whether the field is indexed (only applicable for events)
    pub indexed: bool,
}

/// In-memory implementation of contract schema registry
pub struct InMemorySchemaRegistry {
    schemas: Arc<Mutex<HashMap<String, Vec<ContractSchemaVersion>>>>,
}

impl InMemorySchemaRegistry {
    /// Create a new in-memory schema registry
    pub fn new() -> Self {
        InMemorySchemaRegistry {
            schemas: Arc::new(Mutex::new(HashMap::new())),
        }
    }
    
    fn create_key(chain: &str, address: &str) -> String {
        format!("{}:{}", chain, address)
    }
}

impl ContractSchemaRegistry for InMemorySchemaRegistry {
    fn get_schema(&self, chain: &str, address: &str, version: &str) -> Result<Option<ContractSchema>> {
        let schemas_lock = self.schemas.try_lock()
            .map_err(|_| Error::generic("Failed to acquire lock on schema registry"))?;
        
        let key = Self::create_key(chain, address);
        
        if let Some(versions) = schemas_lock.get(&key) {
            for ver in versions {
                if ver.version == version {
                    return Ok(Some(ver.schema.clone()));
                }
            }
        }
        
        Ok(None)
    }
    
    fn get_latest_schema(&self, chain: &str, address: &str) -> Result<Option<ContractSchema>> {
        let schemas_lock = self.schemas.try_lock()
            .map_err(|_| Error::generic("Failed to acquire lock on schema registry"))?;
        
        let key = Self::create_key(chain, address);
        
        if let Some(versions) = schemas_lock.get(&key) {
            if !versions.is_empty() {
                return Ok(Some(versions.last().unwrap().schema.clone()));
            }
        }
        
        Ok(None)
    }
    
    fn register_schema(&self, schema: ContractSchemaVersion) -> Result<()> {
        let mut schemas_lock = self.schemas.try_lock()
            .map_err(|_| Error::generic("Failed to acquire lock on schema registry"))?;
        
        let key = Self::create_key(&schema.schema.chain, &schema.schema.address);
        
        let versions = schemas_lock.entry(key).or_insert_with(Vec::new);
        versions.push(schema);
        
        Ok(())
    }
}

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
    /// Authentication state
    auth_state: auth::AuthState,
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
        // Create default JWT secret (in production, this should be from config)
        let jwt_secret = b"your-256-bit-secret-key-here-please-change-this-in-production";
        let auth_state = auth::AuthState::new(jwt_secret);
        
        Self {
            event_service,
            schema_registry,
            auth_state,
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
        
        // Start HTTP REST API server if enabled
        if self.config.http_addr.port() != 0 {
            let event_service = self.event_service.clone();
            let schema_registry = self.schema_registry.clone();
            let auth_state = self.auth_state.clone();
            let addr = self.config.http_addr;
            
            // Spawn HTTP server task
            tokio::spawn(async move {
                if let Err(e) = http::start_http_server(addr, event_service, schema_registry, auth_state).await {
                    error!("HTTP REST API server error: {}", e);
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