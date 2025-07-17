//! Trust management system for developer relationships and semantic scoping
//! 
//! This module implements the trust graph that controls which developers can
//! use semantic interpretations from other developers, preventing semantic
//! poisoning attacks while enabling collaborative development.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use super::{TupleKey, TupleKeyspace};
use almanac_infrastructure_core::{Error, Result};

/// Trust relationship key for tuple storage
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DeveloperTrustKey {
    /// Developer granting trust
    pub trustor: String,
    /// Developer being trusted
    pub trustee: String,
    /// What level of trust
    pub scope: TrustScope,
}

impl DeveloperTrustKey {
    /// Create a new trust relationship key
    pub fn new(trustor: String, trustee: String, scope: TrustScope) -> Self {
        Self {
            trustor,
            trustee,
            scope,
        }
    }
    
    /// Convert to tuple key for storage
    pub fn to_tuple_key(&self) -> TupleKey {
        let mut key = TupleKey::new(TupleKeyspace::TrustGraph)
            .string(&self.trustor)
            .string(&self.trustee);
        
        match &self.scope {
            TrustScope::Global => key.string("global"),
            TrustScope::Contract(address) => key.string("contract").string(address),
            TrustScope::Field(contract, field) => {
                key.string("field").string(contract).string(field)
            }
        }
    }
    
    /// Create a prefix key for querying all trust relationships from a trustor
    pub fn prefix_from_trustor(trustor: &str) -> TupleKey {
        TupleKey::new(TupleKeyspace::TrustGraph)
            .string(trustor)
    }
    
    /// Create a prefix key for querying all trust relationships to a trustee
    pub fn prefix_to_trustee(trustee: &str) -> TupleKey {
        // This would require a secondary index or full scan
        // For now, we'll use the trustor prefix
        TupleKey::new(TupleKeyspace::TrustGraph)
    }
}

/// Trust relationship value
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeveloperTrustValue {
    /// Level of trust granted
    pub trust_level: TrustLevel,
    /// When trust was granted
    pub granted_at: u64,
    /// When trust expires (if applicable)
    pub expires_at: Option<u64>,
    /// Optional conditions for trust
    pub conditions: Vec<String>,
    /// Reason for revocation (if revoked)
    pub revocation_reason: Option<String>,
}

impl DeveloperTrustValue {
    /// Check if trust is currently valid
    pub fn is_valid(&self) -> bool {
        if self.revocation_reason.is_some() {
            return false;
        }
        
        if let Some(expires_at) = self.expires_at {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            return now < expires_at;
        }
        
        true
    }
    
    /// Check if trust level allows a specific action
    pub fn allows(&self, required_level: TrustLevel) -> bool {
        self.is_valid() && self.trust_level >= required_level
    }
}

/// Scope of trust relationship
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TrustScope {
    /// Trust for all contracts
    Global,
    /// Trust for specific contract
    Contract(String),
    /// Trust for specific field in contract
    Field(String, String),
}

impl TrustScope {
    /// Check if this scope includes another scope
    pub fn includes(&self, other: &TrustScope) -> bool {
        match (self, other) {
            (TrustScope::Global, _) => true,
            (TrustScope::Contract(c1), TrustScope::Contract(c2)) => c1 == c2,
            (TrustScope::Contract(c1), TrustScope::Field(c2, _)) => c1 == c2,
            (TrustScope::Field(c1, f1), TrustScope::Field(c2, f2)) => c1 == c2 && f1 == f2,
            _ => false,
        }
    }
}

/// Level of trust granted
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum TrustLevel {
    /// No trust
    None = 0,
    /// Can use semantics, cannot modify
    ReadOnly = 1,
    /// Can suggest semantics
    Contribute = 2,
    /// Can approve/reject semantics
    Moderate = 3,
}

/// Trust policy for evaluating semantic metadata
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrustPolicy {
    /// Default behavior for untrusted sources
    pub default_behavior: TrustBehavior,
    /// Explicit trust requirements
    pub explicit_trusts: Vec<String>,
    /// Minimum trust level required
    pub minimum_level: TrustLevel,
}

