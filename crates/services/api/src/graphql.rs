// Enhanced GraphQL API implementation with query complexity analysis, pagination, subscriptions, and versioning
//
// This module provides the primary GraphQL API interface for the Almanac indexer
// with advanced features for production use

use std::sync::Arc;
use std::collections::HashMap;
use std::pin::Pin;
use std::task::{Context as TaskContext, Poll};
use std::time::Duration;

use async_graphql::{
    Context, Schema, SimpleObject, Object, InputObject, Subscription, Enum, ID, Scalar, ScalarType,
    http::GraphiQLSource, connection::{Connection, Edge, query}, 
    InputValueError, InputValueResult, Value as GraphQLValue,
    ComplexityResult, VisitorContext, Positioned, SelectionField, ComplexObject,
    extensions::{Extension, ExtensionFactory, ExtensionContext, NextParseQuery, NextValidation, NextExecute},
    parser::types::ExecutableDocument, ValidationResult, ServerResult, Request, Response,
    futures_util::Stream,
};
use async_graphql_axum::{GraphQLRequest, GraphQLResponse, GraphQLSubscription};
use axum::{
    response::{IntoResponse, Html},
    routing::get,
    Router, extract::{State, WebSocketUpgrade},
    http::Method,
};
use serde_json::Value as JsonValue;
use tracing::{info, warn, error};
use futures_util::StreamExt;
use tokio::sync::broadcast;
use tokio_stream::wrappers::BroadcastStream;
use uuid::Uuid;
use chrono::{DateTime, Utc};

use indexer_core::{Error, Result};
use indexer_core::service::BoxedEventService;
use indexer_core::types::Event;
use crate::{
    ContractSchemaVersion, ContractSchema, EventSchema, FunctionSchema, FieldSchema,
    ContractSchemaRegistry,
};

/// API versioning
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApiVersion {
    V1,
    V2,
}

impl ApiVersion {
    pub fn from_header(version: Option<&str>) -> Self {
        match version {
            Some("v2") => ApiVersion::V2,
            _ => ApiVersion::V1, // Default to v1
        }
    }
}

/// Query complexity analyzer
#[derive(Clone, Debug)]
pub struct QueryComplexityAnalyzer {
    max_complexity: u32,
    max_depth: u32,
}

impl QueryComplexityAnalyzer {
    pub fn new(max_complexity: u32, max_depth: u32) -> Self {
        Self { max_complexity, max_depth }
    }
}

impl ExtensionFactory for QueryComplexityAnalyzer {
    fn create(&self) -> Arc<dyn Extension> {
        Arc::new(QueryComplexityExtension {
            max_complexity: self.max_complexity,
            max_depth: self.max_depth,
        })
    }
}

#[derive(Debug)]
struct QueryComplexityExtension {
    max_complexity: u32,
    max_depth: u32,
}

#[async_trait::async_trait]
impl Extension for QueryComplexityExtension {
    async fn validation(
        &self,
        ctx: &ExtensionContext<'_>,
        next: NextValidation<'_>,
    ) -> ValidationResult {
        let result = next.run(ctx).await;
        
        // Analyze complexity
        let complexity = self.calculate_complexity(ctx.document());
        let depth = self.calculate_depth(ctx.document());
        
        if complexity > self.max_complexity {
            return ValidationResult::new(vec![
                async_graphql::ServerError::new(
                    format!("Query complexity {} exceeds maximum of {}", complexity, self.max_complexity),
                    Some(async_graphql::ErrorExtensions::new())
                )
            ]);
        }
        
        if depth > self.max_depth {
            return ValidationResult::new(vec![
                async_graphql::ServerError::new(
                    format!("Query depth {} exceeds maximum of {}", depth, self.max_depth),
                    Some(async_graphql::ErrorExtensions::new())
                )
            ]);
        }
        
        result
    }
}

impl QueryComplexityExtension {
    fn calculate_complexity(&self, _document: &ExecutableDocument) -> u32 {
        // Simplified complexity calculation
        // In a real implementation, this would analyze the query structure
        10 // Placeholder
    }
    
