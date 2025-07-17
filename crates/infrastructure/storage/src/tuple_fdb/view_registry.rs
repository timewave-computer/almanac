//! View registry for custom developer-defined views over contract state
//!
//! This module provides functionality for developers to define custom views
//! over contract storage, enabling structured queries with trust policies
//! and efficient data access patterns.

use super::{TupleKey, TupleKeyspace, TupleFdbStorage};
use super::trust::{TrustPolicy, TrustLevel};
use almanac_infrastructure_core::{Error, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{debug, info, warn};

/// View definition key for tuple storage
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ViewDefinitionKey {
    /// Unique view identifier
    pub view_id: String,
    /// Optional version for view evolution
    pub version: Option<u32>,
}

impl ViewDefinitionKey {
    /// Create tuple key for storage
    pub fn to_tuple_key(&self) -> TupleKey {
        let mut key = TupleKey::new(TupleKeyspace::ViewRegistry)
            .string(&self.view_id);
        
        if let Some(version) = self.version {
            key = key.int(version as i64);
        }
        
        key
    }
    
    /// Create prefix for all versions of a view
    pub fn view_prefix(view_id: &str) -> TupleKey {
        TupleKey::new(TupleKeyspace::ViewRegistry)
            .string(view_id)
    }
}

/// View definition for custom contract state queries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ViewDefinition {
    /// Unique view identifier
    pub view_id: String,
    /// Human-readable view name
    pub view_name: String,
    /// View description
    pub description: String,
    /// Target contract
    pub chain_id: String,
    pub contract_address: String,
    /// Storage slots included in view
    pub slots: Vec<SlotDefinition>,
    /// Type decoders for values
    pub decode_types: HashMap<String, TypeDecoder>,
    /// Trust policy for this view
    pub trust_policy: TrustPolicy,
    /// View metadata
    pub metadata: ViewMetadata,
    /// View composition (inherits from other views)
    pub extends: Vec<String>,
    /// Access control
    pub access_control: AccessControl,
}

/// Storage slot definition in a view
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlotDefinition {
    /// Field name in the view
    pub field_name: String,
    /// Storage slot identifier
    pub slot_id: String,
    /// Offset within slot (for packed data)
    pub offset: u8,
    /// Size in bytes
    pub size: u8,
    /// Type name for decoding
    pub type_name: String,
    /// Optional description
    pub description: Option<String>,
    /// Whether this field is required
    pub required: bool,
    /// Default value if missing
    pub default_value: Option<Vec<u8>>,
}

/// Type decoder configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeDecoder {
    /// Base type (uint256, address, bool, etc.)
    pub base_type: String,
    /// Encoding format
    pub encoding: EncodingFormat,
    /// Additional parameters
    pub params: HashMap<String, String>,
    /// Custom decoder function (if any)
    pub custom_decoder: Option<String>,
}

/// Encoding format for values
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EncodingFormat {
    /// Raw bytes
    Raw,
    /// ABI encoded
    ABI,
    /// RLP encoded
    RLP,
    /// Borsh encoded
    Borsh,
    /// Custom encoding
    Custom,
}

/// View metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ViewMetadata {
    /// View creator
    pub created_by: String,
    /// Creation timestamp
    pub created_at: u64,
    /// Last update timestamp
    pub updated_at: u64,
    /// View version
    pub version: u32,
    /// Tags for categorization
    pub tags: Vec<String>,
    /// External documentation URL
    pub documentation_url: Option<String>,
    /// View status
    pub status: ViewStatus,
}

/// View status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ViewStatus {
    /// View is active and usable
    Active,
    /// View is deprecated
    Deprecated,
    /// View is in draft mode
    Draft,
    /// View is archived
    Archived,
}

/// Access control for views
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessControl {
    /// Who can read this view
    pub read_access: AccessLevel,
    /// Who can update this view
    pub write_access: AccessLevel,
    /// Specific allowed/denied users
    pub allow_list: Vec<String>,
    pub deny_list: Vec<String>,
    /// Rate limiting
    pub rate_limit: Option<RateLimit>,
}

/// Access level for operations
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AccessLevel {
    /// Anyone can access
    Public,
    /// Only authenticated users
    Authenticated,
    /// Only trusted developers
    Trusted,
    /// Only view owner
    Owner,
    /// Custom access control
    Custom,
}

