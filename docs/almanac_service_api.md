# Almanac Service API Documentation

This document describes how to interact with Almanac as a standalone service or process through its external APIs.

> **Note**: The REST API has been implemented and is available alongside the GraphQL API. Both APIs run simultaneously when you start the almanac service.

## Overview

Almanac can run as a standalone service, providing access to blockchain data through:

1. **HTTP API** - RESTful endpoints for querying indexed data
2. **WebSocket API** - For real-time event subscriptions
3. **Command Line Interface (CLI)** - For direct interaction and administration

## Getting Started

### Prerequisites

- Almanac service installed and running
- Access to the service endpoint(s)

## HTTP API

Almanac exposes a RESTful API for querying indexed blockchain data.

### Base URL

All API endpoints are prefixed with `/api/v1`.

Example: `http://localhost:8000/api/v1/`

### Authentication

Bearer token authentication is supported. Include the token in the `Authorization` header:

```
Authorization: Bearer <your_token>
```

### Endpoints

#### Get Chain Status

```
GET /chains/{chain_id}/status
```

Returns the indexing status of a specific blockchain.

Example request:
```bash
curl -X GET http://localhost:8000/api/v1/chains/ethereum/status
```

Example response:
```json
{
  "chain_id": "ethereum",
  "latest_height": 18256432,
  "finalized_height": 18256400,
  "is_indexing": true,
  "last_indexed_at": 1684231456,
  "error": null
}
```

#### Get Events by Address

```
GET /events/address/{chain_id}/{address}
```

Query parameters:
- `limit` (optional): Maximum number of events to return (default: 100)
- `offset` (optional): Pagination offset
- `ascending` (optional): Sort order (default: false, meaning newest first)

Example request:
```bash
curl -X GET "http://localhost:8000/api/v1/events/address/ethereum/0x1234567890abcdef1234567890abcdef12345678?limit=10"
```

Example response:
```json
{
  "events": [
    {
      "chain_id": "ethereum",
      "height": 18256432,
      "tx_hash": "0xabcdef...",
      "index": 2,
      "address": "0x1234567890abcdef1234567890abcdef12345678",
      "event_type": "Transfer",
      "attributes": {
        "from": "0x...",
        "to": "0x...",
        "value": "1000000000000000000"
      },
      "timestamp": 1684231450
    },
    // More events...
  ],
  "pagination": {
    "total": 583,
    "limit": 10,
    "offset": 0,
    "has_more": true
  }
}
```

#### Get Events by Chain

```
GET /events/chain/{chain_id}
```

Query parameters:
- `from_height` (optional): Start block height
- `to_height` (optional): End block height
- `limit` (optional): Maximum number of events to return (default: 100)
- `offset` (optional): Pagination offset
- `ascending` (optional): Sort order (default: true)

Example request:
```bash
curl -X GET "http://localhost:8000/api/v1/events/chain/cosmos?from_height=15000000&to_height=15001000&limit=50"
```

Example response: (similar to the address endpoint response)

#### Filter Events

```
POST /events/filter
```

Request body:
```json
{
  "chain_id": "ethereum",
  "address": "0x1234567890abcdef1234567890abcdef12345678",
  "event_type": "Transfer",
  "attributes": {
    "to": "0xabcdef1234567890abcdef1234567890abcdef12"
  },
  "from_height": 18000000,
  "limit": 20
}
```

Example request:
```bash
curl -X POST "http://localhost:8000/api/v1/events/filter" \
  -H "Content-Type: application/json" \
  -d '{"chain_id":"ethereum","event_type":"Transfer","attributes":{"to":"0xabcdef1234567890abcdef1234567890abcdef12"},"limit":20}'
```

## WebSocket API

Almanac provides WebSocket endpoints for real-time event subscriptions.

### Base WebSocket URL

```
ws://localhost:8000/api/v1/ws
```

### Subscribing to Events

Connect to the WebSocket endpoint and send a subscription message:

```json
{
  "action": "subscribe",
  "filter": {
    "chain_id": "ethereum",
    "event_type": "Transfer",
    "address": "0x1234567890abcdef1234567890abcdef12345678",
    "attributes": {
      "from": "0x..."
    }
  }
}
```

### Received Events

After subscribing, you'll receive events matching your filter in real-time:

```json
{
  "type": "event",
  "data": {
    "chain_id": "ethereum",
    "height": 18256542,
    "tx_hash": "0x...",
    "index": 3,
    "address": "0x1234567890abcdef1234567890abcdef12345678",
    "event_type": "Transfer",
    "attributes": {
      "from": "0x...",
      "to": "0x...",
      "value": "5000000000000000000"
    },
    "timestamp": 1684231890
  }
}
```

### Unsubscribing

To stop receiving events, send an unsubscribe message:

```json
{
  "action": "unsubscribe",
  "subscription_id": "sub_12345"
}
```

## Command Line Interface

Almanac provides a CLI for managing and interacting with the service.

### Installation

The CLI is included with the Almanac installation:

