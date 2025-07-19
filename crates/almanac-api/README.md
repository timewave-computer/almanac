# almanac-api

HTTP API server for the Almanac multi-chain blockchain indexing system.

## Overview

This crate provides a clean, modular HTTP API server built on Axum with minimal dependencies and clear separation of concerns. It serves as the primary interface for querying indexed blockchain data, managing layouts, and accessing extended storage features.

## Key Components

### Server Core
- `ApiServer` - Main HTTP server implementation
- `ServerConfig` - Server configuration and settings
- `AppState` - Application state management
- `AppConfig` - Environment-based configuration

### API Handlers
- `handlers` - HTTP request handlers for all endpoints
- RESTful API design with JSON responses
- Error handling with proper HTTP status codes
- Request validation and sanitization

### Authentication & Authorization
- `auth` - API key authentication
- Rate limiting and request throttling
- CORS configuration for web clients
- Admin-only endpoints protection

### Storage Integration
- `storage_ext` - Extended storage operations
- `StorageReader` - Read-only storage interface
- Support for both memory and FoundationDB backends
- Optimized query performance

### API Documentation
- `openapi` - OpenAPI/Swagger specification
- Interactive API documentation
- Type-safe request/response models
- Comprehensive endpoint documentation

## Features

### Core API Endpoints
- **Events**: Query blockchain events with rich filtering
- **Blocks**: Get latest block information per chain
- **Health**: Health check and system status
- **Metrics**: Prometheus metrics endpoint

### Extended Features
- **Layout Management**: Submit and query contract layouts
- **View Registry**: Register and query custom views
- **Storage Proofs**: Generate and verify storage proofs
- **Trust System**: Developer trust relationships
- **Semantic Metadata**: Contract field semantics

### Storage Backends
- **Memory**: In-memory storage for testing
- **FoundationDB**: Production-ready distributed storage
- Configurable backend selection
- Automatic failover and retry logic

## Usage

### Running the Server

```rust
use almanac_api::{AppConfig, run_server};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load configuration from environment
    let config = AppConfig::from_env()?;
    
    // Start the server
    run_server(config).await
}
```

### Configuration

Set environment variables:

```bash
# Basic configuration
export API_HOST=0.0.0.0
export API_PORT=8080
export METRICS_PORT=9090

# Storage configuration
export STORAGE_BACKEND=foundationdb  # or "memory"
export FDB_CLUSTER_FILE=/etc/foundationdb/fdb.cluster
export FDB_NAMESPACE=almanac

# Security configuration
export ADMIN_API_KEY=your-admin-key
export RATE_LIMIT_PER_MINUTE=100
export ENABLE_CORS=true
export CORS_ORIGINS=*

# Optional database URL for extended features
export DATABASE_URL=postgresql://user:pass@localhost/almanac
```

### API Usage Examples

#### Query Events
```bash
# Get recent events from Ethereum
curl "http://localhost:8080/api/v1/chains/ethereum/events?limit=10"

# Filter by event type and block range
curl "http://localhost:8080/api/v1/events?chain=ethereum&event_type=Transfer&from_block=1000&to_block=2000"

# Get events with specific transaction hash
curl "http://localhost:8080/api/v1/events?chain=ethereum&tx_hash=0x1234..."
```

#### Block Information
```bash
# Get latest block for a chain
curl "http://localhost:8080/api/v1/chains/ethereum/latest-block"

# Get latest block numbers for all chains
curl "http://localhost:8080/api/v1/chains/*/latest-block"
```

#### Semantic Metadata
```bash
# Submit semantic interpretation
curl -X POST "http://localhost:8080/api/v1/chains/ethereum/contracts/0x1234.../semantics/balance" \
  -H "Content-Type: application/json" \
  -H "X-API-Key: your-api-key" \
  -d '{
    "interpretation": {
      "semantic_tags": ["balance", "token"],
      "confidence": "high"
    }
  }'

# Get semantic interpretations
curl "http://localhost:8080/api/v1/chains/ethereum/contracts/0x1234.../semantics/balance"
```

#### Traverse Integration (Storage Proofs)
```bash
# Generate traverse request for storage proof
curl -X POST "http://localhost:8080/api/v1/traverse/request/ethereum/0x1234..." \
  -H "X-API-Key: your-api-key"

# Verify traverse witness
curl -X POST "http://localhost:8080/api/v1/traverse/verify" \
  -H "Content-Type: application/json" \
  -H "X-API-Key: your-api-key" \
  -d '{"witness_data": "..."}'
```

### Client Libraries

#### JavaScript/TypeScript
```typescript
import { AlmanacClient } from '@almanac/client';

const client = new AlmanacClient({
  baseUrl: 'http://localhost:8080',
  apiKey: 'your-api-key'
});

// Query events
const events = await client.events.query({
  chain: 'ethereum',
  eventType: 'Transfer',
  fromBlock: 1000,
  toBlock: 2000,
  limit: 100
});

// Get latest block
const latestBlock = await client.blocks.getLatest('ethereum');
```

