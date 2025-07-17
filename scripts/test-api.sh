#!/usr/bin/env bash
#
# Script to test Almanac API crates
# This script should be run within nix develop environment

set -euo pipefail

# Check if we're in nix develop environment
if [ -z "${IN_NIX_SHELL:-}" ]; then
    echo "This script should be run within 'nix develop' environment"
    echo "Running: nix develop --command $0 $@"
    exec nix develop --command "$0" "$@"
fi

echo "=== Almanac API Test Suite ==="
echo ""

# Colors for output
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Test results
TOTAL_TESTS=0
PASSED_TESTS=0
FAILED_TESTS=0

# Function to run tests for a crate
run_crate_tests() {
    local crate=$1
    echo -e "${YELLOW}Testing $crate...${NC}"
    
    if cargo test -p "$crate" --no-fail-fast 2>&1; then
        echo -e "${GREEN}✓ $crate tests passed${NC}"
        ((PASSED_TESTS++))
    else
        echo -e "${RED}✗ $crate tests failed${NC}"
        ((FAILED_TESTS++))
    fi
    ((TOTAL_TESTS++))
    echo ""
}

# Check if PostgreSQL is running
echo "Checking database connection..."
if ! pg_isready -q; then
    echo -e "${RED}Error: PostgreSQL is not running${NC}"
    echo "Please run 'init_databases' first"
    exit 1
fi

# Run tests for each API crate
echo "Running unit tests..."
run_crate_tests "almanac-api-core"
run_crate_tests "almanac-api-http"

# Run integration tests if server is not already running
if ! curl -sf http://localhost:8080/health >/dev/null 2>&1; then
    echo "Starting API server for integration tests..."
    cargo build -p almanac-api-http
    
    # Start server in background
    cargo run -p almanac-api-http -- --port 8080 &
    SERVER_PID=$!
    
    # Wait for server to start
    echo "Waiting for server to start..."
    for i in {1..30}; do
        if curl -sf http://localhost:8080/health >/dev/null 2>&1; then
            echo -e "${GREEN}Server started${NC}"
            break
        fi
        sleep 1
    done
    
    # Run integration tests
    echo ""
    echo "Running integration tests..."
    
    # Test health endpoint
    echo -n "Testing /health endpoint... "
    if curl -sf http://localhost:8080/health | jq -e '.status == "ok"' >/dev/null; then
        echo -e "${GREEN}✓${NC}"
        ((PASSED_TESTS++))
    else
        echo -e "${RED}✗${NC}"
        ((FAILED_TESTS++))
    fi
    ((TOTAL_TESTS++))
    
    # Test events endpoint
    echo -n "Testing /api/v1/events endpoint... "
    if curl -sf http://localhost:8080/api/v1/events | jq -e '.success == true' >/dev/null; then
        echo -e "${GREEN}✓${NC}"
        ((PASSED_TESTS++))
    else
        echo -e "${RED}✗${NC}"
        ((FAILED_TESTS++))
    fi
    ((TOTAL_TESTS++))
    
    # Stop server
    echo ""
    echo "Stopping test server..."
    kill $SERVER_PID 2>/dev/null || true
    wait $SERVER_PID 2>/dev/null || true
else
    echo -e "${YELLOW}API server already running, skipping integration tests${NC}"
fi

# Summary
echo ""
echo "=== Test Summary ==="
echo "Total tests: $TOTAL_TESTS"
echo -e "Passed: ${GREEN}$PASSED_TESTS${NC}"
echo -e "Failed: ${RED}$FAILED_TESTS${NC}"

if [ $FAILED_TESTS -eq 0 ]; then
    echo -e "${GREEN}All tests passed!${NC}"
    exit 0
else
    echo -e "${RED}Some tests failed${NC}"
    exit 1
fi