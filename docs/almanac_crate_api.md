# Almanac Crate API Documentation

This document outlines how to use Almanac as an imported Rust crate, focusing on its stable public interfaces.

## Core Concepts

Almanac provides a unified way to access events across multiple blockchains through a consistent API. The core concepts are:

- **Events**: Structured data emitted by on-chain activities
- **Chains**: Different blockchain networks supported by Almanac
- **Addresses**: Resources or contracts that emit events
- **Filtering**: Methods to query for specific events

## API Overview

When using Almanac as a Rust crate, you'll primarily interact with two core traits:

1. `EventStore`: For accessing and querying indexed events
2. `ChainReader`: For accessing chain-specific information

## Using the EventStore API

The `EventStore` trait is the primary interface for accessing indexed blockchain events.

```rust
/// Core trait for accessing indexed events
pub trait EventStore: Send + Sync {
    /// Get events by address (resource)
    async fn get_events_by_address(
        &self, 
        address: &Address,
        options: QueryOptions,
    ) -> Result<Vec<Event>, EventStoreError>;
    
    /// Get events by chain and block range
    async fn get_events_by_chain(
        &self,
        chain_id: &ChainId,
        from_height: Option<u64>,
        to_height: Option<u64>,
        options: QueryOptions,
    ) -> Result<Vec<Event>, EventStoreError>;
    
    /// Subscribe to events matching a filter
    async fn subscribe(
        &self,
        filter: EventFilter,
    ) -> Result<impl Stream<Item = Result<Event, EventStoreError>>, EventStoreError>;
}
```

### Getting Started with Almanac Crate

First, add Almanac to your Cargo.toml:

```toml
[dependencies]
indexer-api = { path = "path/to/almanac/crates/api" }
# Or if published to crates.io:
# indexer-api = "0.1.0"
```

Then, in your code:

```rust
use indexer_api::{AlmanacClient, EventStore, ChainReader};

async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a client
    let client = AlmanacClient::new().await?;
    
    // Use the client as an EventStore
    // Example queries follow...
    
    Ok(())
}
```

### Querying Events by Address

To retrieve events emitted by a specific address (like a contract):

```rust
use indexer_api::{EventStore, Address, ChainId, QueryOptions};

async fn get_contract_events(store: &impl EventStore, contract_address: &str, chain: &str) -> Vec<Event> {
    let address = Address {
        chain_id: ChainId(chain.to_string()),
        value: contract_address.to_string(),
    };
    
    let options = QueryOptions {
        limit: Some(100),
        offset: None,
        ascending: false, // latest events first
    };
    
    match store.get_events_by_address(&address, options).await {
        Ok(events) => events,
        Err(err) => {
            eprintln!("Failed to get events: {}", err);
            vec![]
        }
    }
}
```

### Querying Events by Chain

To retrieve events from a specific blockchain within a block range:

```rust
use indexer_api::{EventStore, ChainId, QueryOptions};

async fn get_recent_chain_events(store: &impl EventStore, chain: &str, last_n_blocks: u64) -> Vec<Event> {
    let chain_id = ChainId(chain.to_string());
    
    // If you know the current height, you could calculate from_height more precisely
    let from_height = None; // or Some(current_height - last_n_blocks)
    let to_height = None; // or Some(current_height) for a specific range
    
    let options = QueryOptions {
        limit: Some(500),
        offset: None,
        ascending: true, // chronological order
    };
    
    match store.get_events_by_chain(&chain_id, from_height, to_height, options).await {
        Ok(events) => events,
        Err(err) => {
            eprintln!("Failed to get chain events: {}", err);
            vec![]
        }
    }
}
```

### Subscribing to Events

For real-time processing of events as they're indexed:

```rust
use indexer_api::{EventStore, EventFilter, ChainId};
use futures::StreamExt;
use std::collections::HashMap;

async fn monitor_events(store: &impl EventStore, chain: &str, event_type: &str) {
    let filter = EventFilter {
        chain_id: Some(ChainId(chain.to_string())),
        address: None, // all addresses
        event_type: Some(event_type.to_string()),
        attributes: HashMap::new(), // no attribute filtering
        from_height: None, // all heights from now
        to_height: None,
    };
    
    match store.subscribe(filter).await {
        Ok(mut stream) => {
            println!("Subscribed to {} events on {}", event_type, chain);
            
            while let Some(result) = stream.next().await {
                match result {
                    Ok(event) => {
                        println!("New event: {}:{} at height {}", 
                            event.event_type, event.address, event.height);
                        // Process the event...
                    },
                    Err(err) => {
                        eprintln!("Error in event stream: {}", err);
                    }
                }
            }
        },
        Err(err) => {
            eprintln!("Failed to subscribe: {}", err);
        }
    }
}
```

## Using the ChainReader API

The `ChainReader` trait provides access to information about indexed chains:

```rust
/// Chain data access
pub trait ChainReader: Send + Sync {
    /// Get status of a chain
    async fn get_chain_status(&self, chain_id: &ChainId) -> Result<ChainStatus, ChainReaderError>;
}
```

### Getting Chain Status

To check the status of an indexed chain:

```rust
use indexer_api::{ChainReader, ChainId};

async fn check_chain_status(reader: &impl ChainReader, chain: &str) {
    let chain_id = ChainId(chain.to_string());
    
    match reader.get_chain_status(&chain_id).await {
        Ok(status) => {
            println!("Chain: {}", chain);
            println!("Latest height: {}", status.latest_height);
            
            if let Some(finalized) = status.finalized_height {
                println!("Finalized height: {}", finalized);
            }
            
            println!("Actively indexing: {}", status.is_indexing);
            println!("Last indexed: {} seconds ago", 
                     std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs() - status.last_indexed_at);
            
            if let Some(error) = status.error {
                println!("Error: {}", error);
            }
        },
        Err(err) => {
            eprintln!("Failed to get chain status: {}", err);
        }
    }
}
```

