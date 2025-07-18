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
curl "http://localhost:8080/api/events?chain=ethereum&limit=10"

# Filter by event type and block range
curl "http://localhost:8080/api/events?chain=ethereum&event_type=Transfer&from_block=1000&to_block=2000"

# Get events with specific transaction hash
curl "http://localhost:8080/api/events?chain=ethereum&tx_hash=0x1234..."
```

#### Block Information
```bash
# Get latest block for a chain
curl "http://localhost:8080/api/blocks/ethereum/latest"

# Get latest block numbers for all chains
curl "http://localhost:8080/api/blocks/latest"
```

#### Layout Management
```bash
# Submit a contract layout
curl -X POST "http://localhost:8080/api/layouts" \
  -H "Content-Type: application/json" \
  -H "X-API-Key: your-api-key" \
  -d '{
    "chain": "ethereum",
    "contract": "0x1234...",
    "developer_id": "dev123",
    "layout": {
      "fields": [
        {"name": "balance", "type": "uint256", "offset": 0},
        {"name": "owner", "type": "address", "offset": 32}
      ]
    }
  }'

# Get contract layout
curl "http://localhost:8080/api/layouts/ethereum/0x1234..."
```

#### Storage Proofs
```bash
# Get storage proof for a specific key
curl "http://localhost:8080/api/storage-proofs/ethereum/0x1234.../1000?storage_key=0x5678..."
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
- `GET /api/events` - Query events with filtering
- `GET /api/events/{event_id}` - Get specific event
- `GET /api/events/count` - Get event count with filtering

### Block Endpoints
- `GET /api/blocks/{chain}/latest` - Get latest block for chain
- `GET /api/blocks/latest` - Get latest blocks for all chains
- `GET /api/blocks/{chain}/{block_number}` - Get specific block

### Layout Endpoints
- `POST /api/layouts` - Submit contract layout
- `GET /api/layouts/{chain}/{contract}` - Get contract layout
- `GET /api/layouts/{chain}/{contract}/versions` - Get layout versions

### View Endpoints
- `POST /api/views` - Register custom view
- `GET /api/views/{view_id}` - Get view definition
- `PUT /api/views/{view_id}` - Update view definition
- `DELETE /api/views/{view_id}` - Delete view
- `POST /api/views/{view_id}/query` - Query view data

### Storage Proof Endpoints
- `GET /api/storage-proofs/{chain}/{contract}/{block}` - Get storage proof
- `POST /api/storage-proofs/verify` - Verify storage proof

### Trust Endpoints
- `POST /api/trust/grant` - Grant trust to developer
- `POST /api/trust/revoke` - Revoke trust from developer
- `GET /api/trust/{developer_id}` - Get trust relationships

### System Endpoints
- `GET /api/health` - Health check
- `GET /api/metrics` - Prometheus metrics
- `GET /api/openapi.json` - OpenAPI specification

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