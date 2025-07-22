//! FoundationDB tuple layer implementation for multi-chain storage
//!
//! This module implements the comprehensive tuple-based storage schema
//! for blockchain state data with semantic metadata support.

#![allow(dead_code, unused_variables)]

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

// Use conditional compilation for FDB imports
#[cfg(feature = "fdb")]
use foundationdb::options::StreamingMode;
#[cfg(feature = "fdb")]
use foundationdb::tuple::{pack, unpack, Bytes, Element, Subspace};
#[cfg(feature = "fdb")]
use foundationdb::{Database, RangeOption, Transaction};

/// Enhanced keyspace definitions for all data types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TupleKeyspace {
    // Core blockchain data
    ContractState = 1, // Contract storage slots
    BlockMeta = 2,     // Block headers and consensus data
    StorageProofs = 3, // Merkle proofs for verification

    // Semantic layer
    SemanticMeta = 4,    // Trust-scoped semantic interpretations
    TrustGraph = 5,      // Developer trust relationships
    ContractLayouts = 6, // Layout commitments from Traverse

    // View layer
    ViewRegistry = 7, // Custom view definitions

    // Lightwave coordination
    LightwaveState = 8,     // Recursive proof coordination
    TrustedCheckpoints = 9, // Network checkpoints

    // Chain-specific
    SolanaAccounts = 10, // Solana account data

    // Network configuration
    NetworkConfig = 11, // Blockchain network configuration
    CircuitKeys = 12,   // SP1 verification keys

    // System
    ChangeTracking = 13,      // State change logs
    ValidatorSignatures = 14, // Proposer validator signatures
    TrustPolicies = 15,       // Developer trust policies
}

/// Trust-scoped semantic metadata key
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SemanticMetadataKey {
    pub chain_id: String,
    pub contract_address: String,
    #[serde(with = "serde_bytes")]
    pub storage_key: [u8; 32],
    pub developer_id: String,
    pub project_id: Option<String>,
}

/// Semantic metadata value with trust context
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SemanticMetadataValue {
    pub zero_semantics: ZeroSemantics,
    pub semantic_source: SemanticSource,
    pub field_index: u16,
    #[serde(with = "serde_bytes")]
    pub expected_slot: [u8; 32],
    pub confidence_level: ConfidenceLevel,
    pub timestamp: u64,
    pub documentation_url: Option<String>,
    #[serde(with = "serde_bytes")]
    pub signature: Option<[u8; 64]>,
}

/// What zero means in storage
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ZeroSemantics {
    NeverWritten = 0,
    ExplicitlyZero = 1,
    Cleared = 2,
    ValidZero = 3,
}

/// How semantic information was derived
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SemanticSource {
    DeclaredViaStructuredData = 0,
    InferredFromABI = 1,
    CommunityVerified = 2,
}

/// Developer confidence in semantic
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ConfidenceLevel {
    Low = 0,
    Medium = 1,
    High = 2,
    Verified = 3,
}

