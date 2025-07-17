/// Contract state storage with tuple-based keys and multi-chain support
use std::collections::HashMap;
use serde::{Deserialize, Serialize};

#[cfg(all(feature = "fdb", not(use_fdb_stub)))]
use foundationdb::{Transaction, TransactError};
#[cfg(any(not(feature = "fdb"), use_fdb_stub))]
use super::fdb_stub::{Transaction, TransactError};

use crate::tuple_fdb::{TupleKey, TupleKeyspace};
use crate::tuple_fdb::semantic::{SemanticMetadataKey, SemanticMetadataValue, TrustScope};
use crate::tuple_fdb::layout::{ContractLayoutKey, ContractLayoutValue};
use almanac_infrastructure_core::{Error, Result};

/// Contract state key for tuple-based storage
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ContractStateKey {
    /// Target chain identifier
    pub chain_id: String,
    /// Contract address (normalized across chains)
    pub contract_address: String,
    /// Storage slot identifier (chain-specific format)
    pub slot_id: StorageSlotId,
    /// Block height for state versioning
    pub block_height: u64,
}

/// Storage slot identifier that handles different chain formats
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StorageSlotId {
    /// Ethereum-style storage slot (32-byte hex)
    Ethereum(String),
    /// Cosmos/SDK key-value path
    Cosmos(String),
    /// Solana account data offset
    Solana { account: String, offset: u64, length: u64 },
    /// Generic key-value for other chains
    Generic(String),
}

impl StorageSlotId {
    /// Get the slot identifier as a string for tuple encoding
    pub fn as_string(&self) -> String {
        match self {
            StorageSlotId::Ethereum(slot) => format!("eth:{}", slot),
            StorageSlotId::Cosmos(path) => format!("cosmos:{}", path),
            StorageSlotId::Solana { account, offset, length } => {
                format!("solana:{}:{}:{}", account, offset, length)
            }
            StorageSlotId::Generic(key) => format!("generic:{}", key),
        }
    }
    
    /// Parse from tuple string
    pub fn from_string(s: &str) -> Result<Self> {
        let parts: Vec<&str> = s.splitn(2, ':').collect();
        if parts.len() != 2 {
            return Err(Error::InvalidInput(format!("Invalid slot ID format: {}", s)));
        }
        
        match parts[0] {
            "eth" => Ok(StorageSlotId::Ethereum(parts[1].to_string())),
            "cosmos" => Ok(StorageSlotId::Cosmos(parts[1].to_string())),
            "solana" => {
                let solana_parts: Vec<&str> = parts[1].splitn(3, ':').collect();
                if solana_parts.len() != 3 {
                    return Err(Error::InvalidInput("Invalid Solana slot format".to_string()));
                }
                Ok(StorageSlotId::Solana {
                    account: solana_parts[0].to_string(),
                    offset: solana_parts[1].parse()
                        .map_err(|_| Error::InvalidInput("Invalid Solana offset".to_string()))?,
                    length: solana_parts[2].parse()
                        .map_err(|_| Error::InvalidInput("Invalid Solana length".to_string()))?,
                })
            }
            "generic" => Ok(StorageSlotId::Generic(parts[1].to_string())),
            _ => Err(Error::InvalidInput(format!("Unknown slot type: {}", parts[0]))),
        }
    }
}

impl ContractStateKey {
    /// Create tuple key for FoundationDB storage
    pub fn to_tuple_key(&self) -> TupleKey {
        TupleKey::new(TupleKeyspace::ContractState)
            .append_string(&self.chain_id)
            .append_string(&self.contract_address)
            .append_string(&self.slot_id.as_string())
            .append_int(self.block_height as i64)
    }
    
    /// Parse from tuple key components
    pub fn from_tuple_components(
        chain_id: String,
        contract_address: String,
        slot_string: String,
        block_height: i64,
    ) -> Result<Self> {
        Ok(Self {
            chain_id,
            contract_address,
            slot_id: StorageSlotId::from_string(&slot_string)?,
            block_height: block_height as u64,
        })
    }
    
