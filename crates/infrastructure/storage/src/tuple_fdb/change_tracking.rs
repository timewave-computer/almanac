//! Storage slot change tracking for efficient state diff computation
//!
//! This module provides functionality to track changes to contract storage slots
//! over time, enabling efficient querying of state changes and computation of
//! state diffs between blocks.

use super::{TupleKey, TupleKeyspace, TupleFdbStorage};
use almanac_infrastructure_core::{Error, Result};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use tracing::{debug, info, warn};

/// Change tracking key for tuple storage
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ChangeTrackingKey {
    /// Chain identifier
    pub chain_id: String,
    /// Contract address
    pub contract_address: String,
    /// Block height where change occurred
    pub block_height: u64,
    /// Optional transaction index for ordering
    pub tx_index: Option<u32>,
}

impl ChangeTrackingKey {
    /// Create tuple key for storage
    pub fn to_tuple_key(&self) -> TupleKey {
        let mut key = TupleKey::new(TupleKeyspace::ContractState)
            .string("changes")
            .string(&self.chain_id)
            .string(&self.contract_address)
            .int(self.block_height as i64);
        
        if let Some(tx_idx) = self.tx_index {
            key = key.int(tx_idx as i64);
        }
        
        key
    }
    
    /// Create prefix for contract changes
    pub fn contract_prefix(chain_id: &str, contract_address: &str) -> TupleKey {
        TupleKey::new(TupleKeyspace::ContractState)
            .string("changes")
            .string(chain_id)
            .string(contract_address)
    }
    
    /// Create prefix for block range
    pub fn block_range_prefix(
        chain_id: &str,
        contract_address: &str,
        from_block: u64,
    ) -> TupleKey {
        TupleKey::new(TupleKeyspace::ContractState)
            .string("changes")
            .string(chain_id)
            .string(contract_address)
            .int(from_block as i64)
    }
}

/// Storage slot change record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageSlotChange {
    /// Storage slot that changed
    pub slot_id: String,
    /// Previous value (None if slot was created)
    pub previous_value: Option<Vec<u8>>,
    /// New value (None if slot was deleted)
    pub new_value: Option<Vec<u8>>,
    /// Type of change
    pub change_type: ChangeType,
    /// Gas used for this change
    pub gas_used: Option<u64>,
    /// Associated event logs
    pub event_logs: Vec<String>,
}

/// Type of storage change
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChangeType {
    /// Slot created (first write)
    Created,
    /// Slot updated
    Updated,
    /// Slot deleted/cleared
    Deleted,
    /// No actual change (same value written)
    NoOp,
}

/// Batch of changes in a block
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangeTrackingValue {
    /// All storage changes in this block/tx
    pub changes: Vec<StorageSlotChange>,
    /// Block metadata
    pub block_hash: String,
    pub block_timestamp: u64,
    /// Transaction metadata
    pub tx_hash: String,
    pub tx_index: u32,
    /// Summary statistics
    pub total_slots_changed: u32,
    pub total_gas_used: u64,
    /// Indexed timestamp
    pub indexed_at: u64,
}

/// Change summary for a contract
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractChangeSummary {
    /// Contract identifier
    pub chain_id: String,
    pub contract_address: String,
    /// Change statistics
    pub total_changes: u64,
    pub unique_slots_changed: u64,
    pub first_change_block: u64,
    pub last_change_block: u64,
    /// Change frequency
    pub changes_per_block: HashMap<u64, u32>,
    /// Most frequently changed slots
    pub hot_slots: Vec<(String, u64)>,
    /// Gas statistics
    pub total_gas_spent: u64,
    pub average_gas_per_change: u64,
}

/// Query parameters for change tracking
#[derive(Debug, Clone, Default)]
pub struct ChangeQuery {
    /// Target contract
    pub chain_id: String,
    pub contract_address: String,
    /// Block range
    pub from_block: Option<u64>,
    pub to_block: Option<u64>,
    /// Filter by slot
    pub slot_ids: Option<Vec<String>>,
    /// Filter by change type
    pub change_types: Option<Vec<ChangeType>>,
    /// Include previous values
    pub include_values: bool,
    /// Maximum results
    pub limit: Option<usize>,
}

/// State diff between two blocks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateDiff {
    /// Contract identifier
    pub chain_id: String,
    pub contract_address: String,
    /// Block range
    pub from_block: u64,
    pub to_block: u64,
    /// Changed slots
    pub changes: HashMap<String, SlotDiff>,
    /// Summary
    pub slots_added: u32,
    pub slots_modified: u32,
    pub slots_deleted: u32,
    pub total_gas_used: u64,
}

/// Diff for a single slot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlotDiff {
    /// Value at from_block
    pub from_value: Option<Vec<u8>>,
    /// Value at to_block
    pub to_value: Option<Vec<u8>>,
    /// All intermediate changes
    pub intermediate_changes: Vec<IntermediateChange>,
}

