use almanac_indexing::{IndexingPipeline, PipelineConfig};
use tokio::time::{sleep, Duration};

#[tokio::test]
async fn test_pipeline_e2e_functionality() {
    // Test that the pipeline actually runs and processes blocks
    let config = PipelineConfig {
        chain_id: "test-chain".to_string(),
        batch_size: 5,
        interval_ms: 100,
        max_concurrent_batches: 2,
        retry_attempts: 3,
        retry_delay_ms: 100,
        finality_confirmations: 1,
        enable_validation: true,
        enable_ordering: true,
        checkpoint_interval: 500,
    };

    let pipeline = IndexingPipeline::new(config);

    // Verify initial state
    assert!(!pipeline.is_running());
    assert_eq!(pipeline.get_last_indexed_block(), 0);
    assert_eq!(pipeline.get_processed_blocks(), 0);
    assert_eq!(pipeline.get_processed_events(), 0);

    // Start the pipeline in background
    let pipeline_clone = pipeline.clone();
    let handle = tokio::spawn(async move { pipeline_clone.start().await });

    // Wait for pipeline to start and process some blocks
    sleep(Duration::from_millis(300)).await;

    // Verify pipeline is running and processing
    assert!(pipeline.is_running());
    assert!(pipeline.get_last_indexed_block() > 0);
    assert!(pipeline.get_processed_blocks() > 0);
    assert!(pipeline.get_processed_events() > 0);

    // Stop the pipeline
    pipeline.stop();

    // Wait for graceful shutdown
    sleep(Duration::from_millis(100)).await;

    // Verify pipeline stopped
    assert!(!pipeline.is_running());

    // Abort the background task
    handle.abort();
}

#[tokio::test]
async fn test_pipeline_metrics_tracking() {
    // Test that metrics are properly tracked
    let config = PipelineConfig {
        chain_id: "metrics-test".to_string(),
        batch_size: 3,
        interval_ms: 50,
        max_concurrent_batches: 1,
        retry_attempts: 1,
        retry_delay_ms: 50,
        finality_confirmations: 1,
        enable_validation: false,
        enable_ordering: false,
        checkpoint_interval: 200,
    };

    let pipeline = IndexingPipeline::new(config);

    // Start the pipeline
    let pipeline_clone = pipeline.clone();
    let handle = tokio::spawn(async move { pipeline_clone.start().await });

    // Let it run for a short time
    sleep(Duration::from_millis(200)).await;

    let blocks_processed = pipeline.get_processed_blocks();
    let events_processed = pipeline.get_processed_events();
    let last_block = pipeline.get_last_indexed_block();

    // Stop and verify metrics
    pipeline.stop();
    sleep(Duration::from_millis(50)).await;

    // Verify some processing occurred
    assert!(blocks_processed > 0, "Should have processed some blocks");
    assert!(events_processed > 0, "Should have processed some events");
    assert!(last_block > 0, "Should have indexed some blocks");

    // Verify relationship between metrics
    assert!(
        events_processed >= blocks_processed,
        "Events should be >= blocks"
    );
    // last_block is the actual block number, processed_blocks is the count
    // They should be approximately equal (last_block might be slightly higher)
    assert!(
        last_block >= blocks_processed,
        "Last block number should be >= count of blocks processed"
    );

    handle.abort();
}

#[tokio::test]
async fn test_pipeline_double_start_error() {
    // Test that starting an already running pipeline returns an error
    let config = PipelineConfig::default();
    let pipeline = IndexingPipeline::new(config);

    // Start the pipeline
    let pipeline_clone = pipeline.clone();
    let handle = tokio::spawn(async move { pipeline_clone.start().await });

    // Wait for it to start
    sleep(Duration::from_millis(50)).await;

    // Try to start again - should fail
    let result = pipeline.start().await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("already running"));

    // Clean up
    pipeline.stop();
    handle.abort();
}
