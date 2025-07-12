/// The API crate provides HTTP and GraphQL API endpoints for the indexer service
use std::net::SocketAddr;
use std::sync::Arc;
use std::collections::HashMap;

use indexer_core::{Error as CoreError};
use indexer_core::service::BoxedEventService;
use indexer_core::types::ApiConfig;
use tracing::{info, error};
use tokio::sync::RwLock;
use async_graphql::SimpleObject;
use axum::response::{Response, IntoResponse};
use axum::http::StatusCode;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use thiserror::Error;

// HTTP and WebSocket modules
pub mod http;
pub mod http_enhanced;
pub mod websocket;
pub mod graphql;
pub mod auth;
pub mod storage_proof_api;
pub mod distributed_query_api;
pub mod distributed_gateway;
pub mod streaming_query_api;
pub mod cli;
pub mod health;
pub mod prometheus_exporter;

/// API-specific errors
#[derive(Error, Debug)]
pub enum ApiError {
    #[error("API error: {0}")]
    Api(String),
    #[error("Authentication error: {0}")]
    Auth(String),
    #[error("Validation error: {0}")]
    Validation(String),
    #[error("Configuration error: {0}")]
    Config(String),
    #[error("Core error: {0}")]
    Core(#[from] CoreError),
    #[error("Not found")]
    NotFound,
    #[error("Bad request: {0}")]
    BadRequest(String),
    #[error("Internal server error: {0}")]
    InternalError(String),
}

pub type Result<T> = std::result::Result<T, ApiError>;

impl ApiError {
    pub fn generic(msg: &str) -> Self {
        ApiError::Api(msg.to_string())
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            ApiError::Api(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
            ApiError::Auth(msg) => (StatusCode::UNAUTHORIZED, msg),
            ApiError::Validation(msg) => (StatusCode::BAD_REQUEST, msg),
            ApiError::Config(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
            ApiError::Core(err) => (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()),
            ApiError::NotFound => (StatusCode::NOT_FOUND, "Resource not found".to_string()),
            ApiError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg),
            ApiError::InternalError(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
        };

        let json = serde_json::json!({
            "error": message,
        });

        (status, axum::Json(json)).into_response()
    }
}

/// Contract schema registry trait
#[async_trait]
pub trait ContractSchemaRegistry: Send + Sync {
    /// Register a new schema version
    async fn register_schema(&self, schema: ContractSchemaVersion) -> Result<()>;
    
    /// Get a specific schema version
    async fn get_schema(&self, chain: &str, address: &str, version: &str) -> Result<Option<ContractSchema>>;
    
    /// Get the latest schema for a contract
    async fn get_latest_schema(&self, chain: &str, address: &str) -> Result<Option<ContractSchema>>;
    
    /// Get all schemas for a chain with pagination
    async fn get_schemas(&self, chain: Option<String>, limit: usize, offset: usize) -> Result<Vec<ContractSchemaVersion>>;
    
