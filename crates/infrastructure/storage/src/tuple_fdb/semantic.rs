//! Semantic metadata types for trust-scoped ZK proof generation
//!
//! This module implements the semantic metadata system that enables Traverse
//! to generate semantically precise ZK proofs while preventing semantic poisoning
//! through explicit trust boundaries.

use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

use almanac_infrastructure_core::{Error, Result};
use super::{TupleKey, TupleKeyspace, keys};

/// Zero semantics enumeration for storage slot interpretation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum ZeroSemantics {
    /// Storage slot was never written to
    NeverWritten = 0,
    /// Deliberately set to zero value
    ExplicitlyZero = 1,
    /// Previously had a value, now cleared
    Cleared = 2,
    /// Zero is a semantically meaningful value
    ValidZero = 3,
}

impl ZeroSemantics {
    /// Parse from u8 representation
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(ZeroSemantics::NeverWritten),
            1 => Some(ZeroSemantics::ExplicitlyZero),
            2 => Some(ZeroSemantics::Cleared),
            3 => Some(ZeroSemantics::ValidZero),
            _ => None,
        }
    }
}

/// Source of semantic interpretation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum SemanticSource {
    /// Declared via type-safe API calls
    DeclaredViaStructuredData = 0,
    /// Inferred from contract ABI
    InferredFromABI = 1,
    /// Verified by multiple developers
    CommunityVerified = 2,
}

impl SemanticSource {
    /// Parse from u8 representation
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(SemanticSource::DeclaredViaStructuredData),
            1 => Some(SemanticSource::InferredFromABI),
            2 => Some(SemanticSource::CommunityVerified),
            _ => None,
        }
    }
}

/// Confidence level for semantic interpretations
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[repr(u8)]
pub enum ConfidenceLevel {
    /// Best guess
    Low = 0,
    /// Reasonable confidence
    Medium = 1,
    /// Very confident
    High = 2,
    /// Cryptographically verified
    Verified = 3,
}

impl ConfidenceLevel {
    /// Parse from u8 representation
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(ConfidenceLevel::Low),
            1 => Some(ConfidenceLevel::Medium),
            2 => Some(ConfidenceLevel::High),
            3 => Some(ConfidenceLevel::Verified),
            _ => None,
        }
    }
}

/// Trust-scoped semantic metadata key
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SemanticMetadataKey {
    /// Chain identifier
    pub chain_id: String,
    /// Contract address
    pub contract_address: String,
    /// Storage key (32 bytes)
    pub storage_key: [u8; 32],
    /// Developer who provided the semantic interpretation
    pub developer_id: String,
    /// Optional project namespace for scoping
    pub project_id: Option<String>,
}

impl SemanticMetadataKey {
    /// Create a new semantic metadata key
    pub fn new(
        chain_id: String,
        contract_address: String,
        storage_key: [u8; 32],
        developer_id: String,
        project_id: Option<String>,
    ) -> Self {
        Self {
            chain_id,
            contract_address,
            storage_key,
            developer_id,
            project_id,
        }
    }

    /// Convert to tuple key for storage
    pub fn to_tuple_key(&self) -> TupleKey {
        let mut key = keys::semantic_meta(
            &self.chain_id,
            &self.contract_address,
            &self.storage_key,
            &self.developer_id,
        );
        
        if let Some(ref project_id) = self.project_id {
            key = key.string(project_id);
        }
        
        key
    }
}

/// Semantic metadata value with trust context
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SemanticMetadataValue {
    /// What zero means for this storage slot
    pub zero_semantics: ZeroSemantics,
    /// How this semantic interpretation was derived
    pub semantic_source: SemanticSource,
    /// Position index in the storage layout
    pub field_index: u16,
    /// Expected storage slot (for verification)
    pub expected_slot: [u8; 32],
    /// Developer confidence in this interpretation
    pub confidence_level: ConfidenceLevel,
    /// When this interpretation was provided
    pub timestamp: u64,
    /// Optional documentation URL
    pub documentation_url: Option<String>,
    /// Optional cryptographic signature
    pub signature: Option<Vec<u8>>,
}

impl SemanticMetadataValue {
    /// Create a new semantic metadata value
    pub fn new(
        zero_semantics: ZeroSemantics,
        semantic_source: SemanticSource,
        field_index: u16,
        expected_slot: [u8; 32],
        confidence_level: ConfidenceLevel,
    ) -> Self {
        Self {
            zero_semantics,
            semantic_source,
            field_index,
            expected_slot,
            confidence_level,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            documentation_url: None,
            signature: None,
        }
    }

