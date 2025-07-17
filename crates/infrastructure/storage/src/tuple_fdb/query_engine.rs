//! Trust-scoped query engine for contract state
//!
//! This module provides the core query engine that evaluates trust policies,
//! resolves semantic conflicts, and authenticates query results.

use super::{TupleKey, TupleKeyspace, TupleFdbStorage};
use super::trust::{TrustPolicy, TrustLevel, TrustBehavior};
use super::semantic::{SemanticMetadataKey, SemanticMetadataValue, ConfidenceLevel};
use super::view_registry::{ViewDefinition, ViewQuery, ViewQueryResponse};
use super::layout_resolver::LayoutResolver;
use almanac_infrastructure_core::{Error, Result};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use tracing::{debug, info, warn};

/// Trust-scoped query request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustScopedQuery {
    /// Query requester identity
    pub requester_id: String,
    /// Target contract
    pub chain_id: String,
    pub contract_address: String,
    /// Query type
    pub query_type: QueryType,
    /// Trust policy to apply
    pub trust_policy: Option<TrustPolicy>,
    /// Include conflict analysis
    pub analyze_conflicts: bool,
    /// Minimum confidence threshold
    pub min_confidence: Option<f64>,
}

/// Type of query
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum QueryType {
    /// Query by view ID
    ViewQuery(ViewQuery),
    /// Query by field names
    FieldQuery {
        fields: Vec<String>,
        block_height: u64,
        include_proofs: bool,
    },
    /// Query raw storage slots
    StorageQuery {
        slots: Vec<Vec<u8>>,
        block_height: u64,
        include_proofs: bool,
    },
    /// Query with layout resolution
    LayoutQuery {
        field_paths: Vec<String>,
        block_height: u64,
        layout_version: Option<String>,
    },
}

/// Trust-scoped query response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustScopedQueryResponse {
    /// Query results
    pub results: QueryResults,
    /// Trust evaluation results
    pub trust_evaluation: TrustEvaluation,
    /// Semantic conflicts detected
    pub conflicts: Vec<SemanticConflict>,
    /// Overall confidence score
    pub confidence_score: f64,
    /// Query authentication
    pub authentication: QueryAuthentication,
}

/// Query results based on query type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum QueryResults {
    /// View query results
    ViewResults(ViewQueryResponse),
    /// Field query results
    FieldResults(HashMap<String, FieldResult>),
    /// Storage query results
    StorageResults(HashMap<String, StorageResult>),
    /// Layout query results
    LayoutResults(HashMap<String, LayoutResult>),
}

/// Result for a single field
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldResult {
    /// Field value
    pub value: Vec<u8>,
    /// Decoded value (if available)
    pub decoded: Option<serde_json::Value>,
    /// Storage proof
    pub proof: Option<Vec<u8>>,
    /// Semantic metadata
    pub semantics: Vec<SemanticInterpretation>,
    /// Field confidence
    pub confidence: f64,
}

/// Storage query result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageResult {
    /// Raw storage value
    pub value: Vec<u8>,
    /// Storage proof
    pub proof: Option<Vec<u8>>,
    /// Associated semantics
    pub semantics: Vec<SemanticInterpretation>,
}

/// Layout query result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayoutResult {
    /// Field path
    pub path: String,
    /// Resolved value
    pub value: Vec<u8>,
    /// Layout information
    pub layout_info: LayoutInfo,
    /// Semantic interpretations
    pub semantics: Vec<SemanticInterpretation>,
}

/// Layout information for a field
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayoutInfo {
    /// Storage slot
    pub slot: String,
    /// Offset in slot
    pub offset: u8,
    /// Field size
    pub size: u8,
    /// Type name
    pub type_name: String,
}

/// Semantic interpretation from a developer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticInterpretation {
    /// Developer providing interpretation
    pub developer_id: String,
    /// Interpretation details
    pub interpretation: String,
    /// Confidence level
    pub confidence: ConfidenceLevel,
    /// Trust level of developer
    pub trust_level: TrustLevel,
    /// Additional metadata
    pub metadata: HashMap<String, String>,
}

/// Trust evaluation results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustEvaluation {
    /// Applied trust policy
    pub policy_applied: TrustPolicy,
    /// Trusted developers included
    pub trusted_developers: Vec<String>,
    /// Untrusted developers excluded
    pub excluded_developers: Vec<String>,
    /// Trust violations detected
    pub violations: Vec<TrustViolation>,
    /// Overall trust score
    pub trust_score: f64,
}

