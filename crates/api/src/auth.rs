/// Authentication and authorization module for the API
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use axum::{
    async_trait,
    extract::{FromRequestParts, State},
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
        match (self, required) {
            (UserRole::Admin, _) => true,
            (UserRole::Write, UserRole::Read) => true,
            (UserRole::Write, UserRole::Write) => true,
            (UserRole::Read, UserRole::Read) => true,
            _ => false,
        }
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
        let mut store = Self {
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
            .map_err(|e| Error::generic(&format!("Failed to hash API key: {}", e)))?;
        
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
            .map_err(|e| Error::generic(&format!("System time error: {}", e)))?
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
            .map_err(|e| Error::generic(&format!("Failed to encode JWT: {}", e)))
    }
    
    /// Validate and decode a JWT token
    pub fn validate_token(&self, token: &str) -> Result<TokenData<Claims>> {
        decode::<Claims>(token, &self.decoding_key, &self.validation)
            .map_err(|e| Error::generic(&format!("Invalid JWT token: {}", e)))
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
                if auth_str.starts_with("Bearer ") {
                    let token = &auth_str[7..];
                    
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