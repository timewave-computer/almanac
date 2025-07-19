# Swagger UI Documentation

## Overview

The Almanac API includes comprehensive Swagger UI documentation that provides interactive API exploration and testing capabilities. This guide covers how to use the Nix-based Swagger UI server and access the documentation.

## Quick Start

### Using Nix (Recommended)

The simplest way to start the Swagger UI server is using the Nix flake:

```bash
# Start with default settings (port 8080)
nix run .#swagger-ui

# Start with custom port
nix run .#swagger-ui -- 8090

# Start with custom host and port
nix run .#swagger-ui -- 8080 0.0.0.0

# Start with custom config file
nix run .#swagger-ui -- 8080 127.0.0.1 config/prod.toml
```

### Alternative Methods

If you prefer to run the server manually:

```bash
# Run the script directly
./scripts/swagger-ui-server.sh

# Or with custom parameters
./scripts/swagger-ui-server.sh 8080 127.0.0.1 config/dev.toml

# Build as a package
nix build .#swagger-ui
./result/bin/almanac-swagger-ui
```

## Access Points

Once the server is running, you can access the documentation at:

| Interface | URL | Description |
|-----------|-----|-------------|
| **Swagger UI** | `http://localhost:8080/api/swagger-ui` | Interactive API explorer (recommended) |
| **OpenAPI Spec** | `http://localhost:8080/api/openapi.json` | Raw OpenAPI 3.0 specification |
| **ReDoc** | `http://localhost:8080/api/redoc` | Alternative documentation interface |
| **Health Check** | `http://localhost:8080/health` | Server health status |

## Features

- **Interactive Testing**: Try API endpoints directly in the browser
- **Automatic Documentation**: Generated from code, always up-to-date
- **Request Examples**: See formatted request/response examples
- **Schema Validation**: Real-time parameter validation
- **Authentication Support**: Test with API keys

## API Endpoints

### Events API

#### `GET /api/v1/events`
List events with optional filtering:

```bash
# Get all events
curl http://localhost:8080/api/v1/events

# Filter by chain
curl http://localhost:8080/api/v1/events?chain=ethereum

# Filter by block range
curl http://localhost:8080/api/v1/events?from_block=1000&to_block=2000

# Complex filtering
curl "http://localhost:8080/api/v1/events?chain=ethereum&event_type=Transfer&limit=50"
```

**Query Parameters:**
- `chain` - Filter by chain identifier
- `from_block` - Starting block number
- `to_block` - Ending block number
- `event_type` - Filter by event type
- `address` - Filter by contract address
- `limit` - Maximum results (default: 100)
- `offset` - Pagination offset

#### `GET /api/v1/events/{id}`
Get a single event by ID:

```bash
curl http://localhost:8080/api/v1/events/event-123
```

#### `GET /api/v1/chains/{chain}/events`
Get events for a specific chain:

```bash
curl http://localhost:8080/api/v1/chains/ethereum/events
curl http://localhost:8080/api/v1/chains/cosmos/events
```

#### `GET /api/v1/chains/{chain}/latest-block`
Get the latest block for a chain:

```bash
curl http://localhost:8080/api/v1/chains/ethereum/latest-block
```

## Configuration

### Server Configuration

The Swagger UI server accepts the following parameters:

```bash
./scripts/swagger-ui-server.sh [PORT] [HOST] [CONFIG_FILE]
```

- **PORT**: Server port (default: 8080)
- **HOST**: Server host (default: 127.0.0.1)
- **CONFIG_FILE**: Path to configuration file (default: config/dev.toml)

### Environment Variables

The server respects these environment variables:

- `RUST_LOG`: Log level (default: info)
- `ALMANAC_HOST`: Override host setting
- `ALMANAC_PORT`: Override port setting

### Configuration Files

The server supports TOML configuration files:

```toml
# config/dev.toml
[server]
host = "127.0.0.1"
port = 8080

[database]
url = "postgresql://localhost/almanac"

[logging]
level = "info"
```

## Development Integration

### Nix Flake Integration

The Swagger UI server is integrated into the Nix flake as both a package and an app:

```nix
# Run as an app
nix run .#swagger-ui

# Build as a package
nix build .#swagger-ui

# Include in development shell
nix develop
```

### Development Workflow

1. **Start Development Environment**:
   ```bash
   nix develop
   ```

2. **Start Required Services**:
   ```bash
   fdb-manage start  # Start FoundationDB
   ```

3. **Launch Swagger UI**:
   ```bash
   nix run .#swagger-ui
   ```

4. **Access Documentation**:
   - Open `http://localhost:8080/api/swagger-ui`
   - Test endpoints interactively
   - View generated schemas

## Extending Documentation

### Adding New Endpoints

1. **Add Handler**: Create handler function with `#[utoipa::path(...)]`
2. **Update Schema**: Add response types with `#[derive(ToSchema)]`
3. **Update OpenAPI**: Add path to `ApiDoc` struct
4. **Test**: Verify documentation updates automatically

### Improving Documentation

1. **Add Examples**: Include request/response examples
2. **Add Descriptions**: Provide detailed endpoint descriptions
3. **Add Tags**: Organize endpoints with tags
4. **Add Security**: Document authentication requirements

## Troubleshooting

- **Port conflicts**: Server automatically finds free ports
- **Build failures**: Falls back to debug build if release fails
- **Dependencies**: All handled by Nix automatically

For more information, see the [API Development Guide](./api-development.md).