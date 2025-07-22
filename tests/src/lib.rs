//! Almanac test suite
//!
//! This crate contains all tests for the Almanac project organized by type:
//! - Unit tests: Test individual components in isolation
//! - Integration tests: Test multiple components working together
//! - E2E tests: Full end-to-end tests with real infrastructure

pub mod common;
pub mod unit;
pub mod integration;
pub mod e2e;
pub mod utils;
pub mod types;
pub mod simple_test;

// Re-export common test types
pub use types::{E2EError, TestConfig, TestOutcome, TestSuiteResults};