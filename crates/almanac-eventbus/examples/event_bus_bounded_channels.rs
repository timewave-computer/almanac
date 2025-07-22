//! Event bus bounded channels example
//!
//! This example demonstrates the improved event bus with bounded channels
//! for production use with proper backpressure handling.

#![allow(clippy::print_literal)] // Allow println! with string literals

use almanac_eventbus::handler::{AsyncFnHandler, HandlerResult};
use almanac_eventbus::{Event, EventBusBuilder};
use std::any::{Any, TypeId};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::SystemTime;
use tokio::time::{sleep, Duration};
use uuid::Uuid;

/// Helper function to create a successful handler result
fn ok_result() -> HandlerResult {
    Ok(())
}

/// Example event for demonstration
#[derive(Debug, Clone)]
struct TestEvent {
    id: u64,
    message: String,
    priority: EventPriority,
    event_id: Uuid,
    timestamp: SystemTime,
}

#[derive(Debug, Clone)]
enum EventPriority {
    Low,
    Normal,
    High,
    Critical,
}

impl Event for TestEvent {
    fn event_type(&self) -> &'static str {
        "TestEvent"
    }

    fn timestamp(&self) -> SystemTime {
        self.timestamp
    }

    fn event_id(&self) -> Uuid {
        self.event_id
    }

    fn clone_event(&self) -> Box<dyn Event> {
        Box::new(self.clone())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Event Bus Bounded Channels Example");
    println!("===================================");
    
    // Example 1: Create event bus with bounded channel
    println!("\n1. Bounded Channel Event Bus");
    println!("----------------------------");
    
    let bounded_bus = EventBusBuilder::new()
        .buffer_size(100) // Set bounded channel size
        .build();
    
    println!("Created event bus with bounded channel (size: 100)");
    
    // Example 2: Register a simple handler
    println!("\n2. Register Event Handler");
    println!("------------------------");
    
    // Create a counter to track processed events
    let processed_count = Arc::new(AtomicUsize::new(0));
    let handler_count = processed_count.clone();
    
    let handler = AsyncFnHandler::new(
        move |event: Arc<dyn Event>| {
            let count = handler_count.clone();
            Box::pin(async move {
                // Downcast the event
                let any_event = event.as_ref() as &dyn Any;
                if let Some(test_event) = any_event.downcast_ref::<TestEvent>() {
                    println!(
                        "Received event: id={}, message='{}', priority={:?}",
                        test_event.id, test_event.message, test_event.priority
                    );
                    count.fetch_add(1, Ordering::Relaxed);
                }
                ok_result()
            })
        },
        vec![TypeId::of::<TestEvent>()],
    );
    
    bounded_bus.register_handler(Box::new(handler)).await?;
    println!("Registered handler for TestEvent");
    
    // Example 3: Publish events
    println!("\n3. Publishing Events");
    println!("-------------------");
    
    for i in 0..5 {
        let event = TestEvent {
            id: i,
            message: format!("Test event {}", i),
            priority: match i % 4 {
                0 => EventPriority::Low,
                1 => EventPriority::Normal,
                2 => EventPriority::High,
                _ => EventPriority::Critical,
            },
            event_id: Uuid::new_v4(),
            timestamp: SystemTime::now(),
        };
        
        bounded_bus.publish(event).await?;
        println!("Published event {}", i);
        
        // Small delay to show processing
        sleep(Duration::from_millis(100)).await;
    }
    
    // Wait a bit for events to be processed
    sleep(Duration::from_millis(500)).await;
    println!("Processed {} events", processed_count.load(Ordering::Relaxed));
    
    // Example 4: Demonstrate backpressure handling
    println!("\n4. Backpressure Handling");
    println!("------------------------");
    
    let backpressure_bus = EventBusBuilder::new()
        .buffer_size(5) // Small buffer to demonstrate backpressure
        .build();
    
    // Register a slow handler
    let slow_processed = Arc::new(AtomicUsize::new(0));
    let slow_count = slow_processed.clone();
    
    let slow_handler = AsyncFnHandler::new(
        move |event: Arc<dyn Event>| {
            let count = slow_count.clone();
            Box::pin(async move {
                let any_event = event.as_ref() as &dyn Any;
                if let Some(test_event) = any_event.downcast_ref::<TestEvent>() {
                    println!("Slow handler processing event {}", test_event.id);
                    sleep(Duration::from_millis(500)).await; // Simulate slow processing
                    count.fetch_add(1, Ordering::Relaxed);
                }
                ok_result()
            })
        },
        vec![TypeId::of::<TestEvent>()],
    );
    
    backpressure_bus.register_handler(Box::new(slow_handler)).await?;
    
    // Try to publish many events quickly using timeouts to detect backpressure
    println!("Publishing 10 events rapidly to small buffer (using timeouts)...");
    
    for i in 0..10 {
        let event = TestEvent {
            id: i,
            message: format!("Backpressure test {}", i),
            priority: EventPriority::Normal,
            event_id: Uuid::new_v4(),
            timestamp: SystemTime::now(),
        };
        
        // Since try_publish doesn't exist, we'll use publish with a timeout to demonstrate backpressure
        let publish_result = tokio::time::timeout(
            Duration::from_millis(10),
            backpressure_bus.publish(event)
        ).await;
        
        match publish_result {
            Ok(Ok(())) => println!("  ✓ Event {} published", i),
            Ok(Err(e)) => println!("  ✗ Event {} error: {:?}", i, e),
            Err(_) => println!("  ✗ Event {} timed out - channel likely full (backpressure)", i),
        }
        
        // Small delay between publishes
        sleep(Duration::from_millis(50)).await;
    }
    
    // Wait for processing to complete
    println!("\nWaiting for all events to be processed...");
    sleep(Duration::from_secs(6)).await;
    println!("Slow handler processed {} events", slow_processed.load(Ordering::Relaxed));
    
    // Example 5: Multiple handlers with different processing speeds
    println!("\n5. Multiple Handlers");
    println!("--------------------");
    
    let multi_bus = EventBusBuilder::new()
        .buffer_size(50)
        .build();
    
    // Track events processed by each handler
    let fast_count = Arc::new(AtomicUsize::new(0));
    let medium_count = Arc::new(AtomicUsize::new(0));
    let slow_multi_count = Arc::new(AtomicUsize::new(0));
    
    // Fast handler
    let fc = fast_count.clone();
    let fast_handler = AsyncFnHandler::new(
        move |event: Arc<dyn Event>| {
            let count = fc.clone();
            Box::pin(async move {
                let any_event = event.as_ref() as &dyn Any;
                if let Some(test_event) = any_event.downcast_ref::<TestEvent>() {
                    println!("  [FAST] Processing event {}", test_event.id);
                    count.fetch_add(1, Ordering::Relaxed);
                }
                ok_result()
            })
        },
        vec![TypeId::of::<TestEvent>()],
    );
    
    // Medium handler
    let mc = medium_count.clone();
    let medium_handler = AsyncFnHandler::new(
        move |event: Arc<dyn Event>| {
            let count = mc.clone();
            Box::pin(async move {
                let any_event = event.as_ref() as &dyn Any;
                if let Some(test_event) = any_event.downcast_ref::<TestEvent>() {
                    sleep(Duration::from_millis(100)).await;
                    println!("  [MEDIUM] Processing event {}", test_event.id);
                    count.fetch_add(1, Ordering::Relaxed);
                }
                ok_result()
            })
        },
        vec![TypeId::of::<TestEvent>()],
    );
    
    // Slow handler
    let sc = slow_multi_count.clone();
    let slow_multi_handler = AsyncFnHandler::new(
        move |event: Arc<dyn Event>| {
            let count = sc.clone();
            Box::pin(async move {
                let any_event = event.as_ref() as &dyn Any;
                if let Some(test_event) = any_event.downcast_ref::<TestEvent>() {
                    sleep(Duration::from_millis(300)).await;
                    println!("  [SLOW] Processing event {}", test_event.id);
                    count.fetch_add(1, Ordering::Relaxed);
                }
                ok_result()
            })
        },
        vec![TypeId::of::<TestEvent>()],
    );
    
    multi_bus.register_handler(Box::new(fast_handler)).await?;
    multi_bus.register_handler(Box::new(medium_handler)).await?;
    multi_bus.register_handler(Box::new(slow_multi_handler)).await?;
    
    println!("Publishing events to multiple handlers...");
    
    for i in 0..5 {
        let event = TestEvent {
            id: i,
            message: format!("Multi-handler test {}", i),
            priority: EventPriority::High,
            event_id: Uuid::new_v4(),
            timestamp: SystemTime::now(),
        };
        
        multi_bus.publish(event).await?;
        println!("Published event {} to multiple handlers", i);
    }
    
    // Wait for all handlers to complete
    sleep(Duration::from_secs(2)).await;
    
    println!("\nHandler statistics:");
    println!("  Fast handler: {} events", fast_count.load(Ordering::Relaxed));
    println!("  Medium handler: {} events", medium_count.load(Ordering::Relaxed));
    println!("  Slow handler: {} events", slow_multi_count.load(Ordering::Relaxed));
    
    println!("\n✓ Example completed successfully!");
    println!("\nKey takeaways:");
    println!("- Bounded channels provide backpressure handling");
    println!("- Timeouts can be used to detect when channels are full");
    println!("- Multiple handlers can process events at different speeds");
    println!("- EventBusBuilder allows configuring channel buffer size");
    
    Ok(())
}