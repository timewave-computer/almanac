//! Layout resolver for mapping Traverse layout information to storage keys
//! 
//! This module provides the bridge between Traverse's layout information
//! and our tuple-based storage keys, enabling efficient queries based on
//! semantic field names rather than raw storage slots.

use crate::tuple_fdb::{TupleKey, TupleKeyspace};
use almanac_infrastructure_core::{Error, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{debug, info, warn};

// Import Traverse types
use traverse_core::{LayoutInfo, StorageEntry, ZeroSemantics};

/// Resolved storage location from a layout query
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResolvedStorageLocation {
    /// The chain ID
    pub chain_id: String,
    /// The contract address
    pub contract_address: String,
    /// The storage slot (as string for compatibility)
    pub slot: String,
    /// Byte offset within the slot (for packed variables)
    pub offset: u8,
    /// Size in bytes of the field
    pub size: u8,
    /// Type name from the layout
    pub type_name: String,
    /// Zero semantics for the field
    pub zero_semantics: ZeroSemantics,
    /// The resolved storage key for FDB
    pub storage_key: Vec<u8>,
}

/// Layout resolver that maps field names to storage locations
pub struct LayoutResolver {
    /// Cached parsed layouts
    layout_cache: HashMap<(String, String), ParsedLayout>,
}

/// Parsed layout information optimized for lookups
#[derive(Debug, Clone)]
struct ParsedLayout {
    /// The original layout info
    layout: LayoutInfo,
    /// Field name to storage entry mapping
    field_map: HashMap<String, StorageEntry>,
    /// Type information map
    type_map: HashMap<String, TypeInfo>,
}

/// Type information extracted from Traverse
#[derive(Debug, Clone)]
struct TypeInfo {
    /// Size in bytes
    size: u8,
    /// Encoding type
    encoding: String,
    /// Base type for arrays/mappings
    base: Option<String>,
    /// Key type for mappings
    key: Option<String>,
    /// Value type for mappings
    value: Option<String>,
}

impl LayoutResolver {
    /// Create a new layout resolver
    pub fn new() -> Self {
        Self {
            layout_cache: HashMap::new(),
        }
    }
    
    /// Parse and cache a layout
    pub fn cache_layout(
        &mut self,
        chain_id: String,
        contract_address: String,
        layout_data: &[u8],
    ) -> Result<()> {
        // Parse the layout data
        let layout: LayoutInfo = serde_json::from_slice(layout_data)
            .map_err(|e| Error::Serialization(format!("Failed to parse layout: {}", e)))?;
        
        // Validate the layout
        layout.validate()
            .map_err(|e| Error::Validation(format!("Invalid layout: {}", e)))?;
        
        // Build field map
        let mut field_map = HashMap::new();
        for entry in &layout.storage {
            field_map.insert(entry.label.clone(), entry.clone());
        }
        
        // Build type map
        let mut type_map = HashMap::new();
        for type_info in &layout.types {
            let size = type_info.number_of_bytes.parse::<u8>()
                .map_err(|_| Error::Validation(format!("Invalid type size: {}", type_info.number_of_bytes)))?;
            
            type_map.insert(type_info.label.clone(), TypeInfo {
                size,
                encoding: type_info.encoding.clone(),
                base: type_info.base.clone(),
                key: type_info.key.clone(),
                value: type_info.value.clone(),
            });
        }
        
        let parsed = ParsedLayout {
            layout,
            field_map,
            type_map,
        };
        
        self.layout_cache.insert((chain_id, contract_address), parsed);
        
        Ok(())
    }
    
    /// Resolve a field name to a storage location
    pub fn resolve_field(
        &self,
        chain_id: &str,
        contract_address: &str,
        field_name: &str,
        block_height: u64,
    ) -> Result<ResolvedStorageLocation> {
        // Get cached layout
        let parsed = self.layout_cache
            .get(&(chain_id.to_string(), contract_address.to_string()))
            .ok_or_else(|| Error::NotFound(format!(
                "No layout cached for contract {} on chain {}",
                contract_address, chain_id
            )))?;
        
        // Find field in layout
        let storage_entry = parsed.field_map
            .get(field_name)
            .ok_or_else(|| Error::NotFound(format!(
                "Field '{}' not found in contract layout",
                field_name
            )))?;
        
        // Get type info
        let type_info = parsed.type_map
            .get(&storage_entry.type_name)
            .ok_or_else(|| Error::NotFound(format!(
                "Type '{}' not found in layout",
                storage_entry.type_name
            )))?;
        
        // Build storage key
        let storage_key = self.build_storage_key(
            chain_id,
            contract_address,
            &storage_entry.slot,
            block_height,
        )?;
        
        Ok(ResolvedStorageLocation {
            chain_id: chain_id.to_string(),
            contract_address: contract_address.to_string(),
            slot: storage_entry.slot.clone(),
            offset: storage_entry.offset,
            size: type_info.size,
            type_name: storage_entry.type_name.clone(),
            zero_semantics: storage_entry.zero_semantics,
            storage_key,
        })
    }
    
    /// Resolve multiple fields in a single operation
    pub fn resolve_fields(
        &self,
        chain_id: &str,
        contract_address: &str,
        field_names: &[String],
        block_height: u64,
    ) -> Result<Vec<ResolvedStorageLocation>> {
        let mut results = Vec::new();
        
        for field_name in field_names {
            match self.resolve_field(chain_id, contract_address, field_name, block_height) {
                Ok(location) => results.push(location),
                Err(e) => {
                    warn!("Failed to resolve field '{}': {}", field_name, e);
                    // Continue with other fields
                }
            }
        }
        
        if results.is_empty() {
            return Err(Error::NotFound("No fields could be resolved".to_string()));
        }
        
        Ok(results)
    }
    
    /// Resolve a mapping access (e.g., balances[address])
    pub fn resolve_mapping(
        &self,
        chain_id: &str,
        contract_address: &str,
        mapping_name: &str,
        key: &[u8],
        block_height: u64,
    ) -> Result<ResolvedStorageLocation> {
        // Get cached layout
        let parsed = self.layout_cache
            .get(&(chain_id.to_string(), contract_address.to_string()))
            .ok_or_else(|| Error::NotFound(format!(
                "No layout cached for contract {} on chain {}",
                contract_address, chain_id
            )))?;
        
        // Find mapping in layout
        let storage_entry = parsed.field_map
            .get(mapping_name)
            .ok_or_else(|| Error::NotFound(format!(
                "Mapping '{}' not found in contract layout",
                mapping_name
            )))?;
        
        // Get type info
        let type_info = parsed.type_map
            .get(&storage_entry.type_name)
            .ok_or_else(|| Error::NotFound(format!(
                "Type '{}' not found in layout",
                storage_entry.type_name
            )))?;
        
        // Verify it's a mapping
        if type_info.encoding != "mapping" {
            return Err(Error::Validation(format!(
                "Field '{}' is not a mapping (type: {})",
                mapping_name, type_info.encoding
            )));
        }
        
        // Calculate mapping slot using keccak256(key || base_slot)
        let mapping_slot = self.calculate_mapping_slot(&storage_entry.slot, key)?;
        
        // Build storage key
        let storage_key = self.build_storage_key(
            chain_id,
            contract_address,
            &mapping_slot,
            block_height,
        )?;
        
        Ok(ResolvedStorageLocation {
            chain_id: chain_id.to_string(),
            contract_address: contract_address.to_string(),
            slot: mapping_slot,
            offset: 0, // Mappings don't have offsets
            size: type_info.size,
            type_name: storage_entry.type_name.clone(),
            zero_semantics: storage_entry.zero_semantics,
            storage_key,
        })
    }
    
    /// Build a storage key for FDB
    fn build_storage_key(
        &self,
        chain_id: &str,
        contract_address: &str,
        slot: &str,
        block_height: u64,
    ) -> Result<Vec<u8>> {
        // Parse slot as hex string
        let slot_bytes = if slot.starts_with("0x") {
            hex::decode(&slot[2..])
                .map_err(|e| Error::Validation(format!("Invalid slot hex: {}", e)))?
        } else {
            hex::decode(slot)
                .map_err(|e| Error::Validation(format!("Invalid slot hex: {}", e)))?
        };
        
        // Pad to 32 bytes if necessary
        let mut slot_array = [0u8; 32];
        if slot_bytes.len() > 32 {
            return Err(Error::Validation("Slot exceeds 32 bytes".to_string()));
        }
        slot_array[32 - slot_bytes.len()..].copy_from_slice(&slot_bytes);
        
        // Build tuple key
        let key = TupleKey::new(TupleKeyspace::ContractState)
            .string(chain_id)
            .string(contract_address)
            .int(block_height as i64)
            .bytes(slot_array.to_vec())
            .build();
        
        Ok(key)
    }
    
    /// Calculate mapping slot using Ethereum-style keccak256 hashing
    fn calculate_mapping_slot(&self, base_slot: &str, key: &[u8]) -> Result<String> {
        use sha3::{Digest, Keccak256};
        
        // Parse base slot
        let base_slot_bytes = if base_slot.starts_with("0x") {
            hex::decode(&base_slot[2..])
                .map_err(|e| Error::Validation(format!("Invalid base slot hex: {}", e)))?
        } else {
            hex::decode(base_slot)
                .map_err(|e| Error::Validation(format!("Invalid base slot hex: {}", e)))?
        };
        
        // Pad to 32 bytes
        let mut padded_base = [0u8; 32];
        padded_base[32 - base_slot_bytes.len()..].copy_from_slice(&base_slot_bytes);
        
        // Pad key to 32 bytes
        let mut padded_key = [0u8; 32];
        if key.len() > 32 {
            return Err(Error::Validation("Mapping key exceeds 32 bytes".to_string()));
        }
        padded_key[32 - key.len()..].copy_from_slice(key);
        
        // Calculate keccak256(key || base_slot)
        let mut hasher = Keccak256::new();
        hasher.update(padded_key);
        hasher.update(padded_base);
        let result = hasher.finalize();
        
        Ok(format!("0x{}", hex::encode(result)))
    }
    
    /// Get all fields for a contract
    pub fn get_all_fields(
        &self,
        chain_id: &str,
        contract_address: &str,
    ) -> Result<Vec<String>> {
        let parsed = self.layout_cache
            .get(&(chain_id.to_string(), contract_address.to_string()))
            .ok_or_else(|| Error::NotFound(format!(
                "No layout cached for contract {} on chain {}",
                contract_address, chain_id
            )))?;
        
        Ok(parsed.field_map.keys().cloned().collect())
    }
    
    /// Check if a layout is cached
    pub fn has_layout(&self, chain_id: &str, contract_address: &str) -> bool {
        self.layout_cache.contains_key(&(chain_id.to_string(), contract_address.to_string()))
    }
    
    /// Clear a cached layout
    pub fn clear_layout(&mut self, chain_id: &str, contract_address: &str) {
        self.layout_cache.remove(&(chain_id.to_string(), contract_address.to_string()));
    }
    
    /// Clear all cached layouts
    pub fn clear_all(&mut self) {
        self.layout_cache.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_layout_resolver() {
        let mut resolver = LayoutResolver::new();
        
        // Create test layout
        let layout = LayoutInfo {
            contract_name: "TestToken".to_string(),
            storage: vec![
                StorageEntry {
                    label: "totalSupply".to_string(),
                    slot: "0".to_string(),
                    offset: 0,
                    type_name: "t_uint256".to_string(),
                    zero_semantics: ZeroSemantics::ValidZero,
                },
                StorageEntry {
                    label: "owner".to_string(),
                    slot: "1".to_string(),
                    offset: 0,
                    type_name: "t_address".to_string(),
                    zero_semantics: ZeroSemantics::NeverWritten,
                },
            ],
            types: vec![
                traverse_core::TypeInfo {
                    label: "t_uint256".to_string(),
                    number_of_bytes: "32".to_string(),
                    encoding: "inplace".to_string(),
                    base: None,
                    key: None,
                    value: None,
                },
                traverse_core::TypeInfo {
                    label: "t_address".to_string(),
                    number_of_bytes: "20".to_string(),
                    encoding: "inplace".to_string(),
                    base: None,
                    key: None,
                    value: None,
                },
            ],
        };
        
        // Cache the layout
        let layout_data = serde_json::to_vec(&layout).unwrap();
        resolver.cache_layout(
            "ethereum-mainnet".to_string(),
            "0x1234567890123456789012345678901234567890".to_string(),
            &layout_data,
        ).unwrap();
        
        // Test field resolution
        let location = resolver.resolve_field(
            "ethereum-mainnet",
            "0x1234567890123456789012345678901234567890",
            "totalSupply",
            12345678,
        ).unwrap();
        
        assert_eq!(location.slot, "0");
        assert_eq!(location.offset, 0);
        assert_eq!(location.size, 32);
        assert_eq!(location.type_name, "t_uint256");
    }
}