//! Authentication module for API HTTP layer

pub mod client;
pub mod permissions;
pub mod mocks;
mod api_key_compat;

pub use client::{HttpAuthService, AuthServiceConfig};
pub use permissions::InMemoryPermissionStore;
pub use api_key_compat::{ApiKeyStore, ApiKeyInfo, auth_middleware, optional_auth_middleware};