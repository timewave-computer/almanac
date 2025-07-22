#!/usr/bin/env bash
#
# Consolidated E2E Testing Script
# Runs all end-to-end tests including multi-chain simulation
#

set -euo pipefail

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"
E2E_DIR="${PROJECT_ROOT}/tests"
cd "$PROJECT_ROOT"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Default values
TEST_FILTER=""
VERBOSE=false
DRY_RUN=false
BUILD_FIRST=true
KEEP_OUTPUTS=false
SIMULATION_ONLY=false
COMPONENTS_ONLY=false

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
Consolidated E2E Testing Script

Usage: $0 [OPTIONS]

Options:
  --filter PATTERN        Run only tests matching pattern
  --verbose              Enable verbose output
  --dry-run             Show what would be tested without running
  --no-build            Skip building binaries
  --keep-outputs        Keep test output files for inspection
  --simulation-only     Run only multi-chain simulation tests
  --components-only     Run only individual component tests
  --help                Show this help message

Test Types:
  1. Basic E2E Tests - Core functionality validation
  2. Component Tests - Individual component validation
  3. Multi-Chain Simulation - Full integration test with blockchain nodes

Examples:
  $0                              # Run all E2E tests
  $0 --filter ethereum            # Run only Ethereum-related tests
  $0 --simulation-only --verbose  # Run simulation tests with verbose output
  $0 --components-only --dry-run  # Show component tests without running
EOF
}

# Parse command line arguments
while [[ $# -gt 0 ]]; do
  case $1 in
    --filter)
      TEST_FILTER="$2"
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
    --no-build)
      BUILD_FIRST=false
      shift
      ;;
    --keep-outputs)
      KEEP_OUTPUTS=true
      shift
      ;;
    --simulation-only)
      SIMULATION_ONLY=true
      shift
      ;;
    --components-only)
      COMPONENTS_ONLY=true
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

run_basic_e2e_tests() {
    print_header "Basic E2E Tests"
    
    if [ "$DRY_RUN" = true ]; then
        echo "Would run: cargo test --package almanac-tests"
        return 0
    fi
    
    echo "Running basic E2E test suite..."
    if cargo test --package almanac-tests ${VERBOSE:+--verbose} ${TEST_FILTER:+-- --test-filter "$TEST_FILTER"} 2>&1; then
        print_success "Basic E2E tests passed"
        ((PASSED_TESTS++))
    else
        print_error "Basic E2E tests failed"
        ((FAILED_TESTS++))
    fi
    ((TOTAL_TESTS++))
}

run_component_tests() {
    print_header "Component E2E Tests"
    
    if [ "$DRY_RUN" = true ]; then
        echo "Would test individual components:"
        echo "  - API integration"
        echo "  - Storage backends"
        echo "  - Configuration loading"
        return 0
    fi
    
    # Test API integration
    echo "Testing API integration..."
    if "$SCRIPT_DIR/api-tests.sh" --backend memory ${VERBOSE:+--verbose} 2>&1; then
        print_success "API component tests passed"
        ((PASSED_TESTS++))
    else
        print_error "API component tests failed"
        ((FAILED_TESTS++))
    fi
    ((TOTAL_TESTS++))
    
    # Test FoundationDB integration
    echo "Testing FoundationDB integration..."
    if "$SCRIPT_DIR/fdb-tests.sh" ${VERBOSE:+--verbose} 2>&1; then
        print_success "FoundationDB component tests passed"
        ((PASSED_TESTS++))
    else
        print_error "FoundationDB component tests failed"
        ((FAILED_TESTS++))
    fi
    ((TOTAL_TESTS++))
    
    # Test configuration loading
    echo "Testing configuration..."
    if cargo test --package almanac-config ${VERBOSE:+--verbose} 2>&1; then
        print_success "Configuration tests passed"
        ((PASSED_TESTS++))
    else
        print_error "Configuration tests failed"
        ((FAILED_TESTS++))
    fi
    ((TOTAL_TESTS++))
}

