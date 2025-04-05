/// GraphQL API implementation
use std::sync::Arc;

use async_graphql::{
    Context, EmptySubscription, InputObject, Object, Schema, SimpleObject,
    ID, Enum, Scalar, ScalarType, 
    http::GraphiQLSource,
    InputValueError, InputValueResult, Value as GraphQLValue,
};
use async_graphql_axum::{GraphQLRequest, GraphQLResponse};
use axum::{
    response::{IntoResponse, Html},
    routing::get,
    Router, extract::State,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value as JsonValue};
use tracing::info;

use indexer_common::{Error, Result};
use indexer_core::service::BoxedEventService;
use indexer_storage::migrations::schema::{
    ContractSchemaVersion, ContractSchema, EventSchema, FunctionSchema, FieldSchema,
    ContractSchemaRegistry,
};

/// JSON scalar for GraphQL
#[derive(Clone)]
struct JSON(JsonValue);

/// Custom scalar for JSON
#[Scalar]
impl ScalarType for JSON {
    fn parse(value: GraphQLValue) -> InputValueResult<Self> {
        if let GraphQLValue::Object(obj) = &value {
            // Convert GraphQL value to serde_json value
            let json_value = serde_json::to_value(obj).map_err(|_| InputValueError::expected_type(value))?;
            Ok(JSON(json_value))
        } else {
            Err(InputValueError::expected_type(value))
        }
    }

    fn to_value(&self) -> GraphQLValue {
        // Try to convert serde_json value to GraphQL value
        match serde_json::from_value::<GraphQLValue>(self.0.clone()) {
            Ok(v) => v,
            Err(_) => GraphQLValue::Null,
        }
    }
}

/// Custom scalar type for representing block hash
#[derive(Clone)]
struct BlockHash(String);

/// Custom scalar for block hash
#[Scalar]
impl ScalarType for BlockHash {
    fn parse(value: GraphQLValue) -> InputValueResult<Self> {
        match value {
            GraphQLValue::String(s) => Ok(BlockHash(s)),
            _ => Err(InputValueError::expected_type(value)),
        }
    }

    fn to_value(&self) -> GraphQLValue {
        GraphQLValue::String(self.0.clone())
    }
}

/// GraphQL schema with query, mutation, and subscription
pub type GraphQLSchema = Schema<QueryRoot, MutationRoot, EmptySubscription>;

/// Query root
pub struct QueryRoot;

#[Object]
impl QueryRoot {
    /// Get an event by ID
    async fn event(&self, ctx: &Context<'_>, id: ID) -> async_graphql::Result<GraphQLEvent> {
        let _state = ctx.data::<AppState>()?;
        
        // Use a mock event for now
        Ok(GraphQLEvent {
            id,
            chain: "ethereum".into(),
            block_number: 100,
            block_hash: "0x123".into(),
            tx_hash: "0xabc".into(),
            timestamp: chrono::Utc::now(),
            event_type: "Transfer".into(),
            data: "SGVsbG8gV29ybGQ=".into(), // Base64 encoded "Hello World"
            attributes: None,
        })
    }

    /// Query events with filter
    async fn events(
        &self, 
        ctx: &Context<'_>,
        filter: Option<EventFilterInput>
    ) -> async_graphql::Result<Vec<GraphQLEvent>> {
        let _state = ctx.data::<AppState>()?;
        
        // Return mock data for now
        Ok(vec![
            GraphQLEvent {
                id: "1".into(),
                chain: "ethereum".into(),
                block_number: 100,
                block_hash: "0x123".into(),
                tx_hash: "0xabc".into(),
                timestamp: chrono::Utc::now(),
                event_type: "Transfer".into(),
                data: "SGVsbG8gV29ybGQ=".into(),
                attributes: None,
            }
        ])
    }

    /// Get latest block
    async fn latest_block(&self, ctx: &Context<'_>, chain: Option<String>) -> async_graphql::Result<ChainBlock> {
        let _state = ctx.data::<AppState>()?;
        let chain = chain.unwrap_or_else(|| "ethereum".to_string());
        
        // Return mock data
        Ok(ChainBlock {
            chain,
            number: 1000,
            hash: "0x123abc".into(),
            timestamp: chrono::Utc::now(),
            finality_status: GraphQLFinalityStatus::Confirmed,
        })
    }

    /// Get latest block with status
    async fn latest_block_with_status(
        &self, 
        ctx: &Context<'_>,
        chain: Option<String>,
        status: GraphQLFinalityStatus
    ) -> async_graphql::Result<ChainBlock> {
        let _state = ctx.data::<AppState>()?;
        let chain = chain.unwrap_or_else(|| "ethereum".to_string());
        
        // Mock implementation
        Ok(ChainBlock {
            chain,
            number: 1000,
            hash: "0x123abc".into(),
            timestamp: chrono::Utc::now(),
            finality_status: status,
        })
    }

