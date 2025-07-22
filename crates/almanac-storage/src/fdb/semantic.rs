//! Semantic metadata types for FDB storage

use crate::fdb::ConfidenceLevel;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Semantic metadata key
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SemanticMetadataKey {
    pub chain_id: String,
    pub contract_address: String,
    pub storage_slot: String,
    pub field_name: String,
    pub trust_scope: SemanticTrustScope,
}

/// Semantic metadata value
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SemanticMetadataValue {
    pub data_type: String,
    pub semantic_tags: Vec<String>,
    pub zero_semantics: ZeroSemantics,
    pub confidence_level: ConfidenceLevel,
    pub source: SemanticSource,
    pub metadata: HashMap<String, String>,
    pub created_at: u64,
    pub updated_at: u64,
}

/// What zero means in storage
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ZeroSemantics {
    NeverWritten,
    ExplicitlyZero,
    Cleared,
    ValidZero,
}

/// Source of semantic information
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SemanticSource {
    ContractCode,
    EventLogs,
    Documentation,
    Community,
    DirectObservation,
}

/// Trust scope for semantic metadata
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SemanticTrustScope {
    Global,
    Contract(String),
    Field(String),
}

/// Semantic conflicts between developers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticConflicts {
    pub chain_id: String,
    pub contract_address: String,
    pub field: String,
    pub conflicting_interpretations: Vec<(String, SemanticMetadataValue)>, // (developer_id, interpretation)
    pub detected_at: u64,
    pub resolution: Option<ConflictResolution>,
}

/// Conflict resolution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictResolution {
    pub chosen_developer: String,
    pub method: String,
    pub explanation: String,
    pub resolved_by: String,
    pub resolved_at: u64,
}

use super::trust::TrustPolicy;

impl TrustPolicy {
    /// Filter semantic metadata based on trust policy
    pub fn filter_semantic_metadata<'a>(
        &self,
        interpretations: &'a [(String, SemanticMetadataValue)],
    ) -> Option<&'a (String, SemanticMetadataValue)> {
        // First, try to find a trusted developer's interpretation
        for item in interpretations {
            if self.explicit_trusts.contains(&item.0) {
                return Some(item);
            }
        }

        // If no trusted developer found, apply default behavior
        match self.default_behavior {
            super::trust::TrustBehavior::Accept => interpretations.first(),
            super::trust::TrustBehavior::Reject => None,
            super::trust::TrustBehavior::RequireVerification => {
                // Return highest confidence interpretation
                interpretations
                    .iter()
                    .max_by_key(|(_, v)| v.confidence_level)
            }
        }
    }
}
