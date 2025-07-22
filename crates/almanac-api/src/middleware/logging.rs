//! Logging and tracing middleware

use axum::{extract::Request, middleware::Next, response::Response};
use std::time::Instant;

/// Log request details
pub async fn log_request(request: Request, next: Next) -> Response {
    let method = request.method().clone();
    let uri = request.uri().clone();
    let start = Instant::now();

    // Create span for this request
    let span = tracing::info_span!(
        "http_request",
        method = %method,
        uri = %uri,
    );

    // Execute request within span
    let response = {
        let _guard = span.enter();
        next.run(request).await
    };

    let status = response.status();
    let duration = start.elapsed();

    // Log completion
    tracing::info!(
        target: "http",
        status = %status,
        duration_ms = %duration.as_millis(),
        "Request completed"
    );

    response
}

/// Add request tracing span
pub fn tracing<B>() -> tower_http::trace::TraceLayer<
    tower_http::classify::SharedClassifier<tower_http::classify::ServerErrorsAsFailures>,
>
where
    B: axum::body::HttpBody,
{
    tower_http::trace::TraceLayer::new_for_http()
}