    /// Health check
    async fn health(&self, ctx: &Context<'_>) -> async_graphql::Result<HealthStatus> {
        let _state = ctx.data::<AppState>()?;
        
        Ok(HealthStatus {
            healthy: true,
            version: env!("CARGO_PKG_VERSION").into(),
            uptime_seconds: 0,
        })
    }

    /// Get contract schema by chain, address, and version
    async fn contract_schema(
        &self,
        ctx: &Context<'_>,
        chain: String,
        address: String,
        version: Option<String>,
    ) -> async_graphql::Result<Option<ContractSchema>> {
        let state = ctx.data::<AppState>()?;
        
        // If version is provided, get that specific version
        // Otherwise, get the latest
        if let Some(version) = version {
            if let Some(schema_version) = state.schema_registry.get_schema(&version, &address, &chain) {
                Ok(Some(schema_version.schema.clone()))
            } else {
                Ok(None)
            }
        } else {
            // Get the latest schema
            if let Some(schema_version) = state.schema_registry.get_latest_schema(&address, &chain) {
                Ok(Some(schema_version.schema.clone()))
            } else {
                Ok(None)
            }
        }
    }
    
    /// Get all registered contract schemas
    async fn contract_schemas(
        &self,
        ctx: &Context<'_>,
        chain: Option<String>,
        limit: Option<i32>,
        offset: Option<i32>,
    ) -> async_graphql::Result<Vec<ContractSchemaVersion>> {
        let _state = ctx.data::<AppState>()?;
        
        // This implementation is a placeholder since we don't have a registry list method yet
        // In a real implementation, we would filter schemas by chain and paginate results
        Ok(vec![])
    }
}

/// GraphQL event type
#[derive(SimpleObject)]
pub struct GraphQLEvent {
    /// Event ID
    id: ID,
    /// Chain ID
    chain: String,
    /// Block number
    block_number: i64,
    /// Block hash
    block_hash: String,
    /// Transaction hash
    tx_hash: String,
    /// Timestamp
    timestamp: chrono::DateTime<chrono::Utc>,
    /// Event type
    event_type: String,
    /// Event data as base64
    data: String,
    /// Custom attributes
    attributes: Option<JsonValue>,
}

/// Event filter input
#[derive(InputObject)]
struct EventFilterInput {
    /// Chain ID
    chain: Option<String>,
    /// Block range
    block_range: Option<Vec<i64>>,
    /// Time range
    time_range: Option<Vec<String>>,
    /// Event types
    event_types: Option<Vec<String>>,
    /// Limit
    limit: Option<i32>,
    /// Offset
    offset: Option<i32>,
    /// Custom attributes
    attributes: Option<JsonValue>,
}

/// Chain block
#[derive(SimpleObject)]
struct ChainBlock {
    /// Chain ID
    chain: String,
    /// Block number
    number: i64,
    /// Block hash
    hash: String,
    /// Timestamp
    timestamp: chrono::DateTime<chrono::Utc>,
    /// Finality status
    finality_status: GraphQLFinalityStatus,
}

/// Finality status
#[derive(Enum, Copy, Clone, Eq, PartialEq)]
enum GraphQLFinalityStatus {
    /// Confirmed but not finalized
    Confirmed,
    /// Safe
    Safe,
    /// Justified
    Justified,
    /// Finalized
    Finalized,
}

/// Determinism level
#[derive(Enum, Copy, Clone, Eq, PartialEq)]
enum GraphQLDeterminismLevel {
    /// Deterministic
    Deterministic,
    /// Non-deterministic
    NonDeterministic,
    /// Light client verifiable
    LightClientVerifiable,
}

/// Health status
#[derive(SimpleObject)]
struct HealthStatus {
    /// Is the API healthy
    healthy: bool,
    /// API version
    version: String,
    /// Uptime in seconds
    uptime_seconds: i64,
}

/// Mutation root (now with schema management operations)
pub struct MutationRoot;

#[Object]
impl MutationRoot {
    /// Register a new contract schema
    async fn register_contract_schema(
        &self, 
        ctx: &Context<'_>, 
        input: ContractSchemaInput
    ) -> async_graphql::Result<RegisterSchemaResult> {
        let state = ctx.data::<AppState>()?;
        
        // Convert input to ContractSchemaVersion
        let schema_version = ContractSchemaVersion {
            id: format!("{}:{}:{}", input.version, input.contract_address, input.chain_id),
            contract_address: input.contract_address,
            chain_id: input.chain_id,
            version: input.version,
            schema: ContractSchema {
                name: input.name,
                events: input.events.into_iter().map(|e| EventSchema {
                    name: e.name,
                    fields: e.fields.into_iter().map(|f| FieldSchema {
                        name: f.name,
                        type_name: f.type_name,
                        indexed: f.indexed,
                    }).collect(),
                }).collect(),
                functions: input.functions.into_iter().map(|f| FunctionSchema {
                    name: f.name,
                    inputs: f.inputs.into_iter().map(|field| FieldSchema {
                        name: field.name,
                        type_name: field.type_name,
                        indexed: field.indexed,
                    }).collect(),
                    outputs: f.outputs.into_iter().map(|field| FieldSchema {
                        name: field.name,
                        type_name: field.type_name,
                        indexed: field.indexed,
                    }).collect(),
                }).collect(),
            },
        };
        
        // Attempt to register the schema
        match state.schema_registry.register_schema(schema_version.clone()) {
            Ok(_) => Ok(RegisterSchemaResult {
                success: true,
                error: None,
                schema_version: Some(schema_version),
            }),
            Err(e) => Ok(RegisterSchemaResult {
                success: false,
                error: Some(e.to_string()),
                schema_version: None,
            }),
        }
    }
    
