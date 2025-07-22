//! Fork choice rules and state management

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

use crate::types::{BlockInfo, ForkInfo, ForkStatus};
use almanac_core::{Error, Result};

/// Fork choice rule type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ForkChoiceRule {
    /// Longest chain rule (PoW)
    LongestChain,
    /// Heaviest chain rule (by difficulty)
    HeaviestChain,
    /// GHOST (Greedy Heaviest Observed SubTree)
    Ghost,
    /// LMD-GHOST (Latest Message Driven GHOST)
    LmdGhost,
    /// Casper FFG
    CasperFfg,
    /// Tendermint BFT
    TendermintBft,
    /// Tower BFT (Solana)
    TowerBft,
    /// Custom rule
    Custom(String),
}

/// Fork choice state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForkChoiceState {
    /// Current canonical chain head
    pub canonical_head: BlockInfo,
    /// Active forks
    pub forks: Vec<ForkInfo>,
    /// Fork choice rule
    pub rule: ForkChoiceRule,
    /// Chain-specific parameters
    pub parameters: HashMap<String, serde_json::Value>,
}

/// Fork choice algorithm trait
#[async_trait]
pub trait ForkChoice: Send + Sync {
    /// Get the fork choice rule
    fn rule(&self) -> &ForkChoiceRule;

    /// Process a new block and update fork choice
    async fn process_block(&self, block: BlockInfo) -> Result<ForkChoiceUpdate>;

    /// Get the current canonical head
    async fn get_canonical_head(&self) -> Result<BlockInfo>;

    /// Get all active forks
    async fn get_active_forks(&self) -> Result<Vec<ForkInfo>>;

    /// Evaluate which fork should be canonical
    async fn evaluate_forks(&self, forks: &[ForkInfo]) -> Result<String>;

    /// Get fork choice state
    async fn get_state(&self) -> Result<ForkChoiceState>;
}

/// Fork choice update result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForkChoiceUpdate {
    /// Whether the canonical head changed
    pub head_changed: bool,
    /// Previous canonical head (if changed)
    pub previous_head: Option<BlockInfo>,
    /// New canonical head
    pub new_head: BlockInfo,
    /// Reorganization depth (if head changed)
    pub reorg_depth: Option<u64>,
    /// Affected forks
    pub affected_forks: Vec<String>,
}

/// Longest chain fork choice implementation
pub struct LongestChainForkChoice {
    /// Current state
    state: Arc<tokio::sync::RwLock<ForkChoiceState>>,
}

impl LongestChainForkChoice {
    /// Create new longest chain fork choice
    pub fn new(initial_head: BlockInfo) -> Self {
        let state = ForkChoiceState {
            canonical_head: initial_head,
            forks: Vec::new(),
            rule: ForkChoiceRule::LongestChain,
            parameters: HashMap::new(),
        };

        Self {
            state: Arc::new(tokio::sync::RwLock::new(state)),
        }
    }
}

#[async_trait]
impl ForkChoice for LongestChainForkChoice {
    fn rule(&self) -> &ForkChoiceRule {
        &ForkChoiceRule::LongestChain
    }

