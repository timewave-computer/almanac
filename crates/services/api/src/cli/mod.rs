// CLI module for command-line interface functionality
//
// This module provides debug commands and development utilities

pub mod debug_commands;

// Re-export commonly used CLI functionality
pub use debug_commands::{DebugInterface, DebugCommand, DebugResponse}; 