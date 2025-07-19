#!/usr/bin/env bash
#
# Consolidated API Testing Script
# Tests Almanac API with both memory and FoundationDB backends
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
BACKEND="${1:-both}"  # memory, fdb, or both
VERBOSE=false
DRY_RUN=false

# Parse command line arguments
while [[ $# -gt 0 ]]; do
  case $1 in
    --backend)
      BACKEND="$2"
      shift 2
      ;;
    --verbose)
      VERBOSE=true
      shift
      ;;
    --dry-run)
      DRY_RUN=true
      shift
      ;;
    --help)
      cat << EOF
Consolidated API Testing Script

Usage: $0 [OPTIONS]

Options:
  --backend BACKEND    Test backend (memory|fdb|both) [default: both]
  --verbose           Enable verbose output
  --dry-run          Show what would be tested without running
  --help             Show this help message

Examples:
  $0                           # Test both backends
  $0 --backend memory          # Test memory backend only
  $0 --backend fdb --verbose   # Test FDB backend with verbose output
EOF
      exit 0
      ;;
    *)
      echo "Unknown option: $1"
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

# Test results tracking
TOTAL_TESTS=0
PASSED_TESTS=0
FAILED_TESTS=0

run_test_suite() {
    local backend=$1
    local backend_name=$2
    
    print_header "Testing Almanac API with $backend_name Backend"
    
    if [ "$DRY_RUN" = true ]; then
        echo "Would test: cargo test -p almanac-api --features $backend"
        return 0
    fi
    
    # Set environment variables for backend
    case $backend in
        "memory")
            export STORAGE_BACKEND=memory
            ;;
        "fdb")
            export STORAGE_BACKEND=foundationdb
            export FDB_CLUSTER_FILE="${FDB_CLUSTER_FILE:-$PROJECT_ROOT/data/fdb.cluster}"
            export FDB_NAMESPACE=almanac_test
            
            # Check if FDB is running
            if ! command -v fdbcli >/dev/null || ! fdbcli --cluster-file="$FDB_CLUSTER_FILE" --exec "status" 2>/dev/null | grep -q "Cluster is available"; then
                print_warning "FoundationDB not available, skipping FDB tests"
                return 0
            fi
            ;;
    esac
    
    # Run API tests
    echo "Running $backend_name API tests..."
    if cargo test -p almanac-api --features "$backend" ${VERBOSE:+--verbose} 2>&1; then
        print_success "$backend_name API tests passed"
        ((PASSED_TESTS++))
    else
        print_error "$backend_name API tests failed"
        ((FAILED_TESTS++))
    fi
    ((TOTAL_TESTS++))
    
    # Test API endpoints if not dry run
    if [ "$backend" = "fdb" ] && [ "$DRY_RUN" = false ]; then
        test_api_endpoints "$backend_name"
    fi
}

test_api_endpoints() {
    local backend_name=$1
    
    echo "Testing $backend_name API endpoints..."
    
    # Start API server in background for testing
    export API_HOST=127.0.0.1
    export API_PORT=8081  # Use different port to avoid conflicts
    
    # Start server
    cargo run -p almanac-api --features fdb &
    local server_pid=$!
    
    # Wait for server to start
    sleep 5
    
    # Test health endpoint
    if curl -s "http://localhost:8081/health" >/dev/null; then
        print_success "$backend_name API health endpoint responding"
        ((PASSED_TESTS++))
    else
        print_error "$backend_name API health endpoint not responding"
        ((FAILED_TESTS++))
    fi
    ((TOTAL_TESTS++))
    
    # Test events endpoint
    if curl -s "http://localhost:8081/api/v1/events?limit=1" >/dev/null; then
        print_success "$backend_name API events endpoint responding"
        ((PASSED_TESTS++))
    else
        print_error "$backend_name API events endpoint not responding"
        ((FAILED_TESTS++))
    fi
    ((TOTAL_TESTS++))
    
    # Clean up
    kill $server_pid 2>/dev/null || true
    wait $server_pid 2>/dev/null || true
}

# Main execution
print_header "Almanac API Test Suite"

case $BACKEND in
    "memory")
        run_test_suite "memory" "Memory"
        ;;
    "fdb")
        run_test_suite "fdb" "FoundationDB"
        ;;
    "both")
        run_test_suite "memory" "Memory"
        run_test_suite "fdb" "FoundationDB"
        ;;
    *)
        echo "Invalid backend: $BACKEND"
        echo "Valid options: memory, fdb, both"
        exit 1
        ;;
esac

# Print summary
print_header "Test Results Summary"
echo "Total tests: $TOTAL_TESTS"
echo "Passed: $PASSED_TESTS"
echo "Failed: $FAILED_TESTS"

if [ $FAILED_TESTS -eq 0 ]; then
    print_success "All tests passed!"
    exit 0
else
    print_error "Some tests failed"
    exit 1
fi