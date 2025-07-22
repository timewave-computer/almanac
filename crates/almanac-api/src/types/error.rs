use crate::types::response::ApiResponse;
use axum::{response::IntoResponse, Json};
use http::StatusCode;
use thiserror::Error;

/// Result type for API operations
pub type ApiResult<T> = Result<T, ApiError>;

/// Common API error type with HTTP status mappings
#[derive(Error, Debug)]
pub enum ApiError {
    #[error("Bad request: {0}")]
    BadRequest(String),

    #[error("Unauthorized: {0}")]
    Unauthorized(String),

    #[error("Forbidden: {0}")]
    Forbidden(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Conflict: {0}")]
    Conflict(String),

    #[error("Rate limit exceeded")]
    RateLimitExceeded,

    #[error("Internal server error: {0}")]
    InternalError(String),

    #[error("Service unavailable: {0}")]
    ServiceUnavailable(String),

    #[error("Gateway timeout")]
    GatewayTimeout,

    #[error("Validation error: {field}: {message}")]
    ValidationError { field: String, message: String },

    #[error("Serialization error: {0}")]
    SerializationError(String),

    #[error("Storage error: {0}")]
    StorageError(String),
}

impl ApiError {
    /// Get the HTTP status code for this error
    pub fn status_code(&self) -> StatusCode {
        match self {
            ApiError::BadRequest(_) => StatusCode::BAD_REQUEST,
            ApiError::Unauthorized(_) => StatusCode::UNAUTHORIZED,
            ApiError::Forbidden(_) => StatusCode::FORBIDDEN,
            ApiError::NotFound(_) => StatusCode::NOT_FOUND,
            ApiError::Conflict(_) => StatusCode::CONFLICT,
            ApiError::RateLimitExceeded => StatusCode::TOO_MANY_REQUESTS,
            ApiError::InternalError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            ApiError::ServiceUnavailable(_) => StatusCode::SERVICE_UNAVAILABLE,
            ApiError::GatewayTimeout => StatusCode::GATEWAY_TIMEOUT,
            ApiError::ValidationError { .. } => StatusCode::BAD_REQUEST,
            ApiError::SerializationError(_) => StatusCode::BAD_REQUEST,
            ApiError::StorageError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    /// Get a user-friendly error message
    pub fn message(&self) -> String {
        self.to_string()
    }

    /// Create a bad request error
    pub fn bad_request(msg: impl Into<String>) -> Self {
        ApiError::BadRequest(msg.into())
    }

    /// Create a not found error
    pub fn not_found(msg: impl Into<String>) -> Self {
        ApiError::NotFound(msg.into())
    }

    /// Create an internal error
    pub fn internal(msg: impl Into<String>) -> Self {
        ApiError::InternalError(msg.into())
    }

    /// Create a validation error
    pub fn validation(field: impl Into<String>, message: impl Into<String>) -> Self {
        ApiError::ValidationError {
            field: field.into(),
            message: message.into(),
        }
    }
}

// Implement From for common error types
impl From<serde_json::Error> for ApiError {
    fn from(err: serde_json::Error) -> Self {
        ApiError::SerializationError(err.to_string())
    }
}

impl From<std::io::Error> for ApiError {
    fn from(err: std::io::Error) -> Self {
        ApiError::InternalError(format!("IO error: {}", err))
    }
}

// Implement IntoResponse for ApiError to make it work with Axum handlers
impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        let status = self.status_code();
        let message = self.message();
        let response = ApiResponse::<String>::error(message);
        (status, Json(response)).into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_status_codes() {
        assert_eq!(
            ApiError::bad_request("test").status_code(),
            StatusCode::BAD_REQUEST
        );
        assert_eq!(
            ApiError::not_found("test").status_code(),
            StatusCode::NOT_FOUND
        );
        assert_eq!(
            ApiError::RateLimitExceeded.status_code(),
            StatusCode::TOO_MANY_REQUESTS
        );
    }

    #[test]
    fn test_error_messages() {
        let err = ApiError::validation("email", "invalid format");
        assert_eq!(err.message(), "Validation error: email: invalid format");
    }
}
