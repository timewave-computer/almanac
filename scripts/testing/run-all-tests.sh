#!/usr/bin/env bash
#
# Master Test Runner for Almanac
# Orchestrates all test suites with proper environment setup
#

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"
cd "$PROJECT_ROOT"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# Default values
VERBOSE=false
DRY_RUN=false
QUICK=false
SKIP_E2E=false

show_help() {
    cat << EOF
Master Test Runner for Almanac

Usage: $0 [OPTIONS]

Options:
  --verbose              Enable verbose output
  --dry-run             Show what would be tested without running
  --quick               Run only fast unit tests
  --skip-e2e            Skip end-to-end tests
  --help                Show this help message

Test Suites:
  1. Unit Tests - Core crate unit tests
  2. API Tests - API functionality with memory and FDB backends
  3. FDB Tests - FoundationDB build and integration tests
  4. E2E Tests - End-to-end integration tests
  5. Simulation Tests - Multi-chain simulation tests

Examples:
  $0                     # Run all test suites
  $0 --quick             # Run only unit tests
  $0 --skip-e2e          # Skip E2E and simulation tests
  $0 --verbose --dry-run # Show all tests that would run
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
    --quick)
      QUICK=true
      shift
      ;;
    --skip-e2e)
      SKIP_E2E=true
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
    echo -e "${YELLOW}Running tests via nix develop...${NC}"
    exec nix develop --command "$0" "$@"
fi

echo -e "${CYAN}=== Almanac Master Test Runner ===${NC}"
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
    
    if [ "$DRY_RUN" = true ]; then
        echo "Would run: $command"
        echo -e "${BLUE}$name (dry run)${NC}"
        ((PASSED_SUITES++))
    elif eval "$command"; then
        echo -e "${GREEN}+ $name passed${NC}"
        ((PASSED_SUITES++))
    else
        echo -e "${RED}- $name failed${NC}"
        ((FAILED_SUITES++))
        FAILED_TESTS="$FAILED_TESTS\n  - $name"
    fi
    echo ""
}
# 1. Unit Tests (always run)
VERBOSE_FLAG=""
if [ "$VERBOSE" = true ]; then
    VERBOSE_FLAG="--verbose"
fi

run_test_suite "Core Unit Tests" "cargo test --package almanac-core $VERBOSE_FLAG"
run_test_suite "Config Unit Tests" "cargo test --package almanac-config $VERBOSE_FLAG"
run_test_suite "Chains Unit Tests" "cargo test --package almanac-chains $VERBOSE_FLAG"
run_test_suite "Storage Unit Tests" "cargo test --package almanac-storage --features memory $VERBOSE_FLAG"

if [ "$QUICK" = true ]; then
    echo -e "${BLUE}Quick mode: Skipping integration and E2E tests${NC}"
else
    # 2. API Tests
    if [ -f "$SCRIPT_DIR/testing/api-tests.sh" ]; then
        run_test_suite "API Tests" "$SCRIPT_DIR/testing/api-tests.sh $VERBOSE_FLAG"
    else
        run_test_suite "API Tests (legacy)" "cargo test --package almanac-api --features memory $VERBOSE_FLAG"
    fi
    
    # 3. FoundationDB Tests
    if [ -f "$SCRIPT_DIR/testing/fdb-tests.sh" ]; then
        run_test_suite "FoundationDB Tests" "$SCRIPT_DIR/testing/fdb-tests.sh $VERBOSE_FLAG"
    fi
    
    # 4. E2E Tests (if not skipped)
    if [ "$SKIP_E2E" = false ]; then
        if [ -f "$SCRIPT_DIR/testing/e2e-tests.sh" ]; then
            run_test_suite "E2E Tests" "$SCRIPT_DIR/testing/e2e-tests.sh --components-only $VERBOSE_FLAG"
            run_test_suite "Simulation Tests" "$SCRIPT_DIR/testing/e2e-tests.sh --simulation-only $VERBOSE_FLAG"
        else
            run_test_suite "E2E Tests (legacy)" "cargo test --package e2e $VERBOSE_FLAG"
        fi
    fi
fi

# 5. Code Quality Checks (always run)
run_test_suite "Code Formatting" "cargo fmt --all -- --check"
run_test_suite "Clippy Lints" "cargo clippy --all-targets --all-features -- -D warnings"

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