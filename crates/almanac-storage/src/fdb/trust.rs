//! Trust management types for FDB storage

use serde::{Serialize, Deserialize};

/// Trust scope for developer relationships
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TrustScope {
    Global,
    Contract(String),
    Field(String, String),
}

/// Trust level
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum TrustLevel {
    None = 0,
    ReadOnly = 1,
    Contribute = 2,
    Moderate = 3,
}

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

/// Developer trust value
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeveloperTrustValue {
    pub trust_level: TrustLevel,
    pub granted_at: u64,
    pub expires_at: Option<u64>,
    pub conditions: Vec<String>,
    pub revocation_reason: Option<String>,
}

impl DeveloperTrustValue {
    pub fn is_valid(&self) -> bool {
        if let Some(expires_at) = self.expires_at {
            chrono::Utc::now().timestamp() as u64 <= expires_at
        } else {
            true
        }
    }
}