/// Rate limiting configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimit {
    /// Requests per minute
    pub requests_per_minute: u32,
    /// Burst limit
    pub burst_limit: u32,
    /// Cost per request
    pub cost_per_request: u32,
}

/// View query request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ViewQuery {
    /// View to query
    pub view_id: String,
    /// Optional view version
    pub version: Option<u32>,
    /// Block height for query
    pub block_height: u64,
    /// Field filters (only return specific fields)
    pub fields: Option<Vec<String>>,
    /// Include proofs
    pub include_proofs: bool,
    /// Trust context
    pub requester_id: String,
}

/// View query response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ViewQueryResponse {
    /// View metadata
    pub view_id: String,
    pub view_version: u32,
    /// Query results
    pub fields: HashMap<String, FieldValue>,
    /// Block context
    pub block_height: u64,
    pub block_hash: String,
    /// Trust context
    pub trust_context: TrustContext,
    /// Query metadata
    pub query_metadata: QueryMetadata,
}

/// Field value in query response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldValue {
    /// Raw value
    pub raw_value: Vec<u8>,
    /// Decoded value (if decoder available)
    pub decoded_value: Option<serde_json::Value>,
    /// Storage proof (if requested)
    pub proof: Option<Vec<u8>>,
    /// Field metadata
    pub metadata: FieldMetadata,
}

/// Field metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldMetadata {
    /// Storage slot
    pub slot_id: String,
    /// Type information
    pub type_name: String,
    /// Whether value is default
    pub is_default: bool,
    /// Semantic interpretation
    pub semantics: Option<String>,
}

/// Trust context for query
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustContext {
    /// Trust policy applied
    pub policy_applied: String,
    /// Trust level of requester
    pub requester_trust_level: TrustLevel,
    /// Conflicting interpretations (if any)
    pub conflicts: Vec<String>,
    /// Confidence score
    pub confidence: f64,
}

/// Query metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryMetadata {
    /// Query execution time
    pub execution_time_ms: u64,
    /// Number of storage reads
    pub storage_reads: u32,
    /// Cache hits
    pub cache_hits: u32,
    /// Query cost
    pub query_cost: u32,
}

impl TupleFdbStorage {
    /// Register a new view definition
    pub async fn register_view(
        &self,
        definition: ViewDefinition,
    ) -> Result<()> {
        // Validate view definition
        self.validate_view_definition(&definition).await?;
        
        let key = ViewDefinitionKey {
            view_id: definition.view_id.clone(),
            version: Some(definition.metadata.version),
        };
        
        let tuple_key = key.to_tuple_key();
        let serialized = serde_json::to_vec(&definition)
            .map_err(|e| Error::Serialization(format!("Failed to serialize view: {}", e)))?;
        
        self.set(tuple_key, serialized).await?;
        
        // Update view index
        self.update_view_index(&definition).await?;
        
        info!(
            "Registered view '{}' version {} for contract {}",
            definition.view_id, definition.metadata.version, definition.contract_address
        );
        
        Ok(())
    }
    
    /// Get a view definition
    pub async fn get_view(
        &self,
        view_id: &str,
        version: Option<u32>,
    ) -> Result<Option<ViewDefinition>> {
        let key = ViewDefinitionKey {
            view_id: view_id.to_string(),
            version,
        };
        
        if version.is_some() {
            // Get specific version
            let tuple_key = key.to_tuple_key();
            
            if let Some(data) = self.get(tuple_key).await? {
                let definition: ViewDefinition = serde_json::from_slice(&data)
                    .map_err(|e| Error::Deserialization(format!("Failed to parse view: {}", e)))?;
                Ok(Some(definition))
            } else {
                Ok(None)
            }
        } else {
            // Get latest version
            let prefix = ViewDefinitionKey::view_prefix(view_id);
            let results = self.range_reverse(prefix, Some(1)).await?;
            
            if let Some((_, data)) = results.into_iter().next() {
                let definition: ViewDefinition = serde_json::from_slice(&data)
                    .map_err(|e| Error::Deserialization(format!("Failed to parse view: {}", e)))?;
                Ok(Some(definition))
            } else {
                Ok(None)
            }
        }
    }
    
