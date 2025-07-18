#!/usr/bin/env bash

# Test Simulation Environment Helper
# This script validates that the simulation environment is properly set up
# and can run the multi-chain E2E test.

set -euo pipefail

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

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

# Check if we're in the right environment
check_environment() {
    log_info "Checking simulation environment..."
    
    # Check if we're in the project root
    if [ ! -f "${PROJECT_ROOT}/Cargo.toml" ]; then
        log_error "Not in the correct project directory"
        exit 1
    fi
    
    # Check if we're in a Nix environment
    if [ -z "${NIX_SHELL:-}" ]; then
        log_error "Not in a Nix development shell"
        log_info "Run: nix develop --impure"
        exit 1
    fi
    
    log_success "Environment checks passed"
}

# Test simulation tools
test_simulation_tools() {
    log_info "Testing simulation tools..."
    
    # Test sim-cluster
    if command -v sim-cluster >/dev/null 2>&1; then
        log_success "sim-cluster available"
    else
        log_error "sim-cluster not found"
        log_info "Make sure you're in the simulation environment: nix develop --impure"
        exit 1
    fi
    
    # Test sim-config
    if command -v sim-config >/dev/null 2>&1; then
        log_success "sim-config available"
    else
        log_error "sim-config not found"
        exit 1
    fi
    
    # Test blockchain node tools
    if command -v anvil >/dev/null 2>&1; then
        log_success "anvil (Ethereum) available"
    else
        log_warning "anvil (Ethereum) not found"
    fi
    
    if command -v neutron >/dev/null 2>&1; then
        log_success "neutron (Cosmos) available"
    else
        log_warning "neutron (Cosmos) not found"
    fi
    
    if command -v solana-test-validator >/dev/null 2>&1; then
        log_success "solana-test-validator available"
    else
        log_warning "solana-test-validator not found"
    fi
    
    # Test FoundationDB
    if command -v fdb-manage >/dev/null 2>&1; then
        log_success "fdb-manage available"
    else
        log_warning "fdb-manage not found - FoundationDB tests may be skipped"
    fi
}

# Initialize simulation configuration
init_simulation_config() {
    log_info "Initializing simulation configuration..."
    
    cd "${PROJECT_ROOT}"
    
    # Create configuration for all chains
    sim-config "ethereum,cosmos,solana,fdb" "./config/simulation"
    
    if [ -f "./config/simulation/simulation.json" ]; then
        log_success "Simulation configuration created"
    else
        log_error "Failed to create simulation configuration"
        exit 1
    fi
}

# Test basic simulation startup
test_simulation_startup() {
    log_info "Testing simulation startup..."
    
    cd "${PROJECT_ROOT}"
    
    # Test configuration initialization
    sim-cluster init "ethereum,fdb"
    
    if [ -f "./config/simulation/simulation.json" ]; then
        log_success "Simulation initialization successful"
    else
        log_error "Simulation initialization failed"
        exit 1
    fi
    
    # Test status check
    sim-cluster status || log_warning "Some services may not be running"
    
    log_success "Simulation startup test completed"
}

# Run minimal E2E test
run_minimal_e2e_test() {
    log_info "Running minimal E2E test..."
    
    cd "${PROJECT_ROOT}/e2e"
    
    # Run just the basic unit tests from the multi-chain simulation
    cargo test --lib multi_chain_simulation_e2e::tests::test_simulation_config_creation -- --nocapture
    
    if [ $? -eq 0 ]; then
        log_success "Minimal E2E test passed"
    else
        log_error "Minimal E2E test failed"
        exit 1
    fi
}

# Clean up test environment
cleanup_test_environment() {
    log_info "Cleaning up test environment..."
    
    cd "${PROJECT_ROOT}"
    
    # Stop any running services
    sim-cluster stop || true
    
    # Clean up temporary files
    rm -f /tmp/sim-*.pid || true
    
    log_success "Test environment cleaned up"
}

# Generate environment report
generate_environment_report() {
    log_info "Generating environment report..."
    
    local report_file="${PROJECT_ROOT}/logs/simulation-env-report.txt"
    mkdir -p "${PROJECT_ROOT}/logs"
    
    cat > "${report_file}" << EOF
Simulation Environment Report
=============================
Date: $(date)
Host: $(hostname)
OS: $(uname -a)
Shell: ${SHELL:-unknown}
Nix Shell: ${NIX_SHELL:-not in nix shell}

Available Tools:
EOF
    
    # Check each tool
    for tool in sim-cluster sim-config anvil neutron solana-test-validator fdb-manage; do
        if command -v "$tool" >/dev/null 2>&1; then
            echo "$tool: $(which "$tool")" >> "${report_file}"
        else
            echo "$tool: not found" >> "${report_file}"
        fi
    done
    
    cat >> "${report_file}" << EOF

Nix Environment Variables:
EOF
    
    env | grep -E '^(NIX_|PATH=)' | sort >> "${report_file}"
    
    cat >> "${report_file}" << EOF

Project Structure:
EOF
    
    find "${PROJECT_ROOT}" -maxdepth 3 -type d -name "*simulation*" | sed 's/^/  /' >> "${report_file}"
    
    log_success "Environment report generated: ${report_file}"
}

# Main function
main() {
    log_info "Testing Simulation Environment Setup"
    echo "====================================="
    
    local command="${1:-all}"
    
    case "${command}" in
        "check")
            check_environment
            test_simulation_tools
            ;;
        "init")
            check_environment
            init_simulation_config
            ;;
        "startup")
            check_environment
            test_simulation_startup
            ;;
        "test")
            check_environment
            run_minimal_e2e_test
            ;;
        "cleanup")
            cleanup_test_environment
            ;;
        "report")
            generate_environment_report
            ;;
        "all")
            check_environment
            test_simulation_tools
            init_simulation_config
            test_simulation_startup
            run_minimal_e2e_test
            generate_environment_report
            ;;
        *)
            echo "Usage: $0 {check|init|startup|test|cleanup|report|all}"
            echo ""
            echo "Commands:"
            echo "  check    - Check environment and tools"
            echo "  init     - Initialize simulation configuration"
            echo "  startup  - Test simulation startup"
            echo "  test     - Run minimal E2E test"
            echo "  cleanup  - Clean up test environment"
            echo "  report   - Generate environment report"
            echo "  all      - Run all tests (default)"
            exit 1
            ;;
    esac
    
    log_success "Simulation environment test completed"
}

# Set cleanup on exit
trap cleanup_test_environment EXIT

# Run main function
main "$@"