    /// Create a prefix key for range queries (without block height)
    pub fn prefix_key(&self) -> TupleKey {
        TupleKey::new(TupleKeyspace::ContractState)
            .string(&self.chain_id)
            .string(&self.contract_address)
            .string(&self.slot_id.as_string())
    }
    
    /// Create contract prefix for all slots
    pub fn contract_prefix(chain_id: &str, contract_address: &str) -> TupleKey {
        TupleKey::new(TupleKeyspace::ContractState)
            .string(chain_id)
            .string(contract_address)
    }
}

/// Contract state value with comprehensive metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractStateValue {
    /// Raw storage value (chain-specific encoding)
    pub raw_value: Vec<u8>,
    /// Decoded value (if layout available)
    pub decoded_value: Option<DecodedValue>,
    /// State metadata
    pub metadata: StateMetadata,
    /// Merkle proof data for verification
    pub proof_data: Option<StateProofData>,
}

/// Decoded contract state value using layout information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DecodedValue {
    /// Simple types
    UInt(String),          // Big integer as string
    Int(String),           // Signed big integer as string
    Bool(bool),
    Address(String),
    Bytes(Vec<u8>),
    String(String),
    
    /// Complex types
    Array(Vec<DecodedValue>),
    Struct(HashMap<String, DecodedValue>),
    Mapping(HashMap<String, DecodedValue>),
    
    /// Chain-specific types
    CosmosAny {
        type_url: String,
        value: Vec<u8>,
    },
    SolanaAccount {
        lamports: u64,
        data: Vec<u8>,
        owner: String,
        executable: bool,
    },
}

impl DecodedValue {
    /// Get type name for semantic metadata
    pub fn type_name(&self) -> &'static str {
        match self {
            DecodedValue::UInt(_) => "uint",
            DecodedValue::Int(_) => "int", 
            DecodedValue::Bool(_) => "bool",
            DecodedValue::Address(_) => "address",
            DecodedValue::Bytes(_) => "bytes",
            DecodedValue::String(_) => "string",
            DecodedValue::Array(_) => "array",
            DecodedValue::Struct(_) => "struct",
            DecodedValue::Mapping(_) => "mapping",
            DecodedValue::CosmosAny { .. } => "cosmos_any",
            DecodedValue::SolanaAccount { .. } => "solana_account",
        }
    }
    
    /// Check if value represents zero/empty state
    pub fn is_zero(&self) -> bool {
        match self {
            DecodedValue::UInt(v) => v == "0",
            DecodedValue::Int(v) => v == "0",
            DecodedValue::Bool(v) => !v,
            DecodedValue::Address(v) => v == "0x0000000000000000000000000000000000000000" || v.is_empty(),
            DecodedValue::Bytes(v) => v.is_empty() || v.iter().all(|&b| b == 0),
            DecodedValue::String(v) => v.is_empty(),
            DecodedValue::Array(v) => v.is_empty(),
            DecodedValue::Struct(v) => v.is_empty(),
            DecodedValue::Mapping(v) => v.is_empty(),
            DecodedValue::CosmosAny { value, .. } => value.is_empty(),
            DecodedValue::SolanaAccount { lamports, data, .. } => *lamports == 0 && data.is_empty(),
        }
    }
}

/// State metadata for contract storage entries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateMetadata {
    /// Block information
    pub block_height: u64,
    pub block_hash: String,
    pub block_timestamp: u64,
    
    /// Transaction context
    pub tx_hash: String,
    pub tx_index: u32,
    pub log_index: Option<u32>,
    
    /// Chain-specific metadata
    pub chain_metadata: ChainStateMetadata,
    
    /// State change type
    pub change_type: StateChangeType,
    
    /// Gas/fee information
    pub gas_used: Option<u64>,
    
    /// State versioning
    pub version: u32,
    
    /// Indexing metadata
    pub indexed_at: u64,
    pub indexer_version: String,
}

