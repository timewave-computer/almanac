//! Core types for the API layer
//!
//! This module contains types that were previously in api-core

pub mod auth;
pub mod error;
pub mod event;
pub mod response;
pub mod storage;
pub mod common;

// Re-export main types
pub use auth::{
    AuthenticatedPrincipal, Operation, Permission, PermissionError, PermissionStore, PermissionManager,
    PermissionCheckRequest, PermissionCheckResponse, traits::ExternalAuthService,
    AuthService, AuthenticationMethod, SecurityContext,
    ResourceIdentifier, ResourceType, AlmanacPermissions, OperationType, PermissionType,
    traits,
};
pub use error::{ApiError, ApiResult};
pub use event::{ApiEvent, ApiEventTrait};
pub use response::{ApiResponse, EventResponse, EventsResponse, PaginationInfo, ApiResponseBuilder, PaginatedResponse};
pub use storage::{StorageReader, MemoryStorageReader};

// Re-export filter types from common
pub use common::{EventFilter, ChainId, BlockNumber};