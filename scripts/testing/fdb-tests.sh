#!/usr/bin/env bash
#
# Consolidated FoundationDB Testing Script
# Tests FDB build, integration, and E2E functionality
#

set -euo pipefail

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"
cd "$PROJECT_ROOT"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Default values
VERBOSE=false
DRY_RUN=false
BUILD_ONLY=false
E2E_ONLY=false

print_header() {
    echo -e "${BLUE}===================================================${NC}"
    echo -e "${BLUE}$1${NC}"
    echo -e "${BLUE}===================================================${NC}"
}

print_success() {
    echo -e "${GREEN}+ $1${NC}"
}

print_error() {
    echo -e "${RED}- $1${NC}"
}

print_warning() {
    echo -e "${YELLOW}! $1${NC}"
}

print_info() {
    echo -e "${BLUE}* $1${NC}"
}

show_help() {
    cat << EOF
Consolidated FoundationDB Testing Script

Usage: $0 [OPTIONS]

Options:
  --verbose              Enable verbose output
  --dry-run             Show what would be tested without running
  --build-only          Test only FDB build configuration
  --e2e-only            Test only FDB E2E functionality
  --help                Show this help message

Test Types:
  1. Build Tests - FDB headers, compilation, linking
  2. Integration Tests - FDB connectivity, basic operations
  3. E2E Tests - Full FDB functionality with almanac-storage

Examples:
  $0                     # Run all FDB tests
  $0 --build-only        # Test only build configuration
  $0 --e2e-only --verbose # Run E2E tests with verbose output
EOF
}

# Parse command line arguments
while [[ $# -gt 0 ]]; do
  case $1 in
    --verbose)
      VERBOSE=true
      shift
      ;;
    --dry-run)
      DRY_RUN=true
      shift
      ;;
    --build-only)
      BUILD_ONLY=true
      shift
      ;;
    --e2e-only)
      E2E_ONLY=true
      shift
      ;;
    --help)
      show_help
      exit 0
      ;;
    *)
      echo "Unknown option: $1"
      show_help
      exit 1
      ;;
  esac
done

# Check if we're in nix develop environment
if [ -z "${IN_NIX_SHELL:-}" ]; then
    echo "This script should be run within 'nix develop' environment"
    echo "Running: nix develop --command $0 $@"
    exec nix develop --command "$0" "$@"
fi

# Test results tracking
TOTAL_TESTS=0
PASSED_TESTS=0
FAILED_TESTS=0

check_fdb_headers() {
    print_header "FoundationDB Header Check"
    
    if [ "$DRY_RUN" = true ]; then
        echo "Would check for FDB headers and setup"
        return 0
    fi
    
    echo "Checking FoundationDB headers..."
    
    # Check if headers are available
    if [ -n "${FDB_INCLUDE_DIR:-}" ] && [ -d "$FDB_INCLUDE_DIR" ]; then
        print_success "FDB_INCLUDE_DIR is set: $FDB_INCLUDE_DIR"
    else
        print_warning "FDB_INCLUDE_DIR not set, checking other locations"
    fi
    
    # Look for fdb_c.h
    if find "${FDB_INCLUDE_DIR:-/usr/include}" -name "fdb_c.h" 2>/dev/null | head -1; then
        print_success "FoundationDB headers found"
        ((PASSED_TESTS++))
    else
        print_error "FoundationDB headers not found"
        print_info "Run: nix develop or scripts/dev/setup-fdb-headers.sh"
        ((FAILED_TESTS++))
    fi
    ((TOTAL_TESTS++))
}

test_fdb_build() {
    print_header "FoundationDB Build Test"
    
    if [ "$DRY_RUN" = true ]; then
        echo "Would test:"
        echo "  - cargo check --features fdb"
        echo "  - cargo build --bin almanac-api --features fdb"
        return 0
    fi
    
    echo "Testing FDB feature compilation..."
    if cargo check --features fdb ${VERBOSE:+--verbose} 2>&1; then
        print_success "FoundationDB features compile successfully"
        ((PASSED_TESTS++))
    else
        print_error "FoundationDB compilation failed"
        ((FAILED_TESTS++))
    fi
    ((TOTAL_TESTS++))
    
    echo "Building almanac-api with FDB..."
    if cargo build --bin almanac-api --features fdb ${VERBOSE:+--verbose} 2>&1; then
        print_success "almanac-api with FDB built successfully"
        ((PASSED_TESTS++))
    else
        print_error "Failed to build almanac-api with FDB"
        ((FAILED_TESTS++))
    fi
    ((TOTAL_TESTS++))
}

