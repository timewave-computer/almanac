#!/usr/bin/env bash
# Almanac API Service Management Script
# This script provides Nix-based service management commands

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Check if running in Nix shell
if [ -z "${IN_NIX_SHELL:-}" ]; then
    echo -e "${YELLOW}Not in Nix shell. Running with nix develop...${NC}"
    exec nix develop --command "$0" "$@"
fi

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

# Default values
SERVICE_NAME="almanac-api"
CONFIG_FILE="${CONFIG_FILE:-$PROJECT_ROOT/.env}"
PID_FILE="${PID_FILE:-$PROJECT_ROOT/data/almanac-api.pid}"
LOG_FILE="${LOG_FILE:-$PROJECT_ROOT/data/almanac-api.log}"

# Functions
print_usage() {
    cat << EOF
Usage: $0 <command> [options]

Commands:
    start       Start the API service
    stop        Stop the API service
    restart     Restart the API service
    status      Check service status
    logs        View service logs
    health      Check API health
    config      Validate configuration

Options:
    -c, --config FILE    Use custom config file (default: .env)
    -d, --daemon         Run as daemon (for start command)
    -f, --follow         Follow logs (for logs command)
    -h, --help           Show this help message

Environment Variables:
    API_HOST             API host (default: 0.0.0.0)
    API_PORT             API port (default: 8080)
    METRICS_PORT         Metrics port (default: 9090)
    DATABASE_URL         PostgreSQL connection URL
    ADMIN_API_KEY        Admin API key (optional)
    RUST_LOG             Logging level (default: info)
EOF
}

load_config() {
    if [ -f "$CONFIG_FILE" ]; then
        echo -e "${GREEN}Loading configuration from $CONFIG_FILE${NC}"
        set -a
        source "$CONFIG_FILE"
        set +a
    fi
    
    # Set defaults if not provided
    export API_HOST="${API_HOST:-0.0.0.0}"
    export API_PORT="${API_PORT:-8080}"
    export METRICS_PORT="${METRICS_PORT:-9090}"
    export RUST_LOG="${RUST_LOG:-almanac_api=info,tower_http=debug}"
}

check_prerequisites() {
    # Check if API binary exists
    if ! command -v almanac-api &> /dev/null; then
        # Try to build it
        echo -e "${YELLOW}API binary not found. Building...${NC}"
        cd "$PROJECT_ROOT"
        cargo build -p almanac-api --release --bin almanac-api
        
        # Add to PATH if built
        export PATH="$PROJECT_ROOT/target/release:$PATH"
    fi
    
    # Check PostgreSQL connection if DATABASE_URL is set
    if [ -n "${DATABASE_URL:-}" ]; then
        if ! psql "$DATABASE_URL" -c "SELECT 1" &> /dev/null; then
            echo -e "${RED}Cannot connect to PostgreSQL at $DATABASE_URL${NC}"
            echo "Please ensure PostgreSQL is running and accessible."
            exit 1
        fi
    fi
}

start_service() {
    local daemon_mode=false
    
    # Parse arguments
    while [[ $# -gt 0 ]]; do
        case $1 in
            -d|--daemon)
                daemon_mode=true
                shift
                ;;
            *)
                shift
                ;;
        esac
    done
    
    load_config
    check_prerequisites
    
    # Check if already running
    if [ -f "$PID_FILE" ] && kill -0 "$(cat "$PID_FILE")" 2>/dev/null; then
        echo -e "${YELLOW}Service is already running (PID: $(cat "$PID_FILE"))${NC}"
        return 0
    fi
    
    # Create data directory
    mkdir -p "$(dirname "$PID_FILE")"
    mkdir -p "$(dirname "$LOG_FILE")"
    
    echo -e "${GREEN}Starting Almanac API service...${NC}"
    echo "API endpoint: http://$API_HOST:$API_PORT"
    echo "Metrics endpoint: http://$API_HOST:$METRICS_PORT/metrics"
    
    if [ "$daemon_mode" = true ]; then
        # Start as daemon
        nohup almanac-api >> "$LOG_FILE" 2>&1 &
        local pid=$!
        echo $pid > "$PID_FILE"
        
        # Wait a bit and check if it started successfully
        sleep 2
        if kill -0 $pid 2>/dev/null; then
            echo -e "${GREEN}Service started successfully (PID: $pid)${NC}"
            echo "Logs: $LOG_FILE"
        else
            echo -e "${RED}Service failed to start${NC}"
            rm -f "$PID_FILE"
            tail -n 20 "$LOG_FILE"
            exit 1
        fi
    else
        # Start in foreground
        exec almanac-api
    fi
}