impl TrustPolicy {
    /// Create a strict trust policy (only explicit trusts)
    pub fn strict(explicit_trusts: Vec<String>) -> Self {
        Self {
            default_behavior: TrustBehavior::Reject,
            explicit_trusts,
            minimum_level: TrustLevel::ReadOnly,
        }
    }
    
    /// Create a permissive trust policy (accept all)
    pub fn permissive() -> Self {
        Self {
            default_behavior: TrustBehavior::Accept,
            explicit_trusts: Vec::new(),
            minimum_level: TrustLevel::None,
        }
    }
    
    /// Create an explicit trust policy (only listed developers)
    pub fn explicit(trusted_developers: Vec<String>) -> Self {
        Self {
            default_behavior: TrustBehavior::Reject,
            explicit_trusts: trusted_developers,
            minimum_level: TrustLevel::ReadOnly,
        }
    }
    
    /// Check if a developer is trusted according to this policy
    pub fn is_trusted(&self, developer: &str, trust_graph: &HashMap<String, TrustLevel>) -> bool {
        // Check explicit trusts first
        if self.explicit_trusts.contains(&developer.to_string()) {
            return true;
        }
        
        // Check trust graph
        if let Some(level) = trust_graph.get(developer) {
            if *level >= self.minimum_level {
                return true;
            }
        }
        
        // Apply default behavior
        match self.default_behavior {
            TrustBehavior::Accept => true,
            TrustBehavior::Reject => false,
            TrustBehavior::RequireVerification => false, // Would need additional verification
        }
    }
}

/// Default behavior for untrusted sources
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TrustBehavior {
    /// Accept all sources
    Accept,
    /// Reject untrusted sources
    Reject,
    /// Require additional verification
    RequireVerification,
}

/// Trust conflict when multiple developers disagree
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrustConflict {
    /// Field with conflicting interpretations
    pub field: String,
    /// Developers in conflict
    pub developers: Vec<String>,
    /// Their respective trust levels
    pub trust_levels: HashMap<String, TrustLevel>,
    /// Detected at timestamp
    pub detected_at: u64,
    /// Resolution if any
    pub resolution: Option<ConflictResolution>,
}

/// How a trust conflict was resolved
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConflictResolution {
    /// Which developer's interpretation was chosen
    pub chosen_developer: String,
    /// Why this resolution was made
    pub reason: String,
    /// Who resolved the conflict
    pub resolved_by: String,
    /// When it was resolved
    pub resolved_at: u64,
}

/// Trust graph operations for the storage layer
impl super::TupleFdbStorage {
    /// Grant trust from one developer to another
    pub async fn grant_trust(
        &self,
        trustor: &str,
        trustee: &str,
        scope: TrustScope,
        level: TrustLevel,
        expires_at: Option<u64>,
        conditions: Vec<String>,
    ) -> Result<()> {
        let key = DeveloperTrustKey::new(
            trustor.to_string(),
            trustee.to_string(),
            scope,
        );
        
        let value = DeveloperTrustValue {
            trust_level: level,
            granted_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            expires_at,
            conditions,
            revocation_reason: None,
        };
        
        let tuple_key = key.to_tuple_key();
        let serialized = serde_json::to_vec(&value)
            .map_err(|e| Error::Serialization(format!("Failed to serialize trust value: {}", e)))?;
        
        self.set(tuple_key, serialized).await?;
        
        tracing::info!(
            "Trust granted: {} trusts {} with level {:?} in scope {:?}",
            trustor, trustee, level, key.scope
        );
        
        Ok(())
    }
    
    /// Revoke trust relationship
    pub async fn revoke_trust(
        &self,
        trustor: &str,
        trustee: &str,
        scope: TrustScope,
        reason: &str,
    ) -> Result<()> {
        let key = DeveloperTrustKey::new(
            trustor.to_string(),
            trustee.to_string(),
            scope,
        );
        
        let tuple_key = key.to_tuple_key();
        
        // Get existing trust relationship
        if let Some(data) = self.get(tuple_key.clone()).await? {
            let mut value: DeveloperTrustValue = serde_json::from_slice(&data)
                .map_err(|e| Error::Serialization(format!("Failed to parse trust value: {}", e)))?;
            
            // Mark as revoked
            value.revocation_reason = Some(reason.to_string());
            
            let serialized = serde_json::to_vec(&value)
                .map_err(|e| Error::Serialization(format!("Failed to serialize trust value: {}", e)))?;
            
            self.set(tuple_key, serialized).await?;
            
            tracing::info!(
                "Trust revoked: {} no longer trusts {} in scope {:?}, reason: {}",
                trustor, trustee, key.scope, reason
            );
        }
        
        Ok(())
    }
    
