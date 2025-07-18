#!/usr/bin/env bash

# Static Swagger UI Server for Almanac API
# This script serves static Swagger UI documentation when the API can't be built

set -euo pipefail

PORT="${1:-8080}"
HOST="${2:-127.0.0.1}"

# Colors for output
BLUE='\033[0;34m'
GREEN='\033[0;32m'
NC='\033[0m' # No Color

print_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

print_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

# Check if we have Python or other HTTP servers available
if command -v python3 &> /dev/null; then
    HTTP_SERVER="python3 -m http.server"
elif command -v python &> /dev/null; then
    HTTP_SERVER="python -m SimpleHTTPServer"
elif command -v caddy &> /dev/null; then
    HTTP_SERVER="caddy file-server"
else
    echo "No HTTP server found. Please install Python or Caddy."
    exit 1
fi

# Create a temporary directory for serving
TEMP_DIR=$(mktemp -d)
cd "$TEMP_DIR"

# Generate a basic HTML page with OpenAPI spec
cat > index.html << 'EOF'
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Almanac API - Swagger UI</title>
    <link rel="stylesheet" href="https://cdn.jsdelivr.net/npm/swagger-ui-dist@5/swagger-ui.css">
    <style>
        body { margin: 0; padding: 20px; font-family: Arial, sans-serif; }
        .header { text-align: center; margin-bottom: 30px; }
        .header h1 { color: #2c3e50; }
        .info { background: #f8f9fa; padding: 20px; border-radius: 8px; margin-bottom: 20px; }
        .endpoints { background: #fff; padding: 20px; border-radius: 8px; border: 1px solid #dee2e6; }
        .endpoint { margin-bottom: 15px; padding: 15px; background: #f8f9fa; border-radius: 5px; }
        .method { padding: 4px 8px; border-radius: 3px; color: white; font-weight: bold; }
        .get { background: #61affe; }
        .post { background: #49cc90; }
        .put { background: #fca130; }
        .delete { background: #f93e3e; }
        .code { background: #2d3748; color: #e2e8f0; padding: 20px; border-radius: 8px; overflow-x: auto; }
        .swagger-container { margin-top: 20px; }
    </style>
</head>
<body>
    <div class="header">
        <h1>Almanac API Documentation</h1>
        <p>High-Performance Multi-Chain Blockchain Indexer</p>
    </div>
    
    <div class="info">
        <h2>API Information</h2>
        <p><strong>Version:</strong> 1.0.0</p>
        <p><strong>Description:</strong> Cross-chain blockchain event indexing and querying system</p>
        <p><strong>Base URL:</strong> <code>http://localhost:8080/api/v1</code></p>
    </div>
    
    <div class="endpoints">
        <h2>Available Endpoints</h2>
        
        <div class="endpoint">
            <h3><span class="method get">GET</span> /api/v1/events</h3>
            <p>List events with optional filtering by chain, block range, event type, and address</p>
            <strong>Query Parameters:</strong>
            <ul>
                <li><code>chain</code> - Filter by chain identifier</li>
                <li><code>from_block</code> - Starting block number</li>
                <li><code>to_block</code> - Ending block number</li>
                <li><code>event_type</code> - Filter by event type</li>
                <li><code>address</code> - Filter by contract address</li>
                <li><code>limit</code> - Maximum number of results (default: 100)</li>
                <li><code>offset</code> - Pagination offset</li>
            </ul>
        </div>
        
        <div class="endpoint">
            <h3><span class="method get">GET</span> /api/v1/events/{id}</h3>
            <p>Get a single event by its unique ID</p>
            <strong>Path Parameters:</strong>
            <ul>
                <li><code>id</code> - Event ID</li>
            </ul>
        </div>
        
        <div class="endpoint">
            <h3><span class="method get">GET</span> /api/v1/chains/{chain}/events</h3>
            <p>List events for a specific blockchain</p>
            <strong>Path Parameters:</strong>
            <ul>
                <li><code>chain</code> - Chain identifier (e.g., "ethereum", "cosmos", "solana")</li>
            </ul>
        </div>
        
        <div class="endpoint">
            <h3><span class="method get">GET</span> /api/v1/chains/{chain}/latest-block</h3>
            <p>Get the latest block number for a specific chain</p>
            <strong>Path Parameters:</strong>
            <ul>
                <li><code>chain</code> - Chain identifier</li>
            </ul>
        </div>
        
        <div class="endpoint">
            <h3><span class="method get">GET</span> /health</h3>
            <p>Health check endpoint for monitoring</p>
        </div>
    </div>
    
    <div class="info">
        <h2>Usage Examples</h2>
        <div class="code">
# Get all events
curl http://localhost:8080/api/v1/events

# Get events for Ethereum chain
curl http://localhost:8080/api/v1/chains/ethereum/events

# Get events with filtering
curl "http://localhost:8080/api/v1/events?chain=ethereum&limit=10&from_block=1000"

# Get latest block for a chain
curl http://localhost:8080/api/v1/chains/ethereum/latest-block

# Check API health
curl http://localhost:8080/health
        </div>
    </div>
    
    <div class="info">
        <h2>Interactive Documentation</h2>
        <p>To access the full interactive Swagger UI, start the Almanac API server:</p>
        <div class="code">
# Start the API server
nix run .#swagger-ui

# Or manually:
cargo build --bin almanac-api
./target/debug/almanac-api --config config/dev.toml
        </div>
        <p>Then visit: <a href="http://localhost:8080/api/swagger-ui">http://localhost:8080/api/swagger-ui</a></p>
    </div>
</body>
</html>
EOF

# Clean up function
cleanup() {
    rm -rf "$TEMP_DIR"
}

trap cleanup EXIT

print_info "Starting static Swagger UI documentation server..."
print_info "Serving on: http://$HOST:$PORT"
print_success "Static documentation available at: http://$HOST:$PORT"

# Start the server
if [[ "$HTTP_SERVER" =~ python ]]; then
    $HTTP_SERVER $PORT
else
    $HTTP_SERVER --listen "$HOST:$PORT"
fi