    /// Add documentation URL
    pub fn with_documentation(mut self, url: String) -> Self {
        self.documentation_url = Some(url);
        self
    }

    /// Add cryptographic signature
    pub fn with_signature(mut self, signature: [u8; 64]) -> Self {
        self.signature = Some(signature);
        self
    }

    /// Validate the semantic metadata
    pub fn validate(&self) -> Result<()> {
        // Check field index is reasonable
        if self.field_index > 1000 {
            return Err(Error::InvalidData("Field index too large".to_string()));
        }

        // Check timestamp is not in the future
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        if self.timestamp > now + 300 { // Allow 5 minutes clock skew
            return Err(Error::InvalidData("Timestamp in the future".to_string()));
        }

        // Validate documentation URL if present
        if let Some(ref url) = self.documentation_url {
            if !url.starts_with("https://") && !url.starts_with("http://") {
                return Err(Error::InvalidData("Invalid documentation URL".to_string()));
            }
        }

        Ok(())
    }
}

/// Trust scope enumeration
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TrustScope {
    /// Trust for all contracts
    Global,
    /// Trust for specific contract
    Contract(String),
    /// Trust for specific field in a contract
    Field(String, String),
}

impl TrustScope {
    /// Convert to string representation for storage
    pub fn to_string(&self) -> String {
        match self {
            TrustScope::Global => "global".to_string(),
            TrustScope::Contract(addr) => format!("contract:{}", addr),
            TrustScope::Field(addr, field) => format!("field:{}:{}", addr, field),
        }
    }

    /// Parse from string representation
    pub fn from_string(s: &str) -> Result<Self> {
        if s == "global" {
            Ok(TrustScope::Global)
        } else if let Some(addr) = s.strip_prefix("contract:") {
            Ok(TrustScope::Contract(addr.to_string()))
        } else if s.starts_with("field:") {
            let parts: Vec<&str> = s.splitn(3, ':').collect();
            if parts.len() == 3 {
                Ok(TrustScope::Field(parts[1].to_string(), parts[2].to_string()))
            } else {
                Err(Error::InvalidData("Invalid field trust scope format".to_string()))
            }
        } else {
            Err(Error::InvalidData("Invalid trust scope format".to_string()))
        }
    }
}

/// Trust level enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[repr(u8)]
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

impl TrustLevel {
    /// Parse from u8 representation
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(TrustLevel::None),
            1 => Some(TrustLevel::ReadOnly),
            2 => Some(TrustLevel::Contribute),
            3 => Some(TrustLevel::Moderate),
            _ => None,
        }
    }
}

/// Developer trust relationship key
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeveloperTrustKey {
    /// Developer granting trust
    pub trustor: String,
    /// Developer being trusted
    pub trustee: String,
    /// Scope of trust
    pub scope: TrustScope,
}

impl DeveloperTrustKey {
    /// Create a new developer trust key
    pub fn new(trustor: String, trustee: String, scope: TrustScope) -> Self {
        Self {
            trustor,
            trustee,
            scope,
        }
    }

    /// Convert to tuple key for storage
    pub fn to_tuple_key(&self) -> TupleKey {
        keys::trust_graph(&self.trustor, &self.trustee, &self.scope.to_string())
    }
}

/// Developer trust relationship value
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeveloperTrustValue {
    /// Level of trust granted
    pub trust_level: TrustLevel,
    /// When trust was granted
    pub granted_at: u64,
    /// Optional expiration time
    pub expires_at: Option<u64>,
    /// Optional conditions for the trust
    pub conditions: Vec<String>,
    /// Optional revocation reason
    pub revocation_reason: Option<String>,
}

impl DeveloperTrustValue {
    /// Create a new developer trust value
    pub fn new(trust_level: TrustLevel) -> Self {
        Self {
            trust_level,
            granted_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            expires_at: None,
            conditions: Vec::new(),
            revocation_reason: None,
        }
    }

    /// Add expiration time
    pub fn with_expiration(mut self, expires_at: u64) -> Self {
        self.expires_at = Some(expires_at);
        self
    }

    /// Add conditions
    pub fn with_conditions(mut self, conditions: Vec<String>) -> Self {
        self.conditions = conditions;
        self
    }

    /// Revoke the trust with a reason
    pub fn revoke(mut self, reason: String) -> Self {
        self.trust_level = TrustLevel::None;
        self.revocation_reason = Some(reason);
        self
    }

