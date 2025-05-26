/// Core types for the causality indexer - compatible with reverse-causality framework
use std::collections::HashMap;
use std::time::SystemTime;

use indexer_core::event::Event;
use indexer_core::types::ChainId;

use crate::error::{CausalityError, Result};

/// Hash type - 32 bytes for compatibility with both SHA256 and Blake3
pub type Hash = [u8; 32];

/// SMT root hash
pub type SmtRoot = Hash;

/// SMT key type
pub type SmtKey = Hash;

/// Entity ID type - content-addressed identifier used throughout reverse-causality
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct EntityId(Hash);

/// Domain ID type - identifies execution domains
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DomainId(Hash);

/// Expression ID type - content-addressed expressions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ExprId(Hash);

/// Value Expression ID type - content-addressed data/state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ValueExprId(Hash);

/// Handler ID type - identifies effect handlers
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct HandlerId(Hash);

/// Intent ID type - identifies intents
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct IntentId(Hash);

/// Resource ID type - identifies resources
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ResourceId(Hash);

/// Transaction ID type - identifies transactions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TransactionId(Hash);

/// Effect ID type - identifies effects
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct EffectId(Hash);

/// Typed Domain - execution environment types from reverse-causality
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum TypedDomain {
    /// Verifiable domain - ZK-provable computations
    VerifiableDomain {
        /// Domain identifier
        domain_id: DomainId,
        /// Capabilities this domain provides
        capabilities: Vec<String>,
    },
    /// Service domain - external service integrations
    ServiceDomain {
        /// Domain identifier
        domain_id: DomainId,
        /// Type of service (e.g., "http", "grpc", "websocket")
        service_type: String,
        /// Optional service endpoint URL
        endpoint: Option<String>,
    },
}

impl Default for TypedDomain {
    fn default() -> Self {
        Self::VerifiableDomain {
            domain_id: DomainId::null(),
            capabilities: Vec::new(),
        }
    }
}

// ID type implementations
impl EntityId {
    /// Create a new EntityId from a hash
    pub fn new(hash: Hash) -> Self {
        Self(hash)
    }
    
    /// Create a null EntityId (all zeros)
    pub fn null() -> Self {
        Self([0u8; 32])
    }
    
    /// Get the inner hash value
    pub fn inner(&self) -> Hash {
        self.0
    }
    
    /// Create EntityId from SSZ-encoded bytes
    pub fn from_ssz_bytes(data: &[u8]) -> Result<Self> {
        if data.len() != 32 {
            return Err(CausalityError::InvalidHash("EntityId must be 32 bytes".to_string()));
        }
        let mut hash = [0u8; 32];
        hash.copy_from_slice(data);
        Ok(Self(hash))
    }
    
    /// Convert to SSZ-encoded bytes
    pub fn as_ssz_bytes(&self) -> Vec<u8> {
        self.0.to_vec()
    }
}

impl DomainId {
    /// Create a new DomainId from a hash
    pub fn new(hash: Hash) -> Self {
        Self(hash)
    }
    
    /// Create a null DomainId (all zeros)
    pub fn null() -> Self {
        Self([0u8; 32])
    }
    
    /// Get the inner hash value
    pub fn inner(&self) -> Hash {
        self.0
    }
    
    /// Convert to SSZ-encoded bytes
    pub fn as_ssz_bytes(&self) -> Vec<u8> {
        self.0.to_vec()
    }
}

impl ExprId {
    /// Create a new ExprId from a hash
    pub fn new(hash: Hash) -> Self {
        Self(hash)
    }
    
    /// Create a null ExprId (all zeros)
    pub fn null() -> Self {
        Self([0u8; 32])
    }
    
    /// Get the inner hash value
    pub fn inner(&self) -> Hash {
        self.0
    }
    
    /// Convert to SSZ-encoded bytes
    pub fn as_ssz_bytes(&self) -> Vec<u8> {
        self.0.to_vec()
    }
}

impl HandlerId {
    /// Create a new HandlerId from a hash
    pub fn new(hash: Hash) -> Self {
        Self(hash)
    }
    
    /// Create a null HandlerId (all zeros)
    pub fn null() -> Self {
        Self([0u8; 32])
    }
    