/// Intermediate change in a diff
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntermediateChange {
    pub block_height: u64,
    pub tx_hash: String,
    pub value: Option<Vec<u8>>,
}

impl TupleFdbStorage {
    /// Track storage slot changes
    pub async fn track_storage_change(
        &self,
        key: ChangeTrackingKey,
        changes: Vec<StorageSlotChange>,
        block_hash: String,
        block_timestamp: u64,
        tx_hash: String,
    ) -> Result<()> {
        let total_gas = changes.iter()
            .filter_map(|c| c.gas_used)
            .sum();
        
        let value = ChangeTrackingValue {
            total_slots_changed: changes.len() as u32,
            changes,
            block_hash,
            block_timestamp,
            tx_hash: tx_hash.clone(),
            tx_index: key.tx_index.unwrap_or(0),
            total_gas_used: total_gas,
            indexed_at: chrono::Utc::now().timestamp() as u64,
        };
        
        let tuple_key = key.to_tuple_key();
        let serialized = rmp_serde::to_vec(&value)
            .map_err(|e| Error::Serialization(format!("Failed to serialize changes: {}", e)))?;
        
        self.set(tuple_key, serialized).await?;
        
        // Update change summary
        self.update_change_summary(&key, &value).await?;
        
        info!(
            "Tracked {} storage changes for contract {} at block {}",
            value.total_slots_changed, key.contract_address, key.block_height
        );
        
        Ok(())
    }
    
    /// Query storage changes
    pub async fn query_changes(
        &self,
        query: ChangeQuery,
    ) -> Result<Vec<(ChangeTrackingKey, ChangeTrackingValue)>> {
        let mut results = Vec::new();
        
        // Build prefix based on query
        let prefix = if let Some(from_block) = query.from_block {
            ChangeTrackingKey::block_range_prefix(
                &query.chain_id,
                &query.contract_address,
                from_block,
            )
        } else {
            ChangeTrackingKey::contract_prefix(&query.chain_id, &query.contract_address)
        };
        
        let kvs = self.range(prefix, query.limit).await?;
        
        for (key_bytes, value_bytes) in kvs {
            // Parse key (simplified - would need proper tuple parsing)
            let block_height = 0u64; // Placeholder
            let tx_index = None;
            
            let key = ChangeTrackingKey {
                chain_id: query.chain_id.clone(),
                contract_address: query.contract_address.clone(),
                block_height,
                tx_index,
            };
            
            let value: ChangeTrackingValue = rmp_serde::from_slice(&value_bytes)
                .map_err(|e| Error::Deserialization(format!("Failed to parse changes: {}", e)))?;
            
            // Apply filters
            if let Some(to_block) = query.to_block {
                if key.block_height > to_block {
                    continue;
                }
            }
            
            if let Some(ref slot_ids) = query.slot_ids {
                let mut filtered_changes = Vec::new();
                for change in value.changes {
                    if slot_ids.contains(&change.slot_id) {
                        filtered_changes.push(change);
                    }
                }
                
                if filtered_changes.is_empty() {
                    continue;
                }
                
                // Create filtered value
                let filtered_value = ChangeTrackingValue {
                    changes: filtered_changes,
                    ..value
                };
                
                results.push((key, filtered_value));
            } else {
                results.push((key, value));
            }
        }
        
        Ok(results)
    }
    
    /// Compute state diff between two blocks
    pub async fn compute_state_diff(
        &self,
        chain_id: &str,
        contract_address: &str,
        from_block: u64,
        to_block: u64,
    ) -> Result<StateDiff> {
        // Query all changes in range
        let query = ChangeQuery {
            chain_id: chain_id.to_string(),
            contract_address: contract_address.to_string(),
            from_block: Some(from_block + 1), // Exclude from_block itself
            to_block: Some(to_block),
            include_values: true,
            ..Default::default()
        };
        
        let changes = self.query_changes(query).await?;
        
        // Build slot diff map
        let mut slot_diffs: HashMap<String, SlotDiff> = HashMap::new();
        let mut slots_added = 0u32;
        let mut slots_modified = 0u32;
        let mut slots_deleted = 0u32;
        let mut total_gas = 0u64;
        
        for (_key, value) in changes {
            total_gas += value.total_gas_used;
            
            for change in value.changes {
                let diff = slot_diffs.entry(change.slot_id.clone())
                    .or_insert_with(|| SlotDiff {
                        from_value: change.previous_value.clone(),
                        to_value: change.new_value.clone(),
                        intermediate_changes: Vec::new(),
                    });
                
                // Update to_value to latest
                diff.to_value = change.new_value.clone();
                
                // Add intermediate change
                diff.intermediate_changes.push(IntermediateChange {
                    block_height: value.block_timestamp,
                    tx_hash: value.tx_hash.clone(),
                    value: change.new_value.clone(),
                });
                
                // Count change types
                match change.change_type {
                    ChangeType::Created => slots_added += 1,
                    ChangeType::Updated => slots_modified += 1,
                    ChangeType::Deleted => slots_deleted += 1,
                    ChangeType::NoOp => {}
                }
            }
        }
        
        Ok(StateDiff {
            chain_id: chain_id.to_string(),
            contract_address: contract_address.to_string(),
            from_block,
            to_block,
            changes: slot_diffs,
            slots_added,
            slots_modified,
            slots_deleted,
            total_gas_used: total_gas,
        })
    }
    
