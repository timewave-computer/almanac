//! Consensus, finality, and reorg handling for Almanac
//!
//! This crate provides:
//! - Chain reorganization detection and handling
//! - Finality tracking and confirmation
//! - Fork choice rules
//! - Consensus state management

pub mod finality;
pub mod fork_choice;
pub mod reorg;
pub mod types;

// Re-exports
pub use finality::{FinalityConfig, FinalityLevel, FinalityProcessor};
pub use fork_choice::{ForkChoiceRule, ForkChoiceState};
pub use reorg::{ReorgDetector, ReorgHandler, ReorgType, Reorganization};
pub use types::{BlockInfo, ConsensusState};