    fn calculate_depth(&self, _document: &ExecutableDocument) -> u32 {
        // Simplified depth calculation
        // In a real implementation, this would analyze the query nesting
        5 // Placeholder
    }
}

/// Pagination information
#[derive(SimpleObject)]
pub struct PageInfo {
    /// Whether there are more pages
    pub has_next_page: bool,
    /// Whether there are previous pages
    pub has_previous_page: bool,
    /// Start cursor
    pub start_cursor: Option<String>,
    /// End cursor
    pub end_cursor: Option<String>,
}

/// Connection wrapper for paginated results
#[derive(SimpleObject)]
pub struct EventConnection {
    /// Page information
    pub page_info: PageInfo,
    /// Event edges
    pub edges: Vec<EventEdge>,
    /// Total count
    pub total_count: Option<i32>,
}

/// Event edge for pagination
#[derive(SimpleObject)]
pub struct EventEdge {
    /// Event node
    pub node: GraphQLEvent,
    /// Cursor for this edge
    pub cursor: String,
}

/// Subscription event types
#[derive(Clone, Debug)]
pub enum SubscriptionEvent {
    NewEvent(GraphQLEvent),
    NewBlock(ChainBlock),
    HealthUpdate(HealthStatus),
}

/// Event stream for subscriptions
pub struct EventStream {
    receiver: broadcast::Receiver<SubscriptionEvent>,
}

impl Stream for EventStream {
    type Item = SubscriptionEvent;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut TaskContext<'_>) -> Poll<Option<Self::Item>> {
        match self.receiver.poll_recv(cx) {
            Poll::Ready(Ok(event)) => Poll::Ready(Some(event)),
            Poll::Ready(Err(broadcast::error::RecvError::Closed)) => Poll::Ready(None),
            Poll::Ready(Err(broadcast::error::RecvError::Lagged(_))) => {
                // Skip lagged messages and continue
                cx.waker().wake_by_ref();
                Poll::Pending
            }
            Poll::Pending => Poll::Pending,
        }
    }
}

/// JSON scalar for GraphQL
#[derive(Clone)]
struct Json(JsonValue);

