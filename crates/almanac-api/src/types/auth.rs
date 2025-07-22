//! Authentication types for the API layer

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum PrincipalType {
    User,
    Developer,
    Service,
    System,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthenticatedPrincipal {
    pub id: String,
    pub principal_id: String, // Alias for compatibility
    pub principal_type: PrincipalType,
    pub permissions: Vec<String>,
    pub metadata: HashMap<String, String>,
    pub attributes: HashMap<String, String>, // Additional attributes field
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct Operation {
    pub action: String,
    pub resource: String,
}

impl Operation {
    pub fn new(action: &str, resource: &str) -> Self {
        Operation {
            action: action.to_string(),
            resource: resource.to_string(),
        }
    }

    // Common operations as associated functions that return instances
    pub fn read_events() -> Self {
        Self::new("read", "events")
    }

    pub fn write_events() -> Self {
        Self::new("write", "events")
    }

    pub fn read_layouts() -> Self {
        Self::new("read", "layouts")
    }

    pub fn write_layouts() -> Self {
        Self::new("write", "layouts")
    }

    pub fn read_views() -> Self {
        Self::new("read", "views")
    }

    pub fn write_views() -> Self {
        Self::new("write", "views")
    }

    pub fn admin() -> Self {
        Self::new("admin", "*")
    }
}

// Simplified: just provide as associated functions
impl Operation {
    // These functions can be called like Operation::read_events()
    #[allow(non_snake_case)]
    pub fn ReadEvents() -> Operation {
        Self::read_events()
    }
    #[allow(non_snake_case)]
    pub fn WriteEvents() -> Operation {
        Self::write_events()
    }
    #[allow(non_snake_case)]
    pub fn ReadLayouts() -> Operation {
        Self::read_layouts()
    }
    #[allow(non_snake_case)]
    pub fn WriteLayouts() -> Operation {
        Self::write_layouts()
    }
    #[allow(non_snake_case)]
    pub fn ReadViews() -> Operation {
        Self::read_views()
    }
    #[allow(non_snake_case)]
    pub fn WriteViews() -> Operation {
        Self::write_views()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Permission {
    pub operation: Operation,
    pub granted: bool,
}

#[derive(Debug, thiserror::Error)]
pub enum PermissionError {
    #[error("Insufficient permissions: {0}")]
    InsufficientPermissions(String),
    #[error("Invalid operation: {0}")]
    InvalidOperation(String),
}

#[async_trait::async_trait]
pub trait PermissionStore: Send + Sync {
    fn check_permission(
        &self,
        principal: &str,
        operation: &Operation,
    ) -> Result<bool, PermissionError>;
    async fn get_permissions(
        &self,
        principal: &AuthenticatedPrincipal,
    ) -> crate::types::error::ApiResult<AlmanacPermissions>;
}

#[async_trait::async_trait]
pub trait PermissionManager: Send + Sync {
    async fn get_permissions(
        &self,
        principal: &AuthenticatedPrincipal,
    ) -> crate::types::error::ApiResult<AlmanacPermissions>;
    async fn set_permissions(
        &self,
        principal: &AuthenticatedPrincipal,
        permissions: AlmanacPermissions,
    ) -> crate::types::error::ApiResult<()>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionCheckRequest {
    pub principal_id: String,
    pub operation: Operation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionCheckResponse {
    pub granted: bool,
    pub reason: Option<String>,
}

#[derive(Debug, Clone)]
pub enum AuthenticationMethod {
    ApiKey,
    Bearer,
    Basic,
}

#[derive(Debug, Clone)]
pub struct SecurityContext {
    pub principal: AuthenticatedPrincipal,
    pub method: AuthenticationMethod,
    pub expires_at: Option<u64>,
}

pub mod traits {
    use super::*;
    use async_trait::async_trait;

    #[async_trait]
    pub trait AuthenticationService: Send + Sync {
        async fn authenticate(
            &self,
            token: &str,
        ) -> Result<AuthenticatedPrincipal, Box<dyn std::error::Error + Send + Sync>>;
        async fn validate_token(
            &self,
            token: &str,
        ) -> crate::types::ApiResult<AuthenticatedPrincipal>;
        async fn health_check(&self) -> crate::types::ApiResult<()>;
    }

    #[async_trait]
    pub trait ExternalAuthService: Send + Sync {
        async fn verify_token(
            &self,
            token: &str,
        ) -> Result<AuthenticatedPrincipal, Box<dyn std::error::Error + Send + Sync>>;
    }
}

pub use traits::AuthenticationService;

// Type aliases for backward compatibility
pub type AuthService = Box<dyn traits::AuthenticationService>;

/// Resource identifier for permission checks
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct ResourceIdentifier {
    pub resource_type: ResourceType,
    pub resource_id: String,
    pub chain_id: Option<String>,
    pub contract_address: Option<String>,
}

/// Resource types in the system
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum ResourceType {
    Chain,
    Contract,
    Event,
    Layout,
    View,
    Trust,
    Semantic,
}

/// Individual permission types  
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum PermissionType {
    ReadEvents,
    WriteEvents,
    ReadLayouts,
    WriteLayouts,
    ReadViews,
    WriteViews,
    ManageTrust,
    ManageSemantics,
    Admin,
}

/// Operation types for permissions
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum OperationType {
    ReadEvents,
    WriteEvents,
    ReadState,
    WriteState,
    ReadProofs,
    WriteProofs,
    ReadLayouts,
    WriteLayouts,
    ManageTrust,
    ManageSemantics,
    Admin,
}

/// Almanac-specific permission set
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AlmanacPermissions {
    pub operations: std::collections::HashSet<OperationType>,
    pub chain_scopes: Vec<String>,
    pub contract_scopes: Vec<String>,
}

impl AlmanacPermissions {
    /// Create read-only permissions
    pub fn read_only() -> Self {
        let mut ops = std::collections::HashSet::new();
        ops.insert(OperationType::ReadEvents);
        ops.insert(OperationType::ReadState);
        ops.insert(OperationType::ReadProofs);

        Self {
            operations: ops,
            chain_scopes: vec![],
            contract_scopes: vec![],
        }
    }

    /// Check if permission allows a specific operation
    pub fn can(&self, _operation: Operation) -> bool {
        // For now, just return true - this would be implemented properly
        true
    }

    /// Check if permission allows access to a specific chain
    pub fn has_chain_access(&self, chain_id: &str) -> bool {
        self.chain_scopes.is_empty() || self.chain_scopes.contains(&chain_id.to_string())
    }

    /// Check if permission allows access to a specific contract
    pub fn has_contract_access(&self, contract: &str) -> bool {
        self.contract_scopes.is_empty() || self.contract_scopes.contains(&contract.to_string())
    }
}

/// Security context for a request
pub mod context {
    use super::*;

    #[derive(Debug, Clone)]
    pub struct SecurityContext {
        pub principal: AuthenticatedPrincipal,
        pub permissions: std::collections::HashSet<String>,
        pub resource_access:
            std::collections::HashMap<ResourceIdentifier, std::collections::HashSet<String>>,
        pub request_id: Option<String>,
        pub client_ip: Option<String>,
    }

    impl SecurityContext {
        pub fn new(
            principal: AuthenticatedPrincipal,
            permissions: AlmanacPermissions,
            request_id: String,
            client_ip: Option<String>,
        ) -> Self {
            // Convert AlmanacPermissions to the format expected by SecurityContext
            let permission_strings: std::collections::HashSet<String> = permissions
                .operations
                .iter()
                .map(|op| format!("{:?}", op)) // Convert OperationType to string
                .collect();

            Self {
                principal,
                permissions: permission_strings,
                resource_access: std::collections::HashMap::new(),
                request_id: Some(request_id),
                client_ip,
            }
        }

        pub fn simple(principal: AuthenticatedPrincipal) -> Self {
            Self {
                principal,
                permissions: std::collections::HashSet::new(),
                resource_access: std::collections::HashMap::new(),
                request_id: None,
                client_ip: None,
            }
        }

        pub fn has_permission(&self, permission: &str) -> bool {
            self.permissions.contains(permission)
        }

        pub fn can_access_resource(&self, resource: &ResourceIdentifier, action: &str) -> bool {
            self.resource_access
                .get(resource)
                .map(|perms| perms.contains(action))
                .unwrap_or(false)
        }

        /// Require a specific permission, returning an error if not granted
        pub fn require_permission(&self, operation: Operation) -> Result<(), PermissionError> {
            let permission_key = format!("{}:{}", operation.action, operation.resource);
            if self.has_permission(&permission_key) {
                Ok(())
            } else {
                Err(PermissionError::InsufficientPermissions(format!(
                    "Permission required: {}",
                    permission_key
                )))
            }
        }

        /// Require access to a specific chain
        pub fn require_chain_access(&self, chain_id: &str) -> Result<(), PermissionError> {
            // For now, implement basic chain access check
            // This would typically check against chain-specific permissions
            if self.principal.principal_type == PrincipalType::System {
                Ok(()) // System principals have access to all chains
            } else {
                // Check if we have permissions for this chain
                // This is a simplified implementation
                let has_access = self.permissions.iter().any(|perm| {
                    perm.contains(&format!("chain:{}", chain_id)) || perm.contains("chain:*")
                });

                if has_access {
                    Ok(())
                } else {
                    Err(PermissionError::InsufficientPermissions(format!(
                        "Chain access required: {}",
                        chain_id
                    )))
                }
            }
        }

        /// Require access to a specific contract
        pub fn require_contract_access(
            &self,
            contract_address: &str,
        ) -> Result<(), PermissionError> {
            // Check contract-specific permissions
            let has_access = self.permissions.iter().any(|perm| {
                perm.contains(&format!("contract:{}", contract_address))
                    || perm.contains("contract:*")
            });

            if has_access {
                Ok(())
            } else {
                Err(PermissionError::InsufficientPermissions(format!(
                    "Contract access required: {}",
                    contract_address
                )))
            }
        }

        /// Get the principal ID
        pub fn principal_id(&self) -> &str {
            &self.principal.principal_id
        }
    }
}
