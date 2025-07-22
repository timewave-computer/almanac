//! Authentication module for API HTTP layer

mod api_key_compat;
pub mod auth_enhanced;
pub mod client;
pub mod mocks;
pub mod permissions;

pub use api_key_compat::{auth_middleware, optional_auth_middleware, ApiKeyInfo, ApiKeyStore};
pub use client::{AuthServiceConfig, HttpAuthService};
pub use permissions::InMemoryPermissionStore;

// Enhanced auth exports
pub use auth_enhanced::{
    enhanced_auth_middleware, ApiKey, ApiKeyError, ApiKeySummary, ApiOperation, ApiPermissions,
    EnhancedApiKeyStore, GenerateKeyRequest, GenerateKeyResponse, KeyMetadata, PermissionContext,
    QuotaError, RateLimitConfig, UsageQuotas, UsageStats,
};
