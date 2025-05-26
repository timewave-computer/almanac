/// Authentication and authorization module for the API
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use axum::{
    async_trait,
    extract::FromRequestParts,
    http::{header::AUTHORIZATION, HeaderMap, StatusCode},
    response::{IntoResponse, Json},
};
use axum::http::request::Parts;
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, TokenData, Validation};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::sync::RwLock;
use tracing::{debug, warn};
use uuid::Uuid;

use indexer_core::{Error, Result};

/// JWT claims structure
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    /// Subject (user ID)
    pub sub: String,
    /// Username
    pub username: String,
    /// User role
    pub role: UserRole,
    /// Issued at (Unix timestamp)
    pub iat: u64,
    /// Expires at (Unix timestamp)
    pub exp: u64,
    /// JWT ID (for revocation)
    pub jti: String,
}

/// User roles for authorization
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum UserRole {
    /// Read-only access
    Read,
    /// Read and write access
    Write,
    /// Administrative access
    Admin,
}

impl UserRole {
    /// Check if this role has permission for another role's actions
    pub fn has_permission(&self, required: &UserRole) -> bool {
        matches!((self, required), (UserRole::Admin, _) | (UserRole::Write, UserRole::Read) | (UserRole::Write, UserRole::Write) | (UserRole::Read, UserRole::Read))
    }
}

/// User information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: String,
    pub username: String,
    pub role: UserRole,
    pub created_at: SystemTime,
    pub last_login: Option<SystemTime>,
    pub active: bool,
}

/// API key information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKey {
    pub id: String,
    pub user_id: String,
    pub name: String,
    pub key_hash: String,
    pub created_at: SystemTime,
    pub last_used: Option<SystemTime>,
    pub expires_at: Option<SystemTime>,
    pub active: bool,
}

/// In-memory user store (in production, this would be a database)
#[derive(Debug, Clone)]
pub struct UserStore {
    users: Arc<RwLock<HashMap<String, User>>>,
    api_keys: Arc<RwLock<HashMap<String, ApiKey>>>,
    revoked_tokens: Arc<RwLock<std::collections::HashSet<String>>>,
}

impl UserStore {
    /// Create a new user store
    pub fn new() -> Self {
        let store = Self {
            users: Arc::new(RwLock::new(HashMap::new())),
            api_keys: Arc::new(RwLock::new(HashMap::new())),
            revoked_tokens: Arc::new(RwLock::new(std::collections::HashSet::new())),
        };
        
        // Create default admin user
        let admin_user = User {
            id: "admin".to_string(),
            username: "admin".to_string(),
            role: UserRole::Admin,
            created_at: SystemTime::now(),
            last_login: None,
            active: true,
        };
        
        // Insert admin user
        let users_clone = store.users.clone();
        tokio::spawn(async move {
            let mut users = users_clone.write().await;
            users.insert("admin".to_string(), admin_user);
        });
        
        store
    }
    
    /// Get user by ID
    pub async fn get_user(&self, user_id: &str) -> Option<User> {
        let users = self.users.read().await;
        users.get(user_id).cloned()
    }
    
    /// Get user by username
    pub async fn get_user_by_username(&self, username: &str) -> Option<User> {
        let users = self.users.read().await;
        users.values().find(|u| u.username == username).cloned()
    }
    
    /// Create a new user
    pub async fn create_user(&self, username: String, role: UserRole) -> Result<User> {
        let mut users = self.users.write().await;
        
        // Check if username already exists
        if users.values().any(|u| u.username == username) {
            return Err(Error::generic("Username already exists"));
        }
        
        let user = User {
            id: Uuid::new_v4().to_string(),
            username,
            role,
            created_at: SystemTime::now(),
            last_login: None,
            active: true,
        };
        
        users.insert(user.id.clone(), user.clone());
        Ok(user)
    }
    
    /// Create an API key for a user
    pub async fn create_api_key(&self, user_id: String, name: String) -> Result<(ApiKey, String)> {
        let users = self.users.read().await;
        if !users.contains_key(&user_id) {
            return Err(Error::generic("User not found"));
        }
        drop(users);
        
        let raw_key = Uuid::new_v4().to_string();
        let key_hash = bcrypt::hash(&raw_key, bcrypt::DEFAULT_COST)
            .map_err(|e| Error::generic(format!("Failed to hash API key: {}", e)))?;
        
        let api_key = ApiKey {
            id: Uuid::new_v4().to_string(),
            user_id,
            name,
            key_hash,
            created_at: SystemTime::now(),
            last_used: None,
            expires_at: None,
            active: true,
        };
        
        let mut api_keys = self.api_keys.write().await;
        api_keys.insert(api_key.id.clone(), api_key.clone());
        
        Ok((api_key, raw_key))
    }
    