/// Chain-specific state metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChainStateMetadata {
    Ethereum {
        storage_trie_root: String,
        account_nonce: u64,
        account_balance: String,
        code_hash: String,
    },
    Cosmos {
        app_hash: String,
        validator_set_hash: String,
        consensus_params_hash: String,
        store_hash: String,
    },
    Solana {
        bank_hash: String,
        previous_blockhash: String,
        slot: u64,
        leader: String,
    },
}

/// Type of state change
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StateChangeType {
    /// New storage slot created
    Created,
    /// Existing slot updated
    Updated,
    /// Slot cleared/deleted
    Deleted,
    /// No change (for reference)
    NoChange,
}

/// State proof data for verification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateProofData {
    /// Merkle proof bytes
    pub proof: Vec<u8>,
    /// Root hash for verification
    pub root_hash: String,
    /// Proof type
    pub proof_type: ProofType,
    /// Additional verification data
    pub verification_data: Option<Vec<u8>>,
}

/// Type of cryptographic proof
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProofType {
    /// Ethereum Merkle Patricia Trie proof
    EthereumMPT,
    /// Cosmos IAVL tree proof
    CosmosIAVL,
    /// Solana account proof
    SolanaAccount,
    /// Generic Merkle proof
    GenericMerkle,
}

/// Contract state summary for efficient queries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractStateSummary {
    /// Contract identifier
    pub chain_id: String,
    pub contract_address: String,
    
    /// State statistics
    pub total_slots: u64,
    pub non_zero_slots: u64,
    pub last_updated_height: u64,
    pub last_updated_timestamp: u64,
    
    /// Storage size metrics
    pub total_storage_bytes: u64,
    pub compressed_size_bytes: u64,
    
    /// Change tracking
    pub total_changes: u64,
    pub changes_in_last_1000_blocks: u64,
    
    /// Layout information
    pub has_layout_commitment: bool,
    pub layout_hash: Option<String>,
    
    /// Trust and verification
    pub has_semantic_metadata: bool,
    pub verification_level: VerificationLevel,
}

/// Level of verification available for contract state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VerificationLevel {
    /// No verification data available
    None,
    /// Basic block inclusion proof
    Block,
    /// State root proof available
    StateRoot,
    /// Full merkle proof with layout verification
    Full,
}

/// Query parameters for contract state
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ContractStateQuery {
    /// Target contract
    pub chain_id: String,
    pub contract_address: String,
    
    /// Slot filters
    pub slot_ids: Option<Vec<StorageSlotId>>,
    pub slot_prefix: Option<String>,
    
    /// Block range
    pub from_height: Option<u64>,
    pub to_height: Option<u64>,
    
    /// Value filters
    pub include_zero_values: bool,
    pub only_changed_values: bool,
    
    /// Response options
    pub include_proofs: bool,
    pub include_decoded: bool,
    pub max_results: Option<usize>,
}

/// State change event for real-time updates
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateChangeEvent {
    /// State key
    pub key: ContractStateKey,
    /// New value
    pub value: ContractStateValue,
    /// Previous value (if available)
    pub previous_value: Option<ContractStateValue>,
    /// Change metadata
    pub change_metadata: StateChangeMetadata,
}

/// Metadata about the state change
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateChangeMetadata {
    /// Change detection method
    pub detection_method: ChangeDetectionMethod,
    /// Confidence level
    pub confidence: f64,
    /// Verification status
    pub verified: bool,
    /// Related changes in same transaction
    pub related_changes: Vec<ContractStateKey>,
}

/// How the state change was detected
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChangeDetectionMethod {
    /// Direct transaction log analysis
    TransactionLogs,
    /// State diff comparison
    StateDiff,
    /// Event emission
    EventEmission,
    /// Manual verification
    Manual,
}

impl ContractStateValue {
    /// Create new contract state value
    pub fn new(
        raw_value: Vec<u8>,
        metadata: StateMetadata,
    ) -> Self {
        Self {
            raw_value,
            decoded_value: None,
            metadata,
            proof_data: None,
        }
    }
    