    /// Get all trust relationships from a developer
    pub async fn get_trust_graph(&self, developer: &str) -> Result<HashMap<String, TrustLevel>> {
        let prefix = DeveloperTrustKey::prefix_from_trustor(developer);
        let results = self.range(prefix, None).await?;
        
        let mut trust_graph = HashMap::new();
        
        for (_key_bytes, value_bytes) in results {
            if let Ok(trust_value) = serde_json::from_slice::<DeveloperTrustValue>(&value_bytes) {
                if trust_value.is_valid() {
                    // Parse trustee from key (simplified - in production would parse tuple properly)
                    // For now, we'll need to enhance this to properly extract the trustee
                    // This is a limitation of the current implementation
                    tracing::warn!("Trust graph parsing needs enhancement for production use");
                }
            }
        }
        
        Ok(trust_graph)
    }
    
    /// Get trust relationship between two developers
    pub async fn get_trust_relationship(
        &self,
        trustor: &str,
        trustee: &str,
        scope: &TrustScope,
    ) -> Result<Option<DeveloperTrustValue>> {
        let key = DeveloperTrustKey::new(
            trustor.to_string(),
            trustee.to_string(),
            scope.clone(),
        );
        
        let tuple_key = key.to_tuple_key();
        
        match self.get(tuple_key).await? {
            Some(data) => {
                let value: DeveloperTrustValue = serde_json::from_slice(&data)
                    .map_err(|e| Error::Serialization(format!("Failed to parse trust value: {}", e)))?;
                Ok(Some(value))
            }
            None => Ok(None),
        }
    }
    
    /// Check if one developer trusts another at a specific level
    pub async fn check_trust(
        &self,
        trustor: &str,
        trustee: &str,
        scope: &TrustScope,
        required_level: TrustLevel,
    ) -> Result<bool> {
        // Check direct trust first
        if let Some(trust) = self.get_trust_relationship(trustor, trustee, scope).await? {
            if trust.allows(required_level) {
                return Ok(true);
            }
        }
        
        // Check broader scopes
        match scope {
            TrustScope::Field(contract, _field) => {
                // Check contract-level trust
                let contract_scope = TrustScope::Contract(contract.clone());
                if let Some(trust) = self.get_trust_relationship(trustor, trustee, &contract_scope).await? {
                    if trust.allows(required_level) {
                        return Ok(true);
                    }
                }
                
                // Check global trust
                if let Some(trust) = self.get_trust_relationship(trustor, trustee, &TrustScope::Global).await? {
                    if trust.allows(required_level) {
                        return Ok(true);
                    }
                }
            }
            TrustScope::Contract(_) => {
                // Check global trust
                if let Some(trust) = self.get_trust_relationship(trustor, trustee, &TrustScope::Global).await? {
                    if trust.allows(required_level) {
                        return Ok(true);
                    }
                }
            }
            TrustScope::Global => {
                // Already checked the most general scope
            }
        }
        
        Ok(false)
    }
    
    /// Get all developers trusted by a specific developer
    pub async fn get_trusted_developers(
        &self,
        trustor: &str,
        min_level: TrustLevel,
    ) -> Result<Vec<(String, TrustScope, TrustLevel)>> {
        let prefix = DeveloperTrustKey::prefix_from_trustor(trustor);
        let results = self.range(prefix, None).await?;
        
        let mut trusted = Vec::new();
        
        for (key_bytes, value_bytes) in results {
            if let Ok(trust_value) = serde_json::from_slice::<DeveloperTrustValue>(&value_bytes) {
                if trust_value.is_valid() && trust_value.trust_level >= min_level {
                    // Parse the key to extract trustee and scope
                    // This is simplified - in production would properly parse the tuple
                    if let Ok(key_str) = String::from_utf8(key_bytes) {
                        // Extract trustee and scope from key string
                        // Format: /namespace/trust_graph/trustor/trustee/scope_type/...
                        let parts: Vec<&str> = key_str.split('/').collect();
                        if parts.len() >= 5 {
                            let trustee = parts[4].to_string();
                            
                            // Parse scope based on remaining parts
                            let scope = if parts.len() == 6 && parts[5] == "global" {
                                TrustScope::Global
                            } else if parts.len() == 7 && parts[5] == "contract" {
                                TrustScope::Contract(parts[6].to_string())
                            } else if parts.len() == 8 && parts[5] == "field" {
                                TrustScope::Field(parts[6].to_string(), parts[7].to_string())
                            } else {
                                continue; // Skip malformed keys
                            };
                            
                            trusted.push((trustee, scope, trust_value.trust_level));
                        }
                    }
                }
            }
        }
        
        Ok(trusted)
    }
    
