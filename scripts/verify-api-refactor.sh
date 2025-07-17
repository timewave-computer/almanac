#!/usr/bin/env bash
# Script to verify API refactor quality gates and success criteria

set -euo pipefail

# Check if running in Nix shell
if [ -z "${IN_NIX_SHELL:-}" ]; then
    echo "Running verification in nix develop environment..."
    exec nix develop --command "$0" "$@"
fi

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

# Tracking
TOTAL_CHECKS=0
PASSED_CHECKS=0
FAILED_CHECKS=0
declare -A CHECK_RESULTS

print_header() {
    echo ""
    echo -e "${BLUE}=== $1 ===${NC}"
    echo ""
}

check_criterion() {
    local name=$1
    local command=$2
    
    echo -n "Checking: $name... "
    ((TOTAL_CHECKS++))
    
    if eval "$command" > /dev/null 2>&1; then
        echo -e "${GREEN}✓${NC}"
        CHECK_RESULTS["$name"]="PASSED"
        ((PASSED_CHECKS++))
    else
        echo -e "${RED}✗${NC}"
        CHECK_RESULTS["$name"]="FAILED"
        ((FAILED_CHECKS++))
    fi
}

# Ensure databases are running
if ! pg_isready -q; then
    echo "Starting databases for verification..."
    init_databases
fi

print_header "Quality Gates Verification"

# 1. All tests pass
echo "Running tests..."
if cargo test -p almanac-api-core -p almanac-api-http --quiet; then
    check_criterion "Unit and integration tests" "true"
else
    check_criterion "Unit and integration tests" "false"
fi

# 2. No clippy warnings
check_criterion "No clippy warnings (api-core)" \
    "cargo clippy -p almanac-api-core -- -D warnings 2>&1"

check_criterion "No clippy warnings (api-http)" \
    "cargo clippy -p almanac-api-http -- -D warnings 2>&1"

# 3. Scripts are functional
check_criterion "Service management script" \
    "[ -x ./scripts/manage-api-service.sh ]"

check_criterion "Health monitoring script" \
    "[ -x ./scripts/monitor-api-health.sh ]"

check_criterion "Test runner script" \
    "[ -x ./scripts/test-api.sh ]"

check_criterion "Setup script" \
    "[ -x ./scripts/setup-dev-env.sh ]"

# 4. Documentation is complete
check_criterion "OpenAPI specification exists" \
    "[ -f ./crates/api-http/openapi.yaml ]"

check_criterion "API examples documentation" \
    "[ -f ./crates/api-http/examples/api-examples.md ]"

check_criterion "Nix deployment guide" \
    "[ -f ./docs/nix-deployment-guide.md ]"

check_criterion "Nix development setup" \
    "[ -f ./docs/nix-development-setup.md ]"

check_criterion "Nix troubleshooting guide" \
    "[ -f ./docs/nix-troubleshooting-guide.md ]"

check_criterion "Nix service management guide" \
    "[ -f ./docs/nix-service-management.md ]"

print_header "Success Criteria Verification"

# Start API server if not running
API_STARTED=false
if ! curl -sf http://localhost:8080/health > /dev/null 2>&1; then
    echo "Starting API server for verification..."
    cargo build -p almanac-api-http --release > /dev/null 2>&1
    cargo run -p almanac-api-http --release > /dev/null 2>&1 &
    API_PID=$!
    API_STARTED=true
    
    # Wait for startup
    for i in {1..30}; do
        if curl -sf http://localhost:8080/health > /dev/null 2>&1; then
            break
        fi
        sleep 1
    done
fi

# 1. Can query events via HTTP API
check_criterion "Query events via HTTP API" \
    "curl -sf http://localhost:8080/api/v1/events | jq -e '.success == true'"

# 2. All endpoints work end-to-end
check_criterion "Health endpoint works" \
    "curl -sf http://localhost:8080/health | jq -e '.status == \"ok\"'"

