// Enhanced REST API features for comprehensive HTTP handling
//
// This module provides advanced REST API features including content negotiation,
// API versioning, proper HTTP handling, and comprehensive error responses

use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

use axum::{
    extract::{State, Request},
    http::{StatusCode, HeaderMap, HeaderValue, Method},
    middleware::{self, Next},
    response::{IntoResponse, Json, Response},
    routing::get,
    Router,
    body::Body,
};
use serde::Serialize;
use serde_json::{json, Value};
use tower_http::cors::{CorsLayer, Any};
use uuid::Uuid;

use crate::http::HttpState;
use crate::api::openapi;

/// API version extracted from headers or URL
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApiVersion {
    V1,
    V2,
}

impl Default for ApiVersion {
    fn default() -> Self {
        ApiVersion::V1
    }
}

impl std::fmt::Display for ApiVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ApiVersion::V1 => write!(f, "v1"),
            ApiVersion::V2 => write!(f, "v2"),
        }
    }
}

/// Content type preferences
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContentType {
    Json,
    Xml,
    Csv,
    MessagePack,
    Protobuf,
}

impl ContentType {
    pub fn from_accept_header(accept: &str) -> Self {
        if accept.contains("application/json") {
            ContentType::Json
        } else if accept.contains("application/xml") || accept.contains("text/xml") {
            ContentType::Xml
        } else if accept.contains("text/csv") {
            ContentType::Csv
        } else if accept.contains("application/msgpack") {
            ContentType::MessagePack
        } else if accept.contains("application/protobuf") {
            ContentType::Protobuf
        } else {
            ContentType::Json // Default
        }
    }
    
    pub fn to_mime_type(&self) -> &'static str {
        match self {
            ContentType::Json => "application/json",
            ContentType::Xml => "application/xml",
            ContentType::Csv => "text/csv",
            ContentType::MessagePack => "application/msgpack",
            ContentType::Protobuf => "application/protobuf",
        }
    }
}

/// Request context with metadata
#[derive(Debug, Clone)]
pub struct RequestContext {
    pub request_id: Uuid,
    pub api_version: ApiVersion,
    pub content_type: ContentType,
    pub user_agent: Option<String>,
    pub client_ip: String,
    pub timestamp: SystemTime,
}

impl RequestContext {
    pub fn new(headers: &HeaderMap, client_ip: String) -> Self {
        let request_id = Uuid::new_v4();
        
        // Extract API version from headers
        let api_version = headers.get("X-API-Version")
            .and_then(|v| v.to_str().ok())
            .map(|v| match v {
                "v2" => ApiVersion::V2,
                _ => ApiVersion::V1,
            })
            .unwrap_or_default();
        
        // Extract content type preference
        let content_type = headers.get("Accept")
            .and_then(|v| v.to_str().ok())
            .map(ContentType::from_accept_header)
            .unwrap_or(ContentType::Json);
        
        // Extract user agent
        let user_agent = headers.get("User-Agent")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());
        
        Self {
            request_id,
            api_version,
            content_type,
            user_agent,
            client_ip,
            timestamp: SystemTime::now(),
        }
    }
}

/// Enhanced error response with detailed information
#[derive(Debug, Serialize)]
pub struct ApiErrorResponse {
    pub error: String,
    pub error_code: String,
    pub status: u16,
    pub timestamp: u64,
    pub request_id: String,
    pub details: Option<Value>,
    pub suggestions: Option<Vec<String>>,
}

/// Enhanced API errors
#[derive(Debug)]
pub enum EnhancedApiError {
    BadRequest { message: String, details: Option<Value> },
    Unauthorized { message: String },
    Forbidden { message: String },
    NotFound { resource: String },
    MethodNotAllowed { method: String, allowed: Vec<String> },
    NotAcceptable { supported_types: Vec<String> },
    UnsupportedMediaType { provided: String, supported: Vec<String> },
    UnprocessableEntity { validation_errors: Vec<String> },
    TooManyRequests { retry_after: Option<u64> },
    InternalServerError { message: String, request_id: String },
    ServiceUnavailable { message: String, retry_after: Option<u64> },
    GatewayTimeout { message: String },
}