    /// Check if the trust is currently valid
    pub fn is_valid(&self) -> bool {
        // Check if not revoked
        if self.trust_level == TrustLevel::None || self.revocation_reason.is_some() {
            return false;
        }

        // Check expiration
        if let Some(expires_at) = self.expires_at {
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            if now > expires_at {
                return false;
            }
        }

        true
    }
}

/// Semantic conflict detection
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SemanticConflict {
    /// Chain and contract where conflict exists
    pub chain_id: String,
    pub contract_address: String,
    pub storage_key: [u8; 32],
    
    /// Conflicting interpretations
    pub interpretations: Vec<(String, SemanticMetadataValue)>, // (developer_id, metadata)
    
    /// When conflict was detected
    pub detected_at: u64,
    
    /// Optional resolution
    pub resolution: Option<ConflictResolution>,
}

/// Resolution of a semantic conflict
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConflictResolution {
    /// Chosen interpretation
    pub chosen_developer: String,
    /// Reason for choice
    pub reason: String,
    /// Who resolved it
    pub resolved_by: String,
    /// When it was resolved
    pub resolved_at: u64,
}

/// Trust policy for querying semantics
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TrustPolicy {
    /// Only use semantics from explicitly trusted developers
    Strict,
    /// Use any semantics, prefer trusted developers
    Permissive,
    /// Only use semantics from specific developers
    Explicit(Vec<String>),
}

impl TrustPolicy {
    /// Check if a developer is trusted under this policy
    pub fn is_trusted(&self, developer_id: &str, trust_graph: &HashMap<String, TrustLevel>) -> bool {
        match self {
            TrustPolicy::Strict => {
                trust_graph.get(developer_id)
                    .map(|level| *level > TrustLevel::None)
                    .unwrap_or(false)
            }
            TrustPolicy::Permissive => true,
            TrustPolicy::Explicit(trusted) => trusted.contains(&developer_id.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zero_semantics_serialization() {
        let semantics = ZeroSemantics::ExplicitlyZero;
        let serialized = serde_json::to_string(&semantics).unwrap();
        let deserialized: ZeroSemantics = serde_json::from_str(&serialized).unwrap();
        assert_eq!(semantics, deserialized);
    }

    #[test]
    fn test_semantic_metadata_validation() {
        let metadata = SemanticMetadataValue::new(
            ZeroSemantics::ValidZero,
            SemanticSource::DeclaredViaStructuredData,
            5,
            [0u8; 32],
            ConfidenceLevel::High,
        );
        
        assert!(metadata.validate().is_ok());
        
        // Test invalid field index
        let mut invalid_metadata = metadata.clone();
        invalid_metadata.field_index = 2000;
        assert!(invalid_metadata.validate().is_err());
    }

    #[test]
    fn test_trust_scope_serialization() {
        let scopes = vec![
            TrustScope::Global,
            TrustScope::Contract("0x123".to_string()),
            TrustScope::Field("0x123".to_string(), "balance".to_string()),
        ];
        
        for scope in scopes {
            let string_repr = scope.to_string();
            let parsed = TrustScope::from_string(&string_repr).unwrap();
            assert_eq!(scope, parsed);
        }
    }

    #[test]
    fn test_trust_validity() {
        let mut trust = DeveloperTrustValue::new(TrustLevel::Contribute);
        assert!(trust.is_valid());
        
        // Test expiration
        trust = trust.with_expiration(0); // Already expired
        assert!(!trust.is_valid());
        
        // Test revocation
        trust = DeveloperTrustValue::new(TrustLevel::Contribute);
        trust = trust.revoke("No longer needed".to_string());
        assert!(!trust.is_valid());
    }

    #[test]
    fn test_trust_policy() {
        let mut trust_graph = HashMap::new();
        trust_graph.insert("alice".to_string(), TrustLevel::Contribute);
        
        let strict_policy = TrustPolicy::Strict;
        assert!(strict_policy.is_trusted("alice", &trust_graph));
        assert!(!strict_policy.is_trusted("bob", &trust_graph));
        
        let permissive_policy = TrustPolicy::Permissive;
        assert!(permissive_policy.is_trusted("alice", &trust_graph));
        assert!(permissive_policy.is_trusted("bob", &trust_graph));
        
        let explicit_policy = TrustPolicy::Explicit(vec!["charlie".to_string()]);
        assert!(!explicit_policy.is_trusted("alice", &trust_graph));
        assert!(explicit_policy.is_trusted("charlie", &trust_graph));
    }
} 