//! Trust management types for FDB storage

use serde::{Deserialize, Serialize};

// Trust scope and other types now imported from api_adapter

/// Trust policy for filtering
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustPolicy {
    pub default_behavior: TrustBehavior,
    pub explicit_trusts: Vec<String>,
    pub minimum_level: TrustLevel,
}

impl TrustPolicy {
    /// Create a strict policy that only accepts explicitly trusted developers
    pub fn strict(trusted_developers: Vec<String>) -> Self {
        Self {
            default_behavior: TrustBehavior::Reject,
            explicit_trusts: trusted_developers,
            minimum_level: TrustLevel::ReadOnly,
        }
    }

    /// Create a permissive policy that accepts all developers
    pub fn permissive() -> Self {
        Self {
            default_behavior: TrustBehavior::Accept,
            explicit_trusts: vec![],
            minimum_level: TrustLevel::None,
        }
    }

    /// Create an explicit policy for specific developers
    pub fn explicit(developers: Vec<String>) -> Self {
        Self {
            default_behavior: TrustBehavior::RequireVerification,
            explicit_trusts: developers,
            minimum_level: TrustLevel::ReadOnly,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum TrustBehavior {
    Accept,
    Reject,
    RequireVerification,
}

// Use trust types from api_adapter module (enum version)
pub use crate::api_adapter::TrustPolicy as EnumTrustPolicy;
pub use crate::api_adapter::{DeveloperTrustValue, TrustLevel, TrustScope};
