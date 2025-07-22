# Querying Guide

This guide covers how to integrate with Almanac as a principle and make queries against indexed blockchain data.

## Overview

Almanac provides a REST API for querying multi-chain blockchain data. The system uses a distributed architecture where chain-specific indexers feed data into a shared FoundationDB storage layer, which is then accessed through the almanac-api server.

## Architecture

```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   Client Apps   │    │   Web Dashboards│    │   CLI Tools     │
└─────────┬───────┘    └─────────┬───────┘    └─────────┬───────┘
          │                      │                      │
          └──────────────────────┼──────────────────────┘
                                 │
                    ┌────────────▼────────────┐
                    │     almanac-api         │
                    │   (REST API Server)     │
                    └────────────┬────────────┘
                                 │
                    ┌────────────▼────────────┐
                    │     FoundationDB        │
                    │   (Shared Storage)      │
                    └─────────────────────────┘
```

## Getting Started

### Prerequisites

- Access to an Almanac API instance
- API key for authentication (for write operations)
- HTTP client (curl, browser, or programming language client)

### Basic Setup

1. **Start the API Server**

For development, start the API server locally:

```bash
# Enter nix development environment (automatically sets up library paths)
nix develop

# Start FoundationDB
fdb-manage start

# Start the API server
cargo run -p almanac-api --features fdb
```

The API will be available at `http://localhost:8080`.

2. **Verify Connection**

Test that the API is responding:

```bash
curl http://localhost:8080/health
```

Expected response:
```json
{
  "status": "ok",
  "version": "1.0.0"
}
```

## Core Query Operations

### Event Queries

Events are the primary data type indexed by Almanac. They represent blockchain transactions, smart contract calls, and state changes.

#### List Events

Get recent events with basic filtering:

```bash
# Get last 10 events
curl "http://localhost:8080/api/v1/events?limit=10"

# Filter by chain
curl "http://localhost:8080/api/v1/events?chain=ethereum&limit=10"

# Filter by block range
curl "http://localhost:8080/api/v1/events?chain=ethereum&from_block=1000&to_block=2000"

# Filter by event type
curl "http://localhost:8080/api/v1/events?chain=ethereum&event_type=Transfer&limit=50"

# Filter by contract address
curl "http://localhost:8080/api/v1/events?chain=ethereum&address=0x1234...&limit=20"
```

#### Get Specific Event

Retrieve a single event by its ID:

```bash
curl "http://localhost:8080/api/v1/events/event-12345"
```

#### Chain-Specific Events

Query events for a specific blockchain:

```bash
curl "http://localhost:8080/api/v1/chains/ethereum/events?limit=10"
curl "http://localhost:8080/api/v1/chains/cosmos/events?event_type=bank.send"
```

### Block Information

Get information about the latest indexed blocks:

```bash
# Latest block for a specific chain
curl "http://localhost:8080/api/v1/chains/ethereum/latest-block"

# Response format:
{
  "chain": "ethereum",
  "latest_block": 18500000
}
```

### Query Parameters

Standard query parameters supported across endpoints:

- `chain` - Filter by blockchain (ethereum, cosmos, solana)
- `from_block` - Starting block number
- `to_block` - Ending block number
- `event_type` - Filter by event type
- `address` - Filter by contract address
- `limit` - Maximum results per page (default: 100)
- `offset` - Pagination offset

## Authentication

### API Key Authentication

For write operations and premium features, include your API key:

```bash
curl -H "X-API-Key: your-api-key" \
  "http://localhost:8080/api/v1/events?chain=ethereum"
```

### Setting API Key Environment Variable

```bash
export ALMANAC_API_KEY="your-api-key"
curl -H "X-API-Key: $ALMANAC_API_KEY" \
  "http://localhost:8080/api/v1/events"
```

## Advanced Features

### Admin API

Manage API keys:

```bash
# Create a new API key
curl -X POST "http://localhost:8080/admin/keys" \
  -H "Content-Type: application/json" \
  -d '{"name": "my-key", "rate_limit": 1000}'

# List all API keys
curl "http://localhost:8080/admin/keys"

# Delete an API key
curl -X DELETE "http://localhost:8080/admin/keys/ak_...

# Update an API key
curl -X PATCH "http://localhost:8080/admin/keys/ak_..." \
  -H "Content-Type: application/json" \
  -d '{"enabled": false}'
```

### Traverse API

Integration with Traverse ZK proofs:

```bash
# Generate a Traverse verification request
curl -X POST "http://localhost:8080/api/v1/traverse/request/ethereum/0x1234..." \
  -H "Content-Type: application/json" \
  -d '{
    "block_height": 12345678,
    "storage_slots": [
      {"storage_key": "0x00..."}
    ],
    "developer_id": "dev123"
  }'

# Verify a Traverse witness
curl -X POST "http://localhost:8080/api/v1/traverse/verify" \
  -H "Content-Type: application/json" \
  -d '{
    "request_id": "traverse-...",
    "witness_data": {},
    "proof": {},
    "developer_id": "dev123"
  }'
```

