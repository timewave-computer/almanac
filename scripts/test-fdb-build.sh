#!/usr/bin/env bash
# Test FoundationDB build in isolation

set -euo pipefail

echo "Testing FoundationDB build..."

# Get project root
PROJECT_ROOT="${PROJECT_ROOT:-$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)}"
cd "$PROJECT_ROOT"

# Source FDB environment
source .env.fdb

# Create temporary test project
TEMP_DIR=$(mktemp -d)
cd "$TEMP_DIR"

echo "Creating test project in $TEMP_DIR"

# Create Cargo.toml
cat > Cargo.toml << 'EOF'
[package]
name = "fdb-test"
version = "0.1.0"
edition = "2021"

[dependencies]
foundationdb = "0.9"
foundationdb-gen = "0.9"
tokio = { version = "1", features = ["full"] }
anyhow = "1"
tracing = "0.1"
tracing-subscriber = "0.3"
EOF

# Create main.rs
mkdir -p src
cat > src/main.rs << 'EOF'
use foundationdb::{api::FdbApiBuilder, Database, RangeOption};
use foundationdb::tuple::{Tuple, Element};
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    
    println!("Initializing FoundationDB...");
    
    // Initialize the FDB API
    let api = FdbApiBuilder::default()
        .build()?;
    
    // Connect to database
    let db = Database::new(None)?;
    
    println!("Connected to FoundationDB!");
    
    // Test tuple operations
    let mut tuple = Tuple::new();
    tuple.push(Element::String("test".to_string()));
    tuple.push(Element::Int(42));
    
    let packed = tuple.pack();
    println!("Packed tuple: {:?}", packed);
    
    let unpacked = Tuple::from_bytes(&packed)?;
    println!("Unpacked tuple: {:?}", unpacked);
    
    // Test transaction
    let tx = db.create_trx()?;
    tx.set(b"test_key", b"test_value");
    tx.commit().await?;
    
    println!("Transaction committed successfully!");
    
    // Read back
    let tx = db.create_trx()?;
    if let Some(value) = tx.get(b"test_key", false).await? {
        println!("Read value: {:?}", std::str::from_utf8(&value)?);
    }
    
    println!("FoundationDB test completed successfully!");
    
    Ok(())
}
EOF

# Try to build
echo "Building test project..."
if cargo build 2>&1; then
    echo "Build successful!"
    
    # Check if FDB server is running
    if command -v fdbcli >/dev/null && fdbcli --exec "status" 2>/dev/null | grep -q "Cluster is available"; then
        echo "FoundationDB server is running, executing test..."
        cargo run
    else
        echo "FoundationDB server not running, skipping execution test"
    fi
else
    echo "Build failed. Check that FoundationDB headers are available."
    echo "FDB_INCLUDE_DIR=$FDB_INCLUDE_DIR"
    echo "CPATH=$CPATH"
fi

# Cleanup
cd ..
rm -rf "$TEMP_DIR"

echo "Test complete!"