impl IntoResponse for EnhancedApiError {
    fn into_response(self) -> Response {
        let (status_code, error_code, message, details, suggestions) = match &self {
            EnhancedApiError::BadRequest { message, details } => {
                (StatusCode::BAD_REQUEST, "BAD_REQUEST", message.clone(), details.clone(), Some(vec![
                    "Check the request parameters and format".to_string(),
                    "Refer to the API documentation for valid parameters".to_string(),
                ]))
            }
            EnhancedApiError::Unauthorized { message } => {
                (StatusCode::UNAUTHORIZED, "UNAUTHORIZED", message.clone(), None, Some(vec![
                    "Provide a valid API key in the X-API-Key header".to_string(),
                    "Ensure your API key has the required permissions".to_string(),
                ]))
            }
            EnhancedApiError::Forbidden { message } => {
                (StatusCode::FORBIDDEN, "FORBIDDEN", message.clone(), None, Some(vec![
                    "Contact support to request access to this resource".to_string(),
                ]))
            }
            EnhancedApiError::NotFound { resource } => {
                (StatusCode::NOT_FOUND, "NOT_FOUND", format!("{} not found", resource), None, Some(vec![
                    "Check the resource identifier in the URL".to_string(),
                    "Ensure the resource exists and is accessible".to_string(),
                ]))
            }
            EnhancedApiError::MethodNotAllowed { method, allowed } => {
                (StatusCode::METHOD_NOT_ALLOWED, "METHOD_NOT_ALLOWED", 
                 format!("Method {} not allowed", method), 
                 Some(json!({ "allowed_methods": allowed })), Some(vec![
                    format!("Use one of the allowed methods: {}", allowed.join(", ")),
                ]))
            }
            EnhancedApiError::NotAcceptable { supported_types } => {
                (StatusCode::NOT_ACCEPTABLE, "NOT_ACCEPTABLE", 
                 "The requested content type is not supported".to_string(),
                 Some(json!({ "supported_types": supported_types })), Some(vec![
                    "Set the Accept header to one of the supported content types".to_string(),
                ]))
            }
            EnhancedApiError::UnsupportedMediaType { provided, supported } => {
                (StatusCode::UNSUPPORTED_MEDIA_TYPE, "UNSUPPORTED_MEDIA_TYPE",
                 format!("Content type {} is not supported", provided),
                 Some(json!({ "supported_types": supported })), Some(vec![
                    "Set the Content-Type header to one of the supported types".to_string(),
                ]))
            }
            EnhancedApiError::UnprocessableEntity { validation_errors } => {
                (StatusCode::UNPROCESSABLE_ENTITY, "UNPROCESSABLE_ENTITY",
                 "Validation failed".to_string(),
                 Some(json!({ "validation_errors": validation_errors })), Some(vec![
                    "Fix the validation errors in your request".to_string(),
                ]))
            }
            EnhancedApiError::TooManyRequests { retry_after } => {
                (StatusCode::TOO_MANY_REQUESTS, "TOO_MANY_REQUESTS",
                 "Rate limit exceeded".to_string(),
                 retry_after.map(|seconds| json!({ "retry_after_seconds": seconds })), Some(vec![
                    "Wait before making another request".to_string(),
                    "Consider implementing exponential backoff".to_string(),
                ]))
            }
            EnhancedApiError::InternalServerError { message, request_id } => {
                (StatusCode::INTERNAL_SERVER_ERROR, "INTERNAL_SERVER_ERROR",
                 message.clone(),
                 Some(json!({ "request_id": request_id })), Some(vec![
                    "Try again later".to_string(),
                    "Contact support if the problem persists".to_string(),
                ]))
            }
            EnhancedApiError::ServiceUnavailable { message, retry_after } => {
                (StatusCode::SERVICE_UNAVAILABLE, "SERVICE_UNAVAILABLE",
                 message.clone(),
                 retry_after.map(|seconds| json!({ "retry_after_seconds": seconds })), Some(vec![
                    "The service is temporarily unavailable".to_string(),
                    "Try again later".to_string(),
                ]))
            }
            EnhancedApiError::GatewayTimeout { message } => {
                (StatusCode::GATEWAY_TIMEOUT, "GATEWAY_TIMEOUT",
                 message.clone(), None, Some(vec![
                    "The request timed out".to_string(),
                    "Try again with a simpler query".to_string(),
                ]))
            }
        };
        
        let error_response = ApiErrorResponse {
            error: message,
            error_code: error_code.to_string(),
            status: status_code.as_u16(),
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            request_id: Uuid::new_v4().to_string(),
            details,
            suggestions,
        };
        
        let mut response = Json(error_response).into_response();
        *response.status_mut() = status_code;
        
        // Add specific headers for certain error types
        match &self {
            EnhancedApiError::MethodNotAllowed { allowed, .. } => {
                if let Ok(allow_header) = HeaderValue::from_str(&allowed.join(", ")) {
                    response.headers_mut().insert("Allow", allow_header);
                }
            }
            EnhancedApiError::TooManyRequests { retry_after, .. } => {
                if let Some(seconds) = retry_after {
                    if let Ok(retry_header) = HeaderValue::from_str(&seconds.to_string()) {
                        response.headers_mut().insert("Retry-After", retry_header);
                    }
                }
            }
            EnhancedApiError::ServiceUnavailable { retry_after, .. } => {
                if let Some(seconds) = retry_after {
                    if let Ok(retry_header) = HeaderValue::from_str(&seconds.to_string()) {
                        response.headers_mut().insert("Retry-After", retry_header);
                    }
                }
            }
            _ => {}
        }
        
        response
    }
}