run_simulation_tests() {
    print_header "Multi-Chain Simulation E2E Tests"
    
    if [ "$DRY_RUN" = true ]; then
        echo "Would run:"
        echo "  - nix run .#sim-cluster init"
        echo "  - nix run .#sim-cluster start"
        echo "  - Multi-chain integration test"
        echo "  - nix run .#sim-cluster stop"
        return 0
    fi
    
    print_info "Testing multi-chain simulation environment..."
    
    # Check if simulation tools are available
    if ! command -v sim-cluster >/dev/null 2>&1; then
        print_warning "sim-cluster not available, using nix run"
        SIM_CMD="nix run .#sim-cluster --"
    else
        SIM_CMD="sim-cluster"
    fi
    
    # Initialize simulation
    echo "Initializing simulation cluster..."
    if $SIM_CMD init ethereum,cosmos,fdb ${VERBOSE:+--verbose} 2>&1; then
        print_success "Simulation initialized"
    else
        print_error "Simulation initialization failed"
        ((FAILED_TESTS++))
        ((TOTAL_TESTS++))
        return
    fi
    
    # Start simulation services
    echo "Starting simulation services..."
    if $SIM_CMD start ${VERBOSE:+--verbose} 2>&1; then
        print_success "Simulation services started"
        
        # Run the actual multi-chain E2E test
        echo "Running multi-chain integration test..."
        if cargo test --package almanac-tests multi_chain_simulation ${VERBOSE:+--verbose} 2>&1; then
            print_success "Multi-chain simulation test passed"
            ((PASSED_TESTS++))
        else
            print_error "Multi-chain simulation test failed"
            ((FAILED_TESTS++))
        fi
        
        # Stop simulation services
        echo "Stopping simulation services..."
        $SIM_CMD stop ${VERBOSE:+--verbose} 2>&1 || print_warning "Some services may not have stopped cleanly"
        
    else
        print_error "Failed to start simulation services"
        ((FAILED_TESTS++))
    fi
    ((TOTAL_TESTS++))
}

build_binaries() {
    if [ "$BUILD_FIRST" = false ]; then
        return 0
    fi
    
    print_header "Building Binaries"
    
    if [ "$DRY_RUN" = true ]; then
        echo "Would build:"
        echo "  - almanac-api"
        echo "  - almanac-indexing" 
        echo "  - e2e test binary"
        return 0
    fi
    
    echo "Building almanac-api..."
    if cargo build --bin almanac-api --features fdb ${VERBOSE:+--verbose} 2>&1; then
        print_success "almanac-api built successfully"
    else
        print_error "Failed to build almanac-api"
        exit 1
    fi
    
    echo "Building almanac tests..."
    if cargo build --package almanac-tests ${VERBOSE:+--verbose} 2>&1; then
        print_success "Almanac tests built successfully"
    else
        print_error "Failed to build almanac tests"
        exit 1
    fi
}

cleanup_outputs() {
    if [ "$KEEP_OUTPUTS" = true ]; then
        print_info "Keeping test outputs in tests/test_outputs/"
        return 0
    fi
    
    if [ -d "tests/test_outputs" ]; then
        echo "Cleaning up test outputs..."
        rm -rf tests/test_outputs
    fi
}

# Main execution
print_header "Almanac E2E Test Suite"

# Build binaries first
build_binaries

# Run tests based on options
if [ "$SIMULATION_ONLY" = true ]; then
    run_simulation_tests
elif [ "$COMPONENTS_ONLY" = true ]; then
    run_component_tests
else
    # Run all test types
    run_basic_e2e_tests
    run_component_tests
    run_simulation_tests
fi

# Cleanup
cleanup_outputs

# Print summary
print_header "Test Results Summary"
echo "Total test suites: $TOTAL_TESTS"
echo "Passed: $PASSED_TESTS"
echo "Failed: $FAILED_TESTS"

if [ $FAILED_TESTS -eq 0 ]; then
    print_success "All E2E tests passed!"
    exit 0
else
    print_error "Some E2E tests failed"
    exit 1
fi