/// GraphQL API server implementation
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use async_graphql::{
    Context, EmptyMutation, EmptySubscription, Object, Schema, SimpleObject, ID,
};
use async_graphql_axum::{GraphQLRequest, GraphQLResponse};
use axum::{
    extract::State,
    response::IntoResponse,
    routing::get,
    Router,
};
use serde::{Deserialize, Serialize};

use indexer_core::Result;
use indexer_core::event::Event;
use indexer_storage::{Storage, EventFilter};

use crate::ApiConfig;

/// GraphQL server state
#[derive(Clone)]
pub struct GraphQLServerState {
    /// Storage backend
    pub storage: Arc<dyn Storage>,
    
    /// Schema
    pub schema: Schema<QueryRoot, EmptyMutation, EmptySubscription>,
}

/// Start the GraphQL server
pub async fn start_graphql_server(
    config: &ApiConfig,
    storage: Arc<dyn Storage>,
) -> Result<()> {
    // Create the schema
    let schema = Schema::build(QueryRoot, EmptyMutation, EmptySubscription)
        .data(storage.clone())
        .finish();
    
    // Create the server state
    let state = GraphQLServerState {
        storage,
        schema,
    };
    
    // Create the router
    let app = Router::new()
        .route("/graphql", get(graphql_playground).post(graphql_handler))
        .with_state(state);
    
    // Parse the address
    // For GraphQL, we'll use the same host but a different port
    let port = config.port + 1;
    let addr: SocketAddr = format!("{}:{}", config.host, port)
        .parse()
        .map_err(|e| indexer_core::Error::generic(format!("Failed to parse address: {}", e)))?;
    
    // Start the server
    tracing::info!("Starting GraphQL server on {}", addr);
    
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .map_err(|e| indexer_core::Error::generic(format!("Failed to start GraphQL server: {}", e)))?;
    
    Ok(())
}

/// GraphQL playground handler
async fn graphql_playground() -> impl IntoResponse {
    axum::response::Html(async_graphql::http::playground_source(
        async_graphql::http::GraphQLPlaygroundConfig::new("/graphql"),
    ))
}

/// GraphQL handler
async fn graphql_handler(
    State(state): State<GraphQLServerState>,
    req: GraphQLRequest,
) -> GraphQLResponse {
    state.schema.execute(req.into_inner()).await.into()
}

/// GraphQL query root
pub struct QueryRoot;

#[Object]
impl QueryRoot {
    /// Get events
    async fn events(
        &self,
        ctx: &Context<'_>,
        chain: Option<String>,
        block_start: Option<u64>,
        block_end: Option<u64>,
        time_start: Option<u64>,
        time_end: Option<u64>,
        event_types: Option<Vec<String>>,
        limit: Option<usize>,
        offset: Option<usize>,
    ) -> async_graphql::Result<Vec<EventObject>> {
        let storage = ctx.data::<Arc<dyn Storage>>()
            .map_err(|e| async_graphql::Error::new(e.message.clone()))?;
        
        let filter = EventFilter {
            chain,
            block_range: if block_start.is_some() || block_end.is_some() {
                Some((
                    block_start.unwrap_or(0),
                    block_end.unwrap_or(u64::MAX),
                ))
            } else {
                None
            },
            time_range: if time_start.is_some() || time_end.is_some() {
                Some((
                    time_start.unwrap_or(0),
                    time_end.unwrap_or(u64::MAX),
                ))
            } else {
                None
            },
            event_types,
            limit,
            offset,
        };
        
        let events = storage.get_events(vec![filter])
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;
        
        let event_objects = events.iter()
            .map(|event| EventObject::from_event(&event))
            .collect();
        
        Ok(event_objects)
    }
    
    /// Get event by ID
    async fn event(
        &self,
        ctx: &Context<'_>,
        chain: String,
        id: ID,
    ) -> async_graphql::Result<Option<EventObject>> {
        let storage = ctx.data::<Arc<dyn Storage>>()
            .map_err(|e| async_graphql::Error::new(e.message.clone()))?;
        
        let filter = EventFilter {
            chain: Some(chain),
            block_range: None,
            time_range: None,
            event_types: None,
            limit: Some(1),
            offset: None,
        };
        
        let events = storage.get_events(vec![filter])
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;
        
        if let Some(event) = events.first() {
            Ok(Some(EventObject::from_event(&event)))
        } else {
            Ok(None)
        }
    }
    
    /// Get latest block for a chain
    async fn latest_block(
        &self,
        ctx: &Context<'_>,
        chain: String,
    ) -> async_graphql::Result<u64> {
        let storage = ctx.data::<Arc<dyn Storage>>()
            .map_err(|e| async_graphql::Error::new(e.message.clone()))?;
        
        let latest_block = storage.get_latest_block(&chain)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;
        
        Ok(latest_block)
    }
}

/// Event object for GraphQL
#[derive(SimpleObject)]
struct EventObject {
    /// Event ID
    id: ID,
    
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

impl EventObject {
    /// Create a new event object from an event
    fn from_event(event: &Box<dyn Event>) -> Self {
        Self {
            id: ID(event.id().to_string()),
            chain: event.chain().to_string(),
            block_number: event.block_number(),
            block_hash: event.block_hash().to_string(),
            tx_hash: event.tx_hash().to_string(),
            timestamp: event.timestamp().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs(),
            event_type: event.event_type().to_string(),
            data: base64::encode(event.raw_data().to_vec()),
        }
    }
} 