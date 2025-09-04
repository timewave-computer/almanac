//! Utility modules for E2E testing

pub mod fixtures;
pub mod nix_env;
pub mod setup;
pub mod validation;

pub use nix_env::{NixEnvironment, OperatingSystem};

// Re-export commonly used utilities
pub use fixtures::*;
pub use setup::*;
pub use validation::*;