    /// Add decoded value with layout
    pub fn with_decoded(mut self, decoded: DecodedValue) -> Self {
        self.decoded_value = Some(decoded);
        self
    }
    
    /// Add proof data
    pub fn with_proof(mut self, proof: StateProofData) -> Self {
        self.proof_data = Some(proof);
        self
    }
    
    /// Check if value represents zero state
    pub fn is_zero_value(&self) -> bool {
        // Check raw value first
        if self.raw_value.is_empty() || self.raw_value.iter().all(|&b| b == 0) {
            return true;
        }
        
        // Check decoded value if available
        if let Some(decoded) = &self.decoded_value {
            return decoded.is_zero();
        }
        
        false
    }
    
    /// Get storage size in bytes
    pub fn storage_size(&self) -> usize {
        self.raw_value.len() + 
        self.decoded_value.as_ref().map(|d| {
            // Rough estimate of decoded value size
            serde_json::to_vec(d).map(|v| v.len()).unwrap_or(0)
        }).unwrap_or(0) +
        self.proof_data.as_ref().map(|p| p.proof.len()).unwrap_or(0)
    }
}

impl StateMetadata {
    /// Create new state metadata
    pub fn new(
        block_height: u64,
        block_hash: String,
        block_timestamp: u64,
        tx_hash: String,
        tx_index: u32,
        chain_metadata: ChainStateMetadata,
        change_type: StateChangeType,
    ) -> Self {
        Self {
            block_height,
            block_hash,
            block_timestamp,
            tx_hash,
            tx_index,
            log_index: None,
            chain_metadata,
            change_type,
            gas_used: None,
            version: 1,
            indexed_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            indexer_version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }
    
    /// Check if metadata is for a recent block
    pub fn is_recent(&self, current_height: u64, threshold: u64) -> bool {
        current_height.saturating_sub(self.block_height) <= threshold
    }
}

/// Contract state operations for tuple FDB storage
impl super::TupleFdbStorage {
    /// Store contract state with semantic metadata integration
    pub async fn store_contract_state(
        &self,
        key: ContractStateKey,
        value: ContractStateValue,
        trust_scope: TrustScope,
    ) -> Result<()> {
        let tuple_key = key.to_tuple_key();
        let serialized_value = rmp_serde::to_vec(&value)
            .map_err(|e| Error::Serialization(e.to_string()))?;
        
        // Store the main state value
        self.set(tuple_key, serialized_value).await?;
        
        // Store semantic metadata if decoded value available
        if let Some(decoded) = &value.decoded_value {
            let semantic_key = SemanticMetadataKey {
                chain_id: key.chain_id.clone(),
                contract_address: key.contract_address.clone(),
                storage_slot: key.slot_id.as_string(),
                field_name: "value".to_string(), // Could be more specific with layout
                trust_scope: trust_scope.clone(),
            };
            
            let semantic_value = SemanticMetadataValue {
                data_type: decoded.type_name().to_string(),
                semantic_tags: vec!["contract_state".to_string()],
                zero_semantics: if decoded.is_zero() {
                    crate::tuple_fdb::semantic::ZeroSemantics::ValidZero
                } else {
                    crate::tuple_fdb::semantic::ZeroSemantics::NeverWritten
                },
                confidence_level: crate::tuple_fdb::semantic::ConfidenceLevel::High,
                source: crate::tuple_fdb::semantic::SemanticSource::DirectObservation,
                metadata: std::collections::HashMap::new(),
                created_at: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
                updated_at: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
            };
            
            let semantic_tuple_key = semantic_key.to_tuple_key();
            let semantic_serialized = rmp_serde::to_vec(&semantic_value)
                .map_err(|e| Error::Serialization(e.to_string()))?;
            
            self.set(semantic_tuple_key, semantic_serialized).await?;
        }
        
        // Update contract summary
        self.update_contract_summary_atomic(&key, &value).await?;
        
        Ok(())
    }
    
