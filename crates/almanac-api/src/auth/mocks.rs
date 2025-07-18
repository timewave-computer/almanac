//! Mock implementations for testing authentication

use crate::types::auth::{AuthenticatedPrincipal, PrincipalType, traits::AuthenticationService};
use crate::types::error::{ApiError, ApiResult};
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Mock auth service for testing
pub struct MockAuthService {
    /// Map of token -> principal for successful validations
    valid_tokens: Arc<RwLock<HashMap<String, AuthenticatedPrincipal>>>,
    /// Set of tokens that should trigger validation failures
    invalid_tokens: Arc<RwLock<Vec<String>>>,
    /// If true, all requests will timeout
    force_timeout: Arc<RwLock<bool>>,
    /// If true, service appears unhealthy
    force_unhealthy: Arc<RwLock<bool>>,
}

impl MockAuthService {
    /// Create a new mock auth service
    pub fn new() -> Self {
        Self {
            valid_tokens: Arc::new(RwLock::new(HashMap::new())),
            invalid_tokens: Arc::new(RwLock::new(Vec::new())),
            force_timeout: Arc::new(RwLock::new(false)),
            force_unhealthy: Arc::new(RwLock::new(false)),
        }
    }
}

impl Default for MockAuthService {
    fn default() -> Self {
        Self::new()
    }
}

impl MockAuthService {
    /// Add a valid token with its associated principal
    pub async fn add_valid_token(&self, token: String, principal: AuthenticatedPrincipal) {
        let mut tokens = self.valid_tokens.write().await;
        tokens.insert(token, principal);
    }
    
    /// Add an invalid token that will fail validation
    pub async fn add_invalid_token(&self, token: String) {
        let mut tokens = self.invalid_tokens.write().await;
        tokens.push(token);
    }
    
    /// Set whether all requests should timeout
    pub async fn set_force_timeout(&self, timeout: bool) {
        let mut force = self.force_timeout.write().await;
        *force = timeout;
    }
    
    /// Set whether the service should appear unhealthy
    pub async fn set_force_unhealthy(&self, unhealthy: bool) {
        let mut force = self.force_unhealthy.write().await;
        *force = unhealthy;
    }
    
    /// Create a mock with some default test data
    pub async fn with_test_data() -> Self {
        let mock = Self::new();
        
        // Add test user
        mock.add_valid_token(
            "test_user_token".to_string(),
            AuthenticatedPrincipal {
                id: "user_123".to_string(),
                principal_id: "user_123".to_string(),
                principal_type: PrincipalType::User,
                permissions: vec!["read".to_string()],
                metadata: HashMap::new(),
                attributes: {
                    let mut attrs = HashMap::new();
                    attrs.insert("email".to_string(), "test@example.com".to_string());
                    attrs
                },
            },
        ).await;
        
        // Add test developer
        mock.add_valid_token(
            "test_dev_token".to_string(),
            AuthenticatedPrincipal {
                id: "dev_456".to_string(),
                principal_id: "dev_456".to_string(),
                principal_type: PrincipalType::Developer,
                permissions: vec!["read".to_string(), "write".to_string()],
                metadata: HashMap::new(),
                attributes: {
                    let mut attrs = HashMap::new();
                    attrs.insert("org".to_string(), "test_org".to_string());
                    attrs
                },
            },
        ).await;
        
        // Add test service
        mock.add_valid_token(
            "test_service_token".to_string(),
            AuthenticatedPrincipal {
                id: "service_789".to_string(),
                principal_id: "service_789".to_string(),
                principal_type: PrincipalType::Service,
                permissions: vec!["read".to_string()],
                metadata: HashMap::new(),
                attributes: {
                    let mut attrs = HashMap::new();
                    attrs.insert("service_name".to_string(), "test_service".to_string());
                    attrs
                },
            },
        ).await;
        
        // Add some invalid tokens
        mock.add_invalid_token("expired_token".to_string()).await;
        mock.add_invalid_token("revoked_token".to_string()).await;
        
        mock
    }
}

#[async_trait]
impl AuthenticationService for MockAuthService {
    async fn authenticate(&self, token: &str) -> Result<AuthenticatedPrincipal, Box<dyn std::error::Error + Send + Sync>> {
        match self.validate_token(token).await {
            Ok(principal) => Ok(principal),
            Err(e) => Err(Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
        }
    }
    
    async fn validate_token(&self, token: &str) -> ApiResult<AuthenticatedPrincipal> {
        // Check if we should timeout
        if *self.force_timeout.read().await {
            // Simulate timeout by returning service unavailable
            return Err(ApiError::ServiceUnavailable("Auth service timeout".to_string()));
        }
        
        // Check invalid tokens first
        let invalid_tokens = self.invalid_tokens.read().await;
        if invalid_tokens.contains(&token.to_string()) {
            return Err(ApiError::Unauthorized("Invalid token".to_string()));
        }
        
        // Check valid tokens
        let valid_tokens = self.valid_tokens.read().await;
        if let Some(principal) = valid_tokens.get(token) {
            Ok(principal.clone())
        } else {
            Err(ApiError::Unauthorized("Unknown token".to_string()))
        }
    }
    
    async fn health_check(&self) -> ApiResult<()> {
        if *self.force_unhealthy.read().await {
            Err(ApiError::ServiceUnavailable("Auth service unhealthy".to_string()))
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_mock_auth_service() {
        let mock = MockAuthService::with_test_data().await;
        
        // Test valid token
        let result = mock.validate_token("test_user_token").await;
        assert!(result.is_ok());
        let principal = result.unwrap();
        assert_eq!(principal.principal_id, "user_123");
        assert_eq!(principal.principal_type, PrincipalType::User);
        
        // Test invalid token
        let result = mock.validate_token("expired_token").await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ApiError::Unauthorized(_)));
        
        // Test unknown token
        let result = mock.validate_token("unknown_token").await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ApiError::Unauthorized(_)));
    }
    
    #[tokio::test]
    async fn test_mock_timeout() {
        let mock = MockAuthService::new();
        mock.add_valid_token(
            "valid_token".to_string(),
            AuthenticatedPrincipal {
                id: "test".to_string(),
                principal_id: "test".to_string(),
                principal_type: PrincipalType::User,
                permissions: vec!["read".to_string()],
                metadata: HashMap::new(),
                attributes: HashMap::new(),
            },
        ).await;
        
        // Should work normally
        assert!(mock.validate_token("valid_token").await.is_ok());
        
        // Enable timeout
        mock.set_force_timeout(true).await;
        
        // Should fail with service unavailable
        let result = mock.validate_token("valid_token").await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ApiError::ServiceUnavailable(_)));
    }
    
    #[tokio::test]
    async fn test_mock_health_check() {
        let mock = MockAuthService::new();
        
        // Should be healthy by default
        assert!(mock.health_check().await.is_ok());
        
        // Make unhealthy
        mock.set_force_unhealthy(true).await;
        
        // Should fail health check
        let result = mock.health_check().await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ApiError::ServiceUnavailable(_)));
    }
}