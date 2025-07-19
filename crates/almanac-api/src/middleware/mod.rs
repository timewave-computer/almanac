//! HTTP middleware components

pub mod auth;
pub mod common;
pub mod logging;

// Re-export auth middleware
pub use auth::{
    auth_middleware,
    optional_auth_middleware,
    AuthState,
};

// Re-export common middleware
pub use common::{
    handle_error,
    add_headers,
    body_limit,
    timeout,
    cors,
    request_id,
};

// Re-export logging middleware
pub use logging::{
    log_request,
    tracing,
};