    /// Execute a view query
    pub async fn query_view(
        &self,
        query: ViewQuery,
    ) -> Result<ViewQueryResponse> {
        // Get view definition
        let view = self.get_view(&query.view_id, query.version).await?
            .ok_or_else(|| Error::NotFound(format!("View '{}' not found", query.view_id)))?;
        
        // Check access control
        self.check_view_access(&view, &query.requester_id).await?;
        
        // Resolve extended views
        let all_slots = self.resolve_view_slots(&view).await?;
        
        // Filter requested fields
        let slots_to_query = if let Some(ref fields) = query.fields {
            all_slots.into_iter()
                .filter(|s| fields.contains(&s.field_name))
                .collect()
        } else {
            all_slots
        };
        
        // Query storage for each slot
        let mut fields = HashMap::new();
        let start_time = std::time::Instant::now();
        let mut storage_reads = 0u32;
        let mut cache_hits = 0u32;
        
        for slot in slots_to_query {
            match self.query_slot(&view, &slot, query.block_height, query.include_proofs).await {
                Ok((value, from_cache)) => {
                    fields.insert(slot.field_name.clone(), value);
                    storage_reads += 1;
                    if from_cache {
                        cache_hits += 1;
                    }
                }
                Err(e) => {
                    warn!("Failed to query slot {}: {}", slot.field_name, e);
                    // Use default value if available
                    if let Some(default) = slot.default_value {
                        fields.insert(slot.field_name.clone(), FieldValue {
                            raw_value: default,
                            decoded_value: None,
                            proof: None,
                            metadata: FieldMetadata {
                                slot_id: slot.slot_id,
                                type_name: slot.type_name,
                                is_default: true,
                                semantics: None,
                            },
                        });
                    } else if slot.required {
                        return Err(Error::NotFound(format!("Required field '{}' not found", slot.field_name)));
                    }
                }
            }
        }
        
        // Get block metadata
        let block_hash = self.get_block_hash(&view.chain_id, query.block_height).await?
            .unwrap_or_else(|| "0x0000000000000000000000000000000000000000000000000000000000000000".to_string());
        
        // Build trust context
        let trust_context = TrustContext {
            policy_applied: format!("{:?}", view.trust_policy),
            requester_trust_level: self.get_requester_trust_level(&query.requester_id).await?,
            conflicts: Vec::new(), // TODO: Detect conflicts
            confidence: 0.95, // TODO: Calculate actual confidence
        };
        
        let execution_time = start_time.elapsed().as_millis() as u64;
        
        Ok(ViewQueryResponse {
            view_id: view.view_id,
            view_version: view.metadata.version,
            fields,
            block_height: query.block_height,
            block_hash,
            trust_context,
            query_metadata: QueryMetadata {
                execution_time_ms: execution_time,
                storage_reads,
                cache_hits,
                query_cost: storage_reads * 10, // Simple cost model
            },
        })
    }
    
    /// Update a view definition
    pub async fn update_view(
        &self,
        view_id: &str,
        updater_id: &str,
        updates: ViewUpdate,
    ) -> Result<()> {
        // Get current view
        let mut view = self.get_view(view_id, None).await?
            .ok_or_else(|| Error::NotFound(format!("View '{}' not found", view_id)))?;
        
        // Check write access
        if view.metadata.created_by != updater_id {
            return Err(Error::Authorization("Only view owner can update".to_string()));
        }
        
        // Apply updates
        if let Some(slots) = updates.slots {
            view.slots = slots;
        }
        if let Some(trust_policy) = updates.trust_policy {
            view.trust_policy = trust_policy;
        }
        if let Some(status) = updates.status {
            view.metadata.status = status;
        }
        
        // Increment version
        view.metadata.version += 1;
        view.metadata.updated_at = chrono::Utc::now().timestamp() as u64;
        
        // Store updated view
        self.register_view(view).await?;
        
        Ok(())
    }
    