### Subscriptions API

Manage account subscriptions for real-time updates:

```bash
# Add accounts to a subscription
curl -X POST "http://localhost:8080/api/v1/subscriptions/accounts" \
  -H "Content-Type: application/json" \
  -d '{
    "accounts": [
      {"chain_id": "ethereum", "address": "0x..."}
    ]
  }'

# Get subscription statistics
curl "http://localhost:8080/api/v1/subscriptions/stats"
```

### Storage Proofs


Verify blockchain state with cryptographic proofs:

```bash
# Get storage proof for a specific contract state
curl "http://localhost:8080/api/v1/storage-proofs/ethereum/0x1234.../1000?storage_key=0x5678..."
```

### Contract Layouts

Submit and query contract storage layouts:

```bash
# Submit a layout (requires API key)
curl -X POST "http://localhost:8080/api/v1/chains/ethereum/contracts/0x1234.../layouts" \
  -H "Content-Type: application/json" \
  -H "X-API-Key: your-api-key" \
  -d '{
    "chain_id": "ethereum",
    "contract_address": "0x1234...",
    "developer_id": "dev123",
    "layout": {
      "fields": [
        {"name": "balance", "type": "uint256", "offset": 0},
        {"name": "owner", "type": "address", "offset": 32}
      ]
    }
  }'

# Query a layout
curl "http://localhost:8080/api/v1/chains/ethereum/contracts/0x1234.../layouts"
```

### Trust Management

Manage developer trust relationships:

```bash
# Grant trust to a developer (requires API key)
curl -X POST "http://localhost:8080/api/v1/trust/grant" \
  -H "Content-Type: application/json" \
  -H "X-API-Key: your-api-key" \
  -d '{
    "trustor": "alice",
    "trustee": "bob", 
    "scope": {"type": "global"},
    "trust_level": "contribute"
  }'

# Check trust relationships
curl "http://localhost:8080/api/v1/trust/relationships"
```

### Semantic Metadata

Query semantic interpretations of contract fields:

```bash
# Get semantic interpretation
curl "http://localhost:8080/api/v1/chains/ethereum/contracts/0x1234.../semantics/balanceOf?trust_policy=permissive"

# Submit semantic interpretation (requires API key)
curl -X POST "http://localhost:8080/api/v1/chains/ethereum/contracts/0x1234.../semantics/balanceOf" \
  -H "Content-Type: application/json" \
  -H "X-API-Key: your-api-key" \
  -d '{
    "developer_id": "dev123",
    "interpretation": {
      "data_type": "uint256",
      "semantic_tags": ["balance", "token"],
      "zero_semantics": "never_written",
      "confidence_level": "high"
    }
  }'
```

## Client Integration

### JavaScript/TypeScript

```javascript
const ALMANAC_BASE_URL = 'http://localhost:8080';
const API_KEY = 'your-api-key';

async function queryEvents(chain, limit = 10) {
  const response = await fetch(
    `${ALMANAC_BASE_URL}/api/v1/events?chain=${chain}&limit=${limit}`,
    {
      headers: {
        'X-API-Key': API_KEY
      }
    }
  );
  return response.json();
}

async function getLatestBlock(chain) {
  const response = await fetch(
    `${ALMANAC_BASE_URL}/api/v1/chains/${chain}/latest-block`
  );
  return response.json();
}

// Usage
const ethereumEvents = await queryEvents('ethereum', 50);
const latestBlock = await getLatestBlock('ethereum');
```

### Python

```python
import requests

class AlmanacClient:
    def __init__(self, base_url, api_key=None):
        self.base_url = base_url
        self.headers = {}
        if api_key:
            self.headers['X-API-Key'] = api_key
    
    def query_events(self, chain=None, limit=10, **filters):
        params = {'limit': limit}
        if chain:
            params['chain'] = chain
        params.update(filters)
        
        response = requests.get(
            f"{self.base_url}/api/v1/events",
            params=params,
            headers=self.headers
        )
        response.raise_for_status()
        return response.json()
    
    def get_latest_block(self, chain):
        response = requests.get(
            f"{self.base_url}/api/v1/chains/{chain}/latest-block",
            headers=self.headers
        )
        response.raise_for_status()
        return response.json()

# Usage
client = AlmanacClient('http://localhost:8080', 'your-api-key')
events = client.query_events(chain='ethereum', event_type='Transfer', limit=50)
latest_block = client.get_latest_block('ethereum')
```

### Go

