use axum::{
    extract::{Path, Query, State, Extension},
    response::IntoResponse,
    Json,
};
use serde::Deserialize;
use utoipa::{ToSchema, IntoParams};
use crate::types::{
    ApiResponseBuilder, EventFilter, ChainId, BlockNumber,
    PaginatedResponse,
    auth::{context::SecurityContext, Operation},
};
use crate::{state::AppState, error::HttpError};

/// Query parameters for event listing
#[derive(Debug, Deserialize, ToSchema, IntoParams)]
pub struct EventQuery {
    /// Filter by chain
    pub chain: Option<String>,
    
    /// Starting block number
    pub from_block: Option<BlockNumber>,
    
    /// Ending block number  
    pub to_block: Option<BlockNumber>,
    
    /// Filter by event type
    pub event_type: Option<String>,
    
    /// Filter by contract address
    pub address: Option<String>,
    
    /// Pagination limit
    #[serde(default = "default_limit")]
    pub limit: u32,
    
    /// Pagination offset
    #[serde(default)]
    pub offset: u32,
}

fn default_limit() -> u32 {
    100
}

/// Latest block response
#[derive(serde::Serialize, ToSchema)]
pub struct LatestBlockResponse {
    /// Chain identifier
    pub chain: String,
    /// Latest block number
    pub latest_block: BlockNumber,
}

/// List events with filtering
#[utoipa::path(
    get,
    path = "/api/v1/events",
    params(EventQuery),
    responses(
        (status = 200, description = "List of events", body = [crate::types::event::ApiEvent]),
        (status = 400, description = "Bad request"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error"),
    ),
    tag = "events"
)]
pub async fn list_events(
    State(state): State<AppState>,
    Query(query): Query<EventQuery>,
    Extension(ctx): Extension<SecurityContext>,
) -> Result<impl IntoResponse, HttpError> {
    // Check basic read permission
    ctx.require_permission(Operation::ReadEvents())
        .map_err(HttpError::from)?;
    // Build filter from query parameters
    let mut filter = EventFilter::new();
    
    if let Some(chain) = &query.chain {
        // Check chain access
        ctx.require_chain_access(chain)
            .map_err(HttpError::from)?;
        filter = filter.chain(chain.clone());
    }
    
    if let (Some(from), Some(to)) = (query.from_block, query.to_block) {
        filter = filter.block_range(from, to);
    }
    
    if let Some(event_type) = query.event_type {
        filter = filter.event_type(event_type);
    }
    
    if let Some(address) = &query.address {
        // Check contract access if filtering by address
        ctx.require_contract_access(address)
            .map_err(HttpError::from)?;
        filter = filter.address(address.clone());
    }
    
    filter = filter.paginate(query.limit, query.offset);
    
    // Get events from storage
    let events = state.storage
        .get_events(filter.clone())
        .await
        .map_err(HttpError::Api)?;
    
    // Check if there are more results
    let _has_more = events.len() >= query.limit as usize;
    
    // Ensure limit is not zero to prevent divide by zero
    let safe_limit = if query.limit == 0 { 1 } else { query.limit };
    
    // Create paginated response
    let paginated = PaginatedResponse::new(
        events,
        u64::MAX, // Total count would require a separate query
        query.offset / safe_limit,
        safe_limit,
    );
    
    let response = ApiResponseBuilder::new(paginated)
        .version(&state.version)
        .build();
    
    Ok(Json(response))
}

/// Get a single event by ID
#[utoipa::path(
    get,
    path = "/api/v1/events/{id}",
    params(
        ("id" = String, Path, description = "Event ID")
    ),
    responses(
        (status = 200, description = "Event details", body = crate::types::event::ApiEvent),
        (status = 404, description = "Event not found"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error"),
    ),
    tag = "events"
)]
pub async fn get_event(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Extension(ctx): Extension<SecurityContext>,
) -> Result<impl IntoResponse, HttpError> {
    // Check basic read permission
    ctx.require_permission(Operation::ReadEvents())
        .map_err(HttpError::from)?;
    
    // For now, we'll search through all events to find by ID
    // In production, you'd want a more efficient lookup
    let filter = EventFilter::new();
    let events = state.storage
        .get_events(filter)
        .await
        .map_err(HttpError::Api)?;
    
    let event = events
        .into_iter()
        .find(|e| e.id == id)
        .ok_or_else(|| HttpError::Api(crate::types::ApiError::NotFound(format!("Event {} not found", id))))?;
    
    let response = ApiResponseBuilder::new(event)
        .version(&state.version)
        .build();
    
    Ok(Json(response))
}

/// List events for a specific chain
#[utoipa::path(
    get,
    path = "/api/v1/chains/{chain}/events",
    params(
        ("chain" = String, Path, description = "Chain identifier"),
        EventQuery
    ),
    responses(
        (status = 200, description = "List of events for the chain", body = [crate::types::event::ApiEvent]),
        (status = 400, description = "Bad request"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error"),
    ),
    tag = "events"
)]
pub async fn list_chain_events(
    State(state): State<AppState>,
    Path(chain): Path<String>,
    Query(mut query): Query<EventQuery>,
    Extension(ctx): Extension<SecurityContext>,
) -> Result<impl IntoResponse, HttpError> {
    // Override chain in query with path parameter
    query.chain = Some(chain);
    list_events(State(state), Query(query), Extension(ctx)).await
}

/// Get latest block for a chain
#[utoipa::path(
    get,
    path = "/api/v1/chains/{chain}/latest-block",
    params(
        ("chain" = String, Path, description = "Chain identifier")
    ),
    responses(
        (status = 200, description = "Latest block information", body = LatestBlockResponse),
        (status = 400, description = "Bad request"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error"),
    ),
    tag = "events"
)]
pub async fn get_latest_block(
    State(state): State<AppState>,
    Path(chain): Path<String>,
    Extension(ctx): Extension<SecurityContext>,
) -> Result<impl IntoResponse, HttpError> {
    // Check read permission and chain access
    ctx.require_permission(Operation::ReadEvents())
        .map_err(HttpError::from)?;
    ctx.require_chain_access(&chain)
        .map_err(HttpError::from)?;
    let chain_id = ChainId::from(chain);
    
    let latest_block = state.storage
        .get_latest_block(&chain_id)
        .await
        .map_err(HttpError::Api)?;
    
    
    let data = LatestBlockResponse {
        chain: chain_id.to_string(),
        latest_block,
    };
    
    let response = ApiResponseBuilder::new(data)
        .version(&state.version)
        .build();
    
    Ok(Json(response))
}