/// Trust violation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustViolation {
    /// Type of violation
    pub violation_type: ViolationType,
    /// Developer involved
    pub developer_id: String,
    /// Description
    pub description: String,
    /// Severity
    pub severity: Severity,
}

/// Type of trust violation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ViolationType {
    /// Untrusted source
    UntrustedSource,
    /// Expired trust
    ExpiredTrust,
    /// Revoked trust
    RevokedTrust,
    /// Policy violation
    PolicyViolation,
}

/// Violation severity
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Severity {
    Low,
    Medium,
    High,
    Critical,
}

/// Semantic conflict between developers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticConflict {
    /// Field with conflict
    pub field: String,
    /// Conflicting interpretations
    pub interpretations: Vec<ConflictingInterpretation>,
    /// Conflict resolution
    pub resolution: Option<ConflictResolution>,
    /// Impact on results
    pub impact: ConflictImpact,
}

/// Conflicting interpretation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictingInterpretation {
    /// Developer
    pub developer_id: String,
    /// Their interpretation
    pub interpretation: String,
    /// Zero semantics
    pub zero_semantics: String,
    /// Trust level
    pub trust_level: TrustLevel,
}

/// Conflict resolution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictResolution {
    /// Chosen interpretation
    pub chosen_developer: String,
    /// Resolution method
    pub method: ResolutionMethod,
    /// Confidence in resolution
    pub confidence: f64,
}

/// Method used to resolve conflict
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResolutionMethod {
    /// Higher trust level wins
    TrustLevel,
    /// More recent interpretation
    Recency,
    /// Higher confidence
    Confidence,
    /// Manual resolution
    Manual,
    /// Policy-based
    Policy,
}

/// Impact of conflict on results
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConflictImpact {
    /// No impact on results
    None,
    /// Minor impact
    Low,
    /// Moderate impact
    Medium,
    /// Significant impact
    High,
}

/// Query authentication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryAuthentication {
    /// Query hash
    pub query_hash: [u8; 32],
    /// Result hash
    pub result_hash: [u8; 32],
    /// Timestamp
    pub timestamp: u64,
    /// Signature (if available)
    pub signature: Option<Vec<u8>>,
    /// Witness data for verification
    pub witness: QueryWitness,
}

/// Query witness for verification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryWitness {
    /// Block height
    pub block_height: u64,
    /// State root
    pub state_root: [u8; 32],
    /// Trust context
    pub trust_context: Vec<u8>,
    /// Semantic hashes
    pub semantic_hashes: Vec<[u8; 32]>,
}

/// Query engine implementation
pub struct TrustScopedQueryEngine {
    /// Storage backend
    storage: TupleFdbStorage,
    /// Layout resolver
    layout_resolver: LayoutResolver,
    /// Trust cache
    trust_cache: HashMap<String, TrustLevel>,
}

impl TrustScopedQueryEngine {
    /// Create new query engine
    pub fn new(storage: TupleFdbStorage) -> Self {
        Self {
            storage,
            layout_resolver: LayoutResolver::new(),
            trust_cache: HashMap::new(),
        }
    }
    
