use crate::auth::ApiKeyStore;
use crate::rate_limit::RateLimiter;
use crate::storage_ext::ExtendedStorageReader;
use almanac_chains::subscription::AccountSubscriptionManager;
use std::sync::Arc;

/// Application state shared across all handlers
#[derive(Clone)]
pub struct AppState {
    /// Storage backend
    pub storage: Arc<dyn ExtendedStorageReader>,

    /// API version
    pub version: String,

    /// API key store (legacy compatibility)
    pub key_store: ApiKeyStore,

    /// Rate limiter
    pub rate_limiter: RateLimiter,

    /// Authentication service (new)
    pub auth_service: Option<Arc<dyn crate::types::traits::AuthenticationService>>,

    /// Permission store (new)
    pub permission_store: Option<Arc<dyn crate::types::PermissionStore>>,

    /// Subscription manager
    pub subscription_manager: Option<Arc<dyn AccountSubscriptionManager>>,
}

impl AppState {
    /// Create new application state
    pub fn new(storage: Arc<dyn ExtendedStorageReader>) -> Self {
        Self {
            storage,
            version: "v1".to_string(),
            key_store: ApiKeyStore::new(),
            rate_limiter: RateLimiter::new(100), // Default 100 requests per minute
            auth_service: None,
            permission_store: None,
            subscription_manager: None,
        }
    }

    /// Set the API version
    pub fn with_version(mut self, version: impl Into<String>) -> Self {
        self.version = version.into();
        self
    }

    /// Set the API key store
    pub fn with_key_store(mut self, key_store: ApiKeyStore) -> Self {
        self.key_store = key_store;
        self
    }

    /// Set the rate limiter
    pub fn with_rate_limiter(mut self, rate_limiter: RateLimiter) -> Self {
        self.rate_limiter = rate_limiter;
        self
    }

    /// Set the authentication service
    pub fn with_auth_service(
        mut self,
        auth_service: impl crate::types::traits::AuthenticationService + 'static,
    ) -> Self {
        self.auth_service = Some(Arc::new(auth_service));
        self
    }

    /// Set the permission store
    pub fn with_permission_store(
        mut self,
        permission_store: impl crate::types::PermissionStore + 'static,
    ) -> Self {
        self.permission_store = Some(Arc::new(permission_store));
        self
    }

    /// Set the subscription manager
    pub fn with_subscription_manager(
        mut self,
        subscription_manager: impl AccountSubscriptionManager + 'static,
    ) -> Self {
        self.subscription_manager = Some(Arc::new(subscription_manager));
        self
    }
}
