//! Layout compatibility checking for queries
//!
//! This module provides functionality to check if layouts are compatible
//! for queries, ensuring that field types and storage locations match
//! between different layout versions.

use super::layout::{ContractLayoutKey, ContractLayoutValue};
use super::TupleFdbStorage;
use almanac_infrastructure_core::{Error, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{debug, info, warn};
use traverse_core::{LayoutInfo, StorageEntry, TypeInfo};

/// Layout compatibility result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompatibilityResult {
    /// Whether layouts are compatible
    pub is_compatible: bool,
    /// Compatibility score (0.0 to 1.0)
    pub compatibility_score: f64,
    /// Breaking changes detected
    pub breaking_changes: Vec<BreakingChange>,
    /// Non-breaking changes
    pub non_breaking_changes: Vec<NonBreakingChange>,
    /// Field compatibility map
    pub field_compatibility: HashMap<String, FieldCompatibility>,
}

/// Breaking change that prevents compatibility
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BreakingChange {
    /// Type of breaking change
    pub change_type: BreakingChangeType,
    /// Field affected
    pub field: String,
    /// Description of the issue
    pub description: String,
    /// Severity
    pub severity: Severity,
}

/// Type of breaking change
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BreakingChangeType {
    /// Field type changed incompatibly
    TypeMismatch,
    /// Field moved to different slot
    SlotMismatch,
    /// Field size changed
    SizeMismatch,
    /// Field removed
    FieldRemoved,
    /// Encoding changed
    EncodingMismatch,
}

/// Non-breaking change
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NonBreakingChange {
    /// Type of change
    pub change_type: NonBreakingChangeType,
    /// Field affected
    pub field: String,
    /// Description
    pub description: String,
}

/// Type of non-breaking change
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NonBreakingChangeType {
    /// New field added
    FieldAdded,
    /// Field renamed (but same slot/type)
    FieldRenamed,
    /// Documentation updated
    DocumentationChanged,
    /// Semantic metadata changed
    SemanticChanged,
}

/// Field compatibility status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldCompatibility {
    /// Field name in old layout
    pub old_field: Option<String>,
    /// Field name in new layout
    pub new_field: Option<String>,
    /// Whether field is compatible
    pub is_compatible: bool,
    /// Compatibility notes
    pub notes: Vec<String>,
}

/// Severity of compatibility issue
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Severity {
    Low,
    Medium,
    High,
    Critical,
}

/// Layout compatibility checker
pub struct LayoutCompatibilityChecker {
    /// Type compatibility rules
    type_rules: HashMap<String, Vec<String>>,
}

impl Default for LayoutCompatibilityChecker {
    fn default() -> Self {
        let mut type_rules = HashMap::new();
        
        // Define compatible type upgrades
        type_rules.insert("t_uint8".to_string(), vec![
            "t_uint16".to_string(),
            "t_uint32".to_string(),
            "t_uint64".to_string(),
            "t_uint128".to_string(),
            "t_uint256".to_string(),
        ]);
        
        type_rules.insert("t_uint16".to_string(), vec![
            "t_uint32".to_string(),
            "t_uint64".to_string(),
            "t_uint128".to_string(),
            "t_uint256".to_string(),
        ]);
        
        type_rules.insert("t_int8".to_string(), vec![
            "t_int16".to_string(),
            "t_int32".to_string(),
            "t_int64".to_string(),
            "t_int128".to_string(),
            "t_int256".to_string(),
        ]);
        
        // Addresses can be implicitly converted to bytes20
        type_rules.insert("t_address".to_string(), vec![
            "t_bytes20".to_string(),
        ]);
        
        Self { type_rules }
    }
}