    /// Delete a contract schema
    async fn delete_contract_schema(
        &self, 
        ctx: &Context<'_>, 
        chain: String, 
        address: String, 
        version: String
    ) -> async_graphql::Result<DeleteSchemaResult> {
        let _state = ctx.data::<AppState>()?;
        
        // The current API doesn't support schema deletion, so we return an error
        // This would be implemented with an actual deletion operation when that functionality is added
        Ok(DeleteSchemaResult {
            success: false,
            error: Some("Schema deletion is not yet implemented".to_string()),
        })
    }
    
    /// Ping (placeholder)
    async fn ping(&self) -> &str {
        "pong"
    }
}

/// Input for registering a contract schema
#[derive(InputObject)]
struct ContractSchemaInput {
    /// Chain identifier
    chain_id: String,
    
    /// Contract address
    contract_address: String,
    
    /// Schema version
    version: String,
    
    /// Contract name
    name: String,
    
    /// Contract events
    events: Vec<EventSchemaInput>,
    
    /// Contract functions
    functions: Vec<FunctionSchemaInput>,
}

/// Input for an event schema
#[derive(InputObject)]
struct EventSchemaInput {
    /// Event name
    name: String,
    
    /// Event fields
    fields: Vec<FieldSchemaInput>,
}

/// Input for a function schema
#[derive(InputObject)]
struct FunctionSchemaInput {
    /// Function name
    name: String,
    
    /// Function inputs
    inputs: Vec<FieldSchemaInput>,
    
    /// Function outputs
    outputs: Vec<FieldSchemaInput>,
}

/// Input for a field schema
#[derive(InputObject)]
struct FieldSchemaInput {
    /// Field name
    name: String,
    
    /// Field type
    type_name: String,
    
    /// Whether the field is indexed (for events)
    indexed: bool,
}

/// Result of registering a contract schema
#[derive(SimpleObject)]
struct RegisterSchemaResult {
    /// Success status
    success: bool,
    
    /// Error message if registration failed
    error: Option<String>,
    
    /// Registered schema version if successful
    schema_version: Option<ContractSchemaVersion>,
}

/// Result of deleting a contract schema
#[derive(SimpleObject)]
struct DeleteSchemaResult {
    /// Success status
    success: bool,
    
    /// Error message if deletion failed
    error: Option<String>,
}

/// GraphQL application state
pub struct AppState {
    /// Event service
    pub event_service: BoxedEventService,
    
    /// Schema registry
    pub schema_registry: Arc<dyn ContractSchemaRegistry + Send + Sync>,
}

/// Create GraphQL schema
pub fn create_schema(
    event_service: BoxedEventService,
    schema_registry: Arc<dyn ContractSchemaRegistry + Send + Sync>,
) -> GraphQLSchema {
    Schema::build(QueryRoot, MutationRoot, EmptySubscription)
        .data(AppState { 
            event_service,
            schema_registry,
        })
        .finish()
}

/// Start GraphQL server
pub async fn start_graphql_server(
    addr: std::net::SocketAddr,
    event_service: BoxedEventService,
    schema_registry: Arc<dyn ContractSchemaRegistry + Send + Sync>,
    enable_playground: bool,
) -> Result<()> {
    info!("Starting GraphQL server on {}", addr);

    // Create schema
    let schema = create_schema(event_service, schema_registry);

    // Create router
    let mut app = Router::new()
        .route("/", get(graphql_handler).post(graphql_handler))
        .with_state(schema);
        
    // Add GraphiQL if enabled
    if enable_playground {
        app = app.route("/graphiql", get(graphiql));
    }

    // Start server
    let listener = tokio::net::TcpListener::bind(&addr).await
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

/// GraphiQL handler
async fn graphiql() -> impl IntoResponse {
    Html(GraphiQLSource::build().endpoint("/").finish())
}

/// GraphQL handler
async fn graphql_handler(
    State(schema): State<GraphQLSchema>,
    req: GraphQLRequest,
) -> GraphQLResponse {
    schema.execute(req.into_inner()).await.into()
} 