#!/usr/bin/env bash
set -euo pipefail

echo "Setting up wasmd v0.31.0 development environment..."

# Set up GOPATH and bin directory
export GOPATH="$HOME/go"
export PATH="$GOPATH/bin:$PATH"
mkdir -p "$GOPATH/bin"

# Find the wasmd-dummy script from the nix store
WASMD_DUMMY=$(command -v wasmd-dummy || echo "")

if [ -z "$WASMD_DUMMY" ]; then
  echo "Error: wasmd-dummy script not found in PATH"
  exit 1
fi

# Install our custom dummy wasmd script
cp "$WASMD_DUMMY" "$GOPATH/bin/wasmd"
chmod +x "$GOPATH/bin/wasmd"

# Check installation
if [ -f "$GOPATH/bin/wasmd" ]; then
  echo "✓ wasmd installed successfully at $GOPATH/bin/wasmd"
  echo ""
  echo "Testing wasmd installation..."
  "$GOPATH/bin/wasmd" version
  echo ""
  echo "Note: This is a simulated wasmd executable for testing purposes."
  echo "To use wasmd, use the wasmd-node command which sets up the test environment."
else
  echo "✗ Failed to install wasmd"
  exit 1
fi 