    /// Get the inner hash value
    pub fn inner(&self) -> Hash {
        self.0
    }
    
    /// Convert to SSZ-encoded bytes
    pub fn as_ssz_bytes(&self) -> Vec<u8> {
        self.0.to_vec()
    }
}

impl ResourceId {
    /// Create a new ResourceId from a hash
    pub fn new(hash: Hash) -> Self {
        Self(hash)
    }
    
    /// Create a null ResourceId (all zeros)
    pub fn null() -> Self {
        Self([0u8; 32])
    }
    
    /// Get the inner hash value
    pub fn inner(&self) -> Hash {
        self.0
    }
    
    /// Convert to SSZ-encoded bytes
    pub fn as_ssz_bytes(&self) -> Vec<u8> {
        self.0.to_vec()
    }
}

/// Helper to create empty hash
pub fn empty_hash() -> Hash {
    [0u8; 32]
}

/// Helper to convert hash to hex string
pub fn hash_to_hex(hash: &Hash) -> String {
    hex::encode(hash)
}

/// Helper to convert hex string to hash
pub fn hash_from_hex(hex_str: &str) -> Result<Hash> {
    let mut hash = [0u8; 32];
    hex::decode_to_slice(hex_str, &mut hash)
        .map_err(|e| CausalityError::InvalidHash(format!("Invalid hex: {}", e)))?;
    Ok(hash)
}

/// SMT children structure for internal nodes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SmtChildren {
    /// Left child hash
    pub left: Hash,
    /// Right child hash  
    pub right: Hash,
}

/// SMT proof for verifying inclusion/exclusion
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SmtProof {
    /// Path of sibling hashes from leaf to root
    pub siblings: Vec<Hash>,
    /// Whether each sibling is on the left (false) or right (true)
    pub directions: Vec<bool>,
}

impl SmtProof {
    /// Create a new SMT proof
    pub fn new(siblings: Vec<Hash>, directions: Vec<bool>) -> Self {
        Self { siblings, directions }
    }

    /// Verify this proof against a root, key, and value
    pub fn verify(&self, root: &Hash, _key: &Hash, value: &[u8], hasher: &dyn SmtHasher) -> bool {
        if self.siblings.len() != self.directions.len() {
            return false;
        }

        let leaf_hash = hasher.hash(value);
        let mut current_hash = leaf_hash;

        // Traverse from leaf to root
        for (i, &is_right) in self.directions.iter().enumerate() {
            let sibling = self.siblings[i];
            
            current_hash = if is_right {
                hasher.merge(&sibling, &current_hash)
            } else {
                hasher.merge(&current_hash, &sibling)
            };
        }

        current_hash == *root
    }
}

/// Trait for SMT hashers
pub trait SmtHasher: Send + Sync {
    /// Hash data
    fn hash(&self, data: &[u8]) -> Hash;
    
    /// Merge two hashes
    fn merge(&self, left: &Hash, right: &Hash) -> Hash;
    
    /// Generate key from context and data
    fn key(&self, context: &str, data: &[u8]) -> Hash;
    
    /// Digest multiple byte arrays
    fn digest(&self, data_list: &[&[u8]]) -> Hash;
}

/// Content-addressable trait for entities
pub trait ContentAddressable {
    /// Get the content-addressed identifier
    fn content_id(&self) -> EntityId;
    
    /// Serialize to SSZ bytes for content addressing
    fn to_ssz_bytes(&self) -> Result<Vec<u8>>;
    
    /// Compute content address from SSZ bytes
    fn compute_content_address(ssz_bytes: &[u8]) -> EntityId {
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(ssz_bytes);
        let result = hasher.finalize();
        let mut hash = [0u8; 32];
        hash.copy_from_slice(&result);
        EntityId::new(hash)
    }
}

/// Resource in the causality system - compatible with reverse-causality Resource
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CausalityResource {
    /// Content-addressed identifier
    pub id: EntityId,
    /// Human-readable name or description
    pub name: String,
    /// Domain this resource belongs to
    pub domain_id: DomainId,
    /// Resource type identifier (e.g., "token", "compute_credits", "bandwidth")
    pub resource_type: String,
    /// Current quantity/amount of this resource
    pub quantity: u64,
    /// When this resource was created or last updated
    pub timestamp: SystemTime,
}

