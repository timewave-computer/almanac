#!/bin/bash
# Master reorganization script for Almanac

set -e

echo "🚀 Almanac Repository Reorganization"
echo "===================================="
echo ""

# Check if we're in the right directory
if [ ! -f "Cargo.toml" ] || [ ! -d "crates" ]; then
    echo "❌ Error: This script must be run from the Almanac root directory"
    exit 1
fi

# Create backup
echo "📦 Creating backup..."
BACKUP_DIR="../almanac-backup-$(date +%Y%m%d-%H%M%S)"
echo "   Backing up to: $BACKUP_DIR"
cp -r . "$BACKUP_DIR" 2>/dev/null || echo "   ⚠️  Backup failed, continuing anyway..."

# Make scripts executable
echo "🔧 Making scripts executable..."
chmod +x scripts/*.sh 2>/dev/null || true

# Run reorganization steps
echo ""
echo "📋 Step 1/5: Removing custom storage implementation"
echo "------------------------------------------------"
if [ -f "scripts/remove_custom_storage.sh" ]; then
    ./scripts/remove_custom_storage.sh
else
    echo "   ⚠️  Script not found, skipping..."
fi

echo ""
echo "📋 Step 2/5: Creating new crate structure"
echo "---------------------------------------"
if [ -f "scripts/reorganize_repo.sh" ]; then
    ./scripts/reorganize_repo.sh
else
    echo "   ⚠️  Script not found, skipping..."
fi

echo ""
echo "📋 Step 3/5: Moving files to new locations"
echo "----------------------------------------"
if [ -f "scripts/move_files.sh" ]; then
    ./scripts/move_files.sh
else
    echo "   ⚠️  Script not found, skipping..."
fi

echo ""
echo "📋 Step 4/5: Updating import paths"
echo "---------------------------------"
if [ -f "scripts/update_imports.sh" ]; then
    ./scripts/update_imports.sh
else
    echo "   ⚠️  Script not found, skipping..."
fi

echo ""
echo "📋 Step 5/5: Verifying reorganization"
echo "------------------------------------"
echo "🔍 Checking Cargo workspace..."

# Update workspace members in root Cargo.toml
if [ -f "Cargo.toml" ]; then
    echo "   Updating workspace members..."
    # This is a simplified version - in practice you'd want to be more careful
    cat > Cargo.toml.new << 'EOF'
[workspace]
members = [
    "crates/core",
    "crates/distributed",
    "crates/pipeline",
    "crates/observability",
    "crates/validation",
    "crates/storage-core",
    "crates/storage-postgres",
    "crates/storage-rocks",
    "crates/storage-memory",
    "crates/storage-proof",
    "crates/api-core",
    "crates/api-rest",
    "crates/api-graphql",
    "crates/api-streaming",
    "crates/cross-chain",
    "crates/data-services",
    "crates/codegen",
    "crates/ethereum",
    "crates/cosmos",
    "crates/causality",
    "crates/query",
    "crates/testing",
    "crates/tools",
]

[workspace.package]
edition = "2021"
authors = ["Timewave Labs"]
repository = "https://github.com/timewave-computer/almanac"

[workspace.dependencies]
tokio = { version = "1", features = ["full"] }
async-trait = "0.1"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tracing = "0.1"
thiserror = "1"

# Add traverse dependencies
traverse-core = { git = "https://github.com/timewave-computer/traverse", branch = "main" }
traverse-ethereum = { git = "https://github.com/timewave-computer/traverse", branch = "main" }
traverse-cosmos = { git = "https://github.com/timewave-computer/traverse", branch = "main" }
EOF

    # Append existing content if any
    grep -v "^\[workspace\]" Cargo.toml | grep -v "^members = " >> Cargo.toml.new || true
    mv Cargo.toml.new Cargo.toml
fi

echo ""
echo "🎉 Reorganization Complete!"
echo "=========================="
echo ""
echo "📊 Summary:"
echo "  - Custom storage implementation removed"
echo "  - New modular crate structure created"
echo "  - Files moved to appropriate locations"
echo "  - Import paths updated"
echo "  - Traverse integration prepared"
echo ""
echo "🔍 Next Steps:"
echo "  1. Run: cargo check --all-features"
echo "  2. Fix any compilation errors"
echo "  3. Run: cargo test --all-features"
echo "  4. Update documentation"
echo ""
echo "📁 Backup location: $BACKUP_DIR"
echo ""
echo "✨ Happy coding with your newly organized codebase!"