    /// Validate view definition
    async fn validate_view_definition(&self, definition: &ViewDefinition) -> Result<()> {
        // Check view ID format
        if definition.view_id.is_empty() {
            return Err(Error::Validation("View ID cannot be empty".to_string()));
        }
        
        // Check for duplicate field names
        let mut field_names = std::collections::HashSet::new();
        for slot in &definition.slots {
            if !field_names.insert(&slot.field_name) {
                return Err(Error::Validation(format!(
                    "Duplicate field name: {}",
                    slot.field_name
                )));
            }
        }
        
        // Validate slot definitions
        for slot in &definition.slots {
            if slot.offset as usize + slot.size as usize > 32 {
                return Err(Error::Validation(format!(
                    "Slot {} exceeds 32-byte boundary",
                    slot.field_name
                )));
            }
        }
        
        // Check extended views exist
        for parent_view in &definition.extends {
            if self.get_view(parent_view, None).await.ok().flatten().is_none() {
                return Err(Error::Validation(format!(
                    "Extended view '{}' not found",
                    parent_view
                )));
            }
        }
        
        Ok(())
    }
    
    /// Update view index
    async fn update_view_index(&self, definition: &ViewDefinition) -> Result<()> {
        // Index by contract
        let contract_key = TupleKey::new(TupleKeyspace::ViewRegistry)
            .string("by_contract")
            .string(&definition.chain_id)
            .string(&definition.contract_address)
            .string(&definition.view_id);
        
        self.set(contract_key, vec![1]).await?;
        
        // Index by creator
        let creator_key = TupleKey::new(TupleKeyspace::ViewRegistry)
            .string("by_creator")
            .string(&definition.metadata.created_by)
            .string(&definition.view_id);
        
        self.set(creator_key, vec![1]).await?;
        
        // Index by tags
        for tag in &definition.metadata.tags {
            let tag_key = TupleKey::new(TupleKeyspace::ViewRegistry)
                .string("by_tag")
                .string(tag)
                .string(&definition.view_id);
            
            self.set(tag_key, vec![1]).await?;
        }
        
        Ok(())
    }
    
    /// Check view access
    async fn check_view_access(
        &self,
        view: &ViewDefinition,
        requester_id: &str,
    ) -> Result<()> {
        // Check deny list
        if view.access_control.deny_list.contains(&requester_id.to_string()) {
            return Err(Error::Authorization("Access denied".to_string()));
        }
        
        // Check allow list if not public
        if view.access_control.read_access != AccessLevel::Public {
            if !view.access_control.allow_list.contains(&requester_id.to_string()) {
                // Check if user meets access level requirements
                match view.access_control.read_access {
                    AccessLevel::Authenticated => {
                        // Check if user is authenticated (placeholder)
                    }
                    AccessLevel::Trusted => {
                        // Check trust level
                        let trust_level = self.get_requester_trust_level(requester_id).await?;
                        if trust_level < TrustLevel::ReadOnly {
                            return Err(Error::Authorization("Insufficient trust level".to_string()));
                        }
                    }
                    AccessLevel::Owner => {
                        if requester_id != view.metadata.created_by {
                            return Err(Error::Authorization("Owner access required".to_string()));
                        }
                    }
                    _ => {}
                }
            }
        }
        
        Ok(())
    }
    
    /// Resolve all slots including inherited ones
    async fn resolve_view_slots(&self, view: &ViewDefinition) -> Result<Vec<SlotDefinition>> {
        let mut all_slots = view.slots.clone();
        let mut seen_fields: std::collections::HashSet<String> = std::collections::HashSet::new();
        
        for slot in &view.slots {
            seen_fields.insert(slot.field_name.clone());
        }
        
        // Resolve extended views
        for parent_id in &view.extends {
            if let Some(parent_view) = self.get_view(parent_id, None).await? {
                for slot in parent_view.slots {
                    if !seen_fields.contains(&slot.field_name) {
                        seen_fields.insert(slot.field_name.clone());
                        all_slots.push(slot);
                    }
                }
            }
        }
        
        Ok(all_slots)
    }
    
    /// Query a single slot
    async fn query_slot(
        &self,
        view: &ViewDefinition,
        slot: &SlotDefinition,
        block_height: u64,
        include_proof: bool,
    ) -> Result<(FieldValue, bool)> {
        // This is simplified - would integrate with contract_state module
        let raw_value = vec![0u8; slot.size as usize]; // Placeholder
        let from_cache = false;
        
        let decoded_value = if let Some(decoder) = view.decode_types.get(&slot.type_name) {
            // Decode value based on type
            match decoder.base_type.as_str() {
                "uint256" => Some(serde_json::json!("0")),
                "address" => Some(serde_json::json!("0x0000000000000000000000000000000000000000")),
                "bool" => Some(serde_json::json!(false)),
                _ => None,
            }
        } else {
            None
        };
        
        let proof = if include_proof {
            Some(vec![0u8; 32]) // Placeholder proof
        } else {
            None
        };
        
        Ok((FieldValue {
            raw_value,
            decoded_value,
            proof,
            metadata: FieldMetadata {
                slot_id: slot.slot_id.clone(),
                type_name: slot.type_name.clone(),
                is_default: false,
                semantics: slot.description.clone(),
            },
        }, from_cache))
    }
    
