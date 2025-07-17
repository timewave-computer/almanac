// Gateway module for distributed system gateway functionality
//
// This module provides distributed gateway functionality for routing
// requests across distributed nodes and handling cross-node communication

pub mod distributed_gateway;

// Re-export gateway functionality
pub use distributed_gateway::*; 