#!/usr/bin/env bash
# Almanac API Health Monitoring Script
# This script monitors the API health and can be used with monitoring systems

set -euo pipefail

# Check if running in Nix shell
if [ -z "${IN_NIX_SHELL:-}" ]; then
    exec nix develop --command "$0" "$@"
fi

# Configuration
API_HOST="${API_HOST:-localhost}"
API_PORT="${API_PORT:-8080}"
METRICS_PORT="${METRICS_PORT:-9090}"
CHECK_INTERVAL="${CHECK_INTERVAL:-30}"
ALERT_WEBHOOK="${ALERT_WEBHOOK:-}"
LOG_FILE="${LOG_FILE:-/tmp/almanac-api-health.log}"

# Monitoring state
CONSECUTIVE_FAILURES=0
MAX_FAILURES=3
LAST_ALERT_TIME=0
ALERT_COOLDOWN=300  # 5 minutes

# Functions
log_message() {
    local level=$1
    shift
    local message="$@"
    local timestamp=$(date '+%Y-%m-%d %H:%M:%S')
    echo "[$timestamp] [$level] $message" | tee -a "$LOG_FILE"
}

send_alert() {
    local message=$1
    local current_time=$(date +%s)
    
    # Check cooldown
    if [ $((current_time - LAST_ALERT_TIME)) -lt $ALERT_COOLDOWN ]; then
        return
    fi
    
    # Send webhook if configured
    if [ -n "$ALERT_WEBHOOK" ]; then
        curl -sf -X POST "$ALERT_WEBHOOK" \
            -H "Content-Type: application/json" \
            -d "{\"text\": \"Almanac API Alert: $message\"}" || \
            log_message "ERROR" "Failed to send webhook alert"
    fi
    
    LAST_ALERT_TIME=$current_time
}

check_endpoint() {
    local name=$1
    local url=$2
    local expected_response=$3
    
    if response=$(curl -sf -w "\\n%{http_code}" "$url" 2>/dev/null); then
        http_code=$(echo "$response" | tail -n1)
        body=$(echo "$response" | head -n-1)
        
        if [ "$http_code" = "200" ]; then
            if [ -n "$expected_response" ] && [[ ! "$body" =~ $expected_response ]]; then
                log_message "WARN" "$name check passed but unexpected response: $body"
                return 1
            fi
            return 0
        else
            log_message "ERROR" "$name returned HTTP $http_code"
            return 1
        fi
    else
        log_message "ERROR" "$name is unreachable"
        return 1
    fi
}

check_api_health() {
    local all_checks_passed=true
    
    # Check main health endpoint
    if check_endpoint "Health" "http://$API_HOST:$API_PORT/health" '"status":"ok"'; then
        log_message "INFO" "Health check: OK"
    else
        all_checks_passed=false
    fi
    
    # Check liveness
    if check_endpoint "Liveness" "http://$API_HOST:$API_PORT/liveness" '"status":"alive"'; then
        log_message "INFO" "Liveness check: OK"
    else
        all_checks_passed=false
    fi
    
    # Check readiness
    if check_endpoint "Readiness" "http://$API_HOST:$API_PORT/readiness" '"status":"ready"'; then
        log_message "INFO" "Readiness check: OK"
    else
        all_checks_passed=false
    fi
    
    # Check metrics endpoint
    if curl -sf "http://$API_HOST:$METRICS_PORT/metrics" > /dev/null 2>&1; then
        log_message "INFO" "Metrics endpoint: OK"
    else
        log_message "WARN" "Metrics endpoint is unreachable"
    fi
    
    # Check API functionality (list events)
    if check_endpoint "API Events" "http://$API_HOST:$API_PORT/api/v1/events" '"success":true'; then
        log_message "INFO" "API functionality: OK"
    else
        all_checks_passed=false
    fi
    
    # Update failure counter
    if [ "$all_checks_passed" = true ]; then
        if [ $CONSECUTIVE_FAILURES -gt 0 ]; then
            log_message "INFO" "Service recovered after $CONSECUTIVE_FAILURES failures"
            send_alert "Service recovered"
        fi
        CONSECUTIVE_FAILURES=0
    else
        CONSECUTIVE_FAILURES=$((CONSECUTIVE_FAILURES + 1))
        
        if [ $CONSECUTIVE_FAILURES -ge $MAX_FAILURES ]; then
            log_message "ERROR" "Service is down ($CONSECUTIVE_FAILURES consecutive failures)"
            send_alert "Service is down after $CONSECUTIVE_FAILURES consecutive failures"
        fi
    fi
}