```go
package main

import (
    "encoding/json"
    "fmt"
    "net/http"
    "net/url"
)

type AlmanacClient struct {
    BaseURL string
    APIKey  string
    Client  *http.Client
}

func NewAlmanacClient(baseURL, apiKey string) *AlmanacClient {
    return &AlmanacClient{
        BaseURL: baseURL,
        APIKey:  apiKey,
        Client:  &http.Client{},
    }
}

func (c *AlmanacClient) QueryEvents(chain string, limit int) (map[string]interface{}, error) {
    u, _ := url.Parse(c.BaseURL + "/api/v1/events")
    q := u.Query()
    if chain != "" {
        q.Set("chain", chain)
    }
    q.Set("limit", fmt.Sprintf("%d", limit))
    u.RawQuery = q.Encode()

    req, _ := http.NewRequest("GET", u.String(), nil)
    if c.APIKey != "" {
        req.Header.Set("X-API-Key", c.APIKey)
    }

    resp, err := c.Client.Do(req)
    if err != nil {
        return nil, err
    }
    defer resp.Body.Close()

    var result map[string]interface{}
    json.NewDecoder(resp.Body).Decode(&result)
    return result, nil
}

func (c *AlmanacClient) GetLatestBlock(chain string) (map[string]interface{}, error) {
    url := fmt.Sprintf("%s/api/v1/chains/%s/latest-block", c.BaseURL, chain)
    req, _ := http.NewRequest("GET", url, nil)
    
    resp, err := c.Client.Do(req)
    if err != nil {
        return nil, err
    }
    defer resp.Body.Close()

    var result map[string]interface{}
    json.NewDecoder(resp.Body).Decode(&result)
    return result, nil
}
```

## Interactive API Exploration

### Swagger UI

For interactive API exploration, use the built-in Swagger UI:

```bash
# Start Swagger UI server
nix run .#swagger-ui --allow-dirty

# Access at: http://localhost:8080/swagger-ui
```

The Swagger UI provides:
- Interactive endpoint testing
- Request/response examples
- Parameter documentation
- Authentication setup

### OpenAPI Specification

Get the machine-readable API specification:

```bash
curl "http://localhost:8080/api/openapi.json"
```

## Error Handling

The API returns standard HTTP status codes with JSON error responses:

### Common Status Codes

- `200 OK` - Request successful
- `400 Bad Request` - Invalid request parameters
- `401 Unauthorized` - Missing or invalid API key
- `403 Forbidden` - Insufficient permissions
- `404 Not Found` - Resource not found
- `429 Too Many Requests` - Rate limit exceeded
- `500 Internal Server Error` - Server error

### Error Response Format

```json
{
  "error": "validation_error",
  "message": "Invalid block number",
  "details": {
    "field": "block_number", 
    "value": -1,
    "constraint": "must be non-negative"
  }
}
```

## Rate Limiting

The API implements rate limiting to prevent abuse:

- Default: 100 requests per minute per API key
- Rate limit headers included in responses:
  - `X-RateLimit-Limit` - Request limit per window
  - `X-RateLimit-Remaining` - Remaining requests
  - `X-RateLimit-Reset` - Window reset time

## Best Practices

### Query Optimization

1. **Use Specific Filters**: Always filter by chain when possible
2. **Limit Result Size**: Use reasonable limit values (100-1000)
3. **Paginate Large Results**: Use offset for pagination
4. **Cache Responses**: Cache frequently accessed data
5. **Batch Requests**: Group related queries when possible

### Security

1. **Protect API Keys**: Never expose API keys in client-side code
2. **Use HTTPS**: Always use encrypted connections in production
3. **Validate Responses**: Verify response structure and data types
4. **Handle Errors**: Implement proper error handling and retries

### Performance

1. **Connection Pooling**: Reuse HTTP connections
2. **Compression**: Enable gzip compression for large responses
3. **Monitoring**: Track query performance and error rates
4. **Caching**: Implement client-side caching for static data

## Troubleshooting

### Common Issues

**Connection Refused**
```bash
curl: (7) Failed to connect to localhost port 8080
```
Solution: Ensure the almanac-api server is running

**Authentication Failed**
```json
{"error": "unauthorized", "message": "Invalid API key"}
```
Solution: Check your API key and header format

**Rate Limited**
```json
{"error": "rate_limited", "message": "Too many requests"}
```
Solution: Reduce request frequency or upgrade API plan

**Invalid Parameters**
```json
{"error": "validation_error", "message": "Invalid chain"}
```
Solution: Check parameter values and API documentation

### Debugging

Enable verbose output for debugging:

```bash
# Curl with verbose output
curl -v "http://localhost:8080/api/v1/events"

# Check API server logs
# (logs appear in terminal where almanac-api is running)
```

## Support

For additional help:

- Check the OpenAPI specification at `/api/openapi.json`
- Use Swagger UI for interactive testing
- Review the source code in `crates/almanac-api/`
- Check integration tests in `crates/almanac-api/tests/`