use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use crate::types::{ApiError as CoreApiError, response::ErrorResponse};
use uuid::Uuid;

/// HTTP-specific error type
#[derive(Debug, thiserror::Error)]
pub enum HttpError {
    #[error("API error: {0}")]
    Api(#[from] CoreApiError),
    
    #[error("Internal server error: {0}")]
    Internal(String),
    
    #[error("Bad request: {0}")]
    BadRequest(String),
    
    #[error("Not found: {0}")]
    NotFound(String),
}

impl HttpError {
    pub fn internal(msg: impl Into<String>) -> Self {
        Self::Internal(msg.into())
    }
    
    pub fn bad_request(msg: impl Into<String>) -> Self {
        Self::BadRequest(msg.into())
    }
    
    pub fn not_found(msg: impl Into<String>) -> Self {
        Self::NotFound(msg.into())
    }
}

impl IntoResponse for HttpError {
    fn into_response(self) -> Response {
        let (status, code, message) = match &self {
            HttpError::Api(api_err) => (
                api_err.status_code(),
                format!("API_{:?}", api_err).to_uppercase(),
                api_err.message(),
            ),
            HttpError::Internal(msg) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "INTERNAL_ERROR".to_string(),
                msg.clone(),
            ),
            HttpError::BadRequest(msg) => (
                StatusCode::BAD_REQUEST,
                "BAD_REQUEST".to_string(),
                msg.clone(),
            ),
            HttpError::NotFound(msg) => (
                StatusCode::NOT_FOUND,
                "NOT_FOUND".to_string(),
                msg.clone(),
            ),
        };
        
        let error_response = ErrorResponse::new(code, message)
            .with_request_id(Uuid::new_v4().to_string());
        
        (status, Json(error_response)).into_response()
    }
}

// Implement From for common error types
impl From<anyhow::Error> for HttpError {
    fn from(err: anyhow::Error) -> Self {
        HttpError::Internal(err.to_string())
    }
}

impl From<serde_json::Error> for HttpError {
    fn from(err: serde_json::Error) -> Self {
        HttpError::BadRequest(format!("Invalid JSON: {}", err))
    }
}

impl From<crate::types::auth::PermissionError> for HttpError {
    fn from(err: crate::types::auth::PermissionError) -> Self {
        match err {
            crate::types::auth::PermissionError::InsufficientPermissions(msg) => {
                HttpError::Api(CoreApiError::Forbidden(msg))
            },
            crate::types::auth::PermissionError::InvalidOperation(msg) => {
                HttpError::Api(CoreApiError::BadRequest(msg))
            },
        }
    }
}