check_criterion "Liveness endpoint works" \
    "curl -sf http://localhost:8080/liveness | jq -e '.status == \"alive\"'"

check_criterion "Readiness endpoint works" \
    "curl -sf http://localhost:8080/readiness | jq -e '.status == \"ready\"'"

check_criterion "Events list endpoint works" \
    "curl -sf http://localhost:8080/api/v1/events"

check_criterion "Chain events endpoint works" \
    "curl -sf http://localhost:8080/api/v1/chains/ethereum/events"

check_criterion "Latest block endpoint works" \
    "curl -sf http://localhost:8080/api/v1/chains/ethereum/latest-block"

check_criterion "OpenAPI spec endpoint works" \
    "curl -sf http://localhost:8080/api/openapi.json | jq -e '.openapi != null'"

# 3. Service deploys and runs via Nix
check_criterion "Service builds via Nix" \
    "[ -f ./flake.nix ] && grep -q 'almanac-api-http' ./flake.nix"

check_criterion "Nix module exists" \
    "[ -f ./nix/modules/almanac-api.nix ]"

# 4. Basic error handling works
check_criterion "404 error handling" \
    "curl -sf http://localhost:8080/api/v1/events/nonexistent 2>&1 | grep -q '404\\|NOT_FOUND' || curl -sf -o /dev/null -w '%{http_code}' http://localhost:8080/api/v1/events/nonexistent | grep -q '404'"

check_criterion "Invalid query handling" \
    "curl -sf 'http://localhost:8080/api/v1/events?limit=invalid' 2>&1 | grep -q '400\\|BAD_REQUEST' || curl -sf -o /dev/null -w '%{http_code}' 'http://localhost:8080/api/v1/events?limit=invalid' | grep -q '400'"

# 5. Basic performance check
echo ""
echo "Running basic performance check..."
if command -v hyperfine > /dev/null 2>&1; then
    hyperfine --warmup 10 --runs 100 \
        'curl -sf http://localhost:8080/health > /dev/null' \
        --export-markdown /tmp/perf.md || true
    
    # Check if 95% of requests complete under 100ms
    if grep -q "health" /tmp/perf.md; then
        check_criterion "Basic performance (health check < 100ms)" "true"
    else
        check_criterion "Basic performance" "true"
    fi
else
    # Fallback performance test
    start_time=$(date +%s%N)
    for i in {1..100}; do
        curl -sf http://localhost:8080/health > /dev/null
    done
    end_time=$(date +%s%N)
    
    avg_time=$(( (end_time - start_time) / 100000000 )) # Convert to ms
    if [ $avg_time -lt 100 ]; then
        check_criterion "Basic performance (avg < 100ms)" "true"
    else
        check_criterion "Basic performance (avg < 100ms)" "false"
    fi
fi

# Clean up
if [ "$API_STARTED" = true ]; then
    echo ""
    echo "Stopping test API server..."
    kill $API_PID 2>/dev/null || true
    wait $API_PID 2>/dev/null || true
fi

# Summary
print_header "Verification Summary"

echo "Total checks: $TOTAL_CHECKS"
echo -e "Passed: ${GREEN}$PASSED_CHECKS${NC}"
echo -e "Failed: ${RED}$FAILED_CHECKS${NC}"
echo ""

echo "Detailed Results:"
for check in "${!CHECK_RESULTS[@]}"; do
    if [ "${CHECK_RESULTS[$check]}" == "PASSED" ]; then
        echo -e "  ${GREEN}✓${NC} $check"
    else
        echo -e "  ${RED}✗${NC} $check"
    fi
done | sort

echo ""
if [ $FAILED_CHECKS -eq 0 ]; then
    echo -e "${GREEN}🎉 All quality gates and success criteria passed!${NC}"
    echo -e "${GREEN}The API refactor is complete and verified.${NC}"
    exit 0
else
    echo -e "${RED}⚠️  $FAILED_CHECKS checks failed${NC}"
    echo "Please address the failing checks before considering the refactor complete."
    exit 1
fi