```bash
# Check if almanac CLI is installed
almanac --version
```

### Configuration

Create a configuration file at `~/.almanac/config.toml` or specify a custom path:

```toml
# ~/.almanac/config.toml
[service]
host = "localhost"
port = 8000

[database]
url = "postgresql://user:password@localhost:5432/almanac"

[indexer]
chains = ["ethereum", "cosmos", "solana"]
```

### CLI Commands

#### Start Almanac Service

```bash
almanac service start
```

Options:
- `--config <path>`: Custom config file path
- `--port <port>`: Override service port
- `--log-level <level>`: Set logging level (debug, info, warn, error)

#### Manage Indexed Chains

Add a new chain to index:

```bash
almanac chains add ethereum --rpc-url https://mainnet.infura.io/v3/YOUR_API_KEY
```

List all indexed chains:

```bash
almanac chains list
```

View chain status:

```bash
almanac chains status ethereum
```

Pause/resume chain indexing:

```bash
almanac chains pause ethereum
almanac chains resume ethereum
```

#### Query Events

Query events from the CLI:

```bash
# Query by address
almanac events address ethereum 0x1234567890abcdef1234567890abcdef12345678 --limit 10

# Query by chain
almanac events chain cosmos --from-height 15000000 --to-height 15001000 --limit 50

# Custom filter
almanac events filter --chain ethereum --type Transfer --attribute to=0x... --limit 20
```

Export events to file:

```bash
almanac events export --chain ethereum --from-height 18000000 --output events.json
```

#### Administrative Commands

Database management:

```bash
# Initialize database schema
almanac db init

# Run migrations
almanac db migrate

# Reset database (WARNING: Deletes all data)
almanac db reset
```

User management:

```bash
# Add a new API user
almanac users add --name "API User" --role read

# List users
almanac users list

# Generate API key for user
almanac users generate-key --id user_123
```

## Example: Using Almanac with curl

### Basic Queries

Fetch status of an Ethereum chain:

```bash
curl -X GET http://localhost:8000/api/v1/chains/ethereum/status
```

Get the latest 10 events from a contract:

```bash
curl -X GET "http://localhost:8000/api/v1/events/address/ethereum/0x1234567890abcdef1234567890abcdef12345678?limit=10"
```

### Using jq for Processing

Fetch and process events with jq:

```bash
curl -s "http://localhost:8000/api/v1/events/address/ethereum/0x1234...?limit=100" | \
  jq '.events[] | select(.event_type == "Transfer") | {txHash: .tx_hash, value: .attributes.value}'
```

### Bash Script Example

A simple monitoring script:

```bash
#!/bin/bash

# Config
API_BASE="http://localhost:8000/api/v1"
CONTRACT="0x1234567890abcdef1234567890abcdef12345678"
CHAIN="ethereum"
THRESHOLD="1000000000000000000"  # 1 ETH in wei

# Monitor large transfers
while true; do
  curl -s "$API_BASE/events/address/$CHAIN/$CONTRACT?limit=10" | \
    jq -r ".events[] | select(.event_type == \"Transfer\" and (.attributes.value | tonumber) > $THRESHOLD) | \
      \"Large transfer: \(.attributes.value) wei from \(.attributes.from) to \(.attributes.to) in tx \(.tx_hash)\"" 
  
  sleep 60
done
```

## Example: Using Almanac with Python

### Installation

```bash
pip install requests websockets
```

### HTTP API Example

```python
import requests
import json

# Configuration
base_url = "http://localhost:8000/api/v1"
api_key = "your_api_key"  # Optional

headers = {}
if api_key:
    headers["Authorization"] = f"Bearer {api_key}"

# Get chain status
response = requests.get(f"{base_url}/chains/ethereum/status", headers=headers)
status = response.json()
print(f"Latest Ethereum block: {status['latest_height']}")

# Query events from a contract
contract = "0x1234567890abcdef1234567890abcdef12345678"
params = {
    "limit": 20,
    "ascending": "false"
}

response = requests.get(
    f"{base_url}/events/address/ethereum/{contract}", 
    headers=headers,
    params=params
)

events = response.json()["events"]
for event in events:
    if event["event_type"] == "Transfer":
        print(f"Transfer: {event['attributes']['value']} at block {event['height']}")
```

### WebSocket Example

```python
import asyncio
import websockets
import json

async def subscribe_to_events():
    uri = "ws://localhost:8000/api/v1/ws"
    
    async with websockets.connect(uri) as websocket:
        # Subscribe to events
        subscription = {
            "action": "subscribe",
            "filter": {
                "chain_id": "ethereum",
                "event_type": "Transfer",
                "address": "0x1234567890abcdef1234567890abcdef12345678"
            }
        }
        
        await websocket.send(json.dumps(subscription))
        print("Subscribed to events, waiting...")
        
        # Listen for events
        while True:
            response = await websocket.recv()
            data = json.loads(response)
            
            if data["type"] == "event":
                event = data["data"]
                print(f"New event: {event['event_type']} at block {event['height']}")
                print(f"  Transaction: {event['tx_hash']}")
                print(f"  Details: {json.dumps(event['attributes'], indent=2)}")

# Run the async function
asyncio.run(subscribe_to_events())
```

