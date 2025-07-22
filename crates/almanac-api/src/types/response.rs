use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

/// Standard API response wrapper
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ApiResponse<T> {
    /// Response data
    pub data: T,

    /// Response metadata
    pub meta: ResponseMetadata,
}

/// Response metadata
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ResponseMetadata {
    /// Request ID for correlation
    pub request_id: String,

    /// Response timestamp
    pub timestamp: DateTime<Utc>,

    /// Processing duration in milliseconds
    pub duration_ms: u64,

    /// API version
    pub version: String,
}

impl ResponseMetadata {
    /// Create new metadata with defaults
    pub fn new() -> Self {
        Self {
            request_id: Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            duration_ms: 0,
            version: "v1".to_string(),
        }
    }

    /// Set the request ID
    pub fn with_request_id(mut self, id: impl Into<String>) -> Self {
        self.request_id = id.into();
        self
    }

    /// Set the duration
    pub fn with_duration_ms(mut self, duration: u64) -> Self {
        self.duration_ms = duration;
        self
    }
}

impl Default for ResponseMetadata {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> ApiResponse<T> {
    /// Create a new API response
    pub fn new(data: T) -> Self {
        Self {
            data,
            meta: ResponseMetadata::new(),
        }
    }

    /// Create with specific metadata
    pub fn with_meta(data: T, meta: ResponseMetadata) -> Self {
        Self { data, meta }
    }

    /// Map the data to a different type
    pub fn map<U, F>(self, f: F) -> ApiResponse<U>
    where
        F: FnOnce(T) -> U,
    {
        ApiResponse {
            data: f(self.data),
            meta: self.meta,
        }
    }

    /// Create a successful response
    pub fn success(data: T) -> Self {
        Self::new(data)
    }

    /// Create an error response (for compatibility)
    pub fn error(data: T) -> Self {
        Self::new(data)
    }
}

/// Builder for API responses
pub struct ApiResponseBuilder<T> {
    data: T,
    meta: ResponseMetadata,
}

impl<T> ApiResponseBuilder<T> {
    /// Create a new builder
    pub fn new(data: T) -> Self {
        Self {
            data,
            meta: ResponseMetadata::new(),
        }
    }

    /// Set the request ID
    pub fn request_id(mut self, id: impl Into<String>) -> Self {
        self.meta.request_id = id.into();
        self
    }

    /// Set the duration
    pub fn duration_ms(mut self, duration: u64) -> Self {
        self.meta.duration_ms = duration;
        self
    }

    /// Set the API version
    pub fn version(mut self, version: impl Into<String>) -> Self {
        self.meta.version = version.into();
        self
    }

    /// Build the response
    pub fn build(self) -> ApiResponse<T> {
        ApiResponse {
            data: self.data,
            meta: self.meta,
        }
    }
}

/// Paginated response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginatedResponse<T> {
    /// Items in this page
    pub items: Vec<T>,

    /// Total number of items
    pub total: u64,

    /// Current page (0-based)
    pub page: u32,

    /// Items per page
    pub per_page: u32,

    /// Whether there are more pages
    pub has_more: bool,
}

impl<T> PaginatedResponse<T> {
    /// Create a new paginated response
    pub fn new(items: Vec<T>, total: u64, page: u32, per_page: u32) -> Self {
        let has_more = (page + 1) * per_page < total as u32;
        Self {
            items,
            total,
            page,
            per_page,
            has_more,
        }
    }
}

/// Response for single event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventResponse {
    pub event: crate::types::ApiEvent,
}

/// Response for multiple events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventsResponse {
    pub events: Vec<crate::types::ApiEvent>,
    pub pagination: PaginationInfo,
}

/// Pagination information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginationInfo {
    pub page: u32,
    pub per_page: u32,
    pub total: u64,
    pub has_more: bool,
}

/// Error response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorResponse {
    /// Error code
    pub code: String,

    /// Error message
    pub message: String,

    /// Additional details
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,

    /// Request ID for correlation
    pub request_id: String,

    /// Timestamp
    pub timestamp: DateTime<Utc>,
}

impl ErrorResponse {
    /// Create a new error response
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            details: None,
            request_id: Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
        }
    }

    /// Add details to the error
    pub fn with_details(mut self, details: serde_json::Value) -> Self {
        self.details = Some(details);
        self
    }

    /// Set the request ID
    pub fn with_request_id(mut self, id: impl Into<String>) -> Self {
        self.request_id = id.into();
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api_response() {
        let response = ApiResponse::new(vec![1, 2, 3]);
        assert_eq!(response.data, vec![1, 2, 3]);
        assert_eq!(response.meta.version, "v1");
    }

    #[test]
    fn test_response_builder() {
        let response = ApiResponseBuilder::new("test")
            .request_id("req-123")
            .duration_ms(42)
            .version("v2")
            .build();

        assert_eq!(response.data, "test");
        assert_eq!(response.meta.request_id, "req-123");
        assert_eq!(response.meta.duration_ms, 42);
        assert_eq!(response.meta.version, "v2");
    }

    #[test]
    fn test_paginated_response() {
        let response = PaginatedResponse::new(vec![1, 2, 3], 10, 0, 3);
        assert_eq!(response.items.len(), 3);
        assert_eq!(response.total, 10);
        assert!(response.has_more);
    }
}
