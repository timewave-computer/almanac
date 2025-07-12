#!/bin/bash
# Repository reorganization script for Almanac

set -e

echo "🔧 Starting Almanac repository reorganization..."

# Create backup
echo "📦 Creating backup..."
cp -r . ../almanac-backup-$(date +%Y%m%d-%H%M%S) 2>/dev/null || true

# Phase 1: Create new crate directories
echo "📁 Creating new crate structure..."

# Distributed computing module
mkdir -p crates/distributed/src
mkdir -p crates/distributed/tests

# Codegen module
mkdir -p crates/codegen/src
mkdir -p crates/codegen/templates

# Pipeline module
mkdir -p crates/pipeline/src

# Observability module
mkdir -p crates/observability/src

# Validation module
mkdir -p crates/validation/src

# Storage modules
mkdir -p crates/storage-core/src
mkdir -p crates/storage-postgres/src
mkdir -p crates/storage-rocks/src
mkdir -p crates/storage-memory/src

# API modules
mkdir -p crates/api-rest/src
mkdir -p crates/api-graphql/src
mkdir -p crates/api-streaming/src
mkdir -p crates/api-core/src

# Feature modules
mkdir -p crates/cross-chain/src
mkdir -p crates/data-services/src

# Phase 2: Create Cargo.toml files
echo "📝 Creating Cargo.toml files..."

# Distributed crate
cat > crates/distributed/Cargo.toml << 'EOF'
[package]
name = "indexer-distributed"
version = "0.1.0"
edition = "2021"

[dependencies]
indexer-core = { path = "../core" }
indexer-storage-core = { path = "../storage-core" }
async-trait = "0.1"
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tracing = "0.1"
foundationdb = { version = "0.9", optional = true }

[features]
default = ["fdb"]
fdb = ["foundationdb"]
EOF

# Pipeline crate
cat > crates/pipeline/Cargo.toml << 'EOF'
[package]
name = "indexer-pipeline"
version = "0.1.0"
edition = "2021"

[dependencies]
indexer-core = { path = "../core" }
async-trait = "0.1"
tokio = { version = "1", features = ["full"] }
futures = "0.3"
tracing = "0.1"
EOF

# Observability crate
cat > crates/observability/Cargo.toml << 'EOF'
[package]
name = "indexer-observability"
version = "0.1.0"
edition = "2021"

[dependencies]
indexer-core = { path = "../core" }
prometheus = "0.13"
tracing = "0.1"
tracing-subscriber = "0.3"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tokio = { version = "1", features = ["full"] }
EOF

# Storage-core crate
cat > crates/storage-core/Cargo.toml << 'EOF'
[package]
name = "indexer-storage-core"
version = "0.1.0"
edition = "2021"

[dependencies]
indexer-core = { path = "../core" }
async-trait = "0.1"
serde = { version = "1", features = ["derive"] }
EOF

# Create lib.rs files to re-export modules
echo "🔄 Creating module re-exports..."

# Distributed lib.rs
cat > crates/distributed/src/lib.rs << 'EOF'
//! Distributed computing functionality for Almanac

pub mod cache;
pub mod coordination;
pub mod correlation;
pub mod event_service;
pub mod gateway;
pub mod monitoring;
pub mod query_api;
pub mod query_planner;

#[cfg(feature = "fdb")]
pub mod fdb;

pub use self::coordination::*;
pub use self::query_planner::*;
EOF

# Pipeline lib.rs
cat > crates/pipeline/src/lib.rs << 'EOF'
//! Data processing pipeline for Almanac

pub mod batch_processor;
pub mod event_ordering;
pub mod finality_processor;
pub mod ingestion_queue;
pub mod pipeline;
pub mod reorg_handler;
pub mod sync_tracker;

pub use pipeline::*;
EOF

echo "📋 Reorganization structure created!"
echo ""
echo "Next steps:"
echo "1. Run: ./scripts/move_files.sh to move files to new locations"
echo "2. Run: ./scripts/update_imports.sh to update import paths"
echo "3. Run: cargo check to verify compilation"
echo "4. Run: cargo test to ensure tests pass"
echo ""
echo "✅ Repository reorganization structure ready!"