/// Block data with cross-chain support
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlockDataKey {
    pub chain_id: String,
    pub block_identifier: BlockIdentifier,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum BlockIdentifier {
    Height(u64),
    Hash([u8; 32]),
    Slot(u64), // Solana
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlockDataValue {
    pub block_height: u64,
    pub block_hash: [u8; 32],
    pub state_root: [u8; 32],
    pub parent_hash: [u8; 32],
    pub timestamp: u64,
    pub gas_limit: Option<u64>,
    pub gas_used: Option<u64>,
    pub validator_set_hash: Option<[u8; 32]>,
    pub app_hash: Option<[u8; 32]>,
}

/// Contract layout for Traverse integration
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContractLayoutKey {
    pub chain_id: String,
    pub contract_address: String,
    pub version: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContractLayoutValue {
    pub layout_commitment: [u8; 32],
    pub layout_data: Vec<u8>,
    pub compiler_version: String,
    pub source_hash: Option<[u8; 32]>,
    pub verification_status: VerificationStatus,
    pub timestamp: u64,
    pub provided_by: String, // Developer trust scope
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VerificationStatus {
    Unverified = 0,
    Pending = 1,
    Verified = 2,
    Failed = 3,
}

/// Developer trust registry
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeveloperTrustKey {
    pub trustor: String,
    pub trustee: String,
    pub scope: TrustScope,
}

// Use trust types from api_adapter module
pub use crate::api_adapter::{DeveloperTrustValue, TrustLevel, TrustPolicy, TrustScope};

// Helper function for prefix-based range scanning with foundationdb 0.8 API
#[cfg(feature = "fdb")]
async fn scan_with_prefix(
    tx: &Transaction,
    prefix: &[u8],
    limit: usize,
) -> Result<Vec<(Vec<u8>, Vec<u8>)>> {
    let subspace = Subspace::from_bytes(prefix);
    let range = subspace.range();

    let range_opt = RangeOption {
        limit: Some(limit),
        reverse: false,
        mode: StreamingMode::WantAll,
        ..RangeOption::from((range.0.as_slice(), range.1.as_slice()))
    };

    let kvs = tx
        .get_range(&range_opt, 1, false)
        .await
        .map_err(|e| anyhow!("Range scan failed: {}", e))?;

    // Convert FdbValues to (Vec<u8>, Vec<u8>) tuples
    let mut results = Vec::new();
    for kv in kvs {
        results.push((kv.key().to_vec(), kv.value().to_vec()));
    }

    Ok(results)
}

/// Lightwave trusted checkpoint
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrustedCheckpointKey {
    pub chain_id: String,
    pub checkpoint_type: CheckpointType,
    pub height_or_slot: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CheckpointType {
    Helios = 0,
    Tendermint = 1,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrustedCheckpointValue {
    pub root_or_hash: [u8; 32],
    pub sync_committee_hash: Option<[u8; 32]>,
    pub validator_set_hash: Option<[u8; 32]>,
    pub trusted_by: Vec<String>,
    pub verification_count: u32,
    pub timestamp: u64,
}

/// Storage proof data
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StorageProofKey {
    pub chain_id: String,
    pub contract_address: String,
    #[serde(with = "serde_bytes")]
    pub storage_key: [u8; 32],
    pub block_identifier: BlockIdentifier,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StorageProofValue {
    pub value: [u8; 32],
    pub proof: Vec<Vec<u8>>,
    pub state_root: [u8; 32],
    pub verified: bool,
    pub verification_timestamp: u64,
}

/// View definition for developer queries
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ViewDefinition {
    pub view_id: String,
    pub contract_address: String,
    pub slots: Vec<SlotDefinition>,
    pub decode_types: std::collections::HashMap<String, String>,
    pub trust_policy: TrustPolicy,
    pub created_by: String,
    pub created_at: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SlotDefinition {
    pub slot: String,
    pub field: String,
    pub decode_type: String,
}

// TrustPolicy enum now imported from api_adapter

/// Solana account data
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SolanaAccountKey {
    pub account_address: [u8; 32],
    pub slot: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SolanaAccountValue {
    pub owner_program: [u8; 32],
    pub lamports: u64,
    pub rent_epoch: u64,
    pub data: Vec<u8>,
    pub block_hash: String,
    pub is_executable: bool,
    pub is_signer: bool,
    pub is_writable: bool,
}

/// Network configuration for Lightwave integration
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NetworkConfigKey {
    pub chain_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NetworkConfigValue {
    pub genesis_root: [u8; 32],
    pub forks: Vec<ForkConfig>,
    pub consensus_config: ConsensusConfig,
    pub finality_threshold: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ForkConfig {
    pub name: String,          // "altair", "bellatrix", etc.
    pub epoch: u64,            // Activation epoch
    pub fork_version: [u8; 4], // Fork identifier
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConsensusConfig {
    pub consensus_type: ConsensusType,
    pub epoch_duration: Option<u64>,     // Ethereum specific
    pub slot_duration: Option<u64>,      // Ethereum specific
    pub block_time: Option<u64>,         // Tendermint specific
    pub validator_set_size: Option<u32>, // Tendermint specific
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConsensusType {
    EthereumBeacon = 0,
    Tendermint = 1,
    SolanaPoH = 2,
}

/// Circuit verification keys for SP1 integration
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CircuitVKKey {
    pub circuit_type: String, // "helios", "tendermint", "recursive"
    pub version: String,      // Circuit version
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CircuitVKValue {
    pub verification_key: String,               // SP1 verification key
    pub circuit_hash: [u8; 32],                 // Circuit commitment
    pub trusted_setup_ceremony: Option<String>, // Ceremony identifier
    pub expires_at: Option<u64>,                // Key expiration
}

/// Validator signatures for block proposals
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidatorSignatureKey {
    pub chain_id: String,
    pub block_identifier: BlockIdentifier,
    pub validator_id: String, // Validator public key or identifier
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidatorSignatureValue {
    pub signature: Vec<u8>,                   // BLS/ECDSA/Ed25519 signature bytes
    pub signature_type: SignatureType,        // Type of signature algorithm used
    pub pubkey: Vec<u8>,                      // Validator public key
    pub block_hash: [u8; 32],                 // Hash of the block being signed
    pub timestamp: u64,                       // Signature timestamp
    pub is_proposer: bool,                    // True if this validator was the block proposer
    pub voting_power: Option<u64>,            // Validator's voting power (for PoS chains)
    pub chain_specific_data: Option<Vec<u8>>, // Additional chain-specific signature data
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SignatureType {
    /// BLS signature (Ethereum 2.0)
    BLS,
    /// ECDSA signature (Ethereum 1.0, Bitcoin)
    ECDSA,
    /// Ed25519 signature (Solana, Cosmos)
    Ed25519,
    /// Schnorr signature (some chains)
    Schnorr,
    /// Custom signature type with name
    Custom(String),
}

/// Tuple storage implementation
pub struct TupleFdbStorage {
    db: Arc<Database>,
    namespace: String,
}

impl TupleFdbStorage {
    /// Create a new tuple storage instance
    pub async fn new(db: Arc<Database>, namespace: String) -> Result<Self> {
        Ok(Self { db, namespace })
    }

    /// Build a tuple key for semantic metadata
    pub fn build_semantic_key(&self, key: &SemanticMetadataKey) -> Vec<Element<'static>> {
        let mut tuple = vec![
            Element::Bytes(Bytes::from(self.namespace.clone().into_bytes())),
            Element::Int(TupleKeyspace::SemanticMeta as i64),
            Element::String(key.chain_id.clone().into()),
            Element::String(key.contract_address.clone().into()),
            Element::Bytes(Bytes::from(key.storage_key.to_vec())),
            Element::String(key.developer_id.clone().into()),
        ];
        if let Some(ref project_id) = key.project_id {
            tuple.push(Element::String(project_id.clone().into()));
        }
        tuple
    }

    /// Build a tuple key for block data
    pub fn build_block_key(&self, key: &BlockDataKey) -> Vec<Element<'static>> {
        let mut tuple = Vec::new();
        tuple.push(Element::Bytes(Bytes::from(
            self.namespace.clone().into_bytes(),
        )));
        tuple.push(Element::Int(TupleKeyspace::BlockMeta as i64));
        tuple.push(Element::String(key.chain_id.clone().into()));

        match &key.block_identifier {
            BlockIdentifier::Height(h) => tuple.push(Element::Int(*h as i64)),
            BlockIdentifier::Hash(h) => tuple.push(Element::Bytes(Bytes::from(h.to_vec()))),
            BlockIdentifier::Slot(s) => tuple.push(Element::Int(*s as i64)),
        }

        tuple
    }

    /// Build a tuple key for contract layout
    pub fn build_layout_key(&self, key: &ContractLayoutKey) -> Vec<Element<'static>> {
        let mut tuple = vec![
            Element::Bytes(Bytes::from(self.namespace.clone().into_bytes())),
            Element::Int(TupleKeyspace::ContractLayouts as i64),
            Element::String(key.chain_id.clone().into()),
            Element::String(key.contract_address.clone().into()),
        ];
        if let Some(ref version) = key.version {
            tuple.push(Element::String(version.clone().into()));
        }
        tuple
    }

    /// Build a tuple key for trust relationships
    pub fn build_trust_key(&self, key: &DeveloperTrustKey) -> Vec<Element<'static>> {
        let mut tuple = vec![
            Element::Bytes(Bytes::from(self.namespace.clone().into_bytes())),
            Element::Int(TupleKeyspace::TrustGraph as i64),
            Element::String(key.trustor.clone().into()),
            Element::String(key.trustee.clone().into()),
        ];

        match &key.scope {
            TrustScope::Global => tuple.push(Element::String("global".to_string().into())),
            TrustScope::Contract {
                chain_id,
                contract_address,
            } => {
                tuple.push(Element::String("contract".to_string().into()));
                tuple.push(Element::String(chain_id.clone().into()));
                tuple.push(Element::String(contract_address.clone().into()));
            }
            TrustScope::Field {
                chain_id,
                contract_address,
                field,
            } => {
                tuple.push(Element::String("field".to_string().into()));
                tuple.push(Element::String(chain_id.clone().into()));
                tuple.push(Element::String(contract_address.clone().into()));
                tuple.push(Element::String(field.clone().into()));
            }
        }

        tuple
    }

    /// Build a tuple key for storage proofs
    pub fn build_proof_key(&self, key: &StorageProofKey) -> Vec<Element<'static>> {
        let mut tuple = vec![
            Element::Bytes(Bytes::from(self.namespace.clone().into_bytes())),
            Element::Int(TupleKeyspace::StorageProofs as i64),
            Element::String(key.chain_id.clone().into()),
            Element::String(key.contract_address.clone().into()),
            Element::Bytes(Bytes::from(key.storage_key.to_vec())),
        ];

        match &key.block_identifier {
            BlockIdentifier::Height(h) => tuple.push(Element::Int(*h as i64)),
            BlockIdentifier::Hash(h) => tuple.push(Element::Bytes(Bytes::from(h.to_vec()))),
            BlockIdentifier::Slot(s) => tuple.push(Element::Int(*s as i64)),
        }

        tuple
    }

    /// Build a tuple key for contract state
    pub fn build_state_key(
        &self,
        chain_id: &str,
        contract: &str,
        block: u64,
        key: &[u8; 32],
    ) -> Vec<Element<'static>> {
        vec![
            Element::Bytes(Bytes::from(self.namespace.clone().into_bytes())),
            Element::Int(TupleKeyspace::ContractState as i64),
            Element::String(chain_id.to_string().into()),
            Element::String(contract.to_string().into()),
            Element::Int(block as i64),
            Element::Bytes(Bytes::from(key.to_vec())),
        ]
    }

    /// Build a tuple key for Lightwave checkpoints
    pub fn build_checkpoint_key(&self, key: &TrustedCheckpointKey) -> Vec<Element<'static>> {
        vec![
            Element::Bytes(Bytes::from(self.namespace.clone().into_bytes())),
            Element::Int(TupleKeyspace::TrustedCheckpoints as i64),
            Element::String(key.chain_id.clone().into()),
            Element::Int(key.checkpoint_type as i64),
            Element::Int(key.height_or_slot as i64),
        ]
    }

    /// Build a tuple key for Solana accounts
    pub fn build_solana_account_key(&self, key: &SolanaAccountKey) -> Vec<Element<'static>> {
        vec![
            Element::Bytes(Bytes::from(self.namespace.clone().into_bytes())),
            Element::Int(TupleKeyspace::SolanaAccounts as i64),
            Element::Bytes(Bytes::from(key.account_address.to_vec())),
            Element::Int(key.slot as i64),
        ]
    }

    /// Build a tuple key for network configuration
    pub fn build_network_config_key(&self, key: &NetworkConfigKey) -> Vec<Element<'static>> {
        vec![
            Element::Bytes(Bytes::from(self.namespace.clone().into_bytes())),
            Element::Int(TupleKeyspace::NetworkConfig as i64),
            Element::String(key.chain_id.clone().into()),
        ]
    }

    /// Build a tuple key for circuit verification keys
    pub fn build_circuit_vk_key(&self, key: &CircuitVKKey) -> Vec<Element<'static>> {
        vec![
            Element::Bytes(Bytes::from(self.namespace.clone().into_bytes())),
            Element::Int(TupleKeyspace::CircuitKeys as i64),
            Element::String(key.circuit_type.clone().into()),
            Element::String(key.version.clone().into()),
        ]
    }

    /// Build a tuple key for validator signatures
    pub fn build_validator_signature_key(
        &self,
        key: &ValidatorSignatureKey,
    ) -> Vec<Element<'static>> {
        let mut tuple = Vec::new();
        tuple.push(Element::Bytes(Bytes::from(
            self.namespace.clone().into_bytes(),
        )));
        tuple.push(Element::Int(TupleKeyspace::ValidatorSignatures as i64));
        tuple.push(Element::String(key.chain_id.clone().into()));

        match &key.block_identifier {
            BlockIdentifier::Height(h) => {
                tuple.push(Element::String("height".to_string().into()));
                tuple.push(Element::Int(*h as i64));
            }
            BlockIdentifier::Hash(h) => {
                tuple.push(Element::String("hash".to_string().into()));
                tuple.push(Element::Bytes(Bytes::from(h.to_vec())));
            }
            BlockIdentifier::Slot(s) => {
                tuple.push(Element::String("slot".to_string().into()));
                tuple.push(Element::Int(*s as i64));
            }
        }

        tuple.push(Element::String(key.validator_id.clone().into()));
        tuple
    }

    /// Store semantic metadata with trust scope
    pub async fn store_semantic_metadata(
        &self,
        key: SemanticMetadataKey,
        value: SemanticMetadataValue,
    ) -> Result<()> {
        let tx = self.db.create_trx()?;
        let tuple_key = self.build_semantic_key(&key);
        let serialized = bincode::serialize(&value)?;

        tx.set(&pack(&tuple_key), &serialized);
        tx.commit()
            .await
            .map_err(|e| anyhow!("Transaction commit failed: {:?}", e))?;

        Ok(())
    }

    /// Get semantic metadata with trust filtering
    pub async fn get_semantic_metadata(
        &self,
        func_key: &SemanticMetadataKey,
        trust_policy: TrustPolicy,
        trusted_developers: &[String],
    ) -> Result<Option<SemanticMetadataValue>> {
        let tx = self.db.create_trx()?;

        // Build key prefix without developer_id to get all interpretations
        let mut prefix = Vec::new();
        prefix.push(Element::Bytes(Bytes::from(self.namespace.as_bytes())));
        prefix.push(Element::Int(TupleKeyspace::SemanticMeta as i64));
        prefix.push(Element::String(func_key.chain_id.clone().into()));
        prefix.push(Element::String(func_key.contract_address.clone().into()));
        prefix.push(Element::Bytes(Bytes::from(&func_key.storage_key[..])));

        let prefix_packed = pack(&prefix);
        // Implement proper range scanning for foundationdb 0.8
        let results = match scan_with_prefix(&tx, &prefix_packed, 1000).await {
            Ok(range_results) => range_results,
            Err(e) => {
                tracing::error!("Range scan failed: {}", e);
                Vec::new()
            }
        };

        // Filter based on trust policy
        let mut best_value: Option<(SemanticMetadataValue, String)> = None;

        for (key, value) in results {
            let tuple: Vec<Element> = unpack(&key)?;

            if let Some(Element::String(developer_id)) = tuple.get(5) {
                let val: SemanticMetadataValue = bincode::deserialize(&value)?;

                match trust_policy {
                    TrustPolicy::Strict => {
                        if trusted_developers
                            .iter()
                            .any(|d| d == developer_id.as_ref())
                        {
                            // Use first trusted developer found
                            return Ok(Some(val));
                        }
                    }
                    TrustPolicy::Permissive => {
                        // Prefer trusted developers but accept others
                        if trusted_developers
                            .iter()
                            .any(|d| d == developer_id.as_ref())
                        {
                            return Ok(Some(val));
                        } else if best_value.is_none() {
                            best_value = Some((val, developer_id.to_string()));
                        }
                    }
                    TrustPolicy::Explicit {
                        developer_id: ref expected_dev,
                    } => {
                        // Only accept explicitly requested developer
                        if developer_id.as_ref() == expected_dev {
                            return Ok(Some(val));
                        }
                    }
                }
            }
        }

        Ok(best_value.map(|(v, _)| v))
    }

    /// Store contract layout from Traverse
    pub async fn store_contract_layout(
        &self,
        key: ContractLayoutKey,
        value: ContractLayoutValue,
    ) -> Result<()> {
        let tx = self.db.create_trx()?;
        let tuple_key = self.build_layout_key(&key);
        let serialized = bincode::serialize(&value)?;

        tx.set(&pack(&tuple_key), &serialized);
        tx.commit()
            .await
            .map_err(|e| anyhow!("Transaction commit failed: {:?}", e))?;

        Ok(())
    }

    /// Get contract layout with trust context
    pub async fn get_contract_layout(
        &self,
        key: &ContractLayoutKey,
    ) -> Result<Option<ContractLayoutValue>> {
        let tx = self.db.create_trx()?;
        let tuple_key = self.build_layout_key(key);

        if let Some(value) = tx.get(&pack(&tuple_key), false).await? {
            let layout: ContractLayoutValue = bincode::deserialize(&value)?;
            Ok(Some(layout))
        } else {
            Ok(None)
        }
    }

    /// Store developer trust relationship
    pub async fn store_trust_relationship(
        &self,
        key: DeveloperTrustKey,
        value: DeveloperTrustValue,
    ) -> Result<()> {
        let tx = self.db.create_trx()?;
        let tuple_key = self.build_trust_key(&key);
        let serialized = bincode::serialize(&value)?;

        tx.set(&pack(&tuple_key), &serialized);
        tx.commit()
            .await
            .map_err(|e| anyhow!("Transaction commit failed: {:?}", e))?;

        Ok(())
    }

    /// Get trust relationships for a developer
    pub async fn get_trust_relationships(
        &self,
        trustor: &str,
    ) -> Result<Vec<(DeveloperTrustKey, DeveloperTrustValue)>> {
        let tx = self.db.create_trx()?;

        let mut prefix = Vec::new();
        prefix.push(Element::Bytes(Bytes::from(self.namespace.as_bytes())));
        prefix.push(Element::Int(TupleKeyspace::TrustGraph as i64));
        prefix.push(Element::String(trustor.to_string().into()));

        let prefix_packed = pack(&prefix);
        // Implement proper range scanning for foundationdb 0.8
        let results = match scan_with_prefix(&tx, &prefix_packed, 1000).await {
            Ok(range_results) => range_results,
            Err(e) => {
                tracing::error!("Range scan failed: {}", e);
                Vec::new()
            }
        };

        let mut relationships = Vec::new();

        for (key, value) in results {
            let tuple: Vec<Element> = unpack(&key)?;

            if let (Some(Element::String(trustee)), Some(Element::String(scope_type))) =
                (tuple.get(3), tuple.get(4))
            {
                let scope = match scope_type.as_ref() {
                    "global" => TrustScope::Global,
                    "contract" => {
                        if let (Some(Element::String(chain_id)), Some(Element::String(addr))) =
                            (tuple.get(5), tuple.get(6))
                        {
                            TrustScope::Contract {
                                chain_id: chain_id.to_string(),
                                contract_address: addr.to_string(),
                            }
                        } else {
                            continue;
                        }
                    }
                    "field" => {
                        if let (
                            Some(Element::String(chain_id)),
                            Some(Element::String(addr)),
                            Some(Element::String(field)),
                        ) = (tuple.get(5), tuple.get(6), tuple.get(7))
                        {
                            TrustScope::Field {
                                chain_id: chain_id.to_string(),
                                contract_address: addr.to_string(),
                                field: field.to_string(),
                            }
                        } else {
                            continue;
                        }
                    }
                    _ => continue,
                };

                let key = DeveloperTrustKey {
                    trustor: trustor.to_string(),
                    trustee: trustee.to_string(),
                    scope,
                };

                let val: DeveloperTrustValue = bincode::deserialize(&value)?;
                relationships.push((key, val));
            }
        }

        Ok(relationships)
    }

    /// Revoke trust relationship with reason
    pub async fn revoke_trust_relationship(
        &self,
        key: &DeveloperTrustKey,
        reason: &str,
    ) -> Result<()> {
        let tx = self.db.create_trx()?;
        let tuple_key = self.build_trust_key(key);

        // Get the existing relationship to modify it
        if let Some(existing_data) = tx.get(&pack(&tuple_key), false).await? {
            let mut trust_value: DeveloperTrustValue = bincode::deserialize(&existing_data)?;

            // Mark as revoked by setting level to None and adding revocation reason
            trust_value.level = crate::api_adapter::TrustLevel::None;
            trust_value.expires_at = Some(chrono::Utc::now().timestamp() as u64); // Expire immediately

            // Store the updated value
            let serialized = bincode::serialize(&trust_value)?;
            tx.set(&pack(&tuple_key), &serialized);
        } else {
            // If relationship doesn't exist, create a revoked entry
            let trust_value = DeveloperTrustValue {
                trustor: key.trustor.clone(),
                trustee: key.trustee.clone(),
                scope: key.scope.clone(),
                level: crate::api_adapter::TrustLevel::None,
                created_at: chrono::Utc::now().timestamp() as u64,
                expires_at: Some(chrono::Utc::now().timestamp() as u64), // Already expired
                conditions: vec![],
                revocation_reason: Some("Revoked".to_string()),
            };

            let serialized = bincode::serialize(&trust_value)?;
            tx.set(&pack(&tuple_key), &serialized);
        }

        tx.commit()
            .await
            .map_err(|e| anyhow!("Transaction commit failed: {:?}", e))?;

        tracing::info!(
            "Revoked trust relationship: {} -> {} (reason: {})",
            key.trustor,
            key.trustee,
            reason
        );
        Ok(())
    }

    /// Get trust policy for a developer
    pub async fn get_trust_policy(
        &self,
        developer_id: &str,
    ) -> Result<Option<crate::api_adapter::TrustPolicy>> {
        let tx = self.db.create_trx()?;

        let mut tuple = Vec::new();
        tuple.push(Element::Bytes(Bytes::from(self.namespace.as_bytes())));
        tuple.push(Element::Int(TupleKeyspace::TrustPolicies as i64));
        tuple.push(Element::String(developer_id.to_string().into()));

        let tuple_key = pack(&tuple);

        if let Some(data) = tx.get(&tuple_key, false).await? {
            let policy: crate::api_adapter::TrustPolicy = bincode::deserialize(&data)?;
            Ok(Some(policy))
        } else {
            Ok(None)
        }
    }

    /// Store trust policy for a developer
    pub async fn store_trust_policy(
        &self,
        developer_id: &str,
        policy: &crate::api_adapter::TrustPolicy,
    ) -> Result<()> {
        let tx = self.db.create_trx()?;

        let mut tuple = Vec::new();
        tuple.push(Element::Bytes(Bytes::from(self.namespace.as_bytes())));
        tuple.push(Element::Int(TupleKeyspace::TrustPolicies as i64));
        tuple.push(Element::String(developer_id.to_string().into()));

        let tuple_key = pack(&tuple);
        let serialized = bincode::serialize(policy)?;

        tx.set(&tuple_key, &serialized);
        tx.commit()
            .await
            .map_err(|e| anyhow!("Transaction commit failed: {:?}", e))?;

        tracing::info!("Stored trust policy for developer: {}", developer_id);
        Ok(())
    }

    /// Store Lightwave checkpoint
    pub async fn store_checkpoint(
        &self,
        key: TrustedCheckpointKey,
        value: TrustedCheckpointValue,
    ) -> Result<()> {
        let tx = self.db.create_trx()?;
        let tuple_key = self.build_checkpoint_key(&key);
        let serialized = bincode::serialize(&value)?;

        tx.set(&pack(&tuple_key), &serialized);
        tx.commit()
            .await
            .map_err(|e| anyhow!("Transaction commit failed: {:?}", e))?;

        Ok(())
    }

    /// Get checkpoints with minimum verification count
    pub async fn get_trusted_checkpoints(
        &self,
        chain_id: &str,
        checkpoint_type: CheckpointType,
        min_verification_count: u32,
    ) -> Result<Vec<(TrustedCheckpointKey, TrustedCheckpointValue)>> {
        let tx = self.db.create_trx()?;

        let mut prefix = Vec::new();
        prefix.push(Element::Bytes(Bytes::from(self.namespace.as_bytes())));
        prefix.push(Element::Int(TupleKeyspace::TrustedCheckpoints as i64));
        prefix.push(Element::String(chain_id.to_string().into()));
        prefix.push(Element::Int(checkpoint_type as i64));

        let prefix_packed = pack(&prefix);
        // Implement proper range scanning for foundationdb 0.8
        let results = match scan_with_prefix(&tx, &prefix_packed, 1000).await {
            Ok(range_results) => range_results,
            Err(e) => {
                tracing::error!("Range scan failed: {}", e);
                Vec::new()
            }
        };

        let mut checkpoints = Vec::new();

        for (key, value) in results {
            let val: TrustedCheckpointValue = bincode::deserialize(&value)?;

            if val.verification_count >= min_verification_count {
                let tuple: Vec<Element> = unpack(&key)?;
                if let Some(Element::Int(height)) = tuple.get(4) {
                    let key = TrustedCheckpointKey {
                        chain_id: chain_id.to_string(),
                        checkpoint_type,
                        height_or_slot: *height as u64,
                    };
                    checkpoints.push((key, val));
                }
            }
        }

        Ok(checkpoints)
    }

    /// Store network configuration
    pub async fn store_network_config(
        &self,
        key: NetworkConfigKey,
        value: NetworkConfigValue,
    ) -> Result<()> {
        let tx = self.db.create_trx()?;
        let tuple_key = self.build_network_config_key(&key);
        let serialized = bincode::serialize(&value)?;

        tx.set(&pack(&tuple_key), &serialized);
        tx.commit()
            .await
            .map_err(|e| anyhow!("Transaction commit failed: {:?}", e))?;

        Ok(())
    }

    /// Get network configuration
    pub async fn get_network_config(
        &self,
        key: &NetworkConfigKey,
    ) -> Result<Option<NetworkConfigValue>> {
        let tx = self.db.create_trx()?;
        let tuple_key = self.build_network_config_key(key);

        if let Some(value) = tx.get(&pack(&tuple_key), false).await? {
            let config: NetworkConfigValue = bincode::deserialize(&value)?;
            Ok(Some(config))
        } else {
            Ok(None)
        }
    }

    /// Store circuit verification key
    pub async fn store_circuit_vk(&self, key: CircuitVKKey, value: CircuitVKValue) -> Result<()> {
        let tx = self.db.create_trx()?;
        let tuple_key = self.build_circuit_vk_key(&key);
        let serialized = bincode::serialize(&value)?;

        tx.set(&pack(&tuple_key), &serialized);
        tx.commit()
            .await
            .map_err(|e| anyhow!("Transaction commit failed: {:?}", e))?;

        Ok(())
    }

    /// Get circuit verification key
    pub async fn get_circuit_vk(&self, key: &CircuitVKKey) -> Result<Option<CircuitVKValue>> {
        let tx = self.db.create_trx()?;
        let tuple_key = self.build_circuit_vk_key(key);

        if let Some(value) = tx.get(&pack(&tuple_key), false).await? {
            let vk: CircuitVKValue = bincode::deserialize(&value)?;
            Ok(Some(vk))
        } else {
            Ok(None)
        }
    }

    /// List all circuit verification keys by type
    pub async fn list_circuit_vks_by_type(
        &self,
        circuit_type: &str,
    ) -> Result<Vec<(CircuitVKKey, CircuitVKValue)>> {
        let tx = self.db.create_trx()?;

        let mut prefix = Vec::new();
        prefix.push(Element::Bytes(Bytes::from(self.namespace.as_bytes())));
        prefix.push(Element::Int(TupleKeyspace::CircuitKeys as i64));
        prefix.push(Element::String(circuit_type.to_string().into()));

        let prefix_packed = pack(&prefix);
        // Implement proper range scanning for foundationdb 0.8
        let results = match scan_with_prefix(&tx, &prefix_packed, 1000).await {
            Ok(range_results) => range_results,
            Err(e) => {
                tracing::error!("Range scan failed: {}", e);
                Vec::new()
            }
        };

        let mut vks = Vec::new();

        for (key, value) in results {
            let tuple: Vec<Element> = unpack(&key)?;

            if let (Some(Element::String(circuit_type_elem)), Some(Element::String(version))) =
                (tuple.get(2), tuple.get(3))
            {
                let key = CircuitVKKey {
                    circuit_type: circuit_type_elem.to_string(),
                    version: version.to_string(),
                };

                let val: CircuitVKValue = bincode::deserialize(&value)?;
                vks.push((key, val));
            }
        }

        Ok(vks)
    }

    /// Store validator signature
    pub async fn store_validator_signature(
        &self,
        key: ValidatorSignatureKey,
        value: ValidatorSignatureValue,
    ) -> Result<()> {
        let tx = self.db.create_trx()?;
        let tuple_key = self.build_validator_signature_key(&key);
        let serialized = bincode::serialize(&value)?;

        tx.set(&pack(&tuple_key), &serialized);
        tx.commit()
            .await
            .map_err(|e| anyhow!("Transaction commit failed: {:?}", e))?;

        Ok(())
    }

    /// Get validator signature
    pub async fn get_validator_signature(
        &self,
        key: &ValidatorSignatureKey,
    ) -> Result<Option<ValidatorSignatureValue>> {
        let tx = self.db.create_trx()?;
        let tuple_key = self.build_validator_signature_key(key);

        if let Some(value) = tx.get(&pack(&tuple_key), false).await? {
            let signature: ValidatorSignatureValue = bincode::deserialize(&value)?;
            Ok(Some(signature))
        } else {
            Ok(None)
        }
    }

    /// List all validator signatures for a block
    pub async fn list_validator_signatures_for_block(
        &self,
        chain_id: &str,
        block_identifier: &BlockIdentifier,
    ) -> Result<Vec<(ValidatorSignatureKey, ValidatorSignatureValue)>> {
        let tx = self.db.create_trx()?;

        let mut prefix = Vec::new();
        prefix.push(Element::Bytes(Bytes::from(self.namespace.as_bytes())));
        prefix.push(Element::Int(TupleKeyspace::ValidatorSignatures as i64));
        prefix.push(Element::String(chain_id.to_string().into()));

        match block_identifier {
            BlockIdentifier::Height(h) => {
                prefix.push(Element::String("height".to_string().into()));
                prefix.push(Element::Int(*h as i64));
            }
            BlockIdentifier::Hash(h) => {
                prefix.push(Element::String("hash".to_string().into()));
                prefix.push(Element::Bytes(Bytes::from(h.to_vec())));
            }
            BlockIdentifier::Slot(s) => {
                prefix.push(Element::String("slot".to_string().into()));
                prefix.push(Element::Int(*s as i64));
            }
        }

        let prefix_packed = pack(&prefix);
        // Implement proper range scanning for foundationdb 0.8
        let results = match scan_with_prefix(&tx, &prefix_packed, 1000).await {
            Ok(range_results) => range_results,
            Err(e) => {
                tracing::error!("Range scan failed: {}", e);
                Vec::new()
            }
        };

        let mut signatures = Vec::new();

        for (key, value) in results {
            let tuple: Vec<Element> = unpack(&key)?;

            if let Some(Element::String(validator_id)) = tuple.get(5) {
                let sig_key = ValidatorSignatureKey {
                    chain_id: chain_id.to_string(),
                    block_identifier: block_identifier.clone(),
                    validator_id: validator_id.to_string(),
                };

                let sig_value: ValidatorSignatureValue = bincode::deserialize(&value)?;
                signatures.push((sig_key, sig_value));
            }
        }

        Ok(signatures)
    }

    /// Get proposer signature for a block
    pub async fn get_proposer_signature(
        &self,
        chain_id: &str,
        block_identifier: &BlockIdentifier,
    ) -> Result<Option<(ValidatorSignatureKey, ValidatorSignatureValue)>> {
        let signatures = self
            .list_validator_signatures_for_block(chain_id, block_identifier)
            .await?;

        // Find the proposer signature
        for (key, value) in signatures {
            if value.is_proposer {
                return Ok(Some((key, value)));
            }
        }

        Ok(None)
    }
}
