//! HTTP client for external authentication service

use crate::types::auth::{traits::AuthenticationService, AuthenticatedPrincipal};
use crate::types::error::{ApiError, ApiResult};
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

/// Configuration for the auth service client
#[derive(Debug, Clone)]
pub struct AuthServiceConfig {
    /// Base URL of the auth service
    pub service_url: String,
    /// Endpoint for token validation
    pub validation_endpoint: String,
    /// Request timeout
    pub timeout: Duration,
    /// Cache TTL for validated tokens
    pub cache_ttl: Duration,
}

impl Default for AuthServiceConfig {
    fn default() -> Self {
        Self {
            service_url: "http://localhost:8080".to_string(),
            validation_endpoint: "/validate".to_string(),
            timeout: Duration::from_secs(5),
            cache_ttl: Duration::from_secs(300), // 5 minutes
        }
    }
}

/// HTTP client for external auth service
pub struct HttpAuthService {
    client: Client,
    config: AuthServiceConfig,
    cache: Arc<RwLock<TokenCache>>,
}

/// Cache for validated tokens
struct TokenCache {
    entries: HashMap<String, CachedToken>,
}

struct CachedToken {
    principal: AuthenticatedPrincipal,
    expires_at: std::time::Instant,
}

impl HttpAuthService {
    /// Create a new auth service client
    pub fn new(config: AuthServiceConfig) -> Self {
        let client = Client::builder()
            .timeout(config.timeout)
            .build()
            .expect("Failed to create HTTP client");

        Self {
            client,
            config,
            cache: Arc::new(RwLock::new(TokenCache {
                entries: HashMap::new(),
            })),
        }
    }

    /// Clear expired entries from cache
    async fn clean_cache(&self) {
        let mut cache = self.cache.write().await;
        let now = std::time::Instant::now();
        cache.entries.retain(|_, entry| entry.expires_at > now);
    }
}

/// Request body for token validation
#[derive(Serialize)]
struct ValidateTokenRequest {
    token: String,
}

/// Response from token validation
#[derive(Deserialize)]
struct ValidateTokenResponse {
    principal_id: String,
    principal_type: String,
    attributes: HashMap<String, String>,
}

#[async_trait]
impl AuthenticationService for HttpAuthService {
    async fn authenticate(
        &self,
        token: &str,
    ) -> Result<AuthenticatedPrincipal, Box<dyn std::error::Error + Send + Sync>> {
        match self.validate_token(token).await {
            Ok(principal) => Ok(principal),
            Err(e) => Err(Box::new(e) as Box<dyn std::error::Error + Send + Sync>),
        }
    }

    async fn validate_token(&self, token: &str) -> ApiResult<AuthenticatedPrincipal> {
        // Check cache first
        {
            let cache = self.cache.read().await;
            if let Some(cached) = cache.entries.get(token) {
                if cached.expires_at > std::time::Instant::now() {
                    return Ok(cached.principal.clone());
                }
            }
        }

        // Clean cache periodically
        self.clean_cache().await;

        // Validate with auth service
        let url = format!(
            "{}{}",
            self.config.service_url, self.config.validation_endpoint
        );
        let request = ValidateTokenRequest {
            token: token.to_string(),
        };

        let response = self
            .client
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| ApiError::ServiceUnavailable(format!("Auth service error: {}", e)))?;

        if response.status() == 401 {
            return Err(ApiError::Unauthorized("Invalid token".to_string()));
        }

        if !response.status().is_success() {
            return Err(ApiError::ServiceUnavailable(format!(
                "Auth service returned status: {}",
                response.status()
            )));
        }

        let validation_response: ValidateTokenResponse = response.json().await.map_err(|e| {
            ApiError::InternalError(format!("Failed to parse auth response: {}", e))
        })?;

        // Convert response to principal
        let principal_type = match validation_response.principal_type.as_str() {
            "user" => crate::types::auth::PrincipalType::User,
            "developer" => crate::types::auth::PrincipalType::Developer,
            "service" => crate::types::auth::PrincipalType::Service,
            _ => {
                return Err(ApiError::InternalError(
                    "Unknown principal type".to_string(),
                ))
            }
        };

        let principal = AuthenticatedPrincipal {
            id: validation_response.principal_id.clone(),
            principal_id: validation_response.principal_id,
            principal_type,
            permissions: vec![], // Default empty, would be populated from permission store
            metadata: HashMap::new(),
            attributes: validation_response.attributes,
        };

        // Cache the result
        {
            let mut cache = self.cache.write().await;
            cache.entries.insert(
                token.to_string(),
                CachedToken {
                    principal: principal.clone(),
                    expires_at: std::time::Instant::now() + self.config.cache_ttl,
                },
            );
        }

        Ok(principal)
    }

    async fn health_check(&self) -> ApiResult<()> {
        let url = format!("{}/health", self.config.service_url);

        let response = self.client.get(&url).send().await.map_err(|e| {
            ApiError::ServiceUnavailable(format!("Auth service health check failed: {}", e))
        })?;

        if response.status().is_success() {
            Ok(())
        } else {
            Err(ApiError::ServiceUnavailable(
                "Auth service unhealthy".to_string(),
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_cache_expiration() {
        let config = AuthServiceConfig {
            cache_ttl: Duration::from_millis(100),
            ..Default::default()
        };

        let service = HttpAuthService::new(config);

        // Add entry to cache
        {
            let mut cache = service.cache.write().await;
            cache.entries.insert(
                "test_token".to_string(),
                CachedToken {
                    principal: AuthenticatedPrincipal {
                        id: "test_user".to_string(),
                        principal_id: "test_user".to_string(),
                        principal_type: crate::types::auth::PrincipalType::User,
                        permissions: vec![],
                        metadata: HashMap::new(),
                        attributes: HashMap::new(),
                    },
                    expires_at: std::time::Instant::now() + Duration::from_millis(50),
                },
            );
        }

        // Should be in cache
        assert!(service
            .cache
            .read()
            .await
            .entries
            .contains_key("test_token"));

        // Wait for expiration
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Clean cache
        service.clean_cache().await;

        // Should be removed
        assert!(!service
            .cache
            .read()
            .await
            .entries
            .contains_key("test_token"));
    }
}
