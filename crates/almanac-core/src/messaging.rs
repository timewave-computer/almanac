//! Pub/Sub messaging abstraction for decoupling ingestion from processing
//!
//! This module provides lightweight, in-process pub/sub patterns to separate
//! block fetching (ingestion) from block processing and storage writes.

use anyhow::Result;
use async_trait::async_trait;
use futures::Stream;
use serde::{Deserialize, Serialize};
use std::pin::Pin;
use tokio_stream::StreamExt;

/// Raw block event containing unprocessed blockchain data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawBlockEvent {
    /// Chain identifier (e.g., "ethereum", "solana", "cosmoshub-4")
    pub chain_id: String,
    /// Block number/height
    pub block_number: u64,
    /// Raw block data (JSON-serialized or chain-specific format)
    pub block_data: Vec<u8>,
    /// Timestamp when the block was fetched
    pub fetched_at: u64,
    /// Optional metadata for processing hints
    pub metadata: std::collections::HashMap<String, String>,
}

impl RawBlockEvent {
    /// Create a new raw block event
    pub fn new(chain_id: String, block_number: u64, block_data: Vec<u8>) -> Self {
        Self {
            chain_id,
            block_number,
            block_data,
            fetched_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            metadata: std::collections::HashMap::new(),
        }
    }

    /// Add metadata to the event
    pub fn with_metadata(mut self, key: String, value: String) -> Self {
        self.metadata.insert(key, value);
        self
    }
}

/// Event publisher trait for sending raw block events
#[async_trait]
pub trait EventPublisher: Send + Sync + 'static {
    /// Publish a raw block event to the specified topic
    async fn publish(&self, topic: &str, event: RawBlockEvent) -> Result<()>;

    /// Check if the publisher is healthy and accepting events
    async fn health_check(&self) -> bool;
}

/// Event subscriber trait for receiving raw block events
#[async_trait]
pub trait EventSubscriber: Send + Sync + 'static {
    /// Subscribe to events from the specified topic
    async fn subscribe(
        &self,
        topic: &str,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<RawBlockEvent>> + Send>>>;

    /// Check if the subscriber is healthy and receiving events
    async fn health_check(&self) -> bool;
}

/// In-process pub/sub implementation using tokio broadcast channels
pub struct InProcessPubSub {
    channels: tokio::sync::RwLock<
        std::collections::HashMap<String, tokio::sync::broadcast::Sender<RawBlockEvent>>,
    >,
    buffer_size: usize,
}

impl InProcessPubSub {
    /// Create a new in-process pub/sub with the specified buffer size
    pub fn new(buffer_size: usize) -> Self {
        Self {
            channels: tokio::sync::RwLock::new(std::collections::HashMap::new()),
            buffer_size,
        }
    }

    /// Get or create a channel for the topic
    async fn get_or_create_channel(
        &self,
        topic: &str,
    ) -> tokio::sync::broadcast::Sender<RawBlockEvent> {
        let mut channels = self.channels.write().await;
        if let Some(sender) = channels.get(topic) {
            sender.clone()
        } else {
            let (sender, _) = tokio::sync::broadcast::channel(self.buffer_size);
            channels.insert(topic.to_string(), sender.clone());
            sender
        }
    }
}

#[async_trait]
impl EventPublisher for InProcessPubSub {
    async fn publish(&self, topic: &str, event: RawBlockEvent) -> Result<()> {
        let sender = self.get_or_create_channel(topic).await;
        sender
            .send(event)
            .map_err(|e| anyhow::anyhow!("Failed to publish event: {}", e))?;
        Ok(())
    }

    async fn health_check(&self) -> bool {
        true // In-process is always healthy
    }
}

#[async_trait]
impl EventSubscriber for InProcessPubSub {
    async fn subscribe(
        &self,
        topic: &str,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<RawBlockEvent>> + Send>>> {
        let sender = self.get_or_create_channel(topic).await;
        let receiver = sender.subscribe();

        let stream = tokio_stream::wrappers::BroadcastStream::new(receiver)
            .map(|result| result.map_err(|e| anyhow::anyhow!("Stream error: {}", e)));

        Ok(Box::pin(stream))
    }

    async fn health_check(&self) -> bool {
        true // In-process is always healthy
    }
}

impl Default for InProcessPubSub {
    fn default() -> Self {
        Self::new(1000) // Default buffer size
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::StreamExt;

    #[tokio::test]
    async fn test_in_process_pubsub() {
        let pubsub = InProcessPubSub::new(10);

        // Test health checks
        assert!(EventPublisher::health_check(&pubsub).await);

        // Test publish/subscribe
        let event =
            RawBlockEvent::new("test_chain".to_string(), 12345, b"test block data".to_vec());

        // Subscribe before publishing
        let mut stream = pubsub.subscribe("test_topic").await.unwrap();

        // Publish event
        pubsub.publish("test_topic", event.clone()).await.unwrap();

        // Receive event
        if let Some(received) = stream.next().await {
            let received_event = received.unwrap();
            assert_eq!(received_event.chain_id, event.chain_id);
            assert_eq!(received_event.block_number, event.block_number);
            assert_eq!(received_event.block_data, event.block_data);
        } else {
            panic!("No event received");
        }
    }

    #[tokio::test]
    async fn test_raw_block_event_creation() {
        let event = RawBlockEvent::new("ethereum".to_string(), 18_000_000, b"block data".to_vec())
            .with_metadata("gas_limit".to_string(), "30000000".to_string());

        assert_eq!(event.chain_id, "ethereum");
        assert_eq!(event.block_number, 18_000_000);
        assert_eq!(
            event.metadata.get("gas_limit"),
            Some(&"30000000".to_string())
        );
        assert!(event.fetched_at > 0);
    }
}