/// Custom scalar for JSON
#[Scalar]
impl ScalarType for Json {
    fn parse(value: GraphQLValue) -> InputValueResult<Self> {
        if let GraphQLValue::Object(obj) = &value {
            // Convert GraphQL value to serde_json value
            let json_value = serde_json::to_value(obj).map_err(|_| InputValueError::expected_type(value))?;
            Ok(Json(json_value))
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
pub type GraphQLSchema = Schema<QueryRoot, MutationRoot, SubscriptionRoot>;

/// Query root with enhanced features
pub struct QueryRoot;

#[Object]
impl QueryRoot {
    /// Get an event by ID
    async fn event(&self, ctx: &Context<'_>, id: ID) -> async_graphql::Result<Option<GraphQLEvent>> {
        let state = ctx.data::<AppState>()?;
        let version = ctx.data::<ApiVersion>().unwrap_or(&ApiVersion::V1);
        
        // Enhanced implementation with actual service call
        match state.event_service.get_event(&id).await {
            Ok(Some(event)) => {
                let graphql_event = self.convert_event_to_graphql(&event, version).await?;
                Ok(Some(graphql_event))
            }
            Ok(None) => Ok(None),
            Err(e) => Err(async_graphql::Error::new(format!("Failed to get event: {}", e))),
        }
    }

    /// Query events with enhanced pagination
    async fn events(
        &self, 
        ctx: &Context<'_>,
        filter: Option<EventFilterInput>,
        first: Option<i32>,
        after: Option<String>,
        last: Option<i32>,
        before: Option<String>,
    ) -> async_graphql::Result<EventConnection> {
        let state = ctx.data::<AppState>()?;
        let version = ctx.data::<ApiVersion>().unwrap_or(&ApiVersion::V1);
        
        // Validate pagination parameters
        let limit = first.or(last).unwrap_or(10).min(100) as usize; // Max 100 items
        
        // Build query parameters
        let mut query_params = HashMap::new();
        
        if let Some(filter) = filter {
            if let Some(chain) = filter.chain {
                query_params.insert("chain".to_string(), chain);
            }
            if let Some(event_types) = filter.event_types {
                query_params.insert("event_types".to_string(), event_types.join(","));
            }
            if let Some(block_range) = filter.block_range {
                if block_range.len() == 2 {
                    query_params.insert("block_from".to_string(), block_range[0].to_string());
                    query_params.insert("block_to".to_string(), block_range[1].to_string());
                }
            }
        }
        
        // Handle cursor-based pagination
        let offset = if let Some(after) = after {
            self.decode_cursor(&after)? + 1
        } else if let Some(before) = before {
            self.decode_cursor(&before).saturating_sub(limit)
        } else {
            0
        };
        
        // Get events from service
        let events = state.event_service.get_events(limit, offset, query_params).await
            .map_err(|e| async_graphql::Error::new(format!("Failed to get events: {}", e)))?;
        
        // Convert to GraphQL events
        let mut graphql_events = Vec::new();
        for event in events {
            let graphql_event = self.convert_event_to_graphql(&event, version).await?;
            graphql_events.push(graphql_event);
        }
        
        // Create edges with cursors
        let edges: Vec<EventEdge> = graphql_events
            .into_iter()
            .enumerate()
            .map(|(i, event)| EventEdge {
                cursor: self.encode_cursor(offset + i),
                node: event,
            })
            .collect();
        
        // Create page info
        let has_next_page = edges.len() == limit;
        let has_previous_page = offset > 0;
        let start_cursor = edges.first().map(|e| e.cursor.clone());
        let end_cursor = edges.last().map(|e| e.cursor.clone());
        
        Ok(EventConnection {
            page_info: PageInfo {
                has_next_page,
                has_previous_page,
                start_cursor,
                end_cursor,
            },
            edges,
            total_count: None, // Could be expensive to calculate
        })
    }

    /// Get latest block with enhanced features
    async fn latest_block(&self, ctx: &Context<'_>, chain: Option<String>) -> async_graphql::Result<Option<ChainBlock>> {
        let state = ctx.data::<AppState>()?;
        let chain = chain.unwrap_or_else(|| "ethereum".to_string());
        
        // Get latest block from service
        match state.event_service.get_latest_block(&chain).await {
            Ok(Some(block)) => {
                let graphql_block = ChainBlock {
                    chain: block.chain_id,
                    number: block.number as i64,
                    hash: block.hash,
                    timestamp: block.timestamp,
                    finality_status: GraphQLFinalityStatus::Confirmed,
                };
                Ok(Some(graphql_block))
            }
            Ok(None) => Ok(None),
            Err(e) => Err(async_graphql::Error::new(format!("Failed to get latest block: {}", e))),
        }
    }

    /// Get latest block with status
    async fn latest_block_with_status(
        &self, 
        ctx: &Context<'_>,
        chain: Option<String>,
        status: GraphQLFinalityStatus
    ) -> async_graphql::Result<Option<ChainBlock>> {
        let state = ctx.data::<AppState>()?;
        let chain = chain.unwrap_or_else(|| "ethereum".to_string());
        
        // Get latest block with specific status
        match state.event_service.get_latest_block_with_status(&chain, status.into()).await {
            Ok(Some(block)) => {
                let graphql_block = ChainBlock {
                    chain: block.chain_id,
                    number: block.number as i64,
                    hash: block.hash,
                    timestamp: block.timestamp,
                    finality_status: status,
                };
                Ok(Some(graphql_block))
            }
            Ok(None) => Ok(None),
            Err(e) => Err(async_graphql::Error::new(format!("Failed to get latest block with status: {}", e))),
        }
    }

    /// Enhanced health check with version info
    async fn health(&self, ctx: &Context<'_>) -> async_graphql::Result<HealthStatus> {
        let state = ctx.data::<AppState>()?;
        let version = ctx.data::<ApiVersion>().unwrap_or(&ApiVersion::V1);
        
        // Get health status from service
        match state.event_service.get_health().await {
            Ok(health) => {
                Ok(HealthStatus {
                    healthy: health.healthy,
                    version: match version {
                        ApiVersion::V1 => format!("v1.{}", env!("CARGO_PKG_VERSION")),
                        ApiVersion::V2 => format!("v2.{}", env!("CARGO_PKG_VERSION")),
                    },
                    uptime_seconds: health.uptime_seconds,
                    api_version: match version {
                        ApiVersion::V1 => "v1".to_string(),
                        ApiVersion::V2 => "v2".to_string(),
                    },
                })
            }
            Err(e) => Err(async_graphql::Error::new(format!("Failed to get health: {}", e))),
        }
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
            match state.schema_registry.get_schema(&chain, &address, &version) {
                Ok(Some(schema)) => Ok(Some(schema)),
                Ok(None) => Ok(None),
                Err(e) => Err(async_graphql::Error::new(format!("Failed to get schema: {}", e))),
            }
        } else {
            // Get the latest schema
            match state.schema_registry.get_latest_schema(&chain, &address) {
                Ok(Some(schema)) => Ok(Some(schema)),
                Ok(None) => Ok(None),
                Err(e) => Err(async_graphql::Error::new(format!("Failed to get latest schema: {}", e))),
            }
        }
    }
    
    /// Get all registered contract schemas with pagination
    async fn contract_schemas(
        &self,
        ctx: &Context<'_>,
        chain: Option<String>,
        first: Option<i32>,
        after: Option<String>,
    ) -> async_graphql::Result<Vec<ContractSchemaVersion>> {
        let state = ctx.data::<AppState>()?;
        let limit = first.unwrap_or(10).min(100) as usize;
        let offset = if let Some(after) = after {
            self.decode_cursor(&after)? + 1
        } else {
            0
        };
        
        // Get paginated schemas
        match state.schema_registry.get_schemas(chain, limit, offset) {
            Ok(schemas) => Ok(schemas),
            Err(e) => Err(async_graphql::Error::new(format!("Failed to get schemas: {}", e))),
        }
    }
}

impl QueryRoot {
    /// Convert internal event to GraphQL event
    async fn convert_event_to_graphql(&self, event: &dyn Event, version: &ApiVersion) -> async_graphql::Result<GraphQLEvent> {
        let attributes = match version {
            ApiVersion::V1 => event.attributes().clone(),
            ApiVersion::V2 => {
                // Enhanced attributes for v2
                let mut attrs = event.attributes().clone();
                attrs.insert("api_version".to_string(), serde_json::Value::String("v2".to_string()));
                attrs
            }
        };
        
        Ok(GraphQLEvent {
            id: event.id().into(),
            chain: event.chain_id().to_string(),
            block_number: event.block_number() as i64,
            block_hash: event.block_hash().to_string(),
            tx_hash: event.transaction_hash().to_string(),
            timestamp: event.timestamp(),
            event_type: event.event_type().to_string(),
            data: base64::encode(event.data()),
            attributes: if attributes.is_empty() { None } else { Some(serde_json::to_value(attributes).unwrap_or_default()) },
        })
    }
    
    /// Encode cursor for pagination
    fn encode_cursor(&self, offset: usize) -> String {
        base64::encode(offset.to_string())
    }
    
    /// Decode cursor for pagination
    fn decode_cursor(&self, cursor: &str) -> async_graphql::Result<usize> {
        let decoded = base64::decode(cursor)
            .map_err(|_| async_graphql::Error::new("Invalid cursor format"))?;
        let offset_str = String::from_utf8(decoded)
            .map_err(|_| async_graphql::Error::new("Invalid cursor encoding"))?;
        offset_str.parse::<usize>()
            .map_err(|_| async_graphql::Error::new("Invalid cursor value"))
    }
}

/// Enhanced GraphQL event type
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
    timestamp: DateTime<Utc>,
    /// Event type
    event_type: String,
    /// Event data as base64
    data: String,
    /// Custom attributes
    attributes: Option<JsonValue>,
}

/// Enhanced event filter input
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
    timestamp: DateTime<Utc>,
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

/// Enhanced health status
#[derive(SimpleObject)]
struct HealthStatus {
    /// Is the API healthy
    healthy: bool,
    /// API version
    version: String,
    /// Uptime in seconds
    uptime_seconds: i64,
    /// API version identifier
    api_version: String,
}

/// Mutation root with enhanced features
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
            version: input.version.clone(),
            schema: ContractSchema {
                chain: input.chain_id.clone(),
                address: input.contract_address.clone(),
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
        let state = ctx.data::<AppState>()?;
        
        // Enhanced deletion with actual implementation
        match state.schema_registry.delete_schema(&chain, &address, &version) {
            Ok(_) => Ok(DeleteSchemaResult {
                success: true,
                error: None,
            }),
            Err(e) => Ok(DeleteSchemaResult {
                success: false,
                error: Some(e.to_string()),
            }),
        }
    }
    
