//! Consensus, finality, and reorg handling for Almanac
//!
//! This crate provides:
//! - Chain reorganization detection and handling
//! - Finality tracking and confirmation
//! - Fork choice rules
//! - Consensus state management

pub mod reorg;
pub mod finality;
pub mod fork_choice;
pub mod types;

// Re-exports
pub use reorg::{ReorgHandler, ReorgDetector, Reorganization, ReorgType};
pub use finality::{FinalityProcessor, FinalityConfig, FinalityLevel};
pub use fork_choice::{ForkChoiceRule, ForkChoiceState};
pub use types::{BlockInfo, ConsensusState};