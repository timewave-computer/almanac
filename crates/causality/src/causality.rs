/// Causality tracking and relationship management
use std::collections::{HashMap, HashSet};
use std::time::SystemTime;

use indexer_core::types::ChainId;

use crate::error::{CausalityError, Result};
use crate::types::{
    CausalityEvent, CausalityResource, CausalityEffect, CausalityTransaction,
    CausalityIntent, CausalityHandler, CausalityDomain, CausalityNullifier,
    CrossChainReference, SmtHasher, Hash, EntityId, DomainId
};

/// Represents a causal relationship between Resource Model entities
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CausalityRelation {
    /// Source entity ID (content-addressed hash)
    pub from_entity: Hash,
    /// Target entity ID (content-addressed hash)
    pub to_entity: Hash,
    /// Type of causal relationship
    pub relation_type: CausalityRelationType,
    /// Strength of the causal relationship (0.0 to 1.0)
    pub strength: f64,
    /// Timestamp when the relationship was established
    pub established_at: SystemTime,
    /// Additional metadata
    pub metadata: HashMap<String, String>,
}

/// Types of causal relationships
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum CausalityRelationType {
    /// Direct causal dependency
    DirectDependency,
    /// Resource flow dependency
    ResourceFlow,
    /// Cross-chain dependency
    CrossChain,
    /// Temporal dependency (happens-before)
    Temporal,
    /// State dependency
    State,
    /// Custom relationship type
    Custom(String),
}

/// Causality graph for tracking relationships between Resource Model entities
#[derive(Debug, Clone)]
pub struct CausalityGraph {
    /// Resources in the graph
    resources: HashMap<EntityId, CausalityResource>,
    /// Effects in the graph
    effects: HashMap<EntityId, CausalityEffect>,
    /// Transactions in the graph
    transactions: HashMap<EntityId, CausalityTransaction>,
    /// Intents in the graph
    intents: HashMap<EntityId, CausalityIntent>,
    /// Handlers in the graph
    handlers: HashMap<EntityId, CausalityHandler>,
    /// Domains in the graph
    domains: HashMap<EntityId, CausalityDomain>,
    /// Nullifiers in the graph
    nullifiers: HashMap<Hash, CausalityNullifier>,
    /// Events in the graph (for cross-chain indexing)
    events: HashMap<String, CausalityEvent>,
    /// Relationships between entities
    relations: HashMap<Hash, Vec<CausalityRelation>>,
    /// Reverse index for efficient lookups
    reverse_relations: HashMap<Hash, Vec<Hash>>,
    /// Domain-specific entity indices
    domain_entities: HashMap<DomainId, HashSet<EntityId>>,
    /// Chain-specific event indices (for cross-chain events)
    chain_events: HashMap<ChainId, HashSet<String>>,
}

impl CausalityGraph {
    /// Create a new empty causality graph
    pub fn new() -> Self {
        Self {
            resources: HashMap::new(),
            effects: HashMap::new(),
            transactions: HashMap::new(),
            intents: HashMap::new(),
            handlers: HashMap::new(),
            domains: HashMap::new(),
            nullifiers: HashMap::new(),
            events: HashMap::new(),
            relations: HashMap::new(),
            reverse_relations: HashMap::new(),
            domain_entities: HashMap::new(),
            chain_events: HashMap::new(),
        }
    }

    /// Add an event to the graph
    pub fn add_event(&mut self, event: CausalityEvent) -> Result<()> {
        let event_id = event.id.clone();
        let chain_id = event.chain_id.clone();

        // Add to events
        self.events.insert(event_id.clone(), event);

        // Add to chain index
        self.chain_events
            .entry(chain_id)
            .or_default()
            .insert(event_id);

        Ok(())
    }

    /// Add a resource to the graph
    pub fn add_resource(&mut self, resource: CausalityResource) -> Result<()> {
        let resource_id = resource.id;
        let domain_id = resource.domain_id;

        // Add to resources
        self.resources.insert(resource_id, resource);

        // Add to domain index
        self.domain_entities
            .entry(domain_id)
            .or_default()
            .insert(resource_id);

        Ok(())
    }

    /// Add an effect to the graph
    pub fn add_effect(&mut self, effect: CausalityEffect) -> Result<()> {
        let effect_id = effect.id;
        let domain_id = effect.domain_id;

        // Add to effects
        self.effects.insert(effect_id, effect);

        // Add to domain index
        self.domain_entities
            .entry(domain_id)
            .or_default()
            .insert(effect_id);

        Ok(())
    }