    /// Ping with enhanced response
    async fn ping(&self, ctx: &Context<'_>) -> async_graphql::Result<String> {
        let version = ctx.data::<ApiVersion>().unwrap_or(&ApiVersion::V1);
        Ok(format!("pong from API {}", match version {
            ApiVersion::V1 => "v1",
            ApiVersion::V2 => "v2",
        }))
    }
}

/// Enhanced subscription root
pub struct SubscriptionRoot;

#[Subscription]
impl SubscriptionRoot {
    /// Subscribe to new events
    async fn event_stream(
        &self,
        ctx: &Context<'_>,
        filter: Option<EventFilterInput>,
    ) -> impl Stream<Item = async_graphql::Result<GraphQLEvent>> {
        let state = ctx.data::<AppState>().unwrap();
        let event_stream = state.event_broadcaster.subscribe();
        
        BroadcastStream::new(event_stream)
            .filter_map(move |event| async move {
                match event {
                    Ok(SubscriptionEvent::NewEvent(event)) => {
                        // Apply filter if provided
                        if let Some(ref filter) = filter {
                            if let Some(ref chain) = filter.chain {
                                if event.chain != *chain {
                                    return None;
                                }
                            }
                            if let Some(ref event_types) = filter.event_types {
                                if !event_types.contains(&event.event_type) {
                                    return None;
                                }
                            }
                        }
                        Some(Ok(event))
                    }
                    _ => None,
                }
            })
    }
    
