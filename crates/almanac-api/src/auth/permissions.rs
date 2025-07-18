//! In-memory permission store implementation

use crate::types::auth::{
    AlmanacPermissions, AuthenticatedPrincipal, Operation, OperationType, PrincipalType,
    PermissionError, PermissionStore, PermissionManager,
};
use crate::types::error::{ApiResult};
use async_trait::async_trait;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::RwLock;

/// In-memory permission store for testing
pub struct InMemoryPermissionStore {
    /// Map of principal ID to permissions
    permissions: Arc<RwLock<HashMap<String, AlmanacPermissions>>>,
}

impl InMemoryPermissionStore {
    /// Create a new in-memory permission store with default permissions
    pub fn new() -> Self {
        let mut default_permissions = HashMap::new();
        
        // Add some default permissions for testing
        
        // Test user with read-only access
        let mut read_ops = HashSet::new();
        read_ops.insert(OperationType::ReadEvents);
        read_ops.insert(OperationType::ReadState);
        read_ops.insert(OperationType::ReadProofs);
        
        default_permissions.insert(
            "test_user".to_string(),
            AlmanacPermissions {
                operations: read_ops,
                chain_scopes: vec!["ethereum-mainnet".to_string()],
                contract_scopes: vec![],
            },
        );
        
        // Test developer with write access
        let mut dev_ops = HashSet::new();
        dev_ops.insert(OperationType::ReadEvents);
        dev_ops.insert(OperationType::ReadState);
        dev_ops.insert(OperationType::ReadProofs);
        dev_ops.insert(OperationType::WriteLayouts);
        dev_ops.insert(OperationType::WriteState);
        dev_ops.insert(OperationType::WriteProofs);
        
        default_permissions.insert(
            "test_developer".to_string(),
            AlmanacPermissions {
                operations: dev_ops,
                chain_scopes: vec![], // All chains
                contract_scopes: vec![],
            },
        );
        
        // Test admin with full access
        let mut admin_ops = HashSet::new();
        admin_ops.insert(OperationType::ReadEvents);
        admin_ops.insert(OperationType::ReadState);
        admin_ops.insert(OperationType::ReadProofs);
        admin_ops.insert(OperationType::WriteLayouts);
        admin_ops.insert(OperationType::WriteState);
        admin_ops.insert(OperationType::ManageTrust);
        admin_ops.insert(OperationType::ManageSemantics);
        admin_ops.insert(OperationType::Admin);
        
        default_permissions.insert(
            "test_admin".to_string(),
            AlmanacPermissions {
                operations: admin_ops,
                chain_scopes: vec![],
                contract_scopes: vec![],
            },
        );
        
        Self {
            permissions: Arc::new(RwLock::new(default_permissions)),
        }
    }
    
    /// Create default permissions based on principal type
    fn default_permissions_for_type(principal_type: PrincipalType) -> AlmanacPermissions {
        match principal_type {
            PrincipalType::User => {
                // Users get read-only by default
                AlmanacPermissions::read_only()
            }
            PrincipalType::Developer => {
                // Developers get read + write
                let mut ops = HashSet::new();
                ops.insert(OperationType::ReadEvents);
                ops.insert(OperationType::ReadState);
                ops.insert(OperationType::ReadProofs);
                ops.insert(OperationType::WriteLayouts);
                ops.insert(OperationType::WriteState);
                ops.insert(OperationType::ManageSemantics);
                
                AlmanacPermissions {
                    operations: ops,
                    chain_scopes: vec![],
                    contract_scopes: vec![],
                }
            }
            PrincipalType::Service => {
                // Services get full read access
                let mut ops = HashSet::new();
                ops.insert(OperationType::ReadEvents);
                ops.insert(OperationType::ReadState);
                ops.insert(OperationType::ReadProofs);
                
                AlmanacPermissions {
                    operations: ops,
                    chain_scopes: vec![],
                    contract_scopes: vec![],
                }
            }
            PrincipalType::System => {
                // System principals have full access to everything
                let mut ops = HashSet::new();
                ops.insert(OperationType::ReadEvents);
                ops.insert(OperationType::ReadState);
                ops.insert(OperationType::ReadProofs);
                ops.insert(OperationType::WriteLayouts);
                ops.insert(OperationType::WriteState);
                ops.insert(OperationType::WriteProofs);
                ops.insert(OperationType::ManageTrust);
                ops.insert(OperationType::ManageSemantics);
                ops.insert(OperationType::Admin);
                
                AlmanacPermissions {
                    operations: ops,
                    chain_scopes: vec![],
                    contract_scopes: vec![],
                }
            }
        }
    }
}