    /// Get contract state for a specific slot and block
    pub async fn get_contract_state(
        &self,
        key: &ContractStateKey,
    ) -> Result<Option<ContractStateValue>> {
        let tuple_key = key.to_tuple_key();
        
        if let Some(value_bytes) = self.get(tuple_key).await? {
            let value: ContractStateValue = rmp_serde::from_slice(&value_bytes)
                .map_err(|e| Error::Deserialization(e.to_string()))?;
            Ok(Some(value))
        } else {
            Ok(None)
        }
    }
    
    /// Get latest contract state for a slot
    pub async fn get_latest_contract_state(
        &self,
        chain_id: &str,
        contract_address: &str,
        slot_id: &StorageSlotId,
    ) -> Result<Option<ContractStateValue>> {
        // Create prefix for this slot
        let prefix_key = TupleKey::new(TupleKeyspace::ContractState)
            .append_string(chain_id)
            .append_string(contract_address)
            .append_string(&slot_id.as_string());
        
        // Get range with limit 1 in reverse order
        let results = self.range_reverse(prefix_key, Some(1)).await?;
        
        if let Some((_, value_bytes)) = results.into_iter().next() {
            let value: ContractStateValue = rmp_serde::from_slice(&value_bytes)
                .map_err(|e| Error::Deserialization(e.to_string()))?;
            Ok(Some(value))
        } else {
            Ok(None)
        }
    }
    
    /// Query contract state with filters
    pub async fn query_contract_state(
        &self,
        query: ContractStateQuery,
    ) -> Result<Vec<(ContractStateKey, ContractStateValue)>> {
        let mut results = Vec::new();
        
        // Build key prefix
        let base_prefix = ContractStateKey::contract_prefix(&query.chain_id, &query.contract_address);
        
        let kvs = self.range(base_prefix, query.max_results).await?;
        
        for (key_bytes, value_bytes) in kvs {
            // Parse key from tuple components
            let key = self.parse_contract_state_key_from_bytes(&key_bytes)?;
            
            // Apply filters
            if let Some(ref slot_ids) = query.slot_ids {
                if !slot_ids.contains(&key.slot_id) {
                    continue;
                }
            }
            
            if let Some(ref slot_prefix) = query.slot_prefix {
                if !key.slot_id.as_string().starts_with(slot_prefix) {
                    continue;
                }
            }
            
            if let Some(from_height) = query.from_height {
                if key.block_height < from_height {
                    continue;
                }
            }
            
            if let Some(to_height) = query.to_height {
                if key.block_height > to_height {
                    continue;
                }
            }
            
            // Parse value
            let value: ContractStateValue = rmp_serde::from_slice(&value_bytes)
                .map_err(|e| Error::Deserialization(e.to_string()))?;
            
            // Apply value filters
            if !query.include_zero_values && value.is_zero_value() {
                continue;
            }
            
            results.push((key, value));
        }
        
        Ok(results)
    }
    
    /// Get contract state summary
    pub async fn get_contract_summary(
        &self,
        chain_id: &str,
        contract_address: &str,
    ) -> Result<Option<ContractStateSummary>> {
        let summary_key = TupleKey::new(TupleKeyspace::ContractState)
            .append_string("summary")
            .append_string(chain_id)
            .append_string(contract_address);
        
        if let Some(summary_bytes) = self.get(summary_key).await? {
            let summary: ContractStateSummary = rmp_serde::from_slice(&summary_bytes)
                .map_err(|e| Error::Deserialization(e.to_string()))?;
            Ok(Some(summary))
        } else {
            Ok(None)
        }
    }
    
