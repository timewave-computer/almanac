#!/usr/bin/env bash
# Script to set up FoundationDB headers for compilation on macOS
# This works around the hardcoded path issue in foundationdb-gen crate

set -euo pipefail

echo "Setting up FoundationDB headers for compilation..."

# Determine project root
PROJECT_ROOT="${PROJECT_ROOT:-$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)}"
FDB_LOCAL_INCLUDE="$PROJECT_ROOT/.fdb_include"

# Create local include directory
mkdir -p "$FDB_LOCAL_INCLUDE/foundationdb"

# Try to find FoundationDB headers in various locations
FOUND_HEADERS=false

# Check Nix store (if in Nix environment)
if [ -n "${NIX_STORE:-}" ]; then
    echo "Checking Nix store for FoundationDB headers..."
    FDB_NIX_INCLUDE=$(find /nix/store -name "fdb_c.h" -type f 2>/dev/null | head -1 | xargs dirname 2>/dev/null || echo "")
    if [ -n "$FDB_NIX_INCLUDE" ] && [ -d "$FDB_NIX_INCLUDE" ]; then
        echo "Found headers in Nix store: $FDB_NIX_INCLUDE"
        cp -r "$FDB_NIX_INCLUDE"/* "$FDB_LOCAL_INCLUDE/foundationdb/" 2>/dev/null || true
        FOUND_HEADERS=true
    fi
fi

# Check Homebrew installation
if ! $FOUND_HEADERS && command -v brew >/dev/null 2>&1; then
    echo "Checking Homebrew for FoundationDB..."
    BREW_PREFIX=$(brew --prefix 2>/dev/null || echo "/opt/homebrew")
    if [ -f "$BREW_PREFIX/include/foundationdb/fdb_c.h" ]; then
        echo "Found headers in Homebrew: $BREW_PREFIX/include/foundationdb"
        cp -r "$BREW_PREFIX/include/foundationdb"/* "$FDB_LOCAL_INCLUDE/foundationdb/" 2>/dev/null || true
        FOUND_HEADERS=true
    fi
fi

# Check system locations
if ! $FOUND_HEADERS; then
    echo "Checking system locations..."
    for dir in /usr/local/include /usr/include /opt/local/include; do
        if [ -f "$dir/foundationdb/fdb_c.h" ]; then
            echo "Found headers in: $dir/foundationdb"
            cp -r "$dir/foundationdb"/* "$FDB_LOCAL_INCLUDE/foundationdb/" 2>/dev/null || true
            FOUND_HEADERS=true
            break
        fi
    done
fi

# Create stub fdb.options file if it doesn't exist
if [ ! -f "$FDB_LOCAL_INCLUDE/foundationdb/fdb.options" ]; then
    echo "Creating stub fdb.options file..."
    cat > "$FDB_LOCAL_INCLUDE/foundationdb/fdb.options" << 'EOF'
# FoundationDB Options File
# This is a stub file for compilation purposes
# The actual options are defined in the FoundationDB client library

# Network options
option "trace_enable" ""
option "trace_roll_size" "10000000"
option "trace_max_logs_size" "100000000"
option "trace_log_group" ""
option "knob" ""
option "TLS_plugin" ""
option "TLS_cert_bytes" ""
option "TLS_cert_path" ""
option "TLS_key_bytes" ""
option "TLS_key_path" ""
option "TLS_verify_peers" ""
option "buggify_enable" ""
option "buggify_disable" ""
option "buggify_section_activated_probability" "50"
option "buggify_section_fired_probability" "25"
option "disable_multi_version_client_api" ""
option "callbacks_on_external_threads" ""
option "external_client_library" ""
option "external_client_directory" ""
option "disable_local_client" ""
option "disable_client_statistics_logging" ""
option "enable_slow_task_profiling" ""

# Database options
option "location_cache_size" "100000"
option "max_watches" "10000"
option "machine_id" ""
option "datacenter_id" ""

# Transaction options
option "causal_write_risky" ""
option "causal_read_risky" ""
option "causal_read_disable" ""
option "next_write_no_write_conflict_range" ""
option "read_your_writes_disable" ""
option "read_ahead_disable" ""
option "durability_datacenter" ""
option "durability_risky" ""
option "durability_dev_null_is_web_scale" ""
option "priority_system_immediate" ""
option "priority_batch" ""
option "initialize_new_database" ""
option "access_system_keys" ""
option "read_system_keys" ""
option "debug_retry_logging" ""
option "transaction_logging_enable" ""
option "timeout" "0"
option "retry_limit" "0"
option "max_retry_delay" "0"
option "snapshot_ryw_enable" ""
option "snapshot_ryw_disable" ""
option "lock_aware" ""
option "used_during_commit_protection_disable" ""
option "read_lock_aware" ""
EOF
fi

# Also try to create symlinks in user-writable locations
if [ "$(uname)" = "Darwin" ]; then
    echo "Setting up macOS-specific symlinks..."
    
    # Try ~/.local/include (user-local)
    USER_LOCAL="$HOME/.local/include"
    mkdir -p "$USER_LOCAL"
    if [ -d "$FDB_LOCAL_INCLUDE/foundationdb" ]; then
        ln -sfn "$FDB_LOCAL_INCLUDE/foundationdb" "$USER_LOCAL/foundationdb" 2>/dev/null || true
    fi
    
    # Try /usr/local/include if writable
    if [ -w /usr/local/include ] 2>/dev/null; then
        ln -sfn "$FDB_LOCAL_INCLUDE/foundationdb" /usr/local/include/foundationdb 2>/dev/null || true
    fi
fi

# Set environment variables
echo "Setting environment variables..."
export FDB_INCLUDE_DIR="$FDB_LOCAL_INCLUDE"
export CPATH="$FDB_LOCAL_INCLUDE:${CPATH:-}"
export C_INCLUDE_PATH="$FDB_LOCAL_INCLUDE:${C_INCLUDE_PATH:-}"
export CPLUS_INCLUDE_PATH="$FDB_LOCAL_INCLUDE:${CPLUS_INCLUDE_PATH:-}"

# Create a .env file for cargo
cat > "$PROJECT_ROOT/.env.fdb" << EOF
# FoundationDB build environment
FDB_INCLUDE_DIR=$FDB_LOCAL_INCLUDE
CPATH=$FDB_LOCAL_INCLUDE:${CPATH:-}
C_INCLUDE_PATH=$FDB_LOCAL_INCLUDE:${C_INCLUDE_PATH:-}
CPLUS_INCLUDE_PATH=$FDB_LOCAL_INCLUDE:${CPLUS_INCLUDE_PATH:-}
EOF

echo "FoundationDB header setup complete!"
echo "Headers available at: $FDB_LOCAL_INCLUDE"
echo ""
echo "To build with FoundationDB support:"
echo "  source $PROJECT_ROOT/.env.fdb"
echo "  cargo build --features fdb"
echo ""

if ! $FOUND_HEADERS; then
    echo "WARNING: No FoundationDB headers found in standard locations."
    echo "The build will use the stub implementation."
    echo "To use real FoundationDB, install it via:"
    echo "  - Nix: nix-shell"
    echo "  - Homebrew: brew install foundationdb"
    echo "  - Or download from: https://www.foundationdb.org/download/"
fi