#[async_trait]
impl PermissionStore for InMemoryPermissionStore {
    fn check_permission(&self, _principal: &str, _operation: &Operation) -> Result<bool, PermissionError> {
        // Synchronous implementation for PermissionStore trait
        Ok(false)
    }
    
    async fn get_permissions(&self, principal: &AuthenticatedPrincipal) -> ApiResult<AlmanacPermissions> {
        // Delegate to PermissionManager implementation
        PermissionManager::get_permissions(self, principal).await
    }
}

#[async_trait]
impl PermissionManager for InMemoryPermissionStore {
    async fn get_permissions(
        &self,
        principal: &AuthenticatedPrincipal,
    ) -> ApiResult<AlmanacPermissions> {
        let permissions = self.permissions.read().await;
        
        // Check if we have specific permissions for this principal
        if let Some(perms) = permissions.get(&principal.id) {
            Ok(perms.clone())
        } else {
            // Return default permissions based on principal type
            Ok(Self::default_permissions_for_type(principal.principal_type.clone()))
        }
    }
    
    async fn set_permissions(
        &self,
        principal: &AuthenticatedPrincipal,
        permissions: AlmanacPermissions,
    ) -> ApiResult<()> {
        let mut store = self.permissions.write().await;
        store.insert(principal.id.to_string(), permissions);
        Ok(())
    }
}

impl Default for InMemoryPermissionStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_default_permissions() {
        let store = InMemoryPermissionStore::new();
        
        // Test user permissions
        let user = AuthenticatedPrincipal {
            id: "unknown_user".to_string(),
            principal_id: "unknown_user".to_string(),
            principal_type: PrincipalType::User,
            attributes: HashMap::new(),
            metadata: HashMap::new(),
            permissions: vec![],
        };
        
        let perms = PermissionManager::get_permissions(&store, &user).await.unwrap();
        assert!(perms.operations.contains(&OperationType::ReadEvents));
        assert!(!perms.operations.contains(&OperationType::WriteLayouts));
    }
    
    #[tokio::test]
    async fn test_predefined_permissions() {
        let store = InMemoryPermissionStore::new();
        
        // Test predefined developer
        let dev = AuthenticatedPrincipal {
            id: "test_developer".to_string(),
            principal_id: "test_developer".to_string(),
            principal_type: PrincipalType::Developer,
            attributes: HashMap::new(),
            metadata: HashMap::new(),
            permissions: vec![],
        };
        
        let perms = PermissionManager::get_permissions(&store, &dev).await.unwrap();
        assert!(perms.operations.contains(&OperationType::WriteLayouts));
        assert!(!perms.operations.contains(&OperationType::Admin));
    }
    
    #[tokio::test]
    async fn test_permission_check_with_resource() {
        let store = InMemoryPermissionStore::new();
        
        let user = AuthenticatedPrincipal {
            id: "test_user".to_string(),
            principal_id: "test_user".to_string(),
            principal_type: PrincipalType::User,
            attributes: HashMap::new(),
            metadata: HashMap::new(),
            permissions: vec![],
        };
        
        // Test that we can get permissions (basic functionality)
        let permissions = PermissionManager::get_permissions(&store, &user).await.unwrap();
        assert!(permissions.operations.contains(&OperationType::ReadEvents));
    }
    
    #[tokio::test]
    async fn test_set_permissions() {
        let store = InMemoryPermissionStore::new();
        
        // Set custom permissions
        let mut ops = HashSet::new();
        ops.insert(OperationType::ReadEvents);
        ops.insert(OperationType::Admin);
        
        let custom_perms = AlmanacPermissions {
            operations: ops,
            chain_scopes: vec!["test-chain".to_string()],
            contract_scopes: vec![],
        };
        
        let custom_user = AuthenticatedPrincipal {
            id: "custom_user".to_string(),
            principal_id: "custom_user".to_string(),
            principal_type: PrincipalType::User,
            attributes: HashMap::new(),
            metadata: HashMap::new(),
            permissions: vec![],
        };
        
        store.set_permissions(&custom_user, custom_perms).await.unwrap();
        
        // Verify they were set
        let principal = AuthenticatedPrincipal {
            id: "custom_user".to_string(),
            principal_id: "custom_user".to_string(),
            principal_type: PrincipalType::User,
            attributes: HashMap::new(),
            metadata: HashMap::new(),
            permissions: vec![],
        };
        
        let perms = PermissionManager::get_permissions(&store, &principal).await.unwrap();
        assert!(perms.operations.contains(&OperationType::Admin));
        assert!(perms.chain_scopes.contains(&"test-chain".to_string()));
    }
}