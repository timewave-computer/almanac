#!/usr/bin/env bash
#
# Script to run all Almanac tests via nix develop
# This ensures consistent test environment

set -euo pipefail

# Colors for output
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# Check if we're in nix develop environment
if [ -z "${IN_NIX_SHELL:-}" ]; then
    echo -e "${YELLOW}Running tests via nix develop...${NC}"
    exec nix develop --command "$0" "$@"
fi

echo -e "${CYAN}=== Running All Almanac Tests ===${NC}"
echo ""

# Test results tracking
TOTAL_SUITES=0
PASSED_SUITES=0
FAILED_SUITES=0
FAILED_TESTS=""

# Function to run a test suite
run_test_suite() {
    local name=$1
    local command=$2
    
    echo -e "${CYAN}Running $name...${NC}"
    ((TOTAL_SUITES++))
    
    if eval "$command"; then
        echo -e "${GREEN}✓ $name passed${NC}"
        ((PASSED_SUITES++))
    else
        echo -e "${RED}✗ $name failed${NC}"
        ((FAILED_SUITES++))
        FAILED_TESTS="$FAILED_TESTS\n  - $name"
    fi
    echo ""
}

# Ensure database is running
echo -e "${CYAN}Checking database status...${NC}"
if ! pg_isready -q; then
    echo "Starting PostgreSQL..."
    init_databases
fi

# Run different test suites
run_test_suite "API Core Tests" "cargo test -p almanac-api-core"
run_test_suite "API HTTP Tests" "cargo test -p almanac-api-http"

# Run clippy checks
run_test_suite "API Core Clippy" "cargo clippy -p almanac-api-core -- -D warnings"
run_test_suite "API HTTP Clippy" "cargo clippy -p almanac-api-http -- -D warnings"

# Run format check
run_test_suite "Code Formatting" "cargo fmt --all -- --check"

# Run integration tests if available
if [ -f "./scripts/test-api.sh" ]; then
    run_test_suite "API Integration Tests" "./scripts/test-api.sh"
fi

# Summary
echo ""
echo -e "${CYAN}=== Test Summary ===${NC}"
echo "Total test suites: $TOTAL_SUITES"
echo -e "Passed: ${GREEN}$PASSED_SUITES${NC}"
echo -e "Failed: ${RED}$FAILED_SUITES${NC}"

if [ $FAILED_SUITES -gt 0 ]; then
    echo -e "${RED}Failed test suites:${NC}$FAILED_TESTS"
    exit 1
else
    echo -e "${GREEN}All tests passed!${NC}"
    exit 0
fi