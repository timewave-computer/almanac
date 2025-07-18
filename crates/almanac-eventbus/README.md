# almanac-eventbus

Internal event bus for decoupled communication in the Almanac multi-chain blockchain indexing system.

## Overview

This crate provides a lightweight, in-process event bus for communication between different components of the Almanac system. It enables decoupled, asynchronous message passing using a publisher-subscriber pattern with type-safe event handling.

## Key Components

### Event Bus
- `EventBus` - Core event bus implementation
- `EventBusBuilder` - Builder pattern for configuration
- Channel-based message passing with async processing
- Type-safe event routing and handling

### Event System
- `Event` trait - Common interface for all events
- `EventType` - Event type classification
- `EventMetadata` - Event metadata and timestamps
- Serializable event data for persistence

### Handler System
- `EventHandler` trait - Interface for event handlers
- `BoxedHandler` - Type-erased handler wrapper
- Async event processing with error handling
- Multiple handlers per event type

### Subscription Management
- `Subscription` - Event subscription tracking
- `SubscriptionId` - Unique subscription identifiers
- Subscription lifecycle management
- Automatic cleanup on handler removal

## Usage

```rust
use almanac_eventbus::{EventBus, Event, EventHandler, EventType, EventMetadata};
use async_trait::async_trait;
use std::sync::Arc;

// Define custom events
#[derive(Debug, Clone)]
struct BlockProcessedEvent {
    metadata: EventMetadata,
    chain_id: String,
    block_number: u64,
    transaction_count: u32,
}

impl Event for BlockProcessedEvent {
    fn event_type(&self) -> EventType {
        EventType::BlockProcessed
    }
    
    fn metadata(&self) -> &EventMetadata {
        &self.metadata
    }
    
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

// Create event handlers
struct BlockProcessedHandler {
    name: String,
}

#[async_trait]
impl EventHandler<BlockProcessedEvent> for BlockProcessedHandler {
    async fn handle(&self, event: Arc<BlockProcessedEvent>) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        println!("{}: Block {} processed on {}", 
            self.name, event.block_number, event.chain_id);
        Ok(())
    }
}

// Set up event bus
let event_bus = EventBus::new();

// Subscribe to events
let handler = Arc::new(BlockProcessedHandler {
    name: "Logger".to_string(),
});

let subscription_id = event_bus.subscribe(handler).await?;

// Publish events
let event = BlockProcessedEvent {
    metadata: EventMetadata::new(EventType::BlockProcessed),
    chain_id: "ethereum".to_string(),
    block_number: 1000,
    transaction_count: 150,
};

event_bus.publish(Arc::new(event)).await?;

// Unsubscribe when done
event_bus.unsubscribe(subscription_id).await?;
```

## Event Types

```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum EventType {
    // Block processing events
    BlockProcessed,
    BlockFinalized,
    BlockReorganized,
    
    // Transaction events
    TransactionProcessed,
    TransactionFailed,
    
    // State change events
    StateChanged,
    ContractDeployed,
    ContractCall,
    
    // System events
    SystemStarted,
    SystemStopped,
    ErrorOccurred,
    
    // Custom events
    Custom(String),
}
```

## Features

- **Type-safe messaging**: Compile-time type checking for events
- **Async processing**: Non-blocking event handling
- **Multiple handlers**: Support for multiple subscribers per event type
- **Error handling**: Comprehensive error propagation
- **Subscription management**: Automatic cleanup and lifecycle management
- **Metadata support**: Event timestamps and tracing information
- **Bounded channels**: Configurable channel sizes for backpressure

## Configuration

```rust
use almanac_eventbus::EventBusBuilder;

let event_bus = EventBusBuilder::new()
    .with_buffer_size(1000)
    .with_max_handlers(50)
    .with_error_handler(|error| {
        eprintln!("Event bus error: {}", error);
    })
    .build();
```

## Error Handling

The event bus provides comprehensive error handling:

```rust
// Handler errors are captured and logged
#[async_trait]
impl EventHandler<MyEvent> for MyHandler {
    async fn handle(&self, event: Arc<MyEvent>) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Handler logic that might fail
        process_event(&event).map_err(|e| {
            Box::new(e) as Box<dyn std::error::Error + Send + Sync>
        })
    }
}

// Bus errors are propagated to callers
match event_bus.publish(event).await {
    Ok(()) => println!("Event published successfully"),
    Err(e) => eprintln!("Failed to publish event: {}", e),
}
```

## Testing

Run tests with:
```bash
cargo test -p almanac-eventbus
```

The test suite covers:
- Event publishing and handling
- Subscription management
- Error propagation
- Concurrent event processing
- Handler lifecycle management

## Performance Considerations

- **Channel-based**: Uses Tokio channels for efficient message passing
- **Async processing**: Non-blocking event handling
- **Type erasure**: Minimal overhead for type-safe routing
- **Bounded queues**: Configurable backpressure handling
- **Parallel processing**: Multiple handlers can process events concurrently

## Use Cases

- **Component communication**: Decouple indexing pipeline components
- **Event logging**: Centralized event logging and monitoring
- **State synchronization**: Coordinate state changes across components
- **Error notification**: Propagate errors across system boundaries
- **Metrics collection**: Gather system metrics and statistics