## Working with Event Data

The `Event` struct contains all the data associated with a blockchain event:

```rust
/// Event data
pub struct Event {
    /// Chain this event came from
    pub chain_id: ChainId,
    
    /// Block height
    pub height: u64,
    
    /// Transaction hash
    pub tx_hash: String,
    
    /// Event index within transaction
    pub index: u32,
    
    /// Address (contract, account) that emitted the event
    pub address: String,
    
    /// Event type or name
    pub event_type: String,
    
    /// Event attributes/fields
    pub attributes: HashMap<String, EventValue>,
    
    /// Raw event data
    pub raw_data: Vec<u8>,
    
    /// Timestamp when this event was created (in seconds since Unix epoch)
    pub timestamp: u64,
}
```

### Accessing Event Attributes

Attributes are stored in a HashMap with values that can be of various types:

```rust
async fn process_event(event: &Event) {
    println!("Processing event: {} from {}", event.event_type, event.address);
    
    for (key, value) in &event.attributes {
        match value {
            EventValue::String(s) => println!("  {} = {}", key, s),
            EventValue::Integer(i) => println!("  {} = {}", key, i),
            EventValue::Float(f) => println!("  {} = {}", key, f),
            EventValue::Boolean(b) => println!("  {} = {}", key, b),
            EventValue::Array(arr) => println!("  {} = array with {} items", key, arr.len()),
            EventValue::Object(obj) => println!("  {} = object with {} properties", key, obj.len()),
            EventValue::Null => println!("  {} = null", key),
        }
    }
    
    // Example: Extract a specific attribute
    if let Some(EventValue::String(recipient)) = event.attributes.get("recipient") {
        println!("Transfer recipient: {}", recipient);
    }
}
```

## Library Integration Examples

### Using Almanac in a Background Service

```rust
use indexer_api::{AlmanacClient, EventStore, EventFilter, ChainId};
use futures::StreamExt;
use tokio::time::{sleep, Duration};
use std::sync::Arc;

async fn run_background_service() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize Almanac client
    let client = Arc::new(AlmanacClient::new().await?);
    
    // Setup filter for events we're interested in
    let filter = EventFilter {
        chain_id: Some(ChainId("ethereum".to_string())),
        event_type: Some("Transfer".to_string()),
        // ... other filter parameters
        ..Default::default()
    };
    
    // Subscribe to events
    let mut stream = client.subscribe(filter).await?;
    
    // Process events in background
    tokio::spawn(async move {
        while let Some(result) = stream.next().await {
            match result {
                Ok(event) => {
                    // Process event
                    println!("Received event: {} at height {}", 
                        event.event_type, event.height);
                    
                    // Perform business logic with event data
                    process_business_logic(&event).await;
                },
                Err(e) => {
                    eprintln!("Error in event stream: {}", e);
                    sleep(Duration::from_secs(5)).await;
                }
            }
        }
    });
    
    Ok(())
}

async fn process_business_logic(event: &Event) {
    // Your business logic here
}
```

### Integrating with a Web Application

```rust
use indexer_api::{AlmanacClient, EventStore, Address, ChainId, QueryOptions};
use axum::{
    routing::get,
    Router,
    extract::Path,
    Json,
};
use std::sync::Arc;

// Shared application state
struct AppState {
    almanac: Arc<AlmanacClient>,
}

#[tokio::main]
async fn main() {
    // Initialize Almanac client
    let almanac = Arc::new(AlmanacClient::new().await.expect("Failed to create Almanac client"));
    
    // Create app state
    let state = Arc::new(AppState { almanac });
    
    // Build router
    let app = Router::new()
        .route("/events/:chain/:address", get(get_address_events))
        .with_state(state);
    
    // Start server
    let addr = "0.0.0.0:3000";
    println!("Listening on {}", addr);
    axum::Server::bind(&addr.parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn get_address_events(
    Path((chain, address)): Path<(String, String)>,
    axum::extract::State(state): axum::extract::State<Arc<AppState>>,
) -> Json<Vec<Event>> {
    let addr = Address {
        chain_id: ChainId(chain),
        value: address,
    };
    
    let options = QueryOptions {
        limit: Some(100),
        offset: None,
        ascending: false,
    };
    
    match state.almanac.get_events_by_address(&addr, options).await {
        Ok(events) => Json(events),
        Err(_) => Json(vec![]),
    }
}
```

## Best Practices for Library Usage

1. **Client Instantiation**
   - Create a single client instance and reuse it throughout your application
   - Use `Arc` for sharing the client between multiple async tasks

2. **Error Handling**
   - Implement proper error handling for all API calls
   - Consider using a retry mechanism for transient errors

3. **Resource Management**
   - Close subscription streams when they're no longer needed
   - Limit concurrent requests to avoid overwhelming resources

4. **Query Optimization**
   - Set reasonable limits to avoid fetching too much data
   - Use pagination (offset) for handling large result sets

5. **Testing**
   - Mock the EventStore and ChainReader traits for unit testing
   - Consider using the AlmanacClient with a test database for integration tests

## Conclusion

The Almanac crate provides a powerful and consistent interface for accessing blockchain event data across different chains. By using the `EventStore` and `ChainReader` traits in your Rust applications, you can efficiently query and process event data for a wide range of use cases. 