    /// Validate an API key and return the associated user
    pub async fn validate_api_key(&self, key: &str) -> Option<User> {
        let api_keys = self.api_keys.read().await;
        
        for api_key in api_keys.values() {
            if !api_key.active {
                continue;
            }
            
            // Check if expired
            if let Some(expires_at) = api_key.expires_at {
                if SystemTime::now() > expires_at {
                    continue;
                }
            }
            
            // Verify key hash
            if bcrypt::verify(key, &api_key.key_hash).unwrap_or(false) {
                let user_id = api_key.user_id.clone();
                drop(api_keys);
                return self.get_user(&user_id).await;
            }
        }
        
        None
    }
    
    /// Revoke a JWT token
    pub async fn revoke_token(&self, jti: &str) {
        let mut revoked = self.revoked_tokens.write().await;
        revoked.insert(jti.to_string());
    }
    
    /// Check if a JWT token is revoked
    pub async fn is_token_revoked(&self, jti: &str) -> bool {
        let revoked = self.revoked_tokens.read().await;
        revoked.contains(jti)
    }
    
    /// List all users
    pub async fn list_users(&self) -> Vec<User> {
        let users = self.users.read().await;
        users.values().cloned().collect()
    }
    
    /// List API keys for a user
    pub async fn list_api_keys(&self, user_id: &str) -> Vec<ApiKey> {
        let api_keys = self.api_keys.read().await;
        api_keys.values()
            .filter(|key| key.user_id == user_id)
            .cloned()
            .collect()
    }
}

impl Default for UserStore {
    fn default() -> Self {
        Self::new()
    }
}

/// JWT token manager
#[derive(Clone)]
pub struct TokenManager {
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
    validation: Validation,
}

impl TokenManager {
    /// Create a new token manager with a secret key
    pub fn new(secret: &[u8]) -> Self {
        Self {
            encoding_key: EncodingKey::from_secret(secret),
            decoding_key: DecodingKey::from_secret(secret),
            validation: Validation::default(),
        }
    }
    
    /// Generate a JWT token for a user
    pub fn generate_token(&self, user: &User) -> Result<String> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| Error::generic(format!("System time error: {}", e)))?
            .as_secs();
        
        let claims = Claims {
            sub: user.id.clone(),
            username: user.username.clone(),
            role: user.role.clone(),
            iat: now,
            exp: now + 24 * 60 * 60, // 24 hours
            jti: Uuid::new_v4().to_string(),
        };
        
        encode(&Header::default(), &claims, &self.encoding_key)
            .map_err(|e| Error::generic(format!("Failed to encode JWT: {}", e)))
    }
    
    /// Validate and decode a JWT token
    pub fn validate_token(&self, token: &str) -> Result<TokenData<Claims>> {
        decode::<Claims>(token, &self.decoding_key, &self.validation)
            .map_err(|e| Error::generic(format!("Invalid JWT token: {}", e)))
    }
}

/// Authentication state for the API
#[derive(Clone)]
pub struct AuthState {
    pub user_store: UserStore,
    pub token_manager: TokenManager,
}

impl AuthState {
    /// Create new authentication state
    pub fn new(jwt_secret: &[u8]) -> Self {
        Self {
            user_store: UserStore::new(),
            token_manager: TokenManager::new(jwt_secret),
        }
    }
    
    /// Authenticate a request using Bearer token or API key
    pub async fn authenticate(&self, headers: &HeaderMap) -> Option<User> {
        if let Some(auth_header) = headers.get(AUTHORIZATION) {
            if let Ok(auth_str) = auth_header.to_str() {
                if let Some(token) = auth_str.strip_prefix("Bearer ") {
                    
                    // Try JWT token first
                    if let Ok(token_data) = self.token_manager.validate_token(token) {
                        let claims = token_data.claims;
                        
                        // Check if token is revoked
                        if self.user_store.is_token_revoked(&claims.jti).await {
                            return None;
                        }
                        
                        // Get fresh user data
                        return self.user_store.get_user(&claims.sub).await;
                    }
                    
                    // Try API key
                    return self.user_store.validate_api_key(token).await;
                }
            }
        }
        
        None
    }
}