    /// Update contract summary atomically
    async fn update_contract_summary_atomic(
        &self,
        key: &ContractStateKey,
        value: &ContractStateValue,
    ) -> Result<()> {
        let summary_key = TupleKey::new(TupleKeyspace::ContractState)
            .append_string("summary")
            .append_string(&key.chain_id)
            .append_string(&key.contract_address);
        
        // Get existing summary or create new one
        let mut summary = if let Some(summary_bytes) = self.get(summary_key.clone()).await? {
            rmp_serde::from_slice::<ContractStateSummary>(&summary_bytes)
                .map_err(|e| Error::Deserialization(e.to_string()))?
        } else {
            ContractStateSummary {
                chain_id: key.chain_id.clone(),
                contract_address: key.contract_address.clone(),
                total_slots: 0,
                non_zero_slots: 0,
                last_updated_height: 0,
                last_updated_timestamp: 0,
                total_storage_bytes: 0,
                compressed_size_bytes: 0,
                total_changes: 0,
                changes_in_last_1000_blocks: 0,
                has_layout_commitment: false,
                layout_hash: None,
                has_semantic_metadata: false,
                verification_level: VerificationLevel::None,
            }
        };
        
        // Update summary stats
        summary.total_slots += 1;
        if !value.is_zero_value() {
            summary.non_zero_slots += 1;
        }
        
        if key.block_height > summary.last_updated_height {
            summary.last_updated_height = key.block_height;
            summary.last_updated_timestamp = value.metadata.block_timestamp;
        }
        
        summary.total_storage_bytes += value.storage_size() as u64;
        summary.total_changes += 1;
        
        if key.block_height > summary.last_updated_height.saturating_sub(1000) {
            summary.changes_in_last_1000_blocks += 1;
        }
        
        // Check for layout commitment
        let layout_key = ContractLayoutKey {
            chain_id: key.chain_id.clone(),
            contract_address: key.contract_address.clone(),
            version: None,
        };
        let layout_tuple_key = layout_key.to_tuple_key();
        
        if let Some(_) = self.get(layout_tuple_key).await? {
            summary.has_layout_commitment = true;
        }
        
        // Update verification level
        summary.verification_level = if value.proof_data.is_some() {
            VerificationLevel::Full
        } else if summary.has_layout_commitment {
            VerificationLevel::StateRoot
        } else {
            VerificationLevel::Block
        };
        
        // Store updated summary
        let summary_bytes = rmp_serde::to_vec(&summary)
            .map_err(|e| Error::Serialization(e.to_string()))?;
        
        self.set(summary_key, summary_bytes).await?;
        
        Ok(())
    }
    
    /// Parse contract state key from raw bytes
    fn parse_contract_state_key_from_bytes(&self, key_bytes: &[u8]) -> Result<ContractStateKey> {
        // This is a simplified parsing - in production would use proper tuple decoding
        // For now, we'll return a placeholder
        Err(Error::NotImplemented("Key parsing from bytes not yet implemented".to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_storage_slot_id_conversion() {
        let ethereum_slot = StorageSlotId::Ethereum("0x1234".to_string());
        let string_repr = ethereum_slot.as_string();
        let parsed = StorageSlotId::from_string(&string_repr).unwrap();
        assert_eq!(ethereum_slot, parsed);
        
        let solana_slot = StorageSlotId::Solana {
            account: "account123".to_string(),
            offset: 32,
            length: 8,
        };
        let string_repr = solana_slot.as_string();
        let parsed = StorageSlotId::from_string(&string_repr).unwrap();
        assert_eq!(solana_slot, parsed);
    }
    
    #[test]
    fn test_decoded_value_zero_detection() {
        assert!(DecodedValue::UInt("0".to_string()).is_zero());
        assert!(!DecodedValue::UInt("1".to_string()).is_zero());
        assert!(DecodedValue::Bool(false).is_zero());
        assert!(!DecodedValue::Bool(true).is_zero());
        assert!(DecodedValue::Array(vec![]).is_zero());
        assert!(!DecodedValue::Array(vec![DecodedValue::UInt("1".to_string())]).is_zero());
    }
    
    #[test]
    fn test_contract_state_key_tuple_conversion() {
        let key = ContractStateKey {
            chain_id: "ethereum".to_string(),
            contract_address: "0x1234".to_string(),
            slot_id: StorageSlotId::Ethereum("0x0".to_string()),
            block_height: 1000,
        };
        
        let tuple_key = key.to_tuple_key();
        assert_eq!(tuple_key.keyspace(), Some(TupleKeyspace::ContractState));
        assert_eq!(tuple_key.components().len(), 4); // chain + address + slot + height
    }
} 