    /// Delete a schema version
    async fn delete_schema(&self, chain: &str, address: &str, version: &str) -> Result<()>;
}

/// Contract schema with version information
#[derive(Debug, Clone, Serialize, Deserialize, SimpleObject)]
pub struct ContractSchemaVersion {
    /// Version string (e.g. "1.0.0")
    pub version: String,
    /// The contract schema
    pub schema: ContractSchema,
}

/// Contract schema definition
#[derive(Debug, Clone, Serialize, Deserialize, SimpleObject)]
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

/// Event schema definition
#[derive(Debug, Clone, Serialize, Deserialize, SimpleObject)]
pub struct EventSchema {
    /// Event name
    pub name: String,
    /// Event fields
    pub fields: Vec<FieldSchema>,
}

/// Function schema definition
#[derive(Debug, Clone, Serialize, Deserialize, SimpleObject)]
pub struct FunctionSchema {
    /// Function name
    pub name: String,
    /// Function inputs
    pub inputs: Vec<FieldSchema>,
    /// Function outputs
    pub outputs: Vec<FieldSchema>,
}

/// Field schema definition
#[derive(Debug, Clone, Serialize, Deserialize, SimpleObject)]
pub struct FieldSchema {
    /// Field name
    pub name: String,
    /// Field type (e.g. "uint256", "address")
    pub type_name: String,
    /// Whether the field is indexed (only applicable for events)
    pub indexed: bool,
}

/// In-memory contract schema registry implementation
pub struct InMemorySchemaRegistry {
    schemas: Arc<RwLock<HashMap<String, ContractSchemaVersion>>>,
}

impl InMemorySchemaRegistry {
    pub fn new() -> Self {
        Self {
            schemas: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    fn make_key(&self, chain: &str, address: &str, version: &str) -> String {
        format!("{}:{}:{}", chain, address, version)
    }
    
    fn make_prefix(&self, chain: &str, address: &str) -> String {
        format!("{}:{}", chain, address)
    }
}

#[async_trait]
impl ContractSchemaRegistry for InMemorySchemaRegistry {
    async fn register_schema(&self, schema: ContractSchemaVersion) -> Result<()> {
        let key = self.make_key(&schema.schema.chain, &schema.schema.address, &schema.version);
        let mut schemas = self.schemas.write().await;
        schemas.insert(key, schema);
        Ok(())
    }
    
    async fn get_schema(&self, chain: &str, address: &str, version: &str) -> Result<Option<ContractSchema>> {
        let key = self.make_key(chain, address, version);
        let schemas = self.schemas.read().await;
        Ok(schemas.get(&key).map(|sv| sv.schema.clone()))
    }
    
    async fn get_latest_schema(&self, chain: &str, address: &str) -> Result<Option<ContractSchema>> {
        let prefix = self.make_prefix(chain, address);
        let schemas = self.schemas.read().await;
        
        // Find the latest version (simplified - would need proper version comparison)
        let latest = schemas.iter()
            .filter(|(key, _)| key.starts_with(&prefix))
            .max_by_key(|(_, schema_version)| &schema_version.version)
            .map(|(_, schema_version)| schema_version.schema.clone());
        
        Ok(latest)
    }
    
    async fn get_schemas(&self, chain: Option<String>, limit: usize, offset: usize) -> Result<Vec<ContractSchemaVersion>> {
        let schemas = self.schemas.read().await;
        let mut filtered: Vec<_> = schemas.values()
            .filter(|schema| {
                if let Some(ref chain_filter) = chain {
                    schema.schema.chain == *chain_filter
                } else {
                    true
                }
            })
            .skip(offset)
            .take(limit)
            .cloned()
            .collect();
        
        // Sort by version (simplified)
        filtered.sort_by(|a, b| a.version.cmp(&b.version));
        
        Ok(filtered)
    }
    
    async fn delete_schema(&self, chain: &str, address: &str, version: &str) -> Result<()> {
        let key = self.make_key(chain, address, version);
        let mut schemas = self.schemas.write().await;
        schemas.remove(&key);
        Ok(())
    }
}

impl Default for InMemorySchemaRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Simple authentication state placeholder
#[derive(Clone)]
pub struct AuthState {
    // JWT secret removed as it's not actually used
}

impl AuthState {
    pub fn new(_secret: &[u8]) -> Self {
        Self {
        }
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
    /// API server configuration
    config: ApiServerConfig,
    /// Running state
    running: Arc<tokio::sync::Mutex<bool>>,
}

impl ApiServer {
    /// Create a new API server
    pub fn new(
        _event_service: BoxedEventService,
        _schema_registry: Arc<dyn ContractSchemaRegistry>,
        config: ApiServerConfig,
    ) -> Self {
        // Create default JWT secret (in production, this should be from config)
        let jwt_secret = b"your-256-bit-secret-key-here-please-change-this-in-production";
        let _auth_state = AuthState::new(jwt_secret);
        
        Self {
            config,
            running: Arc::new(tokio::sync::Mutex::new(false)),
        }
    }

    /// Create a new API server from an API config
    pub fn from_config(
        config: &ApiConfig,
        event_service: BoxedEventService,
        schema_registry: Arc<dyn ContractSchemaRegistry>,
    ) -> Result<Self> {
        // Parse host and port
        let host = &config.host;
        let port = config.port;
        
        // Create HTTP address
        let http_addr = format!("{}:{}", host, port)
            .parse()
            .map_err(|e| ApiError::Config(format!("Invalid HTTP address: {}", e)))?;
        
        // Create GraphQL address
        let graphql_port = port + 1;
        let graphql_addr = format!("{}:{}", host, graphql_port)
            .parse()
            .map_err(|e| ApiError::Config(format!("Invalid GraphQL address: {}", e)))?;
        
        // Create WebSocket address if enabled
        let ws_addr = if true {  // Simplified - assume WebSocket always enabled
            let ws_port = port + 2;
            let addr = format!("{}:{}", host, ws_port)
                .parse()
                .map_err(|e| ApiError::Config(format!("Invalid WebSocket address: {}", e)))?;
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

    /// Start the API server (placeholder implementation)
    pub async fn start(&self) -> Result<()> {
        info!("Starting API server");
        
        // Set running status
        let mut running = self.running.lock().await;
        *running = true;
        drop(running);
        
        // TODO: Implement actual server startup logic
        // For now, just log that we would start servers
        info!("HTTP server would start on: {}", self.config.http_addr);
        info!("GraphQL server would start on: {}", self.config.graphql_addr);
        if let Some(ws_addr) = self.config.ws_addr {
            info!("WebSocket server would start on: {}", ws_addr);
        }
        
        Ok(())
    }

    /// Stop the API server
    pub async fn stop(&self) -> Result<()> {
        let mut running = self.running.lock().await;
        if !*running {
            return Err(ApiError::generic("Server is not running"));
        }
        
        *running = false;
        info!("API server stopped");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{IpAddr, Ipv4Addr};
    
    #[test]
    fn test_schema_registry_creation() {
        let registry = InMemorySchemaRegistry::new();
        // Basic creation test
        assert!(true);
    }
    
    #[tokio::test]
    async fn test_schema_registry_operations() {
        let registry = InMemorySchemaRegistry::new();
        
        let schema = ContractSchema {
            chain: "ethereum".to_string(),
            address: "0x123".to_string(),
            name: "TestContract".to_string(),
            events: vec![],
            functions: vec![],
        };
        
        let schema_version = ContractSchemaVersion {
            version: "1.0.0".to_string(),
            schema: schema.clone(),
        };
        
        // Test registration
        assert!(registry.register_schema(schema_version).await.is_ok());
        
        // Test retrieval by version
        let retrieved = registry.get_schema("ethereum", "0x123", "1.0.0").await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().name, "TestContract");
        
        // Test latest schema retrieval
        let latest = registry.get_latest_schema("ethereum", "0x123").await.unwrap();
        assert!(latest.is_some());
        assert_eq!(latest.unwrap().name, "TestContract");
    }
    
    #[test]
    fn test_api_server_config() {
        let http_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080);
        let graphql_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8081);
        
        let config = ApiServerConfig {
            http_addr,
            graphql_addr,
            ws_addr: None,
            enable_playground: true,
        };
        
        assert_eq!(config.http_addr.port(), 8080);
        assert_eq!(config.graphql_addr.port(), 8081);
        assert!(config.enable_playground);
    }
} 