## Example: Using Almanac with Node.js

### Installation

```bash
npm install axios ws
```

### HTTP API Example

```javascript
const axios = require('axios');

// Configuration
const baseUrl = 'http://localhost:8000/api/v1';
const apiKey = 'your_api_key';  // Optional

const headers = {};
if (apiKey) {
  headers['Authorization'] = `Bearer ${apiKey}`;
}

// Get chain status
async function getChainStatus(chainId) {
  try {
    const response = await axios.get(`${baseUrl}/chains/${chainId}/status`, { headers });
    console.log(`Latest ${chainId} block: ${response.data.latest_height}`);
    return response.data;
  } catch (error) {
    console.error('Error fetching chain status:', error.message);
  }
}

// Query events from a contract
async function getContractEvents(chainId, address, limit = 20) {
  try {
    const response = await axios.get(
      `${baseUrl}/events/address/${chainId}/${address}`, 
      { 
        headers,
        params: { limit, ascending: 'false' }
      }
    );
    
    const events = response.data.events;
    events.forEach(event => {
      if (event.event_type === 'Transfer') {
        console.log(`Transfer: ${event.attributes.value} at block ${event.height}`);
      }
    });
    
    return events;
  } catch (error) {
    console.error('Error fetching events:', error.message);
  }
}

// Usage
async function main() {
  await getChainStatus('ethereum');
  await getContractEvents('ethereum', '0x1234567890abcdef1234567890abcdef12345678');
}

main();
```

### WebSocket Example

```javascript
const WebSocket = require('ws');

function subscribeToEvents() {
  const ws = new WebSocket('ws://localhost:8000/api/v1/ws');
  
  ws.on('open', () => {
    console.log('WebSocket connection established');
    
    // Subscribe to events
    const subscription = {
      action: 'subscribe',
      filter: {
        chain_id: 'ethereum',
        event_type: 'Transfer',
        address: '0x1234567890abcdef1234567890abcdef12345678'
      }
    };
    
    ws.send(JSON.stringify(subscription));
    console.log('Subscribed to events, waiting...');
  });
  
  ws.on('message', (data) => {
    const message = JSON.parse(data);
    
    if (message.type === 'event') {
      const event = message.data;
      console.log(`New event: ${event.event_type} at block ${event.height}`);
      console.log(`  Transaction: ${event.tx_hash}`);
      console.log(`  Details:`, event.attributes);
    }
  });
  
  ws.on('error', (error) => {
    console.error('WebSocket error:', error.message);
  });
  
  ws.on('close', () => {
    console.log('WebSocket connection closed');
  });
  
  // Close connection after 5 minutes
  setTimeout(() => {
    ws.close();
  }, 5 * 60 * 1000);
}

subscribeToEvents();
```

## Troubleshooting

### Common Issues

1. **Connection Refused**
   
   This usually means the Almanac service is not running or is not accessible at the configured address/port.
   
   Solution: Verify the service is running using `almanac service status` and check the configuration.

2. **Authentication Failed**
   
   Invalid or expired API key.
   
   Solution: Generate a new API key with `almanac users generate-key`.

3. **Rate Limiting**
   
   Too many requests in a short time period.
   
   Solution: Add rate limiting headers to your requests to determine current limits and remaining requests.

4. **Timeout on Large Queries**
   
   Queries with large result sets may time out.
   
   Solution: Use pagination and smaller time ranges to break up large queries.

### Logging

Adjust logging level for more detailed information:

```bash
almanac service start --log-level debug
```

View logs:

```bash
# If running as a service
journalctl -u almanac.service -f

# Or check the log file
tail -f ~/.almanac/logs/almanac.log
```

## Advanced Topics

### Securing Your Almanac Instance

Best practices:

1. Use TLS/HTTPS for all API endpoints
2. Configure proper authentication
3. Set up a reverse proxy (like Nginx) in front of Almanac
4. Use IP whitelisting for trusted clients

### Performance Tuning

Tips for optimizing performance:

1. Increase database connection pool size
2. Use indexing hints for frequently queried event attributes
3. Use caching for frequently accessed data
4. Consider horizontal scaling for high-traffic applications

### Healthchecks

Almanac provides a health endpoint:

```
GET /health
```

Use this for monitoring and deployments:

```bash
curl -X GET http://localhost:8000/health
```

Example response:
```json
{
  "status": "healthy",
  "version": "1.2.3",
  "database": "connected",
  "chains": {
    "ethereum": "ok",
    "cosmos": "ok",
    "solana": "error"
  }
}
```

## Further Reading

- [Almanac GitHub Repository](https://github.com/timewave-computer/almanac)
- [Almanac Architecture Documentation](./architecture.md)
- [Contributing to Almanac](./contributing.md) 