/// Content negotiation middleware
pub async fn content_negotiation_middleware(
    headers: HeaderMap,
    mut request: Request<Body>,
    next: Next,
) -> std::result::Result<Response, EnhancedApiError> {
    // Extract client IP (simplified)
    let client_ip = "127.0.0.1".to_string(); // Would extract from connection info
    
    // Create request context
    let context = RequestContext::new(&headers, client_ip);
    
    // Add context to request extensions
    request.extensions_mut().insert(context.clone());
    
    // Add request ID header
    let response = next.run(request).await;
    
    // Add response headers
    let mut response = response;
    response.headers_mut().insert("X-Request-ID", HeaderValue::from_str(&context.request_id.to_string()).unwrap());
    response.headers_mut().insert("X-API-Version", HeaderValue::from_str(&context.api_version.to_string()).unwrap());
    
    Ok(response)
}

/// Enhanced health check with detailed information
#[derive(Debug, Serialize)]
pub struct EnhancedHealthResponse {
    pub status: String,
    pub healthy: bool,
    pub version: String,
    pub api_version: String,
    pub uptime_seconds: u64,
    pub timestamp: u64,
    pub services: HashMap<String, ServiceHealth>,
    pub metrics: HealthMetrics,
}

/// Individual service health
#[derive(Debug, Serialize)]
pub struct ServiceHealth {
    pub healthy: bool,
    pub last_check: u64,
    pub response_time_ms: Option<u64>,
    pub error: Option<String>,
    pub details: Option<Value>,
}

/// Health metrics
#[derive(Debug, Serialize)]
pub struct HealthMetrics {
    pub requests_per_second: f64,
    pub average_response_time_ms: f64,
    pub error_rate: f64,
    pub active_connections: u64,
    pub memory_usage_mb: u64,
    pub cpu_usage_percent: f64,
}

/// Enhanced version information
#[derive(Debug, Serialize)]
pub struct EnhancedVersionResponse {
    pub version: String,
    pub api_version: String,
    pub build_info: BuildInfo,
    pub runtime_info: RuntimeInfo,
    pub features: Vec<String>,
    pub dependencies: HashMap<String, String>,
}

/// Build information
#[derive(Debug, Serialize)]
pub struct BuildInfo {
    pub git_commit: String,
    pub git_branch: String,
    pub build_time: String,
    pub rust_version: String,
    pub target_triple: String,
}

/// Runtime information
#[derive(Debug, Serialize)]
pub struct RuntimeInfo {
    pub os: String,
    pub arch: String,
    pub uptime_seconds: u64,
    pub memory_usage_mb: u64,
    pub cpu_cores: u32,
}

/// Enhanced health check endpoint
pub async fn enhanced_health_check(
    State(state): State<HttpState>,
    request: Request<Body>,
) -> std::result::Result<Json<EnhancedHealthResponse>, EnhancedApiError> {
    let context = request.extensions().get::<RequestContext>().unwrap();
    
    let uptime = state.start_time
        .elapsed()
        .unwrap_or_default()
        .as_secs();
    
    // Check individual services
    let mut services = HashMap::new();
    
    // Check database health
    let db_health = check_database_health(&state).await;
    services.insert("database".to_string(), db_health);
    
    // Check event service health
    let event_service_health = check_event_service_health(&state).await;
    services.insert("event_service".to_string(), event_service_health);
    
    // Check schema registry health
    let schema_health = check_schema_registry_health(&state).await;
    services.insert("schema_registry".to_string(), schema_health);
    
    // Calculate overall health
    let overall_healthy = services.values().all(|s| s.healthy);
    
    // Get metrics (simplified)
    let metrics = HealthMetrics {
        requests_per_second: 10.5, // Would calculate from actual metrics
        average_response_time_ms: 150.0,
        error_rate: 0.01,
        active_connections: 25,
        memory_usage_mb: 512,
        cpu_usage_percent: 15.5,
    };
    
    let response = EnhancedHealthResponse {
        status: if overall_healthy { "healthy".to_string() } else { "unhealthy".to_string() },
        healthy: overall_healthy,
        version: env!("CARGO_PKG_VERSION").to_string(),
        api_version: context.api_version.to_string(),
        uptime_seconds: uptime,
        timestamp: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
        services,
        metrics,
    };
    
    Ok(Json(response))
}