    /// Subscribe to new blocks
    async fn block_stream(
        &self,
        ctx: &Context<'_>,
        chain: Option<String>,
    ) -> impl Stream<Item = async_graphql::Result<ChainBlock>> {
        let state = ctx.data::<AppState>().unwrap();
        let event_stream = state.event_broadcaster.subscribe();
        
        BroadcastStream::new(event_stream)
            .filter_map(move |event| async move {
                match event {
                    Ok(SubscriptionEvent::NewBlock(block)) => {
                        // Apply chain filter if provided
                        if let Some(ref chain_filter) = chain {
                            if block.chain != *chain_filter {
                                return None;
                            }
                        }
                        Some(Ok(block))
                    }
                    _ => None,
                }
            })
    }
    
    /// Subscribe to health updates
    async fn health_stream(
        &self,
        ctx: &Context<'_>,
    ) -> impl Stream<Item = async_graphql::Result<HealthStatus>> {
        let state = ctx.data::<AppState>().unwrap();
        let event_stream = state.event_broadcaster.subscribe();
        
        BroadcastStream::new(event_stream)
            .filter_map(|event| async move {
                match event {
                    Ok(SubscriptionEvent::HealthUpdate(health)) => Some(Ok(health)),
                    _ => None,
                }
            })
    }
}

/// Enhanced GraphQL application state
pub struct AppState {
    /// Event service
    pub event_service: BoxedEventService,
    
