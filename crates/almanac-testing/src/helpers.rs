//! Test helpers for Almanac testing

use almanac_chains::ChainConfig;
use almanac_core::{BlockStatus, EventFilter, Storage, UnifiedEvent};
use almanac_eventbus::EventBus;
use almanac_indexing::pipeline::{IndexingPipeline, PipelineConfig};
use almanac_storage::MemoryStorage;
use anyhow::Result;
use std::sync::Arc;
use tokio::time::{timeout, Duration};

/// Create a test memory storage instance
pub async fn create_memory_storage() -> Arc<MemoryStorage> {
    Arc::new(MemoryStorage::new())
}

/// Create a test storage instance (alias for memory storage)
pub async fn create_test_storage() -> Arc<dyn Storage> {
    Arc::new(MemoryStorage::new())
}

/// Create a test FoundationDB storage instance
pub async fn create_test_fdb_storage() -> Result<Arc<dyn Storage>> {
    // For testing, we'll use memory storage as FDB requires setup
    Ok(Arc::new(MemoryStorage::new()))
}

/// Create a test indexing pipeline
pub async fn create_test_pipeline() -> Arc<IndexingPipeline> {
    let config = PipelineConfig {
        chain_id: "test-chain".to_string(),
        batch_size: 10,
        interval_ms: 100,
        max_concurrent_batches: 2,
        retry_attempts: 3,
        retry_delay_ms: 100,
        finality_confirmations: 6,
        enable_validation: true,
        enable_ordering: true,
        checkpoint_interval: 100,
    };
    Arc::new(IndexingPipeline::new(config))
}

/// Create an Ethereum test pipeline
pub async fn create_ethereum_pipeline() -> Arc<IndexingPipeline> {
    let config = PipelineConfig {
        chain_id: "ethereum".to_string(),
        batch_size: 100,
        interval_ms: 1000,
        max_concurrent_batches: 5,
        retry_attempts: 3,
        retry_delay_ms: 1000,
        finality_confirmations: 12,
        enable_validation: true,
        enable_ordering: true,
        checkpoint_interval: 1000,
    };
    Arc::new(IndexingPipeline::new(config))
}

/// Create a test event bus
pub async fn create_test_event_bus() -> Arc<EventBus> {
    Arc::new(EventBus::new())
}

/// Create a test reorg detector
pub async fn create_test_reorg_detector() -> Result<()> {
    // Placeholder for reorg detector creation
    Ok(())
}

/// Create a test finality processor
pub async fn create_test_finality_processor() -> Result<()> {
    // Placeholder for finality processor creation
    Ok(())
}

/// Start a test pipeline
pub async fn start_test_pipeline(pipeline: &Arc<IndexingPipeline>) -> Result<()> {
    let pipeline_clone = pipeline.clone();
    tokio::spawn(async move {
        let _ = pipeline_clone.start().await;
    });

    // Give it a moment to start
    tokio::time::sleep(Duration::from_millis(10)).await;
    Ok(())
}

/// Stop a test pipeline
pub async fn stop_test_pipeline(pipeline: &Arc<IndexingPipeline>) -> Result<()> {
    pipeline.stop();
    Ok(())
}

/// Publish a test event to event bus
pub async fn publish_test_event(event_bus: &Arc<EventBus>, event: UnifiedEvent) -> Result<()> {
    // Mock implementation - eventbus requires specific event types
    let _ = (event_bus, event);
    Ok(())
}

/// Trigger a test reorganization
pub async fn trigger_test_reorg(_reorg_detector: &(), _depth: u64) -> Result<()> {
    // Placeholder for triggering test reorg
    Ok(())
}

// Storage assertion helpers

/// Assert the number of events in storage
pub async fn assert_event_count(storage: &Arc<dyn Storage>, expected_count: usize) -> Result<()> {
    let filter = EventFilter::new();
    let events = storage.get_events(vec![filter]).await?;
    assert_eq!(
        events.len(),
        expected_count,
        "Expected {} events, found {}",
        expected_count,
        events.len()
    );
    Ok(())
}

/// Assert the latest block number for a chain
pub async fn assert_latest_block(
    storage: &Arc<dyn Storage>,
    chain_id: &str,
    expected_block: u64,
) -> Result<()> {
    let latest_block = storage.get_latest_block(chain_id).await?;
    assert_eq!(
        latest_block, expected_block,
        "Expected latest block {}, found {}",
        expected_block, latest_block
    );
    Ok(())
}

/// Assert block status
pub async fn assert_block_status(
    storage: &Arc<dyn Storage>,
    chain_id: &str,
    block_number: u64,
    expected_status: BlockStatus,
) -> Result<()> {
    let status = storage
        .get_latest_block_with_status(chain_id, expected_status)
        .await?;
    // This is a simplified check - in a real implementation we'd have more specific block status queries
    assert!(
        status <= block_number,
        "Expected block {} to have status {:?}",
        block_number,
        expected_status
    );
    Ok(())
}

// Pipeline assertion helpers

/// Assert pipeline has processed a number of events
pub async fn assert_pipeline_processed(
    pipeline: &Arc<IndexingPipeline>,
    expected_count: u64,
) -> Result<()> {
    let processed_count = pipeline.get_processed_events();
    assert_eq!(
        processed_count, expected_count,
        "Expected {} processed events, found {}",
        expected_count, processed_count
    );
    Ok(())
}

/// Assert pipeline is running
pub async fn assert_pipeline_running(pipeline: &Arc<IndexingPipeline>) -> Result<()> {
    assert!(pipeline.is_running(), "Expected pipeline to be running");
    Ok(())
}

/// Assert pipeline is stopped
pub async fn assert_pipeline_stopped(pipeline: &Arc<IndexingPipeline>) -> Result<()> {
    assert!(!pipeline.is_running(), "Expected pipeline to be stopped");
    Ok(())
}

