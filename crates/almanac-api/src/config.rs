//! Configuration types for HTTP API
//!
//! This module provides configuration types that map from TOML config files
//! to the runtime configuration needed by the HTTP API.

use crate::auth::AuthServiceConfig;
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// HTTP API authentication configuration from TOML
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    /// External authentication service URL
    pub service_url: String,

    /// Request timeout in milliseconds
    pub timeout_ms: u64,

    /// Cache TTL for validated tokens in seconds
    pub cache_ttl_seconds: u64,

    /// Enable authentication (set to false to bypass auth)
    pub enabled: bool,
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            service_url: "http://localhost:8888/v1/auth".to_string(),
            timeout_ms: 5000,
            cache_ttl_seconds: 300,
            enabled: true,
        }
    }
}

impl From<AuthConfig> for AuthServiceConfig {
    fn from(config: AuthConfig) -> Self {
        AuthServiceConfig {
            service_url: config.service_url,
            validation_endpoint: "/validate".to_string(),
            timeout: Duration::from_millis(config.timeout_ms),
            cache_ttl: Duration::from_secs(config.cache_ttl_seconds),
        }
    }
}
