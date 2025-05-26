#!/bin/bash
# Purpose: Compile and run a standalone RocksDB test without any PostgreSQL dependencies

set -e

# Color definitions
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[0;33m'
NC='\033[0m' # No Color

echo -e "${GREEN}=== Running Standalone RocksDB Tests ===${NC}"

# Create temporary directories
TEMP_DIR="${PWD}/tmp/standalone-rocks-test"
BUILD_DIR="${PWD}/tmp/standalone-rocks-build"
mkdir -p "$TEMP_DIR"
mkdir -p "$BUILD_DIR/src"

echo "Setting up test environment..."
echo "Using temp directory: $TEMP_DIR"

# Create a minimal Cargo.toml for just the test
cat > "$BUILD_DIR/Cargo.toml" << EOF
[package]
name = "rocks-standalone-test"
version = "0.1.0"
edition = "2021"

[workspace]

[dependencies]
rocksdb = "0.21.0"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
tempfile = "3.8.0"
EOF

# Copy the test script to the build directory
cp scripts/test-rocks-only.rs "$BUILD_DIR/src/main.rs"

echo "Building standalone RocksDB test..."

# Run in a separate directory to avoid workspace conflicts
cd "$BUILD_DIR"

# Build and run the test inside the Nix environment
nix develop --command bash -c "cd $BUILD_DIR && \
  RUST_BACKTRACE=1 \
  cargo build --quiet && \
  ./target/debug/rocks-standalone-test"

EXIT_CODE=$?

# Return to the original directory
cd "$OLDPWD"

# Check if tests passed or failed
if [ $EXIT_CODE -eq 0 ]; then
  echo -e "${GREEN}✅ Standalone RocksDB tests passed!${NC}"
else
  echo -e "${RED}❌ Standalone RocksDB tests failed with exit code $EXIT_CODE${NC}"
fi

# Clean up
echo -e "${YELLOW}Cleaning up temporary directories...${NC}"
rm -rf "$TEMP_DIR" "$BUILD_DIR"

exit $EXIT_CODE 