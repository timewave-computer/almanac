#!/usr/bin/env bash
# Test the Almanac API with FoundationDB backend

set -euo pipefail

PROJECT_ROOT="${PROJECT_ROOT:-$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)}"
cd "$PROJECT_ROOT"

echo "=== Testing Almanac API with FoundationDB ==="

# Source FDB environment
source .env.fdb

# Set environment variables
export STORAGE_BACKEND=foundationdb
export FDB_CLUSTER_FILE="${FDB_CLUSTER_FILE:-$PROJECT_ROOT/data/fdb.cluster}"
export FDB_NAMESPACE=almanac
export API_PORT=8080
export API_HOST=127.0.0.1

echo "Configuration:"
echo "  Storage Backend: $STORAGE_BACKEND"
echo "  FDB Cluster File: $FDB_CLUSTER_FILE"
echo "  FDB Namespace: $FDB_NAMESPACE"
echo "  API Port: $API_PORT"

# Check if FDB is running
echo ""
echo "Checking FoundationDB status..."
if command -v fdbcli >/dev/null && fdbcli --cluster-file="$FDB_CLUSTER_FILE" --exec "status" 2>/dev/null | grep -q "Cluster is available"; then
    echo "FoundationDB is running"
else
    echo "FoundationDB is not running"
    echo "Starting FoundationDB..."
    nix run .#fdb-manage start
fi

# Build the API with FDB support
echo ""
echo "Building API with FDB support..."
cargo build -p almanac-api --features fdb

# Run the API server in background
echo ""
echo "Starting API server..."
cargo run -p almanac-api --features fdb &
API_PID=$!

# Wait for server to start
echo "Waiting for API server to start..."
sleep 5

# Function to cleanup on exit
cleanup() {
    echo ""
    echo "Shutting down API server..."
    kill $API_PID 2>/dev/null || true
}
trap cleanup EXIT

# Test API endpoints
echo ""
echo "Testing API endpoints..."

# Health check
echo "1. Health check:"
curl -s http://$API_HOST:$API_PORT/health | jq .

# Submit semantic interpretation
echo ""
echo "2. Submit semantic interpretation:"
curl -s -X POST http://$API_HOST:$API_PORT/v1/semantics/ethereum-mainnet/0x1234567890123456789012345678901234567890/balanceOf \
  -H "Content-Type: application/json" \
  -d '{
    "developer_id": "test-developer",
    "chain_id": "ethereum-mainnet",
    "contract_address": "0x1234567890123456789012345678901234567890",
    "field": "balanceOf",
    "interpretation": {
      "data_type": "uint256",
      "semantic_tags": ["balance", "token"],
      "zero_semantics": "never_written",
      "confidence_level": "high",
      "source": "contract_code",
      "metadata": {
        "description": "Token balance mapping"
      }
    }
  }' | jq .

# Get semantic interpretation
echo ""
echo "3. Get semantic interpretation:"
curl -s http://$API_HOST:$API_PORT/v1/semantics/ethereum-mainnet/0x1234567890123456789012345678901234567890/balanceOf | jq .

# Submit layout
echo ""
echo "4. Submit contract layout:"
curl -s -X POST http://$API_HOST:$API_PORT/v1/layouts \
  -H "Content-Type: application/json" \
  -d '{
    "chain_id": "ethereum-mainnet",
    "contract_address": "0x1234567890123456789012345678901234567890",
    "layout_commitment": "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
    "layout_data": {
      "slots": [
        {"slot": "0x0", "name": "owner", "type": "address"},
        {"slot": "0x1", "name": "totalSupply", "type": "uint256"}
      ]
    },
    "compiler_version": "0.8.19",
    "developer_id": "test-developer"
  }' | jq .

# Get layout
echo ""
echo "5. Get contract layout:"
curl -s http://$API_HOST:$API_PORT/v1/layouts/ethereum-mainnet/0x1234567890123456789012345678901234567890 | jq .

# Grant trust
echo ""
echo "6. Grant trust relationship:"
curl -s -X POST http://$API_HOST:$API_PORT/v1/trust/grant \
  -H "Content-Type: application/json" \
  -d '{
    "trustor": "alice",
    "trustee": "bob",
    "scope": {"type": "global"},
    "trust_level": "contribute"
  }' | jq .

# Check trust
echo ""
echo "7. Check trust relationship:"
curl -s http://$API_HOST:$API_PORT/v1/trust/check/alice/bob | jq .

echo ""
echo "API tests completed!"
echo ""
echo "API server is running at http://$API_HOST:$API_PORT"
echo "Press Ctrl+C to stop the server"

# Keep script running until interrupted
wait $API_PID