    async fn process_block(&self, block: BlockInfo) -> Result<ForkChoiceUpdate> {
        let mut state = self.state.write().await;

        // Check if this extends the canonical chain
        if block.parent_hash == state.canonical_head.hash {
            // Direct extension of canonical chain
            let previous = state.canonical_head.clone();
            state.canonical_head = block.clone();

            Ok(ForkChoiceUpdate {
                head_changed: true,
                previous_head: Some(previous),
                new_head: block,
                reorg_depth: None,
                affected_forks: vec![],
            })
        } else {
            // Potential fork
            let fork_id = format!("fork_{}", block.hash);
            let mut fork_found = false;

            // Check if this extends an existing fork
            let mut fork_to_update = None;
            for (idx, fork) in state.forks.iter_mut().enumerate() {
                if block.parent_hash == fork.head_block.to_string() {
                    fork.head_block = block.number;
                    fork.length += 1;
                    fork_found = true;

                    // Check if this fork is now longer than canonical
                    if fork.length > state.canonical_head.number {
                        fork_to_update = Some(idx);
                    }
                    break;
                }
            }

            // Handle canonical head update if needed
            if let Some(idx) = fork_to_update {
                let previous = state.canonical_head.clone();
                state.canonical_head = block.clone();
                state.forks[idx].status = ForkStatus::Canonical;
                let fork_length = state.forks[idx].length;
                let fork_point = state.forks[idx].fork_point;

                return Ok(ForkChoiceUpdate {
                    head_changed: true,
                    previous_head: Some(previous),
                    new_head: block,
                    reorg_depth: Some(fork_length - fork_point),
                    affected_forks: vec![state.forks[idx].fork_id.clone()],
                });
            }

            if !fork_found {
                // New fork
                let fork = ForkInfo {
                    fork_id: fork_id.clone(),
                    fork_point: block.number - 1, // Approximate
                    head_block: block.number,
                    weight: 0, // Not used for longest chain
                    length: 1,
                    status: ForkStatus::Active,
                };
                state.forks.push(fork);
            }

            Ok(ForkChoiceUpdate {
                head_changed: false,
                previous_head: None,
                new_head: state.canonical_head.clone(),
                reorg_depth: None,
                affected_forks: vec![fork_id],
            })
        }
    }

    async fn get_canonical_head(&self) -> Result<BlockInfo> {
        Ok(self.state.read().await.canonical_head.clone())
    }

    async fn get_active_forks(&self) -> Result<Vec<ForkInfo>> {
        Ok(self.state.read().await.forks.clone())
    }

    async fn evaluate_forks(&self, forks: &[ForkInfo]) -> Result<String> {
        // For longest chain, choose the fork with the highest head block
        forks
            .iter()
            .max_by_key(|f| f.head_block)
            .map(|f| f.fork_id.clone())
            .ok_or_else(|| Error::generic("No forks to evaluate"))
    }

    async fn get_state(&self) -> Result<ForkChoiceState> {
        Ok(self.state.read().await.clone())
    }
}

/// LMD-GHOST fork choice implementation
pub struct LmdGhostForkChoice {
    /// Current state
    state: Arc<tokio::sync::RwLock<ForkChoiceState>>,
}

impl LmdGhostForkChoice {
    /// Create new LMD-GHOST fork choice
    pub fn new(initial_head: BlockInfo) -> Self {
        let mut parameters = HashMap::new();
        parameters.insert("algorithm".to_string(), serde_json::json!("lmd-ghost"));

        let state = ForkChoiceState {
            canonical_head: initial_head,
            forks: Vec::new(),
            rule: ForkChoiceRule::LmdGhost,
            parameters,
        };

        Self {
            state: Arc::new(tokio::sync::RwLock::new(state)),
        }
    }
}

#[async_trait]
impl ForkChoice for LmdGhostForkChoice {
    fn rule(&self) -> &ForkChoiceRule {
        &ForkChoiceRule::LmdGhost
    }