    /// Update trust policy for a developer
    pub async fn update_trust_policy(
        &self,
        developer: &str,
        policy: TrustPolicy,
    ) -> Result<()> {
        // Store trust policy in a separate keyspace or as part of developer profile
        // For now, we'll store it under a special trust graph entry
        let key = TupleKey::new(TupleKeyspace::TrustGraph)
            .string(developer)
            .string("_policy");
        
        let serialized = serde_json::to_vec(&policy)
            .map_err(|e| Error::Serialization(format!("Failed to serialize trust policy: {}", e)))?;
        
        self.set(key, serialized).await?;
        
        tracing::info!("Updated trust policy for developer: {}", developer);
        
        Ok(())
    }
    
    /// Get trust policy for a developer
    pub async fn get_trust_policy(&self, developer: &str) -> Result<Option<TrustPolicy>> {
        let key = TupleKey::new(TupleKeyspace::TrustGraph)
            .string(developer)
            .string("_policy");
        
        match self.get(key).await? {
            Some(data) => {
                let policy: TrustPolicy = serde_json::from_slice(&data)
                    .map_err(|e| Error::Serialization(format!("Failed to parse trust policy: {}", e)))?;
                Ok(Some(policy))
            }
            None => Ok(None),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_trust_scope_inclusion() {
        let global = TrustScope::Global;
        let contract = TrustScope::Contract("0x123".to_string());
        let field = TrustScope::Field("0x123".to_string(), "balance".to_string());
        
        assert!(global.includes(&global));
        assert!(global.includes(&contract));
        assert!(global.includes(&field));
        
        assert!(!contract.includes(&global));
        assert!(contract.includes(&contract));
        assert!(contract.includes(&field));
        
        assert!(!field.includes(&global));
        assert!(!field.includes(&contract));
        assert!(field.includes(&field));
    }
    
    #[test]
    fn test_trust_level_ordering() {
        assert!(TrustLevel::None < TrustLevel::ReadOnly);
        assert!(TrustLevel::ReadOnly < TrustLevel::Contribute);
        assert!(TrustLevel::Contribute < TrustLevel::Moderate);
    }
    
    #[test]
    fn test_trust_policy_evaluation() {
        let mut trust_graph = HashMap::new();
        trust_graph.insert("alice".to_string(), TrustLevel::Contribute);
        trust_graph.insert("bob".to_string(), TrustLevel::ReadOnly);
        
        // Strict policy
        let strict = TrustPolicy::strict(vec!["alice".to_string()]);
        assert!(strict.is_trusted("alice", &trust_graph));
        assert!(!strict.is_trusted("bob", &trust_graph));
        assert!(!strict.is_trusted("charlie", &trust_graph));
        
        // Permissive policy
        let permissive = TrustPolicy::permissive();
        assert!(permissive.is_trusted("alice", &trust_graph));
        assert!(permissive.is_trusted("bob", &trust_graph));
        assert!(permissive.is_trusted("charlie", &trust_graph));
        
        // Explicit policy with minimum level
        let mut explicit = TrustPolicy::explicit(vec!["alice".to_string(), "bob".to_string()]);
        explicit.minimum_level = TrustLevel::Contribute;
        assert!(explicit.is_trusted("alice", &trust_graph)); // Has Contribute level
        assert!(!explicit.is_trusted("bob", &trust_graph)); // Only has ReadOnly
    }
}