/// Effect represents a computational effect - compatible with reverse-causality Effect
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CausalityEffect {
    /// Content-addressed identifier
    pub id: EntityId,
    /// Human-readable name
    pub name: String,
    /// Domain this effect belongs to
    pub domain_id: DomainId,
    /// Effect type identifier
    pub effect_type: String,
    /// Resources consumed by this effect
    pub inputs: Vec<ResourceFlow>,
    /// Resources produced by this effect
    pub outputs: Vec<ResourceFlow>,
    /// Optional expression logic (content-addressed)
    pub expression: Option<ExprId>,
    /// When this effect was created/executed
    pub timestamp: SystemTime,
    /// Handler that scopes this effect
    pub scoped_by: HandlerId,
    /// Intent this effect satisfies
    pub intent_id: Option<IntentId>,
    /// Source typed domain
    pub source_typed_domain: TypedDomain,
    /// Target typed domain
    pub target_typed_domain: TypedDomain,
    /// ProcessDataflowBlock instance this effect originates from
    pub originating_dataflow_instance: Option<ResourceId>,
}

/// Transaction represents a collection of effects and intents
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CausalityTransaction {
    /// Content-addressed identifier
    pub id: EntityId,
    /// Human-readable name
    pub name: String,
    /// Domain this transaction belongs to
    pub domain_id: DomainId,
    /// All effects included in this transaction
    pub effects: Vec<EffectId>,
    /// All intents satisfied by this transaction
    pub intents: Vec<IntentId>,
    /// Aggregated resources consumed
    pub inputs: Vec<ResourceFlow>,
    /// Aggregated resources produced
    pub outputs: Vec<ResourceFlow>,
    /// When this transaction was executed
    pub timestamp: SystemTime,
}

/// Intent represents a commitment to transform resources
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CausalityIntent {
    /// Content-addressed identifier
    pub id: EntityId,
    /// Human-readable name
    pub name: String,
    /// Domain this intent belongs to
    pub domain_id: DomainId,
    /// Intent type
    pub intent_type: String,
    /// Required input resources
    pub required_inputs: Vec<ResourceFlow>,
    /// Expected output resources
    pub expected_outputs: Vec<ResourceFlow>,
    /// Satisfaction constraints (SSZ-encoded)
    pub constraints: Vec<u8>,
    /// When this intent was created
    pub timestamp: SystemTime,
    /// Whether this intent has been satisfied
    pub is_satisfied: bool,
    /// Priority level for resolution ordering
    pub priority: u32,
    /// Target typed domain for execution
    pub target_typed_domain: Option<TypedDomain>,
}

/// Handler represents logic for processing specific effect types
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CausalityHandler {
    /// Content-addressed identifier
    pub id: EntityId,
    /// Human-readable name
    pub name: String,
    /// Domain this handler belongs to
    pub domain_id: DomainId,
    /// Effect types this handler can process
    pub effect_types: Vec<String>,
    /// Handler logic (content-addressed expression)
    pub expression: ExprId,
    /// When this handler was registered
    pub timestamp: SystemTime,
}

/// Nullifier represents proof that a resource has been consumed
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CausalityNullifier {
    /// Resource that was consumed
    pub resource_id: EntityId,
    /// Cryptographic nullifier hash
    pub nullifier_hash: Hash,
    /// When the resource was consumed
    pub timestamp: SystemTime,
}

/// Domain represents an execution environment
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CausalityDomain {
    /// Content-addressed identifier
    pub id: EntityId,
    /// Human-readable name
    pub name: String,
    /// Domain type (VerifiableDomain, ServiceDomain, etc.)
    pub domain_type: String,
    /// Capabilities this domain provides
    pub capabilities: Vec<String>,
    /// Configuration data (SSZ-encoded)
    pub config: Vec<u8>,
    /// When this domain was registered
    pub timestamp: SystemTime,
}

/// Unified event type that can represent any causality system event
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CausalityEvent {
    /// Unique event ID
    pub id: String,
    /// Chain where the event occurred (for cross-chain indexing)
    pub chain_id: ChainId,
    /// Block number (for blockchain events)
    pub block_number: u64,
    /// Transaction hash (for blockchain events)
    pub tx_hash: String,
    /// Event type
    pub event_type: CausalityEventType,
    /// Event timestamp
    pub timestamp: SystemTime,
    /// Event-specific data
    pub data: CausalityEventData,
}