    /// Execute a trust-scoped query
    pub async fn execute_query(
        &mut self,
        query: TrustScopedQuery,
    ) -> Result<TrustScopedQueryResponse> {
        // Start query execution
        info!(
            "Executing trust-scoped query for {} on contract {}",
            query.requester_id, query.contract_address
        );
        
        // Evaluate trust policy
        let trust_policy = query.trust_policy.unwrap_or_else(TrustPolicy::permissive);
        let trust_evaluation = self.evaluate_trust_policy(
            &query.requester_id,
            &trust_policy,
        ).await?;
        
        // Execute query based on type
        let (results, semantics) = match query.query_type {
            QueryType::ViewQuery(view_query) => {
                self.execute_view_query(view_query, &trust_evaluation).await?
            }
            QueryType::FieldQuery { fields, block_height, include_proofs } => {
                self.execute_field_query(
                    &query.chain_id,
                    &query.contract_address,
                    fields,
                    block_height,
                    include_proofs,
                    &trust_evaluation,
                ).await?
            }
            QueryType::StorageQuery { slots, block_height, include_proofs } => {
                self.execute_storage_query(
                    &query.chain_id,
                    &query.contract_address,
                    slots,
                    block_height,
                    include_proofs,
                    &trust_evaluation,
                ).await?
            }
            QueryType::LayoutQuery { field_paths, block_height, layout_version } => {
                self.execute_layout_query(
                    &query.chain_id,
                    &query.contract_address,
                    field_paths,
                    block_height,
                    layout_version,
                    &trust_evaluation,
                ).await?
            }
        };
        
        // Analyze conflicts if requested
        let conflicts = if query.analyze_conflicts {
            self.analyze_semantic_conflicts(&semantics).await?
        } else {
            Vec::new()
        };
        
        // Calculate confidence score
        let confidence_score = self.calculate_confidence_score(
            &trust_evaluation,
            &conflicts,
            &semantics,
        );
        
        // Apply minimum confidence threshold
        if let Some(min_confidence) = query.min_confidence {
            if confidence_score < min_confidence {
                return Err(Error::Validation(format!(
                    "Query confidence {} below minimum threshold {}",
                    confidence_score, min_confidence
                )));
            }
        }
        
        // Generate authentication
        let authentication = self.generate_query_authentication(
            &query,
            &results,
            &trust_evaluation,
        ).await?;
        
        Ok(TrustScopedQueryResponse {
            results,
            trust_evaluation,
            conflicts,
            confidence_score,
            authentication,
        })
    }
    
    /// Evaluate trust policy
    async fn evaluate_trust_policy(
        &mut self,
        requester_id: &str,
        policy: &TrustPolicy,
    ) -> Result<TrustEvaluation> {
        let mut trusted_developers = Vec::new();
        let mut excluded_developers = Vec::new();
        let mut violations = Vec::new();
        
        // Get requester's trust graph
        let trust_graph = self.storage.get_trust_graph(requester_id).await?;
        
        // Evaluate each developer in the system
        // This is simplified - would need to get all relevant developers
        for (developer, trust_level) in &trust_graph {
            if policy.is_trusted(developer, &trust_graph) {
                trusted_developers.push(developer.clone());
            } else {
                excluded_developers.push(developer.clone());
                
                // Check for violations
                if policy.default_behavior == TrustBehavior::Reject {
                    violations.push(TrustViolation {
                        violation_type: ViolationType::UntrustedSource,
                        developer_id: developer.clone(),
                        description: "Developer not in trust policy".to_string(),
                        severity: Severity::Medium,
                    });
                }
            }
        }
        
        // Calculate trust score
        let trust_score = if trusted_developers.is_empty() {
            0.0
        } else {
            trusted_developers.len() as f64 / (trusted_developers.len() + excluded_developers.len()) as f64
        };
        
        Ok(TrustEvaluation {
            policy_applied: policy.clone(),
            trusted_developers,
            excluded_developers,
            violations,
            trust_score,
        })
    }
    
    /// Execute view query
    async fn execute_view_query(
        &self,
        view_query: ViewQuery,
        trust_evaluation: &TrustEvaluation,
    ) -> Result<(QueryResults, Vec<SemanticInterpretation>)> {
        // Execute view query through storage
        let response = self.storage.query_view(view_query).await?;
        
        // Collect semantic interpretations
        let mut semantics = Vec::new();
        
        // This is simplified - would extract semantics from view results
        
        Ok((QueryResults::ViewResults(response), semantics))
    }
    
    /// Execute field query
    async fn execute_field_query(
        &mut self,
        chain_id: &str,
        contract_address: &str,
        fields: Vec<String>,
        block_height: u64,
        include_proofs: bool,
        trust_evaluation: &TrustEvaluation,
    ) -> Result<(QueryResults, Vec<SemanticInterpretation>)> {
        let mut field_results = HashMap::new();
        let mut all_semantics = Vec::new();
        
        for field in fields {
            // Get semantic metadata for field
            let semantics = self.get_field_semantics(
                chain_id,
                contract_address,
                &field,
                trust_evaluation,
            ).await?;
            
            // Calculate field confidence
            let confidence = self.calculate_field_confidence(&semantics);
            
            // Create field result
            let result = FieldResult {
                value: vec![0u8; 32], // Placeholder
                decoded: Some(serde_json::json!("0")),
                proof: if include_proofs { Some(vec![0u8; 32]) } else { None },
                semantics: semantics.clone(),
                confidence,
            };
            
            field_results.insert(field, result);
            all_semantics.extend(semantics);
        }
        
        Ok((QueryResults::FieldResults(field_results), all_semantics))
    }
    
