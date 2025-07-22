#!/usr/bin/env bash
# Almanac Development Environment Setup Script
# This script sets up a complete development environment using Nix

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

print_header() {
    echo -e "${BLUE}======================================${NC}"
    echo -e "${BLUE}$1${NC}"
    echo -e "${BLUE}======================================${NC}"
}

print_success() {
    echo -e "${GREEN}$1${NC}"
}

print_warning() {
    echo -e "${YELLOW}$1${NC}"
}

print_error() {
    echo -e "${RED}$1${NC}"
}

check_nix() {
    print_header "Checking Nix Installation"
    
    if ! command -v nix &> /dev/null; then
        print_error "Nix is not installed"
        echo "Please install Nix first: https://nixos.org/download.html"
        echo "For macOS/Linux: curl -L https://nixos.org/nix/install | sh"
        exit 1
    fi
    
    print_success "Nix is installed: $(nix --version)"
    
    # Check if flakes are enabled
    if ! nix flake --help &> /dev/null 2>&1; then
        print_warning "Nix flakes are not enabled"
        echo "Adding flakes configuration..."
        
        mkdir -p ~/.config/nix
        echo "experimental-features = nix-command flakes" >> ~/.config/nix/nix.conf
        
        print_success "Nix flakes enabled"
    else
        print_success "Nix flakes are enabled"
    fi
}

setup_project() {
    print_header "Setting Up Project Environment"
    
    cd "$PROJECT_ROOT"
    
    # Create necessary directories
    mkdir -p data/{postgres,rocksdb}
    mkdir -p logs
    print_success "Created data directories"
    
    # Create default .env file if it doesn't exist
    if [ ! -f .env ]; then
        cat > .env << 'EOF'
# Almanac API Configuration
API_HOST=0.0.0.0
API_PORT=8080
METRICS_PORT=9090

# Database Configuration
DATABASE_URL=postgresql://postgres:postgres@localhost:5432/indexer

# Logging
RUST_LOG=almanac_api=info,tower_http=debug

# Security
ENABLE_CORS=true
CORS_ORIGINS=*
RATE_LIMIT_PER_MINUTE=100

# Optional: Admin API Key
# ADMIN_API_KEY=your-secure-admin-key
EOF
        print_success "Created default .env file"
    else
        print_success ".env file already exists"
    fi
    
    # Create .envrc for direnv (optional)
    if command -v direnv &> /dev/null && [ ! -f .envrc ]; then
        echo "use flake" > .envrc
        direnv allow
        print_success "Created .envrc for direnv"
    fi
}

build_project() {
    print_header "Building Project"
    
    cd "$PROJECT_ROOT"
    
    # Enter Nix shell and build
    echo "Entering Nix development shell..."
    nix develop --command bash -c '
        echo "Building API crates..."
        cargo build -p almanac-api
        
        if [ -f Cargo.nix ]; then
            echo "Cargo.nix found, updating..."
            update_cargo_nix
        else
            echo "Generating Cargo.nix..."
            generate_cargo_nix
        fi
    '
    
    print_success "Project built successfully"
}

setup_databases() {
    print_header "Setting Up Databases"
    
    cd "$PROJECT_ROOT"
    
    # Initialize databases using Nix shell
    nix develop --command init_databases
    
    print_success "Databases initialized"
}

run_tests() {
    print_header "Running Tests"
    
    cd "$PROJECT_ROOT"
    
    echo "Running API tests..."
    if nix develop --command cargo test -p almanac-api; then
        print_success "All tests passed"
    else
        print_warning "Some tests failed"
    fi
}

print_next_steps() {
    print_header "Setup Complete!"
    
    cat << EOF

Your Almanac development environment is ready!

${GREEN}Next Steps:${NC}

1. Enter the development shell:
   ${BLUE}nix develop${NC}

2. Start the API server:
   ${BLUE}nix develop --command start_api_server${NC}
   
   Or use the service management script:
   ${BLUE}./scripts/services/manage-api-service.sh start${NC}

3. Check API health:
   ${BLUE}curl http://localhost:8080/health${NC}

4. View API documentation:
   ${BLUE}open http://localhost:8080/api/docs${NC}

5. Run tests:
   ${BLUE}nix develop --command test_api${NC}

${GREEN}Available Development Shells:${NC}
- ${BLUE}nix develop${NC} - Default shell with all tools
- ${BLUE}nix develop .#api${NC} - API development with auto-reload
- ${BLUE}nix develop .#ci${NC} - Minimal CI environment
- ${BLUE}nix develop .#blockchain${NC} - Blockchain tools
- ${BLUE}nix develop .#bench${NC} - Benchmarking tools

${GREEN}Useful Commands:${NC}
- ${BLUE}init_databases${NC} - Initialize PostgreSQL and RocksDB
- ${BLUE}stop_databases${NC} - Stop database services
- ${BLUE}start_api_server${NC} - Start the API server
- ${BLUE}test_api${NC} - Run API tests
- ${BLUE}build_api${NC} - Build API in release mode
- ${BLUE}api_health_check${NC} - Check if API is running

${GREEN}Service Management:${NC}
- ${BLUE}./scripts/services/manage-api-service.sh start${NC} - Start as daemon
- ${BLUE}./scripts/services/manage-api-service.sh status${NC} - Check status
- ${BLUE}./scripts/services/manage-api-service.sh logs -f${NC} - Follow logs
- ${BLUE}./scripts/services/monitor-api-health.sh${NC} - Monitor health

${YELLOW}Configuration:${NC}
Edit ${BLUE}.env${NC} to customize your environment settings.

EOF
}

main() {
    print_header "Almanac Development Environment Setup"
    
    check_nix
    setup_project
    build_project
    setup_databases
    run_tests
    print_next_steps
}

# Run main function
main "$@"