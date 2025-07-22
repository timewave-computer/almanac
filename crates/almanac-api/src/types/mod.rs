//! Core types for the API layer
//!
//! This module contains types that were previously in api-core

pub mod auth;
pub mod common;
pub mod error;
pub mod event;
pub mod response;
pub mod storage;

// Re-export main types
pub use auth::{
    traits, traits::ExternalAuthService, AlmanacPermissions, AuthService, AuthenticatedPrincipal,
    AuthenticationMethod, Operation, OperationType, Permission, PermissionCheckRequest,
    PermissionCheckResponse, PermissionError, PermissionManager, PermissionStore, PermissionType,
    ResourceIdentifier, ResourceType, SecurityContext,
};
pub use error::{ApiError, ApiResult};
pub use event::{ApiEvent, ApiEventTrait};
pub use response::{
    ApiResponse, ApiResponseBuilder, EventResponse, EventsResponse, PaginatedResponse,
    PaginationInfo,
};
pub use storage::{MemoryStorageReader, StorageReader};

// Re-export filter types from common
pub use common::{BlockNumber, ChainId, EventFilter};