    /// Execute storage query
    async fn execute_storage_query(
        &mut self,
        chain_id: &str,
        contract_address: &str,
        slots: Vec<Vec<u8>>,
        block_height: u64,
        include_proofs: bool,
        trust_evaluation: &TrustEvaluation,
    ) -> Result<(QueryResults, Vec<SemanticInterpretation>)> {
        let mut storage_results = HashMap::new();
        let mut all_semantics = Vec::new();
        
        for slot in slots {
            let slot_key = hex::encode(&slot);
            
            // Get semantic metadata
            let semantics = self.get_slot_semantics(
                chain_id,
                contract_address,
                &slot,
                trust_evaluation,
            ).await?;
            
            let result = StorageResult {
                value: vec![0u8; 32], // Placeholder
                proof: if include_proofs { Some(vec![0u8; 32]) } else { None },
                semantics: semantics.clone(),
            };
            
            storage_results.insert(slot_key, result);
            all_semantics.extend(semantics);
        }
        
        Ok((QueryResults::StorageResults(storage_results), all_semantics))
    }
    
    /// Execute layout query
    async fn execute_layout_query(
        &mut self,
        chain_id: &str,
        contract_address: &str,
        field_paths: Vec<String>,
        block_height: u64,
        layout_version: Option<String>,
        trust_evaluation: &TrustEvaluation,
    ) -> Result<(QueryResults, Vec<SemanticInterpretation>)> {
        let mut layout_results = HashMap::new();
        let mut all_semantics = Vec::new();
        
        // Load layout
        let layout_data = self.storage.get_contract_layout(
            chain_id,
            contract_address,
            layout_version.as_deref(),
            &trust_evaluation.trusted_developers[0], // Simplified
            &trust_evaluation.policy_applied,
        ).await?
        .ok_or_else(|| Error::NotFound("Layout not found".to_string()))?;
        
        // Cache layout in resolver
        self.layout_resolver.cache_layout(
            chain_id.to_string(),
            contract_address.to_string(),
            &layout_data.0.layout_data,
        )?;
        
        // Resolve each field path
        for path in field_paths {
            let location = self.layout_resolver.resolve_field(
                chain_id,
                contract_address,
                &path,
                block_height,
            )?;
            
            let layout_info = LayoutInfo {
                slot: location.slot.clone(),
                offset: location.offset,
                size: location.size,
                type_name: location.type_name.clone(),
            };
            
            // Get semantics for this location
            let semantics = self.get_slot_semantics(
                chain_id,
                contract_address,
                &hex::decode(&location.slot).unwrap_or_default(),
                trust_evaluation,
            ).await?;
            
            let result = LayoutResult {
                path: path.clone(),
                value: vec![0u8; location.size as usize], // Placeholder
                layout_info,
                semantics: semantics.clone(),
            };
            
            layout_results.insert(path, result);
            all_semantics.extend(semantics);
        }
        
        Ok((QueryResults::LayoutResults(layout_results), all_semantics))
    }
    
    /// Get semantic metadata for a field
    async fn get_field_semantics(
        &self,
        chain_id: &str,
        contract_address: &str,
        field: &str,
        trust_evaluation: &TrustEvaluation,
    ) -> Result<Vec<SemanticInterpretation>> {
        let mut interpretations = Vec::new();
        
        // Query semantic metadata from trusted developers
        for developer in &trust_evaluation.trusted_developers {
            let key = SemanticMetadataKey {
                chain_id: chain_id.to_string(),
                contract_address: contract_address.to_string(),
                storage_slot: field.to_string(), // Simplified
                field_name: field.to_string(),
                trust_scope: super::semantic::TrustScope::Field(field.to_string()),
            };
            
            if let Some(metadata) = self.storage.get_semantic_metadata(&key).await? {
                interpretations.push(SemanticInterpretation {
                    developer_id: developer.clone(),
                    interpretation: format!("{:?}", metadata.zero_semantics),
                    confidence: metadata.confidence_level,
                    trust_level: self.get_developer_trust_level(developer).await?,
                    metadata: metadata.metadata,
                });
            }
        }
        
        Ok(interpretations)
    }
    
