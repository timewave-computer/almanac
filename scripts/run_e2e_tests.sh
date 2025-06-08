#!/usr/bin/env bash

# End-to-end test runner for Almanac CLI
# This script runs all CLI commands documented in the docs/ and README with mock data

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
PURPLE='\033[0;35m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# Default values
VERBOSE=false
FILTER=""
BUILD_FIRST=true
CLEAN_OUTPUTS=true

# Function to print colored output
print_header() {
    echo -e "${BLUE}===================================================${NC}"
    echo -e "${BLUE}$1${NC}"
    echo -e "${BLUE}===================================================${NC}"
}

print_success() {
    echo -e "${GREEN}✓ $1${NC}"
}

print_error() {
    echo -e "${RED}❌ $1${NC}"
}

print_warning() {
    echo -e "${YELLOW}⚠ $1${NC}"
}

print_info() {
    echo -e "${CYAN}ℹ $1${NC}"
}

# Function to show usage
show_usage() {
    cat << EOF
Usage: $0 [OPTIONS]

Run comprehensive end-to-end tests for Almanac CLI commands.

OPTIONS:
    -h, --help              Show this help message
    -v, --verbose           Enable verbose output
    -f, --filter PATTERN    Run only tests matching PATTERN
    -n, --no-build          Skip building almanac binary
    -k, --keep-outputs      Keep test output directories
    --comprehensive         Run comprehensive CLI tests only
    --cosmos                Run cosmos CLI tests only
    --ethereum              Run ethereum CLI tests only
    --validation            Run validation tests only
    --config                Run configuration tests only

EXAMPLES:
    $0                              # Run all tests
    $0 --verbose                    # Run all tests with verbose output
    $0 --filter cosmos              # Run only cosmos-related tests
    $0 --comprehensive              # Run comprehensive CLI tests only
    $0 --no-build --keep-outputs    # Skip build and keep test outputs

EOF
}

# Parse command line arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        -h|--help)
            show_usage
            exit 0
            ;;
        -v|--verbose)
            VERBOSE=true
            shift
            ;;
        -f|--filter)
            FILTER="$2"
            shift 2
            ;;
        -n|--no-build)
            BUILD_FIRST=false
            shift
            ;;
        -k|--keep-outputs)
            CLEAN_OUTPUTS=false
            shift
            ;;
        --comprehensive)
            FILTER="comprehensive"
            shift
            ;;
        --cosmos)
            FILTER="cosmos"
            shift
            ;;
        --ethereum)
            FILTER="ethereum"
            shift
            ;;
        --validation)
            FILTER="validation"
            shift
            ;;
        --config)
            FILTER="config"
            shift
            ;;
        *)
            print_error "Unknown option: $1"
            show_usage
            exit 1
            ;;
    esac
done

# Check if we're in the right directory
if [[ ! -f "flake.nix" ]] || [[ ! -d "e2e" ]]; then
    print_error "This script must be run from the almanac project root directory"
    exit 1
fi

print_header "Almanac CLI End-to-End Test Runner"

# Enter nix environment
print_info "Entering nix development environment..."
if ! command -v nix &> /dev/null; then
    print_error "Nix is not installed or not in PATH"
    exit 1
fi

# Clean previous test outputs if requested
if [[ "$CLEAN_OUTPUTS" == "true" ]]; then
    print_info "Cleaning previous test outputs..."
    rm -rf e2e/test_outputs/* 2>/dev/null || true
    rm -rf generated/* 2>/dev/null || true
fi

# Build almanac binary if requested
if [[ "$BUILD_FIRST" == "true" ]]; then
    print_info "Building almanac binary..."
    if ! nix build .#indexer-api --no-link; then
        print_error "Failed to build almanac binary"
        exit 1
    fi
    print_success "Almanac binary built successfully"
fi

# Prepare test command
TEST_CMD="cargo run --manifest-path e2e/Cargo.toml --"

if [[ "$VERBOSE" == "true" ]]; then
    TEST_CMD="$TEST_CMD --verbose"
fi

if [[ -n "$FILTER" ]]; then
    TEST_CMD="$TEST_CMD --filter=$FILTER"
fi

print_info "Running tests with command: $TEST_CMD"

# Run in nix environment
print_header "Executing Test Suite"

export RUST_LOG=info
if [[ "$VERBOSE" == "true" ]]; then
    export RUST_LOG=debug
fi

# Create a temporary script to run in nix environment
TEMP_SCRIPT=$(mktemp)
cat > "$TEMP_SCRIPT" << EOF
#!/usr/bin/env bash
set -euo pipefail

# Ensure we have the necessary environment
export PATH="\${PATH}:\$(nix eval --raw .#indexer-api.outPath)/bin"

# Run the test suite
$TEST_CMD

exit_code=\$?

echo ""
if [[ \$exit_code -eq 0 ]]; then
    echo -e "${GREEN}🎉 All tests completed successfully!${NC}"
else
    echo -e "${RED}💥 Some tests failed (exit code: \$exit_code)${NC}"
fi

exit \$exit_code
EOF

chmod +x "$TEMP_SCRIPT"

# Execute in nix environment
if nix develop --command "$TEMP_SCRIPT"; then
    print_success "Test suite completed successfully"
    exit_code=0
else
    exit_code=$?
    print_error "Test suite failed with exit code: $exit_code"
fi

# Cleanup
rm -f "$TEMP_SCRIPT"

# Show summary information
if [[ "$CLEAN_OUTPUTS" == "false" ]]; then
    print_info "Test outputs preserved in e2e/test_outputs/"
    if [[ -d "e2e/test_outputs" ]]; then
        output_count=$(find e2e/test_outputs -mindepth 1 -maxdepth 1 -type d 2>/dev/null | wc -l)
        print_info "Generated $output_count test output directories"
    fi
fi

if [[ "$VERBOSE" == "true" ]]; then
    print_info "Test logs are available in the terminal output above"
fi

print_header "Test Runner Complete"
exit $exit_code 