stop_service() {
    if [ ! -f "$PID_FILE" ]; then
        echo -e "${YELLOW}Service is not running (no PID file found)${NC}"
        return 0
    fi
    
    local pid=$(cat "$PID_FILE")
    
    if ! kill -0 $pid 2>/dev/null; then
        echo -e "${YELLOW}Service is not running (stale PID file)${NC}"
        rm -f "$PID_FILE"
        return 0
    fi
    
    echo -e "${GREEN}Stopping Almanac API service (PID: $pid)...${NC}"
    
    # Try graceful shutdown first
    kill -TERM $pid
    
    # Wait up to 10 seconds for graceful shutdown
    local count=0
    while kill -0 $pid 2>/dev/null && [ $count -lt 10 ]; do
        sleep 1
        count=$((count + 1))
    done
    
    # Force kill if still running
    if kill -0 $pid 2>/dev/null; then
        echo -e "${YELLOW}Force stopping service...${NC}"
        kill -KILL $pid
    fi
    
    rm -f "$PID_FILE"
    echo -e "${GREEN}Service stopped${NC}"
}

restart_service() {
    stop_service
    sleep 1
    start_service "$@"
}

service_status() {
    if [ ! -f "$PID_FILE" ]; then
        echo -e "${RED}Service is not running (no PID file)${NC}"
        return 1
    fi
    
    local pid=$(cat "$PID_FILE")
    
    if kill -0 $pid 2>/dev/null; then
        echo -e "${GREEN}Service is running (PID: $pid)${NC}"
        
        # Check API health
        if curl -sf "http://localhost:${API_PORT:-8080}/health" > /dev/null; then
            echo -e "${GREEN}API health check: OK${NC}"
        else
            echo -e "${YELLOW}API health check: FAILED${NC}"
        fi
        
        # Show resource usage
        if command -v ps &> /dev/null; then
            echo -e "\nResource usage:"
            ps -p $pid -o pid,vsz,rss,pcpu,pmem,comm
        fi
    else
        echo -e "${RED}Service is not running (stale PID file)${NC}"
        rm -f "$PID_FILE"
        return 1
    fi
}

view_logs() {
    local follow=false
    
    # Parse arguments
    while [[ $# -gt 0 ]]; do
        case $1 in
            -f|--follow)
                follow=true
                shift
                ;;
            *)
                shift
                ;;
        esac
    done
    
    if [ ! -f "$LOG_FILE" ]; then
        echo -e "${YELLOW}No log file found at $LOG_FILE${NC}"
        return 1
    fi
    
    if [ "$follow" = true ]; then
        tail -f "$LOG_FILE"
    else
        tail -n 100 "$LOG_FILE"
    fi
}

check_health() {
    load_config
    
    local api_url="http://localhost:${API_PORT}/health"
    local metrics_url="http://localhost:${METRICS_PORT}/metrics"
    
    echo "Checking API health..."
    
    # Check main API
    if response=$(curl -sf "$api_url"); then
        echo -e "${GREEN}API Health: OK${NC}"
        echo "  Response: $response"
    else
        echo -e "${RED}API Health: FAILED${NC}"
        echo "  Could not reach $api_url"
    fi
    
    # Check metrics endpoint
    if curl -sf "$metrics_url" > /dev/null; then
        echo -e "${GREEN}Metrics: OK${NC}"
        echo "  Endpoint: $metrics_url"
    else
        echo -e "${YELLOW}Metrics: Not available${NC}"
    fi
    
    # Check database connection
    if [ -n "${DATABASE_URL:-}" ]; then
        if psql "$DATABASE_URL" -c "SELECT 1" &> /dev/null; then
            echo -e "${GREEN}Database: Connected${NC}"
        else
            echo -e "${RED}Database: Connection failed${NC}"
        fi
    else
        echo -e "${YELLOW}Database: Not configured${NC}"
    fi
}

validate_config() {
    echo "Validating configuration..."
    
    load_config
    
    # Required variables
    local required_vars=()
    local optional_vars=("DATABASE_URL" "ADMIN_API_KEY")
    
    # Check environment variables
    echo -e "\nEnvironment Variables:"
    echo "  API_HOST=$API_HOST"
    echo "  API_PORT=$API_PORT"
    echo "  METRICS_PORT=$METRICS_PORT"
    echo "  RUST_LOG=$RUST_LOG"
    
    for var in "${optional_vars[@]}"; do
        if [ -n "${!var:-}" ]; then
            echo "  $var=<set>"
        else
            echo "  $var=<not set>"
        fi
    done
    
    # Test port availability
    for port in $API_PORT $METRICS_PORT; do
        if nc -z localhost $port 2>/dev/null; then
            echo -e "${YELLOW}Port $port is already in use${NC}"
        else
            echo -e "${GREEN}Port $port is available${NC}"
        fi
    done
    
    echo -e "\n${GREEN}Configuration is valid${NC}"
}

# Main command handling
case "${1:-}" in
    start)
        shift
        start_service "$@"
        ;;
    stop)
        stop_service
        ;;
    restart)
        shift
        restart_service "$@"
        ;;
    status)
        service_status
        ;;
    logs)
        shift
        view_logs "$@"
        ;;
    health)
        check_health
        ;;
    config)
        validate_config
        ;;
    -h|--help|help)
        print_usage
        ;;
    *)
        echo -e "${RED}Unknown command: ${1:-<none>}${NC}"
        print_usage
        exit 1
        ;;
esac