collect_metrics() {
    # Collect and log key metrics
    local metrics_url="http://$API_HOST:$METRICS_PORT/metrics"
    
    if metrics=$(curl -sf "$metrics_url" 2>/dev/null); then
        # Extract key metrics
        local http_requests=$(echo "$metrics" | grep -E '^http_requests_total' | awk '{print $2}' || echo "0")
        local http_errors=$(echo "$metrics" | grep -E '^http_requests_errors_total' | awk '{print $2}' || echo "0")
        local response_time=$(echo "$metrics" | grep -E '^http_request_duration_seconds{.*quantile="0.99"' | awk '{print $2}' || echo "0")
        
        log_message "METRICS" "requests=$http_requests errors=$http_errors p99_response_time=$response_time"
    fi
}

print_usage() {
    cat << EOF
Almanac API Health Monitoring

Usage: $0 [options]

Options:
    -h, --host HOST              API host (default: localhost)
    -p, --port PORT              API port (default: 8080)
    -m, --metrics-port PORT      Metrics port (default: 9090)
    -i, --interval SECONDS       Check interval (default: 30)
    -w, --webhook URL            Alert webhook URL
    -1, --once                   Run once and exit
    --help                       Show this help message

Environment Variables:
    API_HOST                     API host
    API_PORT                     API port
    METRICS_PORT                 Metrics port
    CHECK_INTERVAL               Check interval in seconds
    ALERT_WEBHOOK                Webhook URL for alerts
    LOG_FILE                     Log file path

Examples:
    # Basic monitoring
    $0
    
    # Monitor remote host
    $0 --host api.example.com --port 443
    
    # One-time health check
    $0 --once
    
    # With alerts
    $0 --webhook https://hooks.slack.com/services/YOUR/WEBHOOK/URL
EOF
}

# Parse command line arguments
RUN_ONCE=false

while [[ $# -gt 0 ]]; do
    case $1 in
        -h|--host)
            API_HOST="$2"
            shift 2
            ;;
        -p|--port)
            API_PORT="$2"
            shift 2
            ;;
        -m|--metrics-port)
            METRICS_PORT="$2"
            shift 2
            ;;
        -i|--interval)
            CHECK_INTERVAL="$2"
            shift 2
            ;;
        -w|--webhook)
            ALERT_WEBHOOK="$2"
            shift 2
            ;;
        -1|--once)
            RUN_ONCE=true
            shift
            ;;
        --help)
            print_usage
            exit 0
            ;;
        *)
            echo "Unknown option: $1"
            print_usage
            exit 1
            ;;
    esac
done

# Main monitoring loop
log_message "INFO" "Starting Almanac API health monitoring"
log_message "INFO" "Target: http://$API_HOST:$API_PORT"
log_message "INFO" "Check interval: ${CHECK_INTERVAL}s"

if [ -n "$ALERT_WEBHOOK" ]; then
    log_message "INFO" "Alerts enabled via webhook"
fi

# Signal handler for graceful shutdown
trap 'log_message "INFO" "Monitoring stopped"; exit 0' SIGINT SIGTERM

if [ "$RUN_ONCE" = true ]; then
    check_api_health
    collect_metrics
else
    while true; do
        check_api_health
        collect_metrics
        sleep "$CHECK_INTERVAL"
    done
fi