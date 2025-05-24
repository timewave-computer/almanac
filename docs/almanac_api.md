# Almanac API Documentation

This document outlines the stable public interface of Almanac, an indexing and event storage system for blockchain data.

## Core Concepts

Almanac provides a unified way to access events across multiple blockchains through a consistent API. The core concepts are:

- **Events**: Structured data emitted by on-chain activities
- **Chains**: Different blockchain networks supported by Almanac
- **Addresses**: Resources or contracts that emit events
- **Filtering**: Methods to query for specific events

## API Overview

Almanac's API is built around two core traits:

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

## Best Practices

1. **Use QueryOptions Effectively**
   - Set reasonable limits to avoid fetching too much data
   - Use ascending/descending based on your use case

2. **Error Handling**
   - Always handle errors from API calls appropriately
   - Consider implementing retries for transient errors

3. **Subscriptions**
   - Keep subscription handlers lightweight
   - Consider processing events in separate tasks for long-running operations

4. **Chain Status Monitoring**
   - Periodically check chain status to ensure indexing is operational
   - Have fallback mechanisms if a chain falls behind or encounters errors

## Examples of Common Use Cases

### Monitoring Token Transfers

```rust
use indexer_api::{EventStore, EventFilter, ChainId};
use std::collections::HashMap;
use futures::StreamExt;

async fn monitor_token_transfers(store: &impl EventStore, token_address: &str, chain: &str) {
    let mut attributes = HashMap::new();
    attributes.insert("token".to_string(), EventValue::String(token_address.to_string()));
    
    let filter = EventFilter {
        chain_id: Some(ChainId(chain.to_string())),
        address: Some(token_address.to_string()),
        event_type: Some("Transfer".to_string()),
        attributes,
        from_height: None,
        to_height: None,
    };
    
    if let Ok(mut stream) = store.subscribe(filter).await {
        while let Some(Ok(event)) = stream.next().await {
            println!("Transfer detected: {} {}", 
                event.address,
                event.tx_hash);
            
            // Extract from/to/amount fields from attributes
            // Process as needed...
        }
    }
}
```

### Indexing Historical Activity

```rust
use indexer_api::{EventStore, ChainId, QueryOptions};

async fn index_historical_activity(store: &impl EventStore, chain: &str, start_block: u64, end_block: u64) {
    let chain_id = ChainId(chain.to_string());
    let batch_size = 1000;
    
    for block_start in (start_block..=end_block).step_by(batch_size) {
        let block_end = std::cmp::min(block_start + batch_size as u64 - 1, end_block);
        
        let options = QueryOptions {
            limit: None, // get all events in the range
            offset: None,
            ascending: true,
        };
        
        match store.get_events_by_chain(&chain_id, Some(block_start), Some(block_end), options).await {
            Ok(events) => {
                println!("Processed blocks {}-{}, found {} events", 
                    block_start, block_end, events.len());
                
                // Process the events batch...
            },
            Err(err) => {
                eprintln!("Error fetching blocks {}-{}: {}", block_start, block_end, err);
            }
        }
    }
}
```

## Advanced Topics

### Combining Multiple Queries

Sometimes you may need to combine data from multiple queries:

```rust
use indexer_api::{EventStore, Address, ChainId, QueryOptions};
use std::collections::HashSet;

async fn find_related_contracts(
    store: &impl EventStore, 
    primary_contract: &str,
    chain: &str
) -> HashSet<String> {
    let address = Address {
        chain_id: ChainId(chain.to_string()),
        value: primary_contract.to_string(),
    };
    
    let options = QueryOptions {
        limit: Some(1000),
        offset: None,
        ascending: false,
    };
    
    let mut related_contracts = HashSet::new();
    
    if let Ok(events) = store.get_events_by_address(&address, options).await {
        for event in events {
            // For each event from the primary contract,
            // extract any referenced contract addresses
            if let Some(EventValue::String(related)) = event.attributes.get("related_contract") {
                related_contracts.insert(related.clone());
            }
        }
    }
    
    related_contracts
}
```

## Conclusion

The Almanac API provides a powerful and consistent interface for accessing blockchain event data across different chains. By leveraging the `EventStore` and `ChainReader` traits, applications can efficiently query and process event data for a wide range of use cases. 