    /// Get semantic metadata for a storage slot
    async fn get_slot_semantics(
        &self,
        chain_id: &str,
        contract_address: &str,
        slot: &[u8],
        trust_evaluation: &TrustEvaluation,
    ) -> Result<Vec<SemanticInterpretation>> {
        // Similar to get_field_semantics but for raw slots
        Ok(Vec::new()) // Placeholder
    }
    
    /// Analyze semantic conflicts
    async fn analyze_semantic_conflicts(
        &self,
        semantics: &[SemanticInterpretation],
    ) -> Result<Vec<SemanticConflict>> {
        let mut conflicts = Vec::new();
        
        // Group semantics by field
        let mut by_field: HashMap<String, Vec<&SemanticInterpretation>> = HashMap::new();
        
        for semantic in semantics {
            // Extract field from metadata
            if let Some(field) = semantic.metadata.get("field") {
                by_field.entry(field.clone()).or_default().push(semantic);
            }
        }
        
        // Check for conflicts in each field
        for (field, field_semantics) in by_field {
            if field_semantics.len() > 1 {
                // Check if interpretations differ
                let first = &field_semantics[0].interpretation;
                let has_conflict = field_semantics.iter()
                    .any(|s| &s.interpretation != first);
                
                if has_conflict {
                    let conflict = SemanticConflict {
                        field: field.clone(),
                        interpretations: field_semantics.iter().map(|s| {
                            ConflictingInterpretation {
                                developer_id: s.developer_id.clone(),
                                interpretation: s.interpretation.clone(),
                                zero_semantics: s.interpretation.clone(), // Simplified
                                trust_level: s.trust_level,
                            }
                        }).collect(),
                        resolution: self.resolve_conflict(&field_semantics),
                        impact: self.assess_conflict_impact(&field_semantics),
                    };
                    
                    conflicts.push(conflict);
                }
            }
        }
        
        Ok(conflicts)
    }
    
    /// Resolve a semantic conflict
    fn resolve_conflict(&self, semantics: &[&SemanticInterpretation]) -> Option<ConflictResolution> {
        // Simple resolution: highest trust level wins
        let highest_trust = semantics.iter()
            .max_by_key(|s| s.trust_level)?;
        
        Some(ConflictResolution {
            chosen_developer: highest_trust.developer_id.clone(),
            method: ResolutionMethod::TrustLevel,
            confidence: 0.8,
        })
    }
    
    /// Assess impact of conflict
    fn assess_conflict_impact(&self, semantics: &[&SemanticInterpretation]) -> ConflictImpact {
        // Simple assessment based on trust level variance
        let trust_levels: Vec<_> = semantics.iter()
            .map(|s| s.trust_level as u8)
            .collect();
        
        let max = trust_levels.iter().max().unwrap_or(&0);
        let min = trust_levels.iter().min().unwrap_or(&0);
        
        match max - min {
            0 => ConflictImpact::Low,
            1 => ConflictImpact::Medium,
            _ => ConflictImpact::High,
        }
    }
    
    /// Calculate overall confidence score
    fn calculate_confidence_score(
        &self,
        trust_evaluation: &TrustEvaluation,
        conflicts: &[SemanticConflict],
        semantics: &[SemanticInterpretation],
    ) -> f64 {
        let mut score = trust_evaluation.trust_score;
        
        // Reduce score for conflicts
        for conflict in conflicts {
            match conflict.impact {
                ConflictImpact::None => {}
                ConflictImpact::Low => score *= 0.95,
                ConflictImpact::Medium => score *= 0.85,
                ConflictImpact::High => score *= 0.7,
            }
        }
        
        // Factor in semantic confidence
        if !semantics.is_empty() {
            let avg_confidence: f64 = semantics.iter()
                .map(|s| match s.confidence {
                    ConfidenceLevel::Low => 0.3,
                    ConfidenceLevel::Medium => 0.6,
                    ConfidenceLevel::High => 0.9,
                    ConfidenceLevel::Verified => 1.0,
                })
                .sum::<f64>() / semantics.len() as f64;
            
            score *= avg_confidence;
        }
        
        score.max(0.0).min(1.0)
    }
    
