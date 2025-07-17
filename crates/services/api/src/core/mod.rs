// Core module for base service functionality
//
// This module provides core service traits, base implementations,
// and subscription management functionality

pub mod service_base;
pub mod subscription;

// Re-export core functionality
pub use service_base::*;
pub use subscription::*; 