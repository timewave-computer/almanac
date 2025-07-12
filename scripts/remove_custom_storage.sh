#!/bin/bash
# Script to remove custom storage implementation and prepare for traverse integration

set -e

echo "🧹 Removing custom storage implementation..."

# Remove custom storage proof implementation files
echo "📁 Removing custom storage proof files..."
rm -f crates/api/src/storage_proof_api.rs
rm -f crates/core/src/semantic_validation.rs
rm -rf crates/storage/src/tests/rocks_benchmark.rs.disabled
rm -rf crates/storage/src/tests/sync_benchmark.rs.disabled

# Remove RocksDB FFI wrapper
echo "🔧 Removing custom RocksDB FFI..."
rm -rf rocksdb_ffi/
rm -rf nix/librocksdb-sys-wrapper.nix
rm -rf nix/rocksdb.nix
rm -rf nix/test-rocksdb.nix

# Update Cargo.toml files to use traverse
echo "📦 Updating dependencies for traverse..."

# Update root Cargo.toml
if [ -f "Cargo.toml" ]; then
    cat >> Cargo.toml << 'EOF'

# Traverse dependencies
[dependencies.traverse-core]
git = "https://github.com/timewave-computer/traverse"
branch = "main"

[dependencies.traverse-ethereum]
git = "https://github.com/timewave-computer/traverse"
branch = "main"
optional = true

[dependencies.traverse-cosmos]
git = "https://github.com/timewave-computer/traverse"
branch = "main"
optional = true
EOF
fi

# Create new storage proof module using traverse
echo "✨ Creating new storage proof module with traverse..."
mkdir -p crates/storage-proof/src

cat > crates/storage-proof/Cargo.toml << 'EOF'
[package]
name = "indexer-storage-proof"
version = "0.1.0"
edition = "2021"

[dependencies]
indexer-core = { path = "../core" }
traverse-core = { git = "https://github.com/timewave-computer/traverse", branch = "main" }
traverse-ethereum = { git = "https://github.com/timewave-computer/traverse", branch = "main", optional = true }
traverse-cosmos = { git = "https://github.com/timewave-computer/traverse", branch = "main", optional = true }
async-trait = "0.1"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tokio = { version = "1", features = ["full"] }
tracing = "0.1"

[features]
default = ["ethereum", "cosmos"]
ethereum = ["traverse-ethereum"]
cosmos = ["traverse-cosmos"]
EOF

cat > crates/storage-proof/src/lib.rs << 'EOF'
//! Storage proof generation and verification using traverse

use indexer_core::{Result, Error};
use traverse_core::{ProofGenerator, ProofVerifier, StorageProof};
use async_trait::async_trait;
use serde::{Serialize, Deserialize};

#[cfg(feature = "ethereum")]
use traverse_ethereum::EthereumProofGenerator;

#[cfg(feature = "cosmos")]
use traverse_cosmos::CosmosProofGenerator;

/// Storage proof service using traverse
pub struct StorageProofService {
    #[cfg(feature = "ethereum")]
    ethereum_generator: EthereumProofGenerator,
    
    #[cfg(feature = "cosmos")]
    cosmos_generator: CosmosProofGenerator,
}

impl StorageProofService {
    pub fn new() -> Result<Self> {
        Ok(Self {
            #[cfg(feature = "ethereum")]
            ethereum_generator: EthereumProofGenerator::new()?,
            
            #[cfg(feature = "cosmos")]
            cosmos_generator: CosmosProofGenerator::new()?,
        })
    }
    
