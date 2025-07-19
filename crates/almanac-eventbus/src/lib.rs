//! Event bus for internal communication in Almanac
//!
//! This crate provides an in-process event bus for decoupled communication
//! between different components of the Almanac system.

pub mod bus;
pub mod event;
pub mod handler;
pub mod subscription;

// Re-exports
pub use bus::{EventBus, EventBusBuilder};
pub use event::{Event, EventMetadata, EventType};
pub use handler::EventHandler;
pub use subscription::{Subscription, SubscriptionId};