/// Authenticated user extractor for protected endpoints
#[derive(Debug, Clone)]
pub struct AuthenticatedUser(pub User);

#[async_trait]
impl<S> FromRequestParts<S> for AuthenticatedUser
where
    S: Send + Sync,
    S: AsRef<crate::http::HttpState>,
{
    type Rejection = AuthError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> std::result::Result<Self, Self::Rejection> {
        let http_state = state.as_ref();
        
        if let Some(user) = http_state.auth_state.authenticate(&parts.headers).await {
            debug!("Authenticated user: {}", user.username);
            Ok(AuthenticatedUser(user))
        } else {
            warn!("Authentication failed for request");
            Err(AuthError::Unauthorized)
        }
    }
}

/// Optional authenticated user (allows anonymous access)
#[derive(Debug, Clone)]
pub struct OptionalUser(pub Option<User>);

#[async_trait]
impl<S> FromRequestParts<S> for OptionalUser
where
    S: Send + Sync,
    S: AsRef<crate::http::HttpState>,
{
    type Rejection = AuthError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> std::result::Result<Self, Self::Rejection> {
        let http_state = state.as_ref();
        
        let user = http_state.auth_state.authenticate(&parts.headers).await;
        if let Some(ref user) = user {
            debug!("Authenticated user: {}", user.username);
        }
        
        Ok(OptionalUser(user))
    }
}

/// Authorization errors
#[derive(Debug)]
pub enum AuthError {
    Unauthorized,
    Forbidden,
    InternalError,
}

impl IntoResponse for AuthError {
    fn into_response(self) -> axum::response::Response {
        let (status, message) = match self {
            AuthError::Unauthorized => (StatusCode::UNAUTHORIZED, "Unauthorized"),
            AuthError::Forbidden => (StatusCode::FORBIDDEN, "Forbidden"),
            AuthError::InternalError => (StatusCode::INTERNAL_SERVER_ERROR, "Internal Server Error"),
        };
        
        let body = Json(json!({
            "error": message,
            "status": status.as_u16()
        }));
        
        (status, body).into_response()
    }
}

/// Authentication endpoints
pub mod endpoints {
    use super::*;
    use axum::{extract::State, Json};
    
    /// Login request
    #[derive(Deserialize)]
    pub struct LoginRequest {
        pub username: String,
        pub password: String,
    }
    
    /// Login response
    #[derive(Serialize)]
    pub struct LoginResponse {
        pub token: String,
        pub user: User,
        pub expires_in: u64,
    }
    
    /// API key creation request
    #[derive(Deserialize)]
    pub struct CreateApiKeyRequest {
        pub name: String,
    }
    
    /// API key creation response
    #[derive(Serialize)]
    pub struct CreateApiKeyResponse {
        pub api_key: String,
        pub key_id: String,
        pub name: String,
        pub expires_at: Option<u64>,
    }
    
    /// User creation request
    #[derive(Deserialize)]
    pub struct CreateUserRequest {
        pub username: String,
        pub role: UserRole,
    }
    
    /// Generate an API key for the authenticated user
    pub async fn create_api_key(
        State(http_state): State<crate::http::HttpState>,
        AuthenticatedUser(user): AuthenticatedUser,
        Json(request): Json<CreateApiKeyRequest>,
    ) -> std::result::Result<Json<CreateApiKeyResponse>, AuthError> {
        match http_state.auth_state.user_store.create_api_key(user.id, request.name.clone()).await {
            Ok((api_key, raw_key)) => {
                let response = CreateApiKeyResponse {
                    api_key: raw_key,
                    key_id: api_key.id,
                    name: request.name,
                    expires_at: api_key.expires_at.map(|t| {
                        t.duration_since(UNIX_EPOCH).unwrap_or_default().as_secs()
                    }),
                };
                Ok(Json(response))
            }
            Err(_) => Err(AuthError::InternalError),
        }
    }
    
    /// Create a new user (admin only)
    pub async fn create_user(
        State(http_state): State<crate::http::HttpState>,
        AuthenticatedUser(admin): AuthenticatedUser,
        Json(request): Json<CreateUserRequest>,
    ) -> std::result::Result<Json<User>, AuthError> {
        // Check if user has admin role
        if admin.role != UserRole::Admin {
            return Err(AuthError::Forbidden);
        }
        
        match http_state.auth_state.user_store.create_user(request.username, request.role).await {
            Ok(user) => Ok(Json(user)),
            Err(_) => Err(AuthError::InternalError),
        }
    }
    