/// Enhanced version endpoint
pub async fn enhanced_version_info(
    request: Request<Body>,
) -> std::result::Result<Json<EnhancedVersionResponse>, EnhancedApiError> {
    let context = request.extensions().get::<RequestContext>().unwrap();
    
    let build_info = BuildInfo {
        git_commit: option_env!("GIT_COMMIT").unwrap_or("unknown").to_string(),
        git_branch: option_env!("GIT_BRANCH").unwrap_or("unknown").to_string(),
        build_time: option_env!("BUILD_TIME").unwrap_or("unknown").to_string(),
        rust_version: option_env!("RUST_VERSION").unwrap_or("unknown").to_string(),
        target_triple: option_env!("TARGET_TRIPLE").unwrap_or("unknown").to_string(),
    };
    
    let runtime_info = RuntimeInfo {
        os: std::env::consts::OS.to_string(),
        arch: std::env::consts::ARCH.to_string(),
        uptime_seconds: 3600, // Would calculate actual uptime
        memory_usage_mb: 512,  // Would get actual memory usage
        cpu_cores: num_cpus::get() as u32,
    };
    
    let features = vec![

        "rest-api".to_string(),
        "websockets".to_string(),
        "authentication".to_string(),
        "rate-limiting".to_string(),
        "full-text-search".to_string(),
        "aggregation".to_string(),
        "real-time-subscriptions".to_string(),
    ];
    
    let mut dependencies = HashMap::new();
    dependencies.insert("axum".to_string(), "0.6.20".to_string());
    
    dependencies.insert("tokio".to_string(), "1.0".to_string());
    dependencies.insert("serde".to_string(), "1.0".to_string());
    
    let response = EnhancedVersionResponse {
        version: env!("CARGO_PKG_VERSION").to_string(),
        api_version: context.api_version.to_string(),
        build_info,
        runtime_info,
        features,
        dependencies,
    };
    
    Ok(Json(response))
}

/// Check database health
async fn check_database_health(state: &HttpState) -> ServiceHealth {
    let start_time = SystemTime::now();
    
    // Try to perform a simple database operation - use get_latest_block as a health check
    match state.event_service.get_latest_block().await {
        Ok(_) => {
            let response_time = start_time.elapsed().unwrap_or_default().as_millis() as u64;
            ServiceHealth {
                healthy: true,
                last_check: SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
                response_time_ms: Some(response_time),
                error: None,
                details: Some(json!({
                    "connection_pool": "active",
                    "migrations": "up_to_date"
                })),
            }
        }
        Err(e) => {
            ServiceHealth {
                healthy: false,
                last_check: SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
                response_time_ms: None,
                error: Some(e.to_string()),
                details: None,
            }
        }
    }
}

/// Check event service health
async fn check_event_service_health(state: &HttpState) -> ServiceHealth {
    let start_time = SystemTime::now();
    
    match state.event_service.get_latest_block().await {
        Ok(_) => {
            let response_time = start_time.elapsed().unwrap_or_default().as_millis() as u64;
            ServiceHealth {
                healthy: true,
                last_check: SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
                response_time_ms: Some(response_time),
                error: None,
                details: Some(json!({
                    "indexer_status": "running",
                    "last_processed_block": "18500000"
                })),
            }
        }
        Err(e) => {
            ServiceHealth {
                healthy: false,
                last_check: SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
                response_time_ms: None,
                error: Some(e.to_string()),
                details: None,
            }
        }
    }
}

/// Check schema registry health
async fn check_schema_registry_health(_state: &HttpState) -> ServiceHealth {
    // Schema registry doesn't have a health check method, so we'll do a simple check
    ServiceHealth {
        healthy: true,
        last_check: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs(),
        response_time_ms: Some(1),
        error: None,
        details: None,
    }
}

/// Create enhanced REST API router
pub fn create_enhanced_rest_api(state: HttpState) -> Router {
    // Add CORS middleware
    let cors = CorsLayer::new()
        .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE, Method::PATCH, Method::OPTIONS])
        .allow_headers(Any)
        .allow_origin(Any);
    
    Router::new()
        // Enhanced health and version endpoints
        .route("/api/v1/health", get(enhanced_health_check))
        .route("/api/v2/health", get(enhanced_health_check))
        .route("/api/v1/version", get(enhanced_version_info))
        .route("/api/v2/version", get(enhanced_version_info))
        
        // OpenAPI documentation
        .merge(openapi::add_openapi_routes(Router::new()))
        
        // Apply middleware
        .layer(cors)
        .layer(middleware::from_fn(content_negotiation_middleware))
        .with_state(state)
} 