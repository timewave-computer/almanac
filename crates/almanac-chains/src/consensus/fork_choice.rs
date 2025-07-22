//! Fork choice rules and state management

use std::collections::HashMap;
use std::sync::Arc;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::consensus::types::{BlockInfo, ForkInfo, ForkStatus};

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
    async fn process_block(&self, block: BlockInfo) -> Result<ForkChoiceUpdate, String>;
    
    /// Get the current canonical head
    async fn get_canonical_head(&self) -> Result<BlockInfo, String>;
    
    /// Get all active forks
    async fn get_active_forks(&self) -> Result<Vec<ForkInfo>, String>;
    
    /// Evaluate which fork should be canonical
    async fn evaluate_forks(&self, forks: &[ForkInfo]) -> Result<String, String>;
    
    /// Get fork choice state
    async fn get_state(&self) -> Result<ForkChoiceState, String>;
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
    
    async fn process_block(&self, block: BlockInfo) -> Result<ForkChoiceUpdate, String> {
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
            for fork in &mut state.forks {
                if block.parent_hash == fork.head_block.to_string() {
                    fork.head_block = block.number;
                    fork.length += 1;
                    fork_found = true;
                    
                    // Check if this fork is now longer than canonical
                    if fork.length > state.canonical_head.number {
                        // Fork is now canonical
                        let previous = state.canonical_head.clone();
                        state.canonical_head = block.clone();
                        fork.status = ForkStatus::Canonical;
                        
                        return Ok(ForkChoiceUpdate {
                            head_changed: true,
                            previous_head: Some(previous),
                            new_head: block,
                            reorg_depth: Some(fork.length - fork.fork_point),
                            affected_forks: vec![fork.fork_id.clone()],
                        });
                    }
                    break;
                }
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
    
    async fn get_canonical_head(&self) -> Result<BlockInfo, String> {
        Ok(self.state.read().await.canonical_head.clone())
    }
    
    async fn get_active_forks(&self) -> Result<Vec<ForkInfo>, String> {
        Ok(self.state.read().await.forks.clone())
    }
    
    async fn evaluate_forks(&self, forks: &[ForkInfo]) -> Result<String, String> {
        // For longest chain, choose the fork with the highest head block
        forks.iter()
            .max_by_key(|f| f.head_block)
            .map(|f| f.fork_id.clone())
            .ok_or_else(|| "No forks to evaluate".to_string())
    }
    
    async fn get_state(&self) -> Result<ForkChoiceState, String> {
        Ok(self.state.read().await.clone())
    }
}

/// Factory for creating fork choice implementations
pub fn create_fork_choice(rule: ForkChoiceRule, initial_head: BlockInfo) -> Box<dyn ForkChoice> {
    match rule {
        ForkChoiceRule::LongestChain => Box::new(LongestChainForkChoice::new(initial_head)),
        // Use longest chain as fallback for all other rules for now
        _ => Box::new(LongestChainForkChoice::new(initial_head)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::SystemTime;
    
    fn create_test_block(number: u64, hash: &str, parent_hash: &str) -> BlockInfo {
        BlockInfo {
            number,
            hash: hash.to_string(),
            parent_hash: parent_hash.to_string(),
            timestamp: SystemTime::now(),
            confirmations: 0,
        }
    }
    
    #[tokio::test]
    async fn test_longest_chain_fork_choice() {
        let genesis = create_test_block(0, "genesis", "");
        let fork_choice = LongestChainForkChoice::new(genesis.clone());
        
        // Test getting canonical head
        let head = fork_choice.get_canonical_head().await.unwrap();
        assert_eq!(head.number, 0);
        assert_eq!(head.hash, "genesis");
        
        // Test processing a block that extends the chain
        let block1 = create_test_block(1, "block1", "genesis");
        let update = fork_choice.process_block(block1.clone()).await.unwrap();
        
        assert!(update.head_changed);
        assert_eq!(update.new_head.number, 1);
        assert_eq!(update.previous_head.unwrap().number, 0);
        
        // Test getting updated canonical head
        let new_head = fork_choice.get_canonical_head().await.unwrap();
        assert_eq!(new_head.number, 1);
        assert_eq!(new_head.hash, "block1");
    }
    
    #[tokio::test]
    async fn test_fork_creation() {
        let genesis = create_test_block(0, "genesis", "");
        let fork_choice = LongestChainForkChoice::new(genesis);
        
        // Add first block
        let block1 = create_test_block(1, "block1", "genesis");
        fork_choice.process_block(block1).await.unwrap();
        
        // Create a fork from genesis (shorter chain)
        let fork_block = create_test_block(1, "fork1", "genesis");
        let update = fork_choice.process_block(fork_block).await.unwrap();
        
        // Should not change head since fork is not longer
        assert!(!update.head_changed);
        assert_eq!(update.new_head.hash, "block1");
        
        // Check that fork was recorded
        let forks = fork_choice.get_active_forks().await.unwrap();
        assert_eq!(forks.len(), 1);
        assert_eq!(forks[0].fork_id, "fork_fork1");
    }
    
    #[test]
    fn test_fork_choice_factory() {
        let genesis = create_test_block(0, "genesis", "");
        
        // Test longest chain
        let longest_chain = create_fork_choice(ForkChoiceRule::LongestChain, genesis.clone());
        assert_eq!(*longest_chain.rule(), ForkChoiceRule::LongestChain);
        
        // Test fallback for unimplemented rules
        let ghost = create_fork_choice(ForkChoiceRule::Ghost, genesis.clone());
        assert_eq!(*ghost.rule(), ForkChoiceRule::LongestChain); // Should fallback
    }
}