    /// Get change summary for a contract
    pub async fn get_change_summary(
        &self,
        chain_id: &str,
        contract_address: &str,
    ) -> Result<Option<ContractChangeSummary>> {
        let summary_key = TupleKey::new(TupleKeyspace::ContractState)
            .string("change_summary")
            .string(chain_id)
            .string(contract_address);
        
        if let Some(data) = self.get(summary_key).await? {
            let summary: ContractChangeSummary = rmp_serde::from_slice(&data)
                .map_err(|e| Error::Deserialization(format!("Failed to parse summary: {}", e)))?;
            Ok(Some(summary))
        } else {
            Ok(None)
        }
    }
    
    /// Update change summary (internal)
    async fn update_change_summary(
        &self,
        key: &ChangeTrackingKey,
        value: &ChangeTrackingValue,
    ) -> Result<()> {
        let summary_key = TupleKey::new(TupleKeyspace::ContractState)
            .string("change_summary")
            .string(&key.chain_id)
            .string(&key.contract_address);
        
        let mut summary = if let Some(data) = self.get(summary_key.clone()).await? {
            rmp_serde::from_slice::<ContractChangeSummary>(&data)
                .map_err(|e| Error::Deserialization(format!("Failed to parse summary: {}", e)))?
        } else {
            ContractChangeSummary {
                chain_id: key.chain_id.clone(),
                contract_address: key.contract_address.clone(),
                total_changes: 0,
                unique_slots_changed: 0,
                first_change_block: key.block_height,
                last_change_block: key.block_height,
                changes_per_block: HashMap::new(),
                hot_slots: Vec::new(),
                total_gas_spent: 0,
                average_gas_per_change: 0,
            }
        };
        
        // Update summary
        summary.total_changes += value.total_slots_changed as u64;
        summary.last_change_block = key.block_height.max(summary.last_change_block);
        summary.first_change_block = key.block_height.min(summary.first_change_block);
        summary.total_gas_spent += value.total_gas_used;
        
        *summary.changes_per_block.entry(key.block_height).or_insert(0) += value.total_slots_changed;
        
        // Recalculate average
        if summary.total_changes > 0 {
            summary.average_gas_per_change = summary.total_gas_spent / summary.total_changes;
        }
        
        // Store updated summary
        let serialized = rmp_serde::to_vec(&summary)
            .map_err(|e| Error::Serialization(format!("Failed to serialize summary: {}", e)))?;
        
        self.set(summary_key, serialized).await?;
        
        Ok(())
    }
    
    /// Find hot slots (most frequently changed)
    pub async fn find_hot_slots(
        &self,
        chain_id: &str,
        contract_address: &str,
        limit: usize,
    ) -> Result<Vec<(String, u64)>> {
        // Query all changes
        let query = ChangeQuery {
            chain_id: chain_id.to_string(),
            contract_address: contract_address.to_string(),
            include_values: false,
            ..Default::default()
        };
        
        let changes = self.query_changes(query).await?;
        
        // Count changes per slot
        let mut slot_counts: HashMap<String, u64> = HashMap::new();
        
        for (_key, value) in changes {
            for change in value.changes {
                *slot_counts.entry(change.slot_id).or_insert(0) += 1;
            }
        }
        
        // Sort by count and take top N
        let mut hot_slots: Vec<(String, u64)> = slot_counts.into_iter().collect();
        hot_slots.sort_by(|a, b| b.1.cmp(&a.1));
        hot_slots.truncate(limit);
        
        Ok(hot_slots)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_change_tracking_key() {
        let key = ChangeTrackingKey {
            chain_id: "ethereum".to_string(),
            contract_address: "0x123".to_string(),
            block_height: 1000,
            tx_index: Some(5),
        };
        
        let tuple_key = key.to_tuple_key();
        assert_eq!(tuple_key.keyspace(), Some(TupleKeyspace::ContractState));
    }
    
    #[test]
    fn test_change_type_tracking() {
        let change = StorageSlotChange {
            slot_id: "0x0".to_string(),
            previous_value: None,
            new_value: Some(vec![1, 2, 3]),
            change_type: ChangeType::Created,
            gas_used: Some(21000),
            event_logs: vec![],
        };
        
        assert_eq!(change.change_type, ChangeType::Created);
        assert!(change.previous_value.is_none());
        assert!(change.new_value.is_some());
    }
}