    async fn process_block(&self, block: BlockInfo) -> Result<ForkChoiceUpdate> {
        let mut state = self.state.write().await;

        // For LMD-GHOST, we need to consider the weight of each fork
        // In a simplified implementation, weight = number of attestations/votes

        if block.parent_hash == state.canonical_head.hash {
            // Direct extension of canonical chain
            let previous = state.canonical_head.clone();
            state.canonical_head = block.clone();

            Ok(ForkChoiceUpdate {
                head_changed: true,
                previous_head: Some(previous),
                new_head: block,
                reorg_depth: None,
                affected_forks: vec![],
            })
        } else {
            // Handle fork processing with GHOST algorithm
            let fork_id = format!("fork_{}", block.hash);
            let mut fork_found = false;
            let mut heaviest_fork_idx = None;
            let mut max_weight = 0u128;

            // Update existing fork or create new one
            for (idx, fork) in state.forks.iter_mut().enumerate() {
                if block.parent_hash == fork.head_block.to_string() {
                    fork.head_block = block.number;
                    fork.length += 1;
                    fork.weight += 1; // Simplified weight increment
                    fork_found = true;

                    if fork.weight > max_weight {
                        max_weight = fork.weight;
                        heaviest_fork_idx = Some(idx);
                    }
                    break;
                } else if fork.weight > max_weight {
                    max_weight = fork.weight;
                    heaviest_fork_idx = Some(idx);
                }
            }

            if !fork_found {
                // New fork - start with weight 1
                let fork = ForkInfo {
                    fork_id: fork_id.clone(),
                    fork_point: block.number - 1,
                    head_block: block.number,
                    weight: 1u128,
                    length: 1,
                    status: ForkStatus::Active,
                };

                state.forks.push(fork);
                let new_idx = state.forks.len() - 1;

                if max_weight == 0 {
                    heaviest_fork_idx = Some(new_idx);
                    max_weight = 1;
                }
            }

            // Check if the heaviest fork should become canonical
            let current_canonical_weight = state
                .forks
                .iter()
                .find(|f| f.status == ForkStatus::Canonical)
                .map(|f| f.weight)
                .unwrap_or(1);

            if let Some(idx) = heaviest_fork_idx {
                if max_weight > current_canonical_weight {
                    let previous = state.canonical_head.clone();
                    state.canonical_head = block.clone();

                    // Update fork statuses
                    for fork in state.forks.iter_mut() {
                        fork.status = ForkStatus::Active;
                    }
                    state.forks[idx].status = ForkStatus::Canonical;

                    let fork_length = state.forks[idx].length;
                    let fork_point = state.forks[idx].fork_point;

                    return Ok(ForkChoiceUpdate {
                        head_changed: true,
                        previous_head: Some(previous),
                        new_head: block,
                        reorg_depth: Some(fork_length - fork_point),
                        affected_forks: vec![state.forks[idx].fork_id.clone()],
                    });
                }
            }

            Ok(ForkChoiceUpdate {
                head_changed: false,
                previous_head: None,
                new_head: state.canonical_head.clone(),
                reorg_depth: None,
                affected_forks: vec![fork_id],
            })
        }
    }

    async fn get_canonical_head(&self) -> Result<BlockInfo> {
        Ok(self.state.read().await.canonical_head.clone())
    }

    async fn get_active_forks(&self) -> Result<Vec<ForkInfo>> {
        Ok(self.state.read().await.forks.clone())
    }

    async fn evaluate_forks(&self, forks: &[ForkInfo]) -> Result<String> {
        // For LMD-GHOST, choose the fork with the highest weight
        forks
            .iter()
            .max_by_key(|f| f.weight)
            .map(|f| f.fork_id.clone())
            .ok_or_else(|| Error::generic("No forks to evaluate"))
    }

    async fn get_state(&self) -> Result<ForkChoiceState> {
        Ok(self.state.read().await.clone())
    }
}

/// Heaviest chain fork choice implementation
pub struct HeaviestChainForkChoice {
    /// Current state
    state: Arc<tokio::sync::RwLock<ForkChoiceState>>,
}

impl HeaviestChainForkChoice {
    /// Create new heaviest chain fork choice
    pub fn new(initial_head: BlockInfo) -> Self {
        let mut parameters = HashMap::new();
        parameters.insert("algorithm".to_string(), serde_json::json!("heaviest-chain"));

        let state = ForkChoiceState {
            canonical_head: initial_head,
            forks: Vec::new(),
            rule: ForkChoiceRule::HeaviestChain,
            parameters,
        };

        Self {
            state: Arc::new(tokio::sync::RwLock::new(state)),
        }
    }
}

#[async_trait]
impl ForkChoice for HeaviestChainForkChoice {
    fn rule(&self) -> &ForkChoiceRule {
        &ForkChoiceRule::HeaviestChain
    }

