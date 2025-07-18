//! Consensus module for blockchain finality, fork choice, and reorganization handling

pub mod finality;
pub mod fork_choice;
pub mod reorg;
pub mod types;

// Re-exports to match the original almanac-consensus structure
pub use finality::{FinalityProcessor, FinalityConfig, FinalityLevel};
pub use fork_choice::{ForkChoiceRule, ForkChoiceState};
pub use reorg::{ReorgHandler, ReorgDetector, Reorganization, ReorgType};
pub use types::{BlockInfo, ConsensusState};

// Also re-export the common types
pub use types::*;