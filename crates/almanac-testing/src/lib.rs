//! Testing utilities for Almanac
//! 
//! This crate provides comprehensive testing utilities for the Almanac blockchain indexing system,
//! including fixtures for test data generation, helpers for common testing patterns, and mock
//! implementations for testing isolated components.

pub mod fixtures;
pub mod helpers;
pub mod mocks;

// Re-exports for convenience
pub use fixtures::*;
pub use helpers::*;
pub use mocks::*;