    /// List all users (admin only)
    pub async fn list_users(
        State(http_state): State<crate::http::HttpState>,
        AuthenticatedUser(admin): AuthenticatedUser,
    ) -> std::result::Result<Json<Vec<User>>, AuthError> {
        // Check if user has admin role
        if admin.role != UserRole::Admin {
            return Err(AuthError::Forbidden);
        }
        
        let users = http_state.auth_state.user_store.list_users().await;
        Ok(Json(users))
    }
    
    /// Get current user info
    pub async fn get_current_user(
        AuthenticatedUser(user): AuthenticatedUser,
    ) -> Json<User> {
        Json(user)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    
    #[test]
    fn test_user_role_permissions() {
        // Admin has all permissions
        assert!(UserRole::Admin.has_permission(&UserRole::Read));
        assert!(UserRole::Admin.has_permission(&UserRole::Write));
        assert!(UserRole::Admin.has_permission(&UserRole::Admin));
        
        // Write has read and write permissions
        assert!(UserRole::Write.has_permission(&UserRole::Read));
        assert!(UserRole::Write.has_permission(&UserRole::Write));
        assert!(!UserRole::Write.has_permission(&UserRole::Admin));
        
        // Read only has read permission
        assert!(UserRole::Read.has_permission(&UserRole::Read));
        assert!(!UserRole::Read.has_permission(&UserRole::Write));
        assert!(!UserRole::Read.has_permission(&UserRole::Admin));
    }
    
    #[tokio::test]
    async fn test_user_store_creation() {
        let store = UserStore::new();
        
        // Wait a bit for the admin user to be inserted
        tokio::time::sleep(Duration::from_millis(10)).await;
        
        // Check that admin user exists
        let admin = store.get_user("admin").await;
        assert!(admin.is_some());
        
        let admin_user = admin.unwrap();
        assert_eq!(admin_user.username, "admin");
        assert_eq!(admin_user.role, UserRole::Admin);
        assert!(admin_user.active);
    }
    
    #[tokio::test]
    async fn test_user_creation() {
        let store = UserStore::new();
        
        // Create a new user
        let user = store.create_user("testuser".to_string(), UserRole::Write).await;
        assert!(user.is_ok());
        
        let created_user = user.unwrap();
        assert_eq!(created_user.username, "testuser");
        assert_eq!(created_user.role, UserRole::Write);
        assert!(created_user.active);
        
        // Try to create duplicate user
        let duplicate = store.create_user("testuser".to_string(), UserRole::Read).await;
        assert!(duplicate.is_err());
        
        // Retrieve user by username
        let retrieved = store.get_user_by_username("testuser").await;
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().id, created_user.id);
    }
    
    #[tokio::test]
    async fn test_api_key_creation() {
        let store = UserStore::new();
        
        // Create a user first
        let user = store.create_user("keyuser".to_string(), UserRole::Read).await.unwrap();
        
        // Create API key
        let result = store.create_api_key(user.id.clone(), "test-key".to_string()).await;
        assert!(result.is_ok());
        
        let (api_key, raw_key) = result.unwrap();
        assert_eq!(api_key.user_id, user.id);
        assert_eq!(api_key.name, "test-key");
        assert!(api_key.active);
        assert!(!raw_key.is_empty());
        
        // Validate the API key
        let validated_user = store.validate_api_key(&raw_key).await;
        assert!(validated_user.is_some());
        assert_eq!(validated_user.unwrap().id, user.id);
        
        // Try with invalid key
        let invalid = store.validate_api_key("invalid-key").await;
        assert!(invalid.is_none());
    }
    
    #[test]
    fn test_token_manager() {
        let secret = b"test-secret-key-for-jwt-tokens-32-bytes";
        let token_manager = TokenManager::new(secret);
        
        let user = User {
            id: "test-user".to_string(),
            username: "testuser".to_string(),
            role: UserRole::Write,
            created_at: SystemTime::now(),
            last_login: None,
            active: true,
        };
        
        // Generate token
        let token = token_manager.generate_token(&user);
        assert!(token.is_ok());
        
        let token_str = token.unwrap();
        assert!(!token_str.is_empty());
        
        // Validate token
        let validation = token_manager.validate_token(&token_str);
        assert!(validation.is_ok());
        
        let token_data = validation.unwrap();
        assert_eq!(token_data.claims.sub, user.id);
        assert_eq!(token_data.claims.username, user.username);
        assert_eq!(token_data.claims.role, user.role);
        
        // Try with invalid token
        let invalid = token_manager.validate_token("invalid.token.here");
        assert!(invalid.is_err());
    }
    