test_fdb_connectivity() {
    print_header "FoundationDB Connectivity Test"
    
    if [ "$DRY_RUN" = true ]; then
        echo "Would test:"
        echo "  - FDB cluster status"
        echo "  - FDB client connectivity"
        echo "  - Basic FDB operations"
        return 0
    fi
    
    # Check if FDB cluster is running
    echo "Checking FoundationDB cluster status..."
    FDB_CLUSTER_FILE="${FDB_CLUSTER_FILE:-$PROJECT_ROOT/data/fdb.cluster}"
    
    if command -v fdbcli >/dev/null && [ -f "$FDB_CLUSTER_FILE" ]; then
        if fdbcli --cluster-file="$FDB_CLUSTER_FILE" --exec "status" 2>/dev/null | grep -q "Cluster is available"; then
            print_success "FoundationDB cluster is available"
            ((PASSED_TESTS++))
        else
            print_error "FoundationDB cluster not available"
            print_info "Start FDB with: fdb-manage start"
            ((FAILED_TESTS++))
        fi
    else
        print_warning "fdbcli not available or cluster file missing"
        print_info "Cluster file: $FDB_CLUSTER_FILE"
        ((FAILED_TESTS++))
    fi
    ((TOTAL_TESTS++))
}

test_fdb_storage() {
    print_header "FoundationDB Storage Integration Test"
    
    if [ "$DRY_RUN" = true ]; then
        echo "Would test almanac-storage with FDB backend"
        return 0
    fi
    
    echo "Testing almanac-storage with FoundationDB..."
    
    # Set environment for FDB testing
    export STORAGE_BACKEND=foundationdb
    export FDB_CLUSTER_FILE="${FDB_CLUSTER_FILE:-$PROJECT_ROOT/data/fdb.cluster}"
    export FDB_NAMESPACE=almanac_test
    
    if cargo test --package almanac-storage --features fdb ${VERBOSE:+--verbose} 2>&1; then
        print_success "FoundationDB storage tests passed"
        ((PASSED_TESTS++))
    else
        print_error "FoundationDB storage tests failed"
        ((FAILED_TESTS++))
    fi
    ((TOTAL_TESTS++))
}

test_fdb_e2e() {
    print_header "FoundationDB E2E Test"
    
    if [ "$DRY_RUN" = true ]; then
        echo "Would run full E2E test with FDB backend"
        return 0
    fi
    
    echo "Running E2E tests with FoundationDB backend..."
    
    # Set environment for E2E testing
    export STORAGE_BACKEND=foundationdb
    export FDB_CLUSTER_FILE="${FDB_CLUSTER_FILE:-$PROJECT_ROOT/data/fdb.cluster}"
    export FDB_NAMESPACE=almanac_e2e_test
    
    # Run FDB-specific E2E tests
    if cargo test --package e2e fdb ${VERBOSE:+--verbose} 2>&1; then
        print_success "FoundationDB E2E tests passed"
        ((PASSED_TESTS++))
    else
        print_error "FoundationDB E2E tests failed"
        ((FAILED_TESTS++))
    fi
    ((TOTAL_TESTS++))
    
    # Test API with FDB backend
    echo "Testing API with FoundationDB backend..."
    if "$SCRIPT_DIR/api-tests.sh" --backend fdb ${VERBOSE:+--verbose} 2>&1; then
        print_success "API with FoundationDB tests passed"
        ((PASSED_TESTS++))
    else
        print_error "API with FoundationDB tests failed"
        ((FAILED_TESTS++))
    fi
    ((TOTAL_TESTS++))
}

# Main execution
print_header "FoundationDB Test Suite"

# Run tests based on options
if [ "$BUILD_ONLY" = true ]; then
    check_fdb_headers
    test_fdb_build
elif [ "$E2E_ONLY" = true ]; then
    test_fdb_storage
    test_fdb_e2e
else
    # Run all FDB tests
    check_fdb_headers
    test_fdb_build
    test_fdb_connectivity
    test_fdb_storage
    test_fdb_e2e
fi

# Print summary
print_header "Test Results Summary"
echo "Total tests: $TOTAL_TESTS"
echo "Passed: $PASSED_TESTS"
echo "Failed: $FAILED_TESTS"

if [ $FAILED_TESTS -eq 0 ]; then
    print_success "All FoundationDB tests passed!"
    exit 0
else
    print_error "Some FoundationDB tests failed"
    exit 1
fi