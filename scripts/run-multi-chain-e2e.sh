#!/usr/bin/env bash

# Multi-Chain Simulation E2E Test Runner
# This script runs the comprehensive E2E test that validates the complete
# multi-chain simulation environment with FoundationDB and Almanac services.

set -euo pipefail

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
E2E_DIR="${PROJECT_ROOT}/e2e"
LOGS_DIR="${PROJECT_ROOT}/logs"
DATA_DIR="${PROJECT_ROOT}/data"
CONFIG_DIR="${PROJECT_ROOT}/config"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Logging functions
log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Cleanup function
cleanup() {
    log_info "Cleaning up test environment..."
    
    # Stop simulation cluster
    if command -v sim-cluster >/dev/null 2>&1; then
        sim-cluster stop || true
    fi
    
    # Kill any remaining processes
    pkill -f "anvil" || true
    pkill -f "neutron" || true
    pkill -f "solana-test-validator" || true
    pkill -f "almanac-api" || true
    pkill -f "almanac-indexing" || true
    
    # Clean up temporary files
    rm -rf /tmp/sim-*.pid || true
    
    log_success "Cleanup completed"
}

# Set up cleanup on exit
trap cleanup EXIT

# Check prerequisites
check_prerequisites() {
    log_info "Checking prerequisites..."
    
    # Check if we're in a Nix environment
    if [ -z "${NIX_SHELL:-}" ]; then
        log_error "This script must be run in a Nix development shell"
        log_info "Run: nix develop"
        exit 1
    fi
    
    # Check if simulation tools are available
    if ! command -v sim-cluster >/dev/null 2>&1; then
        log_error "Simulation tools not available. Make sure you're in the simulation environment."
        log_info "Run: nix develop --impure"
        exit 1
    fi
    
    # Check if FoundationDB is available
    if ! command -v fdb-manage >/dev/null 2>&1; then
        log_warning "FoundationDB management tools not found. Some tests may be skipped."
    fi
    
    log_success "Prerequisites check passed"
}

# Setup test environment
setup_test_environment() {
    log_info "Setting up test environment..."
    
    # Create necessary directories
    mkdir -p "${LOGS_DIR}" "${DATA_DIR}" "${CONFIG_DIR}"
    
    # Change to project root
    cd "${PROJECT_ROOT}"
    
    # Set environment variables
    export RUST_LOG="info"
    export RUST_BACKTRACE="1"
    export FDB_CLUSTER_FILE="${DATA_DIR}/fdb.cluster"
    export FDB_NAMESPACE="almanac_e2e_test"
    
    log_success "Test environment setup completed"
}

# Run the E2E test
run_e2e_test() {
    log_info "Running multi-chain simulation E2E test..."
    
    # Change to E2E directory
    cd "${E2E_DIR}"
    
    # Run the test with timeout
    timeout 600 cargo test --release --test multi_chain_simulation_e2e -- --nocapture --test-threads=1
    
    if [ $? -eq 0 ]; then
        log_success "E2E test completed successfully"
        return 0
    else
        log_error "E2E test failed"
        return 1
    fi
}

# Run the standalone test function
run_standalone_test() {
    log_info "Running standalone multi-chain simulation test..."
    
    # Change to E2E directory
    cd "${E2E_DIR}"
    
    # Run the standalone test function
    cargo run --release --bin test-runner -- multi-chain-simulation
    
    if [ $? -eq 0 ]; then
        log_success "Standalone test completed successfully"
        return 0
    else
        log_error "Standalone test failed"
        return 1
    fi
}

# Generate test report
generate_test_report() {
    log_info "Generating test report..."
    
    local report_file="${LOGS_DIR}/multi-chain-e2e-report.txt"
    
    cat > "${report_file}" << EOF
Multi-Chain Simulation E2E Test Report
=====================================
Date: $(date)
Environment: $(uname -a)

Test Configuration:
- Enabled chains: Ethereum, Cosmos, Solana, FoundationDB
- Test duration: 60 seconds
- Transactions per chain: 5
- API endpoint: http://localhost:3000
- Log directory: ${LOGS_DIR}
- Data directory: ${DATA_DIR}

Test Results:
EOF
    
    if [ -f "${LOGS_DIR}/almanac-api.log" ]; then
        echo "- Almanac API: $(if grep -q "started" "${LOGS_DIR}/almanac-api.log"; then echo "Started"; else echo "Failed"; fi)" >> "${report_file}"
    fi
    
    if [ -f "${LOGS_DIR}/anvil.log" ]; then
        echo "- Ethereum (Anvil): $(if grep -q "Listening" "${LOGS_DIR}/anvil.log"; then echo "Running"; else echo "Failed"; fi)" >> "${report_file}"
    fi
    
    if [ -f "${LOGS_DIR}/neutron.log" ]; then
        echo "- Cosmos (Neutron): $(if grep -q "started" "${LOGS_DIR}/neutron.log"; then echo "Running"; else echo "Failed"; fi)" >> "${report_file}"
    fi
    
    if [ -f "${LOGS_DIR}/solana.log" ]; then
        echo "- Solana: $(if grep -q "RPC" "${LOGS_DIR}/solana.log"; then echo "Running"; else echo "Failed"; fi)" >> "${report_file}"
    fi
    
    cat >> "${report_file}" << EOF

Service Endpoints:
- Ethereum RPC: http://localhost:8545
- Cosmos RPC: http://localhost:26657
- Solana RPC: http://localhost:8899
- Almanac API: http://localhost:3000
- Almanac Swagger UI: http://localhost:3000/api/swagger-ui

Log Files:
EOF
    
    find "${LOGS_DIR}" -name "*.log" -type f | sed 's/^/- /' >> "${report_file}"
    
    log_success "Test report generated: ${report_file}"
}

# Main function
main() {
    log_info "Starting Multi-Chain Simulation E2E Test"
    echo "========================================"
    
    # Check command line arguments
    TEST_TYPE="${1:-test}"
    
    case "${TEST_TYPE}" in
        "test")
            check_prerequisites
            setup_test_environment
            run_e2e_test
            ;;
        "standalone")
            check_prerequisites
            setup_test_environment
            run_standalone_test
            ;;
        "report")
            generate_test_report
            ;;
        "clean")
            cleanup
            ;;
        *)
            echo "Usage: $0 {test|standalone|report|clean}"
            echo ""
            echo "Commands:"
            echo "  test       - Run the full E2E test suite"
            echo "  standalone - Run the standalone test function"
            echo "  report     - Generate a test report"
            echo "  clean      - Clean up test environment"
            exit 1
            ;;
    esac
    
    # Generate report unless we're just cleaning
    if [ "${TEST_TYPE}" != "clean" ]; then
        generate_test_report
    fi
    
    log_success "Multi-Chain Simulation E2E Test completed"
}

# Run main function
main "$@"