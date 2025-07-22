//! Minimal API key compatibility layer for transitioning to new auth

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// API key information
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct ApiKeyInfo {
    pub key: String,
    pub name: String,
    pub rate_limit: Option<usize>,
    pub enabled: bool,
}

/// Simple API key store for compatibility
#[derive(Clone)]
pub struct ApiKeyStore {
    keys: Arc<RwLock<HashMap<String, ApiKeyInfo>>>,
}

impl ApiKeyStore {
    pub fn new() -> Self {
        Self {
            keys: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn add_key(&self, info: ApiKeyInfo) {
        let mut keys = self.keys.write().await;
        keys.insert(info.key.clone(), info);
    }

    pub async fn get_key(&self, key: &str) -> Option<ApiKeyInfo> {
        let keys = self.keys.read().await;
        keys.get(key).cloned()
    }

    pub async fn list_keys(&self) -> Vec<ApiKeyInfo> {
        let keys = self.keys.read().await;
        keys.values().cloned().collect()
    }

    pub async fn remove_key(&self, key: &str) -> bool {
        let mut keys = self.keys.write().await;
        keys.remove(key).is_some()
    }

    pub async fn validate_key(&self, key: &str) -> Option<ApiKeyInfo> {
        let keys = self.keys.read().await;
        keys.get(key).filter(|info| info.enabled).cloned()
    }
}

impl Default for ApiKeyStore {
    fn default() -> Self {
        Self::new()
    }
}

use crate::types::ApiError;
use axum::{
    extract::{Request, State},
    middleware::Next,
    response::Response,
};

// Legacy middleware functions
pub async fn auth_middleware(
    State(_key_store): State<ApiKeyStore>,
    request: Request,
    next: Next,
) -> Result<Response, ApiError> {
    // For now, just pass through - proper implementation would check API keys
    Ok(next.run(request).await)
}

pub async fn optional_auth_middleware(
    State(_key_store): State<ApiKeyStore>,
    request: Request,
    next: Next,
) -> Result<Response, ApiError> {
    // Optional auth always passes through
    Ok(next.run(request).await)
}