/// Types of events in the causality system
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum CausalityEventType {
    /// Resource creation, update, or consumption
    ResourceEvent,
    /// Effect execution
    EffectEvent,
    /// Transaction execution
    TransactionEvent,
    /// Intent creation or satisfaction
    IntentEvent,
    /// Handler registration or execution
    HandlerEvent,
    /// Domain registration or update
    DomainEvent,
    /// Nullifier creation
    NullifierEvent,
    /// Cross-domain message
    CrossDomainMessage,
    /// TEG (Temporal Effect Graph) state change
    TegStateChange,
}

/// Event-specific data
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum CausalityEventData {
    /// Resource-related event
    Resource(CausalityResource),
    /// Effect-related event
    Effect(CausalityEffect),
    /// Transaction-related event
    Transaction(CausalityTransaction),
    /// Intent-related event
    Intent(CausalityIntent),
    /// Handler-related event
    Handler(CausalityHandler),
    /// Domain-related event
    Domain(CausalityDomain),
    /// Nullifier-related event
    Nullifier(CausalityNullifier),
    /// Cross-domain message
    CrossDomainMessage {
        /// Source domain
        source_domain: Hash,
        /// Target domain
        target_domain: Hash,
        /// Message type
        message_type: String,
        /// Message payload
        payload: Vec<u8>,
    },
    /// TEG state change
    TegStateChange {
        /// Previous state root
        previous_root: Hash,
        /// New state root
        new_root: Hash,
        /// State transition details
        transition: Vec<u8>, // SSZ-encoded transition
    },
}

impl CausalityEvent {
    /// Create a new causality event from a core event
    pub fn from_event(event: &dyn Event) -> Self {
        Self {
            id: event.id().to_string(),
            chain_id: ChainId(event.chain().to_string()),
            block_number: event.block_number(),
            tx_hash: event.tx_hash().to_string(),
            event_type: CausalityEventType::CrossDomainMessage, // Default type for generic events
            timestamp: event.timestamp(),
            data: CausalityEventData::CrossDomainMessage {
                source_domain: empty_hash(),
                target_domain: empty_hash(),
                message_type: event.event_type().to_string(),
                payload: event.raw_data().to_vec(),
            },
        }
    }

    /// Get the SMT key for this event
    pub fn smt_key(&self, hasher: &dyn SmtHasher) -> SmtKey {
        hasher.key(&format!("event:{}", self.chain_id.0), self.id.as_bytes())
    }

    /// Serialize to bytes for SMT storage
    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        #[cfg(feature = "serde")]
        {
            serde_json::to_vec(self).map_err(CausalityError::from)
        }
        #[cfg(not(feature = "serde"))]
        {
            // Simple binary serialization without serde
            let mut bytes = Vec::new();
            bytes.extend_from_slice(self.id.as_bytes());
            bytes.extend_from_slice(&[0]); // separator
            bytes.extend_from_slice(self.chain_id.0.as_bytes());
            bytes.extend_from_slice(&[0]); // separator
            bytes.extend_from_slice(&self.block_number.to_le_bytes());
            // Note: CausalityEventData is complex, so we'll just use a placeholder for now
            bytes.extend_from_slice(b"event_data");
            Ok(bytes)
        }
    }

    /// Deserialize from bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        #[cfg(feature = "serde")]
        {
            serde_json::from_slice(bytes).map_err(CausalityError::from)
        }
        #[cfg(not(feature = "serde"))]
        {
            Err(CausalityError::serialization_error("Deserialization not supported without serde feature"))
        }
    }
}

impl CausalityResource {
    /// Get the SMT key for this resource
    pub fn smt_key(&self, hasher: &dyn SmtHasher) -> SmtKey {
        hasher.key(&format!("resource:{}", hash_to_hex(&self.domain_id.inner())), &self.id.inner())
    }

