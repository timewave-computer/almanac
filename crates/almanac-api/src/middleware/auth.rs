//! Authentication middleware for Axum

use crate::types::auth::{
    context::SecurityContext,
    traits::AuthenticationService,
};
use crate::types::PermissionStore;
use crate::types::error::ApiError;
use axum::{
    extract::{Request, State},
    middleware::Next,
    response::Response,
};
use std::sync::Arc;
use uuid::Uuid;

/// State required for authentication middleware
#[derive(Clone)]
pub struct AuthState {
    pub auth_service: Arc<dyn AuthenticationService>,
    pub permission_store: Arc<dyn PermissionStore>,
}

/// Extract bearer token from Authorization header
fn extract_bearer_token(request: &Request) -> Option<String> {
    request
        .headers()
        .get("Authorization")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| {
            value.strip_prefix("Bearer ").map(|stripped| stripped.to_string())
        })
}

/// Extract client IP from request
fn extract_client_ip(request: &Request) -> Option<std::net::IpAddr> {
    // Try X-Forwarded-For first (for proxied requests)
    request
        .headers()
        .get("X-Forwarded-For")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(',').next())
        .and_then(|ip| ip.trim().parse().ok())
        .or_else(|| {
            // Fall back to X-Real-IP
            request
                .headers()
                .get("X-Real-IP")
                .and_then(|value| value.to_str().ok())
                .and_then(|ip| ip.parse().ok())
        })
}

/// Authentication middleware that validates tokens and creates security context
pub async fn auth_middleware(
    State(auth_state): State<AuthState>,
    mut request: Request,
    next: Next,
) -> Result<Response, ApiError> {
    // Extract request ID or generate one
    let request_id = request
        .headers()
        .get("X-Request-ID")
        .and_then(|v| v.to_str().ok())
        .map(String::from)
        .unwrap_or_else(|| Uuid::new_v4().to_string());
    
    // Extract bearer token
    let token = extract_bearer_token(&request)
        .ok_or_else(|| ApiError::Unauthorized("Missing bearer token".to_string()))?;
    
    // Validate token with auth service
    let principal = auth_state
        .auth_service
        .validate_token(&token)
        .await
        .map_err(|e| ApiError::Unauthorized(format!("Token validation failed: {}", e)))?;
    
    // Get permissions for principal
    let permissions = auth_state
        .permission_store
        .get_permissions(&principal)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to load permissions: {}", e)))?;
    
    // Extract client IP
    let client_ip = extract_client_ip(&request);
    
    // Create security context
    let security_context = SecurityContext::new(
        principal,
        permissions,
        request_id.clone(),
        client_ip.map(|ip| ip.to_string()),
    );
    
    // Log authentication
    tracing::info!(
        request_id = %request_id,
        principal_id = %security_context.principal_id(),
        client_ip = ?client_ip,
        "Authenticated request"
    );
    
    // Insert security context into request extensions
    request.extensions_mut().insert(security_context);
    
    // Continue with request
    Ok(next.run(request).await)
}

/// Optional authentication middleware that allows unauthenticated requests
pub async fn optional_auth_middleware(
    State(auth_state): State<AuthState>,
    mut request: Request,
    next: Next,
) -> Result<Response, ApiError> {
    // Extract request ID or generate one
    let request_id = request
        .headers()
        .get("X-Request-ID")
        .and_then(|v| v.to_str().ok())
        .map(String::from)
        .unwrap_or_else(|| Uuid::new_v4().to_string());
    
    // Try to extract bearer token
    if let Some(token) = extract_bearer_token(&request) {
        // Try to validate token
        if let Ok(principal) = auth_state.auth_service.validate_token(&token).await {
            // Try to get permissions
            if let Ok(permissions) = auth_state.permission_store.get_permissions(&principal).await {
                // Extract client IP
                let client_ip = extract_client_ip(&request);
                
                // Create security context
                let security_context = SecurityContext::new(
                    principal,
                    permissions,
                    request_id.clone(),
                    client_ip.map(|ip| ip.to_string()),
                );
                
                // Log authentication
                tracing::debug!(
                    request_id = %request_id,
                    principal_id = %security_context.principal_id(),
                    "Optional auth succeeded"
                );
                
                // Insert security context into request extensions
                request.extensions_mut().insert(security_context);
            }
        }
    }
    
    // Continue with request (authenticated or not)
    Ok(next.run(request).await)
}


#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_extract_bearer_token() {
        use axum::body::Body;
        
        let request = Request::builder()
            .uri("/")
            .header("Authorization", "Bearer test_token")
            .body(Body::empty())
            .unwrap();
        
        assert_eq!(extract_bearer_token(&request), Some("test_token".to_string()));
    }
    
    #[test]
    fn test_extract_bearer_token_invalid() {
        use axum::body::Body;
        
        let request = Request::builder()
            .uri("/")
            .header("Authorization", "Basic dGVzdDp0ZXN0")
            .body(Body::empty())
            .unwrap();
        
        assert_eq!(extract_bearer_token(&request), None);
    }
    
    #[test]
    fn test_extract_client_ip() {
        use axum::body::Body;
        
        let request = Request::builder()
            .uri("/")
            .header("X-Forwarded-For", "192.168.1.1, 10.0.0.1")
            .body(Body::empty())
            .unwrap();
        
        assert_eq!(
            extract_client_ip(&request),
            Some("192.168.1.1".parse().unwrap())
        );
    }
}