    async fn process_block(&self, block: BlockInfo) -> Result<ForkChoiceUpdate> {
        let mut state = self.state.write().await;

        // Similar to LongestChain but uses cumulative weight instead of length
        if block.parent_hash == state.canonical_head.hash {
            let previous = state.canonical_head.clone();
            state.canonical_head = block.clone();

            Ok(ForkChoiceUpdate {
                head_changed: true,
                previous_head: Some(previous),
                new_head: block,
                reorg_depth: None,
                affected_forks: vec![],
            })
        } else {
            let fork_id = format!("fork_{}", block.hash);
            let mut fork_found = false;
            let mut heaviest_weight = state.canonical_head.number as u128; // Use block number as weight proxy
            let mut heaviest_fork_idx = None;

            for (idx, fork) in state.forks.iter_mut().enumerate() {
                if block.parent_hash == fork.head_block.to_string() {
                    fork.head_block = block.number;
                    fork.weight += block.number as u128; // Add block number as weight
                    fork.length += 1;
                    fork_found = true;

                    if fork.weight > heaviest_weight {
                        heaviest_weight = fork.weight;
                        heaviest_fork_idx = Some(idx);
                    }
                }
            }

            if !fork_found {
                let fork = ForkInfo {
                    fork_id: fork_id.clone(),
                    fork_point: block.number - 1,
                    head_block: block.number,
                    weight: block.number as u128,
                    length: 1,
                    status: ForkStatus::Active,
                };
                state.forks.push(fork);
            }

            // Check for reorg based on weight
            if let Some(idx) = heaviest_fork_idx {
                let previous = state.canonical_head.clone();
                state.canonical_head = block.clone();
                state.forks[idx].status = ForkStatus::Canonical;

                return Ok(ForkChoiceUpdate {
                    head_changed: true,
                    previous_head: Some(previous),
                    new_head: block,
                    reorg_depth: Some(state.forks[idx].length - state.forks[idx].fork_point),
                    affected_forks: vec![state.forks[idx].fork_id.clone()],
                });
            }

            Ok(ForkChoiceUpdate {
                head_changed: false,
                previous_head: None,
                new_head: state.canonical_head.clone(),
                reorg_depth: None,
                affected_forks: vec![fork_id],
            })
        }
    }

    async fn get_canonical_head(&self) -> Result<BlockInfo> {
        Ok(self.state.read().await.canonical_head.clone())
    }

    async fn get_active_forks(&self) -> Result<Vec<ForkInfo>> {
        Ok(self.state.read().await.forks.clone())
    }

    async fn evaluate_forks(&self, forks: &[ForkInfo]) -> Result<String> {
        forks
            .iter()
            .max_by_key(|f| f.weight)
            .map(|f| f.fork_id.clone())
            .ok_or_else(|| Error::generic("No forks to evaluate"))
    }

    async fn get_state(&self) -> Result<ForkChoiceState> {
        Ok(self.state.read().await.clone())
    }
}

/// Factory for creating fork choice implementations
pub fn create_fork_choice(rule: ForkChoiceRule, initial_head: BlockInfo) -> Box<dyn ForkChoice> {
    match rule {
        ForkChoiceRule::LongestChain => Box::new(LongestChainForkChoice::new(initial_head)),
        ForkChoiceRule::HeaviestChain => Box::new(HeaviestChainForkChoice::new(initial_head)),
        ForkChoiceRule::LmdGhost => Box::new(LmdGhostForkChoice::new(initial_head)),
        ForkChoiceRule::Ghost => Box::new(LmdGhostForkChoice::new(initial_head)), // Use LMD-GHOST for GHOST
        // For unimplemented rules, fall back to LongestChain with a warning
        _ => {
            eprintln!(
                "Warning: Fork choice rule {:?} not implemented, falling back to LongestChain",
                rule
            );
            Box::new(LongestChainForkChoice::new(initial_head))
        }
    }
}
