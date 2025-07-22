//! HTTP middleware components

pub mod auth;
pub mod common;
pub mod logging;

// Re-export auth middleware
pub use auth::{auth_middleware, optional_auth_middleware, AuthState};

// Re-export common middleware
pub use common::{add_headers, body_limit, cors, handle_error, request_id, timeout};

// Re-export logging middleware
pub use logging::{log_request, tracing};
