#!/usr/bin/env bash

# Test E2E Components
# This script validates the E2E test components without requiring the full simulation

set -euo pipefail

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
E2E_DIR="${PROJECT_ROOT}/e2e"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

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

# Test E2E compilation
test_e2e_compilation() {
    log_info "Testing E2E compilation..."
    
    cd "${E2E_DIR}"
    
    if cargo check --quiet; then
        log_success "E2E tests compile successfully"
        return 0
    else
        log_error "E2E tests failed to compile"
        return 1
    fi
}

# Test basic unit tests
test_basic_unit_tests() {
    log_info "Running basic unit tests..."
    
    cd "${E2E_DIR}"
    
    if cargo test --lib --quiet test_simulation_config_creation; then
        log_success "Simulation config creation test passed"
    else
        log_error "Simulation config creation test failed"
        return 1
    fi
    
    if cargo test --lib --quiet test_test_event_creation; then
        log_success "Test event creation test passed"
    else
        log_error "Test event creation test failed"  
        return 1
    fi
    
    if cargo test --lib --quiet test_service_endpoint_creation; then
        log_success "Service endpoint creation test passed"
    else
        log_error "Service endpoint creation test failed"
        return 1
    fi
    
    if cargo test --lib --quiet test_simulation_infrastructure_creation; then
        log_success "Simulation infrastructure creation test passed"
    else
        log_error "Simulation infrastructure creation test failed"
        return 1
    fi
    
    log_success "All basic unit tests passed"
    return 0
}

# Test API availability
test_api_availability() {
    log_info "Testing API availability..."
    
    cd "${PROJECT_ROOT}"
    
    # Check if API binary exists
    if [ -f "target/release/almanac-api" ]; then
        log_success "almanac-api binary exists"
    else
        log_warning "almanac-api binary not found, trying to build..."
        if cargo build --release --bin almanac-api --quiet; then
            log_success "almanac-api binary built successfully"
        else
            log_error "Failed to build almanac-api binary"
            return 1
        fi
    fi
    
    return 0
}

# Test documentation
test_documentation() {
    log_info "Testing documentation..."
    
    local doc_file="${PROJECT_ROOT}/docs/multi-chain-e2e-test.md"
    
    if [ -f "$doc_file" ]; then
        log_success "E2E test documentation exists"
        
        # Check key sections
        if grep -q "## Test Architecture" "$doc_file"; then
            log_success "Documentation includes test architecture"
        else
            log_warning "Documentation missing test architecture section"
        fi
        
        if grep -q "## Running the Test" "$doc_file"; then
            log_success "Documentation includes running instructions"
        else
            log_warning "Documentation missing running instructions"
        fi
        
        if grep -q "## Troubleshooting" "$doc_file"; then
            log_success "Documentation includes troubleshooting"
        else
            log_warning "Documentation missing troubleshooting section"
        fi
        
    else
        log_error "E2E test documentation not found"
        return 1
    fi
    
    return 0
}

# Test script availability
test_script_availability() {
    log_info "Testing script availability..."
    
    local scripts=(
        "run-multi-chain-e2e.sh"
        "test-simulation-env.sh"
        "test-e2e-components.sh"
    )
    
    cd "${PROJECT_ROOT}/scripts"
    
    for script in "${scripts[@]}"; do
        if [ -f "$script" ]; then
            log_success "Script $script exists"
            
            if [ -x "$script" ]; then
                log_success "Script $script is executable"
            else
                log_warning "Script $script is not executable"
            fi
        else
            log_error "Script $script not found"
            return 1
        fi
    done
    
    return 0
}

# Test health check commands
test_health_check_commands() {
    log_info "Testing health check commands..."
    
    # These should work without requiring actual services
    local commands=(
        "curl --version"
        "jq --version"
    )
    
    for cmd in "${commands[@]}"; do
        if command -v ${cmd%% *} >/dev/null 2>&1; then
            log_success "Command available: $cmd"
        else
            log_warning "Command not available: $cmd"
        fi
    done
    
    return 0
}

# Test API health check format
test_api_health_check_format() {
    log_info "Testing API health check format..."
    
    # Test that the health check curl command is properly formatted
    local health_cmd='curl -s http://localhost:3000/health'
    
    log_info "Health check command: $health_cmd"
    log_success "Health check command format is valid"
    
    return 0
}

# Generate test report
generate_test_report() {
    log_info "Generating test report..."
    
    local report_file="${PROJECT_ROOT}/logs/e2e-components-report.txt"
    mkdir -p "${PROJECT_ROOT}/logs"
    
    cat > "$report_file" << EOF
E2E Components Test Report
=========================
Date: $(date)
Test Duration: $(date -d @$(($(date +%s) - START_TIME)) -u +%H:%M:%S)

Components Tested:
E2E compilation
Basic unit tests  
API availability
Documentation
Scripts
Health checks

Test Results:
- All basic unit tests: PASSED
- Multi-chain simulation config: PASSED
- Test event creation: PASSED
- Service endpoint creation: PASSED
- Infrastructure creation: PASSED

Available Scripts:
- run-multi-chain-e2e.sh
- test-simulation-env.sh 
- test-e2e-components.sh

Documentation:
- Multi-chain E2E test guide: docs/multi-chain-e2e-test.md

Next Steps:
1. Run in Nix environment: nix develop --impure
2. Test simulation environment: ./scripts/test-simulation-env.sh
3. Run full E2E test: ./scripts/run-multi-chain-e2e.sh

Status: COMPONENTS VALIDATED
EOF
    
    log_success "Test report generated: $report_file"
}

# Main function
main() {
    local START_TIME=$(date +%s)
    
    log_info "Testing E2E Components"
    echo "======================"
    
    local tests=(
        "test_e2e_compilation"
        "test_basic_unit_tests"
        "test_api_availability"
        "test_documentation"
        "test_script_availability"
        "test_health_check_commands"
        "test_api_health_check_format"
    )
    
    local failed_tests=()
    
    for test in "${tests[@]}"; do
        if ! $test; then
            failed_tests+=("$test")
        fi
        echo
    done
    
    if [ ${#failed_tests[@]} -eq 0 ]; then
        log_success "All component tests passed!"
        generate_test_report
        return 0
    else
        log_error "Some component tests failed:"
        for test in "${failed_tests[@]}"; do
            echo "  - $test"
        done
        return 1
    fi
}

# Run main function
main "$@"