    /// Add a transaction to the graph
    pub fn add_transaction(&mut self, transaction: CausalityTransaction) -> Result<()> {
        let transaction_id = transaction.id;
        let domain_id = transaction.domain_id;

        // Add to transactions
        self.transactions.insert(transaction_id, transaction);

        // Add to domain index
        self.domain_entities
            .entry(domain_id)
            .or_default()
            .insert(transaction_id);

        Ok(())
    }

    /// Add an intent to the graph
    pub fn add_intent(&mut self, intent: CausalityIntent) -> Result<()> {
        let intent_id = intent.id;
        let domain_id = intent.domain_id;

        // Add to intents
        self.intents.insert(intent_id, intent);

        // Add to domain index
        self.domain_entities
            .entry(domain_id)
            .or_default()
            .insert(intent_id);

        Ok(())
    }

    /// Add a handler to the graph
    pub fn add_handler(&mut self, handler: CausalityHandler) -> Result<()> {
        let handler_id = handler.id;
        let domain_id = handler.domain_id;

        // Add to handlers
        self.handlers.insert(handler_id, handler);

        // Add to domain index
        self.domain_entities
            .entry(domain_id)
            .or_default()
            .insert(handler_id);

        Ok(())
    }

    /// Add a domain to the graph
    pub fn add_domain(&mut self, domain: CausalityDomain) -> Result<()> {
        let domain_id = domain.id;
        let domain_id_as_domain = DomainId::new(domain_id.inner());

        // Add to domains
        self.domains.insert(domain_id, domain);

        // Ensure domain exists in domain_entities index
        self.domain_entities
            .entry(domain_id_as_domain)
            .or_default();

        Ok(())
    }

    /// Add a nullifier to the graph
    pub fn add_nullifier(&mut self, nullifier: CausalityNullifier) -> Result<()> {
        let nullifier_id = nullifier.nullifier_hash;
        
        // Add to nullifiers (using nullifier_hash as the key)
        self.nullifiers.insert(nullifier_id, nullifier);

        Ok(())
    }

    /// Get all entities for a specific domain
    pub fn get_domain_entities(&self, domain_id: &DomainId) -> Vec<EntityId> {
        self.domain_entities
            .get(domain_id)
            .map(|entities| entities.iter().copied().collect())
            .unwrap_or_default()
    }

    /// Add a causal relationship between entities
    pub fn add_relation(&mut self, relation: CausalityRelation) -> Result<()> {
        let from_entity = relation.from_entity;
        let to_entity = relation.to_entity;

        // Verify both entities exist (check in any of the entity collections)
        if !self.entity_exists(&from_entity) {
            return Err(CausalityError::RelationNotFound(format!(
                "Source entity not found: {:?}", from_entity
            )));
        }
        if !self.entity_exists(&to_entity) {
            return Err(CausalityError::RelationNotFound(format!(
                "Target entity not found: {:?}", to_entity
            )));
        }

        // Add forward relation
        self.relations
            .entry(from_entity)
            .or_default()
            .push(relation);

        // Add reverse relation
        self.reverse_relations
            .entry(to_entity)
            .or_default()
            .push(from_entity);

        Ok(())
    }

    /// Check if an entity exists in any collection
    fn entity_exists(&self, entity_id: &Hash) -> bool {
        let entity_id_typed = EntityId::new(*entity_id);
        self.resources.contains_key(&entity_id_typed) ||
        self.effects.contains_key(&entity_id_typed) ||
        self.transactions.contains_key(&entity_id_typed) ||
        self.intents.contains_key(&entity_id_typed) ||
        self.handlers.contains_key(&entity_id_typed) ||
        self.domains.contains_key(&entity_id_typed) ||
        self.nullifiers.contains_key(entity_id)
    }

    /// Convert string to hash (simple implementation for now)
    fn string_to_hash(&self, s: &str) -> Result<Hash> {
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(s.as_bytes());
        let result = hasher.finalize();
        let mut hash = [0u8; 32];
        hash.copy_from_slice(&result);
        Ok(hash)
    }

    /// Convert hash to string representation
    fn hash_to_string(&self, hash: &Hash) -> String {
        hex::encode(hash)
    }

    /// Get all events that depend on the given event
    pub fn get_dependents(&self, event_id: &str) -> Vec<&CausalityRelation> {
        // Convert string event_id to hash for lookup
        if let Some(_event) = self.events.get(event_id) {
            if let Ok(event_hash) = self.string_to_hash(event_id) {
                return self.relations
                    .get(&event_hash)
                    .map(|relations| relations.iter().collect())
                    .unwrap_or_default();
            }
        }
        Vec::new()
    }