    /// Generate a storage proof for the given parameters
    pub async fn generate_proof(
        &self,
        chain: &str,
        contract_address: &str,
        storage_key: Vec<u8>,
        block_number: u64,
    ) -> Result<StorageProof> {
        match chain {
            #[cfg(feature = "ethereum")]
            "ethereum" => {
                self.ethereum_generator
                    .generate_proof(contract_address, storage_key, block_number)
                    .await
                    .map_err(|e| Error::generic(format!("Ethereum proof generation failed: {}", e)))
            }
            
            #[cfg(feature = "cosmos")]
            "cosmos" => {
                self.cosmos_generator
                    .generate_proof(contract_address, storage_key, block_number)
                    .await
                    .map_err(|e| Error::generic(format!("Cosmos proof generation failed: {}", e)))
            }
            
            _ => Err(Error::generic(format!("Unsupported chain: {}", chain))),
        }
    }
    
    /// Verify a storage proof
    pub async fn verify_proof(&self, proof: &StorageProof) -> Result<bool> {
        // Use traverse's verification
        proof.verify()
            .map_err(|e| Error::generic(format!("Proof verification failed: {}", e)))
    }
}

/// API request/response types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerateProofRequest {
    pub chain_id: String,
    pub contract_address: String,
    pub storage_key: Vec<u8>,
    pub block_number: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofResponse {
    pub proof_id: String,
    pub proof: StorageProof,
    pub metadata: ProofMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofMetadata {
    pub generated_at: u64,
    pub chain: String,
    pub contract: String,
    pub block: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_proof_service_creation() {
        let service = StorageProofService::new();
        assert!(service.is_ok());
    }
}
EOF

# Update API to use new storage proof module
echo "🔄 Creating updated API endpoints for traverse..."
cat > crates/api-core/src/storage_proof.rs << 'EOF'
//! Storage proof API endpoints using traverse

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
};
use indexer_storage_proof::{
    StorageProofService, GenerateProofRequest, ProofResponse, ProofMetadata
};
use serde_json::json;
use std::sync::Arc;
use uuid::Uuid;

/// Generate a storage proof
pub async fn generate_proof(
    State(proof_service): State<Arc<StorageProofService>>,
    Json(request): Json<GenerateProofRequest>,
) -> Result<Json<ProofResponse>, StatusCode> {
    let proof = proof_service
        .generate_proof(
            &request.chain_id,
            &request.contract_address,
            request.storage_key,
            request.block_number,
        )
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    let response = ProofResponse {
        proof_id: Uuid::new_v4().to_string(),
        proof,
        metadata: ProofMetadata {
            generated_at: chrono::Utc::now().timestamp() as u64,
            chain: request.chain_id,
            contract: request.contract_address,
            block: request.block_number,
        },
    };
    
    Ok(Json(response))
}

/// Verify a storage proof
pub async fn verify_proof(
    State(proof_service): State<Arc<StorageProofService>>,
    Json(proof): Json<traverse_core::StorageProof>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let is_valid = proof_service
        .verify_proof(&proof)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    Ok(Json(json!({
        "valid": is_valid,
        "timestamp": chrono::Utc::now().timestamp(),
    })))
}
EOF

# Remove old storage implementations from core lib.rs
echo "📝 Updating core lib.rs..."
sed -i '' '/\/\/ Storage proof verification - Alternative implementation/d' crates/core/src/lib.rs
sed -i '' '/pub mod semantic_validation;/d' crates/core/src/lib.rs

# Update feature flags
echo "🏷️ Updating feature flags..."
find . -name "Cargo.toml" -type f -exec sed -i '' 's/fdb = \["foundationdb"\]/fdb = ["foundationdb"]\ntraverse = ["indexer-storage-proof"]/g' {} \;

echo "✅ Custom storage implementation removed!"
echo ""
echo "🔗 Traverse integration prepared at:"
echo "  - crates/storage-proof/ - Storage proof service using traverse"
echo "  - crates/api-core/src/storage_proof.rs - API endpoints"
echo ""
echo "Next steps:"
echo "1. Run: chmod +x scripts/*.sh"
echo "2. Run: ./scripts/reorganize_repo.sh"
echo "3. Run: ./scripts/move_files.sh"
echo "4. Run: ./scripts/update_imports.sh"
echo "5. Run: cargo check --all-features"
echo "6. Fix any compilation errors"
echo "7. Run: cargo test"