    /// Serialize to bytes for SMT storage
    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        #[cfg(feature = "serde")]
        {
            serde_json::to_vec(self).map_err(CausalityError::from)
        }
        #[cfg(not(feature = "serde"))]
        {
            let mut bytes = Vec::new();
            bytes.extend_from_slice(&self.id.inner());
            bytes.extend_from_slice(&self.name.as_bytes());
            bytes.extend_from_slice(&self.domain_id.inner());
            bytes.extend_from_slice(self.resource_type.as_bytes());
            bytes.extend_from_slice(&self.quantity.to_le_bytes());
            Ok(bytes)
        }
    }
}

impl CausalityEffect {
    /// Get the SMT key for this effect
    pub fn smt_key(&self, hasher: &dyn SmtHasher) -> SmtKey {
        hasher.key(&format!("effect:{}", hash_to_hex(&self.domain_id.inner())), &self.id.inner())
    }

    /// Serialize to bytes for SMT storage
    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        #[cfg(feature = "serde")]
        {
            serde_json::to_vec(self).map_err(CausalityError::from)
        }
        #[cfg(not(feature = "serde"))]
        {
            let mut bytes = Vec::new();
            bytes.extend_from_slice(&self.id.inner());
            bytes.extend_from_slice(&self.name.as_bytes());
            bytes.extend_from_slice(&self.domain_id.inner());
            bytes.extend_from_slice(self.effect_type.as_bytes());
            Ok(bytes)
        }
    }
}

impl CausalityTransaction {
    /// Get the SMT key for this transaction
    pub fn smt_key(&self, hasher: &dyn SmtHasher) -> SmtKey {
        hasher.key(&format!("transaction:{}", hash_to_hex(&self.domain_id.inner())), &self.id.inner())
    }

    /// Serialize to bytes for SMT storage
    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        #[cfg(feature = "serde")]
        {
            serde_json::to_vec(self).map_err(CausalityError::from)
        }
        #[cfg(not(feature = "serde"))]
        {
            let mut bytes = Vec::new();
            bytes.extend_from_slice(&self.id.inner());
            bytes.extend_from_slice(&self.name.as_bytes());
            bytes.extend_from_slice(&self.domain_id.inner());
            Ok(bytes)
        }
    }
}

/// Resource flow between events
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ResourceFlow {
    /// Source event ID
    pub from_event: String,
    /// Target event ID
    pub to_event: String,
    /// Resource type
    pub resource_type: String,
    /// Resource amount or identifier
    pub resource_data: Vec<u8>,
    /// Flow timestamp
    pub timestamp: SystemTime,
}

/// Cross-chain reference
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CrossChainReference {
    /// Source chain
    pub source_chain: ChainId,
    /// Target chain
    pub target_chain: ChainId,
    /// Reference type (e.g., "bridge", "oracle", "message")
    pub ref_type: String,
    /// Reference data
    pub ref_data: Vec<u8>,
    /// Reference timestamp
    pub timestamp: SystemTime,
}

/// Causality proof that can be verified
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CausalityProof {
    /// Root hash of the causality tree
    pub root: SmtRoot,
    /// SMT proofs for included events
    pub event_proofs: HashMap<String, SmtProof>,
    /// SMT proofs for included resources
    pub resource_proofs: HashMap<String, SmtProof>,
    /// Metadata about the proof
    pub metadata: ProofMetadata,
}

/// Metadata for causality proofs
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ProofMetadata {
    /// Proof generation timestamp
    pub generated_at: SystemTime,
    /// Chains included in the proof
    pub chains: Vec<ChainId>,
    /// Block range covered
    pub block_range: Option<(u64, u64)>,
    /// Proof type
    pub proof_type: String,
}

/// Causality index that tracks relationships
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CausalityIndex {
    /// Current SMT root
    pub root: SmtRoot,
    /// Event count
    pub event_count: u64,
    /// Resource count
    pub resource_count: u64,
    /// Last update timestamp
    pub last_updated: SystemTime,
    /// Indexed chains
    pub chains: Vec<ChainId>,
}

impl CausalityIndex {
    /// Create a new empty causality index
    pub fn new() -> Self {
        Self {
            root: empty_hash(),
            event_count: 0,
            resource_count: 0,
            last_updated: SystemTime::now(),
            chains: Vec::new(),
        }
    }
}

impl Default for CausalityIndex {
    fn default() -> Self {
        Self::new()
    }
} 