impl LayoutCompatibilityChecker {
    /// Check compatibility between two layouts
    pub fn check_compatibility(
        &self,
        old_layout: &LayoutInfo,
        new_layout: &LayoutInfo,
    ) -> CompatibilityResult {
        let mut breaking_changes = Vec::new();
        let mut non_breaking_changes = Vec::new();
        let mut field_compatibility = HashMap::new();
        
        // Build maps for efficient lookup
        let old_fields: HashMap<_, _> = old_layout.storage.iter()
            .map(|s| (s.label.clone(), s))
            .collect();
        
        let new_fields: HashMap<_, _> = new_layout.storage.iter()
            .map(|s| (s.label.clone(), s))
            .collect();
        
        let old_types: HashMap<_, _> = old_layout.types.iter()
            .map(|t| (t.label.clone(), t))
            .collect();
        
        let new_types: HashMap<_, _> = new_layout.types.iter()
            .map(|t| (t.label.clone(), t))
            .collect();
        
        // Check each field in old layout
        for (field_name, old_entry) in &old_fields {
            if let Some(new_entry) = new_fields.get(field_name) {
                // Field exists in both - check compatibility
                let compat = self.check_field_compatibility(
                    old_entry,
                    new_entry,
                    &old_types,
                    &new_types,
                );
                
                if !compat.is_compatible {
                    for note in &compat.notes {
                        breaking_changes.push(BreakingChange {
                            change_type: self.classify_breaking_change(note),
                            field: field_name.clone(),
                            description: note.clone(),
                            severity: Severity::High,
                        });
                    }
                }
                
                field_compatibility.insert(field_name.clone(), compat);
            } else {
                // Field removed
                breaking_changes.push(BreakingChange {
                    change_type: BreakingChangeType::FieldRemoved,
                    field: field_name.clone(),
                    description: format!("Field '{}' was removed", field_name),
                    severity: Severity::Critical,
                });
                
                field_compatibility.insert(field_name.clone(), FieldCompatibility {
                    old_field: Some(field_name.clone()),
                    new_field: None,
                    is_compatible: false,
                    notes: vec!["Field removed".to_string()],
                });
            }
        }
        
        // Check for new fields
        for (field_name, new_entry) in &new_fields {
            if !old_fields.contains_key(field_name) {
                non_breaking_changes.push(NonBreakingChange {
                    change_type: NonBreakingChangeType::FieldAdded,
                    field: field_name.clone(),
                    description: format!("New field '{}' added", field_name),
                });
                
                field_compatibility.insert(field_name.clone(), FieldCompatibility {
                    old_field: None,
                    new_field: Some(field_name.clone()),
                    is_compatible: true,
                    notes: vec!["New field".to_string()],
                });
            }
        }
        
        // Calculate compatibility score
        let total_fields = old_fields.len().max(new_fields.len());
        let compatible_fields = field_compatibility.values()
            .filter(|f| f.is_compatible)
            .count();
        
        let compatibility_score = if total_fields > 0 {
            compatible_fields as f64 / total_fields as f64
        } else {
            1.0
        };
        
        CompatibilityResult {
            is_compatible: breaking_changes.is_empty(),
            compatibility_score,
            breaking_changes,
            non_breaking_changes,
            field_compatibility,
        }
    }
    
    /// Check compatibility between two fields
    fn check_field_compatibility(
        &self,
        old_entry: &StorageEntry,
        new_entry: &StorageEntry,
        old_types: &HashMap<String, &TypeInfo>,
        new_types: &HashMap<String, &TypeInfo>,
    ) -> FieldCompatibility {
        let mut notes = Vec::new();
        let mut is_compatible = true;
        
        // Check slot match
        if old_entry.slot != new_entry.slot {
            notes.push(format!(
                "Storage slot changed from {} to {}",
                old_entry.slot, new_entry.slot
            ));
            is_compatible = false;
        }
        
        // Check offset match
        if old_entry.offset != new_entry.offset {
            notes.push(format!(
                "Offset changed from {} to {}",
                old_entry.offset, new_entry.offset
            ));
            is_compatible = false;
        }
        
        // Check type compatibility
        if let (Some(old_type), Some(new_type)) = (
            old_types.get(&old_entry.type_name),
            new_types.get(&new_entry.type_name),
        ) {
            if !self.are_types_compatible(old_type, new_type) {
                notes.push(format!(
                    "Type changed from {} to {}",
                    old_entry.type_name, new_entry.type_name
                ));
                is_compatible = false;
            }
        }
        
        // Check zero semantics compatibility
        if old_entry.zero_semantics != new_entry.zero_semantics {
            notes.push(format!(
                "Zero semantics changed from {:?} to {:?}",
                old_entry.zero_semantics, new_entry.zero_semantics
            ));
            // This is not a breaking change for queries
        }
        
        FieldCompatibility {
            old_field: Some(old_entry.label.clone()),
            new_field: Some(new_entry.label.clone()),
            is_compatible,
            notes,
        }
    }
    