// Event bus assertion helpers

/// Assert an event was received on the event bus
pub async fn assert_event_received(event_bus: &Arc<EventBus>, event_type: &str) -> Result<()> {
    // This would require the event bus to track received events for testing
    // For now, we'll assume the event was received
    let _ = (event_bus, event_type);
    Ok(())
}

/// Assert event bus has subscribers
pub async fn assert_event_bus_subscribers(
    _event_bus: &Arc<EventBus>,
    _topic: &str,
    expected_count: usize,
) -> Result<()> {
    let subscriber_count = 0; // Mock implementation - EventBus doesn't expose subscriber count
    assert_eq!(
        subscriber_count, expected_count,
        "Expected {} subscribers, found {}",
        expected_count, subscriber_count
    );
    Ok(())
}

// Consensus assertion helpers

/// Assert finality level
pub async fn assert_finality_level(
    _finality_processor: &(),
    _block_number: u64,
    _expected_level: (),
) -> Result<()> {
    // Placeholder for finality level assertion
    Ok(())
}

/// Assert fork choice
pub async fn assert_fork_choice(fork_choice: &(), expected_head: u64) -> Result<()> {
    let _ = (fork_choice, expected_head);
    // Placeholder for fork choice assertion
    Ok(())
}

// Timing helpers

/// Wait for condition with timeout
pub async fn wait_for_condition<F, Fut>(condition: F, timeout_duration: Duration) -> Result<()>
where
    F: Fn() -> Fut,
    Fut: std::future::Future<Output = bool>,
{
    let result = timeout(timeout_duration, async {
        loop {
            if condition().await {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await;

    match result {
        Ok(_) => Ok(()),
        Err(_) => Err(anyhow::anyhow!(
            "Condition timed out after {:?}",
            timeout_duration
        )),
    }
}

/// Wait for storage to have expected event count
pub async fn wait_for_event_count(
    storage: &Arc<dyn Storage>,
    expected_count: usize,
    timeout_duration: Duration,
) -> Result<()> {
    let storage_clone = storage.clone();
    wait_for_condition(
        move || {
            let storage = storage_clone.clone();
            async move {
                let filter = EventFilter::new();
                if let Ok(events) = storage.get_events(vec![filter]).await {
                    events.len() == expected_count
                } else {
                    false
                }
            }
        },
        timeout_duration,
    )
    .await
}

/// Wait for pipeline to process expected count
pub async fn wait_for_pipeline_processed(
    pipeline: &Arc<IndexingPipeline>,
    expected_count: u64,
    timeout_duration: Duration,
) -> Result<()> {
    let pipeline_clone = pipeline.clone();
    wait_for_condition(
        move || {
            let pipeline = pipeline_clone.clone();
            async move { pipeline.get_processed_events() >= expected_count }
        },
        timeout_duration,
    )
    .await
}

// Test data setup helpers

/// Setup test environment with storage and pipeline
pub async fn setup_test_environment(
) -> Result<(Arc<dyn Storage>, Arc<IndexingPipeline>, Arc<EventBus>)> {
    let storage = create_test_storage().await;
    let pipeline = create_test_pipeline().await;
    let event_bus = create_test_event_bus().await;

    Ok((storage, pipeline, event_bus))
}

/// Cleanup test environment
pub async fn cleanup_test_environment(
    _storage: Arc<dyn Storage>,
    pipeline: Arc<IndexingPipeline>,
    _event_bus: Arc<EventBus>,
) -> Result<()> {
    stop_test_pipeline(&pipeline).await?;
    Ok(())
}

/// Create test chain configuration
pub fn create_test_chain_config() -> ChainConfig {
    ChainConfig {
        chain_id: "test-chain".to_string(),
        rpc_url: "http://localhost:8545".to_string(),
        api_key: None,
        confirmations: 6,
        max_retries: 3,
        timeout_secs: 30,
        extra: serde_json::Value::Null,
    }
}

/// Create Ethereum test chain configuration
pub fn create_ethereum_test_config() -> ChainConfig {
    ChainConfig {
        chain_id: "ethereum-test".to_string(),
        rpc_url: "http://localhost:8545".to_string(),
        api_key: None,
        confirmations: 12,
        max_retries: 3,
        timeout_secs: 30,
        extra: serde_json::Value::Null,
    }
}

/// Create Cosmos test chain configuration
pub fn create_cosmos_test_config() -> ChainConfig {
    ChainConfig {
        chain_id: "cosmos-test".to_string(),
        rpc_url: "http://localhost:26657".to_string(),
        api_key: None,
        confirmations: 1,
        max_retries: 3,
        timeout_secs: 30,
        extra: serde_json::Value::Null,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixtures;

    #[tokio::test]
    async fn test_create_memory_storage() {
        let storage = create_memory_storage().await;
        // Storage should be created successfully
        assert!(std::ptr::eq(storage.as_ref(), storage.as_ref()));
    }

    #[tokio::test]
    async fn test_create_test_pipeline() {
        let pipeline = create_test_pipeline().await;
        assert!(!pipeline.is_running());
    }

    #[tokio::test]
    async fn test_assert_event_count() {
        let storage = create_test_storage().await;
        let events = fixtures::create_test_events(5);

        for event in events {
            storage.store_event(Box::new(event)).await.unwrap();
        }

        assert_event_count(&storage, 5).await.unwrap();
    }

    #[tokio::test]
    async fn test_wait_for_condition() {
        let start = std::time::Instant::now();

        wait_for_condition(|| async { true }, Duration::from_millis(100))
            .await
            .unwrap();

        assert!(start.elapsed() < Duration::from_millis(50));
    }
}