    /// Schema registry
    pub schema_registry: Arc<dyn ContractSchemaRegistry + Send + Sync>,
    
    /// Event broadcaster for subscriptions
    pub event_broadcaster: broadcast::Sender<SubscriptionEvent>,
}

/// Create enhanced GraphQL schema
pub fn create_schema(
    event_service: BoxedEventService,
    schema_registry: Arc<dyn ContractSchemaRegistry + Send + Sync>,
    event_broadcaster: broadcast::Sender<SubscriptionEvent>,
) -> GraphQLSchema {
    Schema::build(QueryRoot, MutationRoot, SubscriptionRoot)
        .data(AppState { 
            event_service,
            schema_registry,
            event_broadcaster,
        })
        .extension(QueryComplexityAnalyzer::new(100, 10)) // Max complexity 100, max depth 10
        .finish()
}

/// Start enhanced GraphQL server
pub async fn start_graphql_server(
    addr: std::net::SocketAddr,
    event_service: BoxedEventService,
    schema_registry: Arc<dyn ContractSchemaRegistry + Send + Sync>,
    enable_playground: bool,
) -> Result<()> {
    info!("Starting enhanced GraphQL server on {}", addr);

    // Create event broadcaster for subscriptions
    let (event_broadcaster, _) = broadcast::channel(1000);

    // Create schema
    let schema = create_schema(event_service, schema_registry, event_broadcaster.clone());

    // Create router with enhanced features
    let mut app = Router::new()
        .route("/", get(graphql_handler).post(graphql_handler))
        .route("/ws", get(graphql_subscription_handler))
        .with_state(schema);
        
    // Add GraphiQL if enabled
    if enable_playground {
        app = app.route("/graphiql", get(graphiql));
    }

    // Start server
    let listener = tokio::net::TcpListener::bind(&addr).await
        .map_err(|e| Error::api(format!("Failed to bind to address: {}", e)))?;
    
    info!("GraphQL server started with subscriptions enabled");
    
    // Start server
    axum::serve(listener, app)
        .await
        .map_err(|e| Error::api(format!("Server error: {}", e)))?;
    
    Ok(())
}

/// GraphiQL handler
async fn graphiql() -> impl IntoResponse {
    Html(GraphiQLSource::build().endpoint("/").subscription_endpoint("/ws").finish())
}

/// Enhanced GraphQL handler with versioning
async fn graphql_handler(
    State(schema): State<GraphQLSchema>,
    headers: axum::http::HeaderMap,
    req: GraphQLRequest,
) -> GraphQLResponse {
    // Extract API version from headers
    let version = headers.get("X-API-Version")
        .and_then(|v| v.to_str().ok())
        .map(ApiVersion::from_header)
        .unwrap_or(ApiVersion::V1);
    
    // Execute with version context
    let request = req.into_inner().data(version);
    schema.execute(request).await.into()
}

/// GraphQL subscription handler
async fn graphql_subscription_handler(
    State(schema): State<GraphQLSchema>,
    headers: axum::http::HeaderMap,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    // Extract API version from headers
    let version = headers.get("X-API-Version")
        .and_then(|v| v.to_str().ok())
        .map(ApiVersion::from_header)
        .unwrap_or(ApiVersion::V1);
    
    ws.on_upgrade(move |socket| {
        GraphQLSubscription::new(schema)
            .data(version)
            .serve(socket)
    })
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

impl From<GraphQLFinalityStatus> for indexer_core::types::FinalityLevel {
    fn from(status: GraphQLFinalityStatus) -> Self {
        match status {
            GraphQLFinalityStatus::Confirmed => indexer_core::types::FinalityLevel::Confirmed,
            GraphQLFinalityStatus::Safe => indexer_core::types::FinalityLevel::Safe,
            GraphQLFinalityStatus::Justified => indexer_core::types::FinalityLevel::Justified,
            GraphQLFinalityStatus::Finalized => indexer_core::types::FinalityLevel::Finalized,
        }
    }
} 