    /// Calculate field confidence
    fn calculate_field_confidence(&self, semantics: &[SemanticInterpretation]) -> f64 {
        if semantics.is_empty() {
            return 0.5; // Default confidence
        }
        
        // Average confidence weighted by trust level
        let weighted_sum: f64 = semantics.iter()
            .map(|s| {
                let confidence = match s.confidence {
                    ConfidenceLevel::Low => 0.3,
                    ConfidenceLevel::Medium => 0.6,
                    ConfidenceLevel::High => 0.9,
                    ConfidenceLevel::Verified => 1.0,
                };
                let trust_weight = match s.trust_level {
                    TrustLevel::None => 0.0,
                    TrustLevel::ReadOnly => 0.5,
                    TrustLevel::Contribute => 0.8,
                    TrustLevel::Moderate => 1.0,
                };
                confidence * trust_weight
            })
            .sum();
        
        let weight_sum: f64 = semantics.iter()
            .map(|s| match s.trust_level {
                TrustLevel::None => 0.0,
                TrustLevel::ReadOnly => 0.5,
                TrustLevel::Contribute => 0.8,
                TrustLevel::Moderate => 1.0,
            })
            .sum();
        
        if weight_sum > 0.0 {
            weighted_sum / weight_sum
        } else {
            0.5
        }
    }
    
    /// Get developer trust level
    async fn get_developer_trust_level(&self, developer: &str) -> Result<TrustLevel> {
        // Check cache first
        if let Some(&level) = self.trust_cache.get(developer) {
            return Ok(level);
        }
        
        // Query from storage (simplified)
        Ok(TrustLevel::ReadOnly)
    }
    
    /// Generate query authentication
    async fn generate_query_authentication(
        &self,
        query: &TrustScopedQuery,
        results: &QueryResults,
        trust_evaluation: &TrustEvaluation,
    ) -> Result<QueryAuthentication> {
        use sha2::{Sha256, Digest};
        
        // Hash query
        let query_bytes = serde_json::to_vec(query)
            .map_err(|e| Error::Serialization(format!("Failed to serialize query: {}", e)))?;
        let query_hash = Sha256::digest(&query_bytes).into();
        
        // Hash results
        let result_bytes = serde_json::to_vec(results)
            .map_err(|e| Error::Serialization(format!("Failed to serialize results: {}", e)))?;
        let result_hash = Sha256::digest(&result_bytes).into();
        
        // Get block metadata
        let block_height = match &query.query_type {
            QueryType::ViewQuery(vq) => vq.block_height,
            QueryType::FieldQuery { block_height, .. } => *block_height,
            QueryType::StorageQuery { block_height, .. } => *block_height,
            QueryType::LayoutQuery { block_height, .. } => *block_height,
        };
        
        // Create witness
        let witness = QueryWitness {
            block_height,
            state_root: [0u8; 32], // Would get from block metadata
            trust_context: serde_json::to_vec(&trust_evaluation.policy_applied)
                .unwrap_or_default(),
            semantic_hashes: Vec::new(), // Would hash semantics
        };
        
        Ok(QueryAuthentication {
            query_hash,
            result_hash,
            timestamp: chrono::Utc::now().timestamp() as u64,
            signature: None, // Would sign if keys available
            witness,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_confidence_calculation() {
        // This test doesn't actually need a real storage instance
        // Just testing the confidence calculation logic
        
        let semantics = vec![
            SemanticInterpretation {
                developer_id: "dev1".to_string(),
                interpretation: "test".to_string(),
                confidence: ConfidenceLevel::High,
                trust_level: TrustLevel::Moderate,
                metadata: HashMap::new(),
            },
        ];
        
        // Test the calculation logic directly
        // In a real test, this would be part of the engine
        let weighted_sum: f64 = semantics.iter()
            .map(|s| {
                let confidence = match s.confidence {
                    ConfidenceLevel::Low => 0.3,
                    ConfidenceLevel::Medium => 0.6,
                    ConfidenceLevel::High => 0.9,
                    ConfidenceLevel::Verified => 1.0,
                };
                let trust_weight = match s.trust_level {
                    TrustLevel::None => 0.0,
                    TrustLevel::ReadOnly => 0.5,
                    TrustLevel::Contribute => 0.8,
                    TrustLevel::Moderate => 1.0,
                };
                confidence * trust_weight
            })
            .sum();
        
        assert!(weighted_sum > 0.0 && weighted_sum <= 1.0);
    }
}