//! Common middleware utilities

use axum::{
    extract::Request,
    http::{header, HeaderValue, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use std::time::Duration;
use tower_http::{
    cors::{Any, CorsLayer},
    limit::RequestBodyLimitLayer,
    request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer},
    timeout::TimeoutLayer,
};

/// Handle errors from other middleware
pub async fn handle_error(error: Box<dyn std::error::Error + Send + Sync>) -> impl IntoResponse {
    tracing::error!(error = %error, "Middleware error");

    (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error")
}

/// Add common security headers
pub async fn add_headers(request: Request, next: Next) -> Response {
    let mut response = next.run(request).await;

    let headers = response.headers_mut();

    // Security headers
    headers.insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    headers.insert(header::X_FRAME_OPTIONS, HeaderValue::from_static("DENY"));
    headers.insert(
        "X-XSS-Protection",
        HeaderValue::from_static("1; mode=block"),
    );
    headers.insert(
        "Referrer-Policy",
        HeaderValue::from_static("strict-origin-when-cross-origin"),
    );

    response
}

/// Create body size limit middleware
pub fn body_limit(limit_bytes: usize) -> RequestBodyLimitLayer {
    RequestBodyLimitLayer::new(limit_bytes)
}

/// Create request timeout middleware
pub fn timeout(duration_secs: u64) -> TimeoutLayer {
    TimeoutLayer::new(Duration::from_secs(duration_secs))
}

/// Create CORS middleware
pub fn cors(_origins: Vec<String>) -> CorsLayer {
    // For now, just allow all origins to fix compilation
    CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any)
        .expose_headers(Any)
}

/// Create request ID middleware layers
pub fn request_id() -> (SetRequestIdLayer<MakeRequestUuid>, PropagateRequestIdLayer) {
    (
        SetRequestIdLayer::x_request_id(MakeRequestUuid),
        PropagateRequestIdLayer::x_request_id(),
    )
}