    /// Get block hash
    async fn get_block_hash(&self, chain_id: &str, block_height: u64) -> Result<Option<String>> {
        // This would integrate with block_meta module
        Ok(Some(format!("0x{:064x}", block_height)))
    }
    
    /// Get requester trust level
    async fn get_requester_trust_level(&self, requester_id: &str) -> Result<TrustLevel> {
        // This would integrate with trust module
        Ok(TrustLevel::ReadOnly)
    }
}

/// View update request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ViewUpdate {
    /// Updated slots
    pub slots: Option<Vec<SlotDefinition>>,
    /// Updated trust policy
    pub trust_policy: Option<TrustPolicy>,
    /// Updated status
    pub status: Option<ViewStatus>,
}

/// View discovery query
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ViewDiscoveryQuery {
    /// Filter by contract
    pub chain_id: Option<String>,
    pub contract_address: Option<String>,
    /// Filter by creator
    pub created_by: Option<String>,
    /// Filter by tags
    pub tags: Option<Vec<String>>,
    /// Filter by status
    pub status: Option<ViewStatus>,
    /// Pagination
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

impl TupleFdbStorage {
    /// Discover views based on filters
    pub async fn discover_views(
        &self,
        query: ViewDiscoveryQuery,
    ) -> Result<Vec<ViewDefinition>> {
        let mut results = Vec::new();
        
        // Determine which index to use
        let prefix = if let (Some(chain_id), Some(contract_address)) = (&query.chain_id, &query.contract_address) {
            TupleKey::new(TupleKeyspace::ViewRegistry)
                .string("by_contract")
                .string(chain_id)
                .string(contract_address)
        } else if let Some(created_by) = &query.created_by {
            TupleKey::new(TupleKeyspace::ViewRegistry)
                .string("by_creator")
                .string(created_by)
        } else if let Some(tags) = &query.tags {
            if let Some(first_tag) = tags.first() {
                TupleKey::new(TupleKeyspace::ViewRegistry)
                    .string("by_tag")
                    .string(first_tag)
            } else {
                return Ok(results);
            }
        } else {
            // No filter, scan all views
            TupleKey::new(TupleKeyspace::ViewRegistry)
        };
        
        let kvs = self.range(prefix, query.limit).await?;
        
        for (key_bytes, _) in kvs {
            // Extract view ID from key
            // This is simplified - would need proper tuple parsing
            if let Ok(view_id) = String::from_utf8(key_bytes[key_bytes.len()-32..].to_vec()) {
                if let Some(view) = self.get_view(&view_id, None).await? {
                    // Apply additional filters
                    if let Some(status) = query.status {
                        if view.metadata.status != status {
                            continue;
                        }
                    }
                    
                    results.push(view);
                }
            }
        }
        
        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_view_definition_key() {
        let key = ViewDefinitionKey {
            view_id: "my_view".to_string(),
            version: Some(1),
        };
        
        let tuple_key = key.to_tuple_key();
        assert_eq!(tuple_key.keyspace(), Some(TupleKeyspace::ViewRegistry));
    }
    
    #[test]
    fn test_slot_validation() {
        let slot = SlotDefinition {
            field_name: "balance".to_string(),
            slot_id: "0x0".to_string(),
            offset: 0,
            size: 32,
            type_name: "uint256".to_string(),
            description: Some("User balance".to_string()),
            required: true,
            default_value: None,
        };
        
        // Valid slot
        assert!(slot.offset as usize + slot.size as usize <= 32);
        
        // Invalid slot
        let invalid_slot = SlotDefinition {
            offset: 20,
            size: 32,
            ..slot
        };
        assert!(invalid_slot.offset as usize + invalid_slot.size as usize > 32);
    }
}