    /// Check if two types are compatible
    fn are_types_compatible(&self, old_type: &TypeInfo, new_type: &TypeInfo) -> bool {
        // Same type is always compatible
        if old_type.label == new_type.label {
            return true;
        }
        
        // Check size compatibility
        let old_size = old_type.number_of_bytes.parse::<u32>().unwrap_or(0);
        let new_size = new_type.number_of_bytes.parse::<u32>().unwrap_or(0);
        
        if new_size < old_size {
            return false; // Can't shrink types
        }
        
        // Check encoding compatibility
        if old_type.encoding != new_type.encoding {
            return false;
        }
        
        // Check type upgrade rules
        if let Some(compatible_types) = self.type_rules.get(&old_type.label) {
            if compatible_types.contains(&new_type.label) {
                return true;
            }
        }
        
        false
    }
    
    /// Classify the type of breaking change from a note
    fn classify_breaking_change(&self, note: &str) -> BreakingChangeType {
        if note.contains("slot changed") {
            BreakingChangeType::SlotMismatch
        } else if note.contains("Type changed") {
            BreakingChangeType::TypeMismatch
        } else if note.contains("Offset changed") {
            BreakingChangeType::SizeMismatch
        } else if note.contains("encoding") {
            BreakingChangeType::EncodingMismatch
        } else {
            BreakingChangeType::TypeMismatch // Default
        }
    }
}

impl TupleFdbStorage {
    /// Check layout compatibility for a contract
    pub async fn check_layout_compatibility(
        &self,
        chain_id: &str,
        contract_address: &str,
        old_version: Option<&str>,
        new_version: Option<&str>,
    ) -> Result<CompatibilityResult> {
        // Get old layout
        let old_layout_data = self.get_contract_layout(
            chain_id,
            contract_address,
            old_version,
            "system", // System check
            &super::semantic::TrustPolicy::permissive(),
        ).await?
        .ok_or_else(|| Error::NotFound("Old layout version not found".to_string()))?;
        
        // Get new layout
        let new_layout_data = self.get_contract_layout(
            chain_id,
            contract_address,
            new_version,
            "system",
            &super::semantic::TrustPolicy::permissive(),
        ).await?
        .ok_or_else(|| Error::NotFound("New layout version not found".to_string()))?;
        
        // Parse layouts
        let old_layout: LayoutInfo = serde_json::from_slice(&old_layout_data.0.layout_data)
            .map_err(|e| Error::Deserialization(format!("Failed to parse old layout: {}", e)))?;
        
        let new_layout: LayoutInfo = serde_json::from_slice(&new_layout_data.0.layout_data)
            .map_err(|e| Error::Deserialization(format!("Failed to parse new layout: {}", e)))?;
        
        // Check compatibility
        let checker = LayoutCompatibilityChecker::default();
        let result = checker.check_compatibility(&old_layout, &new_layout);
        
        if !result.is_compatible {
            warn!(
                "Layout compatibility check failed for contract {} with {} breaking changes",
                contract_address, result.breaking_changes.len()
            );
        }
        
        Ok(result)
    }
    
    /// Find compatible layout versions for a query
    pub async fn find_compatible_layouts(
        &self,
        chain_id: &str,
        contract_address: &str,
        required_fields: &[String],
    ) -> Result<Vec<String>> {
        let mut compatible_versions = Vec::new();
        
        // Get all layout versions
        let prefix = ContractLayoutKey {
            chain_id: chain_id.to_string(),
            contract_address: contract_address.to_string(),
            version: None,
        }.to_tuple_key();
        
        let layouts = self.range(prefix, None).await?;
        
        for (_key, value) in layouts {
            let layout_value: ContractLayoutValue = rmp_serde::from_slice(&value)
                .map_err(|e| Error::Deserialization(format!("Failed to parse layout: {}", e)))?;
            
            let layout: LayoutInfo = serde_json::from_slice(&layout_value.layout_data)
                .map_err(|e| Error::Deserialization(format!("Failed to parse layout info: {}", e)))?;
            
            // Check if layout has all required fields
            let layout_fields: Vec<_> = layout.storage.iter()
                .map(|s| &s.label)
                .collect();
            
            let has_all_fields = required_fields.iter()
                .all(|f| layout_fields.contains(&f));
            
            if has_all_fields {
                // Extract version from key
                compatible_versions.push("v1".to_string()); // Placeholder
            }
        }
        
        Ok(compatible_versions)
    }
}