    /// Get all events that the given event depends on
    pub fn get_dependencies(&self, event_id: &str) -> Vec<String> {
        // Convert string event_id to hash for lookup
        if let Ok(event_hash) = self.string_to_hash(event_id) {
            return self.reverse_relations
                .get(&event_hash)
                .map(|hashes| hashes.iter().map(|h| self.hash_to_string(h)).collect())
                .unwrap_or_default();
        }
        Vec::new()
    }

    /// Get events for a specific chain
    pub fn get_chain_events(&self, chain_id: &ChainId) -> Vec<&CausalityEvent> {
        self.chain_events
            .get(chain_id)
            .map(|event_ids| {
                event_ids
                    .iter()
                    .filter_map(|id| self.events.get(id))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Find causal paths between two events
    pub fn find_causal_path(&self, from: &str, to: &str) -> Vec<Vec<String>> {
        let mut paths = Vec::new();
        let mut visited = HashSet::new();
        let mut current_path = Vec::new();

        self.find_paths_recursive(from, to, &mut visited, &mut current_path, &mut paths);
        paths
    }

    /// Recursive helper for finding causal paths
    fn find_paths_recursive(
        &self,
        current: &str,
        target: &str,
        visited: &mut HashSet<String>,
        current_path: &mut Vec<String>,
        paths: &mut Vec<Vec<String>>,
    ) {
        if visited.contains(current) {
            return; // Avoid cycles
        }

        visited.insert(current.to_string());
        current_path.push(current.to_string());

        if current == target {
            paths.push(current_path.clone());
        } else {
            // Explore all outgoing relations
            if let Ok(current_hash) = self.string_to_hash(current) {
                if let Some(relations) = self.relations.get(&current_hash) {
                    for relation in relations {
                        let to_entity_str = self.hash_to_string(&relation.to_entity);
                        self.find_paths_recursive(
                            &to_entity_str,
                            target,
                            visited,
                            current_path,
                            paths,
                        );
                    }
                }
            }
        }

        current_path.pop();
        visited.remove(current);
    }

    /// Get strongly connected components (cycles) in the graph
    pub fn find_cycles(&self) -> Vec<Vec<String>> {
        // Simplified cycle detection - could be enhanced with Tarjan's algorithm
        let mut cycles = Vec::new();
        let mut visited = HashSet::new();

        for event_id in self.events.keys() {
            if !visited.contains(event_id) {
                let mut path = Vec::new();
                self.detect_cycle_dfs(event_id, &mut visited, &mut path, &mut cycles);
            }
        }

        cycles
    }

    /// DFS-based cycle detection
    fn detect_cycle_dfs(
        &self,
        current: &str,
        visited: &mut HashSet<String>,
        path: &mut Vec<String>,
        cycles: &mut Vec<Vec<String>>,
    ) {
        if path.contains(&current.to_string()) {
            // Found a cycle
            let cycle_start = path.iter().position(|x| x == current).unwrap();
            cycles.push(path[cycle_start..].to_vec());
            return;
        }

        if visited.contains(current) {
            return;
        }

        visited.insert(current.to_string());
        path.push(current.to_string());

        // Convert current string to hash for lookup
        if let Ok(current_hash) = self.string_to_hash(current) {
            if let Some(relations) = self.relations.get(&current_hash) {
                for relation in relations {
                    let to_entity_str = self.hash_to_string(&relation.to_entity);
                    self.detect_cycle_dfs(&to_entity_str, visited, path, cycles);
                }
            }
        }

        path.pop();
    }
}

impl Default for CausalityGraph {
    fn default() -> Self {
        Self::new()
    }
}

/// Causality tracker for managing causal relationships
pub struct CausalityTracker {
    /// The causality graph
    graph: CausalityGraph,
    /// SMT hasher for key generation
    hasher: Box<dyn SmtHasher>,
}

impl CausalityTracker {
    /// Create a new causality tracker
    pub fn new(hasher: Box<dyn SmtHasher>) -> Self {
        Self {
            graph: CausalityGraph::new(),
            hasher,
        }
    }

    /// Add an event and automatically detect causal relationships
    pub fn add_event(&mut self, event: CausalityEvent) -> Result<()> {
        // Extract relationships based on event data type
        match &event.data {
            crate::types::CausalityEventData::Effect(effect) => {
                // Add relationships based on effect inputs/outputs
                for input in &effect.inputs {
                    if self.graph.events.contains_key(&input.from_event) {
                        // Create a causal relationship for resource flow
                        let from_hash = self.graph.string_to_hash(&input.from_event)?;
                        let to_hash = self.graph.string_to_hash(&input.to_event)?;
                        
                        let relation = CausalityRelation {
                            from_entity: from_hash,
                            to_entity: to_hash,
                            relation_type: CausalityRelationType::ResourceFlow,
                            strength: 1.0,
                            established_at: std::time::SystemTime::now(),
                            metadata: std::collections::HashMap::new(),
                        };
                        
                        self.graph.add_relation(relation)?;
                    }
                }
            },
            crate::types::CausalityEventData::CrossDomainMessage { source_domain, target_domain, .. } => {
                // Record cross-domain relationship
                let _cross_domain_key = self.hasher.key(
                    &format!("cross-domain:{:?}:{:?}", source_domain, target_domain),
                    event.id.as_bytes(),
                );
            },
            _ => {
                // For other event types, we could add specific relationship detection logic
            }
        }

        // Add the event to the graph
        self.graph.add_event(event)?;

        Ok(())
    }

    /// Get the causality graph
    pub fn graph(&self) -> &CausalityGraph {
        &self.graph
    }

    /// Get mutable access to the causality graph
    pub fn graph_mut(&mut self) -> &mut CausalityGraph {
        &mut self.graph
    }

    /// Analyze causal relationships for a specific event
    pub fn analyze_event_causality(&self, event_id: &str) -> Option<EventCausalityAnalysis> {
        let event = self.graph.events.get(event_id)?;
        let dependencies = self.graph.get_dependencies(event_id);
        let dependents = self.graph.get_dependents(event_id);

        Some(EventCausalityAnalysis {
            event_id: event_id.to_string(),
            chain_id: event.chain_id.clone(),
            dependency_count: dependencies.len(),
            dependent_count: dependents.len(),
            causal_depth: self.calculate_causal_depth(event_id),
            is_root: dependencies.is_empty(),
            is_leaf: dependents.is_empty(),
        })
    }

    /// Calculate the causal depth of an event (longest path from root)
    fn calculate_causal_depth(&self, event_id: &str) -> usize {
        let mut max_depth = 0;
        let dependencies = self.graph.get_dependencies(event_id);

        for dep_id in dependencies {
            let depth = 1 + self.calculate_causal_depth(&dep_id);
            max_depth = max_depth.max(depth);
        }

        max_depth
    }
}

/// Analysis result for an event's causal relationships
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct EventCausalityAnalysis {
    /// Event ID
    pub event_id: String,
    /// Chain where the event occurred
    pub chain_id: ChainId,
    /// Number of events this event depends on
    pub dependency_count: usize,
    /// Number of events that depend on this event
    pub dependent_count: usize,
    /// Causal depth (longest path from root events)
    pub causal_depth: usize,
    /// Whether this is a root event (no dependencies)
    pub is_root: bool,
    /// Whether this is a leaf event (no dependents)
    pub is_leaf: bool,
}

/// Cross-chain causality tracker
pub struct CrossChainCausality {
    /// Per-chain causality trackers
    chain_trackers: HashMap<ChainId, CausalityTracker>,
    /// Cross-chain relationships
    cross_chain_relations: Vec<CrossChainReference>,
    /// SMT hasher
    #[allow(dead_code)]
    hasher: Box<dyn SmtHasher>,
}

impl CrossChainCausality {
    /// Create a new cross-chain causality tracker
    pub fn new(hasher: Box<dyn SmtHasher>) -> Self {
        Self {
            chain_trackers: HashMap::new(),
            cross_chain_relations: Vec::new(),
            hasher,
        }
    }

    /// Get or create a tracker for a specific chain
    pub fn get_chain_tracker(&mut self, chain_id: &ChainId) -> &mut CausalityTracker {
        self.chain_trackers
            .entry(chain_id.clone())
            .or_insert_with(|| CausalityTracker::new(Box::new(crate::smt::Sha256SmtHasher)))
    }

    /// Add a cross-chain reference
    pub fn add_cross_chain_reference(&mut self, reference: CrossChainReference) {
        self.cross_chain_relations.push(reference);
    }

    /// Find cross-chain causal paths
    pub fn find_cross_chain_paths(&self, _from_chain: &ChainId, _to_chain: &ChainId) -> Vec<CrossChainPath> {
        // This would implement sophisticated cross-chain path finding
        // For now, return empty vector as placeholder
        Vec::new()
    }
}

/// Represents a causal path across multiple chains
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CrossChainPath {
    /// Chains involved in the path
    pub chains: Vec<ChainId>,
    /// Events in the path
    pub events: Vec<String>,
    /// Cross-chain references used
    pub references: Vec<CrossChainReference>,
    /// Total path length
    pub length: usize,
} 