#### Python
```python
from almanac_client import AlmanacClient

client = AlmanacClient(
    base_url='http://localhost:8080',
    api_key='your-api-key'
)

# Query events
events = await client.events.query(
    chain='ethereum',
    event_type='Transfer',
    from_block=1000,
    to_block=2000,
    limit=100
)

# Get latest block
latest_block = await client.blocks.get_latest('ethereum')
```

## API Reference

### Event Endpoints
- `GET /api/v1/events` - Query events with filtering
- `GET /api/v1/events/{event_id}` - Get specific event
- `GET /api/v1/chains/{chain}/events` - Query events for specific chain

### Block Endpoints
- `GET /api/v1/chains/{chain}/latest-block` - Get latest block for chain

### Semantic Metadata Endpoints
- `POST /api/v1/chains/{chain}/contracts/{address}/semantics/{field}` - Submit semantic interpretation
- `GET /api/v1/chains/{chain}/contracts/{address}/semantics/{field}` - Get semantic interpretations
- `PUT /api/v1/chains/{chain}/contracts/{address}/semantics/{field}` - Update semantic interpretation
- `DELETE /api/v1/chains/{chain}/contracts/{address}/semantics/{field}` - Delete semantic interpretation
- `POST /api/v1/chains/{chain}/contracts/{address}/semantics/{field}/resolve` - Resolve semantic conflict
- `GET /api/v1/semantics/conflicts/{chain}/{contract}` - Get semantic conflicts

### Traverse Integration Endpoints (Storage Proofs)
- `POST /api/v1/traverse/request/{chain}/{contract}` - Generate traverse request
- `POST /api/v1/traverse/verify` - Verify traverse witness
- `GET /api/v1/traverse/request/{request_id}` - Get traverse request status
- `GET /api/v1/traverse/requests/{chain}/{contract}` - List traverse requests
- `GET /api/v1/traverse/witness/{chain}/{contract}/{slot}` - Generate witness for slot
- `POST /api/v1/traverse/context/{request_id}` - Submit semantic context

### Trust Management Endpoints
- `POST /api/v1/trust/grant` - Grant trust to developer
- `POST /api/v1/trust/revoke` - Revoke trust from developer
- `GET /api/v1/trust/policy/{developer_id}` - Get trust policy
- `PUT /api/v1/trust/policy/{developer_id}` - Update trust policy
- `GET /api/v1/trust/relationships` - Get trust relationships
- `GET /api/v1/trust/check/{trustor}/{trustee}` - Check trust
- `GET /api/v1/trust/graph/{developer_id}` - Get trust graph

### Admin Endpoints
- `POST /admin/keys` - Create API key
- `GET /admin/keys` - List API keys
- `DELETE /admin/keys/{key}` - Delete API key
- `PATCH /admin/keys/{key}` - Update API key

### System Endpoints
- `GET /health` - Health check
- `GET /liveness` - Liveness probe
- `GET /readiness` - Readiness probe
- `GET /swagger-ui` - Interactive API documentation
- `GET /openapi.json` - OpenAPI specification

## Error Handling

The API returns standard HTTP status codes with JSON error responses:

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

### Error Codes
- `400 Bad Request` - Invalid request parameters
- `401 Unauthorized` - Missing or invalid API key
- `403 Forbidden` - Insufficient permissions
- `404 Not Found` - Resource not found
- `429 Too Many Requests` - Rate limit exceeded
- `500 Internal Server Error` - Server error

## Security

### Authentication
- API key authentication for write operations
- Rate limiting to prevent abuse
- CORS configuration for web clients
- Input validation and sanitization

### Authorization
- Role-based access control
- Admin-only endpoints protection
- Developer trust system
- Audit logging for sensitive operations

## Testing

Run tests with:
```bash
cargo test -p almanac-api
```

Integration tests with storage backends:
```bash
# Test with memory storage
cargo test -p almanac-api --features memory

# Test with FoundationDB
cargo test -p almanac-api --features fdb
```

## Performance

### Benchmarks
- 1000+ requests/second on single core
- Sub-10ms response times for simple queries
- Efficient pagination for large result sets
- Connection pooling for database operations

### Optimization
- Response caching for frequently accessed data
- Batch operations for multiple requests
- Streaming responses for large datasets
- Compression for API responses

## Deployment

### Docker
```dockerfile
FROM rust:1.75 as builder
WORKDIR /app
COPY . .
RUN cargo build --release --bin almanac-api

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/almanac-api /usr/local/bin/almanac-api
EXPOSE 8080
CMD ["almanac-api"]
```

### Kubernetes
```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: almanac-api
spec:
  replicas: 3
  selector:
    matchLabels:
      app: almanac-api
  template:
    metadata:
      labels:
        app: almanac-api
    spec:
      containers:
      - name: almanac-api
        image: almanac-api:latest
        ports:
        - containerPort: 8080
        env:
        - name: STORAGE_BACKEND
          value: "foundationdb"
        - name: FDB_CLUSTER_FILE
          value: "/etc/foundationdb/fdb.cluster"
```

## Monitoring

### Metrics
- Request/response metrics
- Error rates and latency
- Storage operation performance
- System resource usage

### Logging
- Structured logging with JSON format
- Request/response logging
- Error logging with stack traces
- Performance logging for slow operations

### Health Checks
- Database connectivity
- Storage backend health
- API endpoint availability
- System resource checks