    #[tokio::test]
    async fn test_auth_state() {
        let secret = b"test-secret-key-for-jwt-tokens-32-bytes";
        let auth_state = AuthState::new(secret);
        
        // Create a user
        let user = auth_state.user_store.create_user("authuser".to_string(), UserRole::Admin).await.unwrap();
        
        // Create API key
        let (_, raw_key) = auth_state.user_store.create_api_key(user.id.clone(), "auth-test".to_string()).await.unwrap();
        
        // Test authentication with API key
        let mut headers = HeaderMap::new();
        headers.insert(AUTHORIZATION, format!("Bearer {}", raw_key).parse().unwrap());
        
        let authenticated = auth_state.authenticate(&headers).await;
        assert!(authenticated.is_some());
        assert_eq!(authenticated.unwrap().id, user.id);
        
        // Test with invalid header
        let mut invalid_headers = HeaderMap::new();
        invalid_headers.insert(AUTHORIZATION, "Bearer invalid-key".parse().unwrap());
        
        let not_authenticated = auth_state.authenticate(&invalid_headers).await;
        assert!(not_authenticated.is_none());
    }
    
    #[tokio::test]
    async fn test_token_revocation() {
        let store = UserStore::new();
        let jti = "test-token-id";
        
        // Initially not revoked
        assert!(!store.is_token_revoked(jti).await);
        
        // Revoke token
        store.revoke_token(jti).await;
        
        // Now should be revoked
        assert!(store.is_token_revoked(jti).await);
    }
    
    #[tokio::test]
    async fn test_user_listing() {
        let store = UserStore::new();
        
        // Wait for admin user
        tokio::time::sleep(Duration::from_millis(10)).await;
        
        // Create additional users
        let _user1 = store.create_user("user1".to_string(), UserRole::Read).await.unwrap();
        let _user2 = store.create_user("user2".to_string(), UserRole::Write).await.unwrap();
        
        // List all users
        let users = store.list_users().await;
        assert_eq!(users.len(), 3); // admin + 2 created users
        
        // Check usernames
        let usernames: Vec<&str> = users.iter().map(|u| u.username.as_str()).collect();
        assert!(usernames.contains(&"admin"));
        assert!(usernames.contains(&"user1"));
        assert!(usernames.contains(&"user2"));
    }
    
    #[tokio::test]
    async fn test_api_key_listing() {
        let store = UserStore::new();
        
        // Create a user
        let user = store.create_user("keylistuser".to_string(), UserRole::Write).await.unwrap();
        
        // Create multiple API keys
        let _key1 = store.create_api_key(user.id.clone(), "key1".to_string()).await.unwrap();
        let _key2 = store.create_api_key(user.id.clone(), "key2".to_string()).await.unwrap();
        
        // List API keys for user
        let keys = store.list_api_keys(&user.id).await;
        assert_eq!(keys.len(), 2);
        
        // Check key names
        let key_names: Vec<&str> = keys.iter().map(|k| k.name.as_str()).collect();
        assert!(key_names.contains(&"key1"));
        assert!(key_names.contains(&"key2"));
        
        // List keys for non-existent user
        let empty_keys = store.list_api_keys("non-existent").await;
        assert!(empty_keys.is_empty());
    }
    
    #[test]
    fn test_user_serialization() {
        let user = User {
            id: "test-id".to_string(),
            username: "testuser".to_string(),
            role: UserRole::Admin,
            created_at: SystemTime::now(),
            last_login: Some(SystemTime::now()),
            active: true,
        };
        
        // Test JSON serialization
        let json = serde_json::to_string(&user);
        assert!(json.is_ok());
        
        let json_str = json.unwrap();
        assert!(json_str.contains("test-id"));
        assert!(json_str.contains("testuser"));
        assert!(json_str.contains("admin"));
        
        // Test deserialization
        let deserialized = serde_json::from_str::<User>(&json_str);
        assert!(deserialized.is_ok());
        
        let user2 = deserialized.unwrap();
        assert_eq!(user.id, user2.id);
        assert_eq!(user.username, user2.username);
        assert_eq!(user.role, user2.role);
    }
} 