//! Integration tests for almanac-testing

use almanac_testing::{fixtures, helpers, mocks};
use almanac_core::{EventFilter, BlockStatus, Storage};
use almanac_chains::{FinalityStatus, ChainAdapter};
use tokio::time::Duration;

#[tokio::test]
async fn test_full_testing_workflow() {
    // Create test fixtures
    let test_events = fixtures::create_test_events(10);
    let _ethereum_events = fixtures::create_ethereum_events(5);
    
    // Create mock storage with test data
    let storage = mocks::MockStorage::new()
        .with_events(test_events.clone())
        .with_latest_block("ethereum", 1000)
        .with_block_status("ethereum", 1000, BlockStatus::Finalized);
    
    // Test storage operations
    let filter = EventFilter::new().with_chain("test-chain");
    let retrieved_events = storage.get_events(vec![filter]).await.unwrap();
    assert_eq!(retrieved_events.len(), 10);
    
    // Test latest block
    let latest_block = storage.get_latest_block("ethereum").await.unwrap();
    assert_eq!(latest_block, 1000);
    
    // Test block status update
    storage.update_block_status("ethereum", 1001, BlockStatus::Pending).await.unwrap();
}

#[tokio::test]
async fn test_mock_chain_adapter() {
    // Create test chain data
    let chain_data = fixtures::create_ethereum_chain_data();
    
    // Create mock adapter
    let adapter = mocks::MockChainAdapter::new()
        .with_chain_id("ethereum")
        .with_latest_block(18000000)
        .with_block_data(18000000, chain_data);
    
    // Test adapter operations
    assert_eq!(adapter.chain_id(), "ethereum");
    assert_eq!(adapter.get_latest_block().await.unwrap(), 18000000);
    
    let block_data = adapter.fetch_block(18000000).await.unwrap();
    assert_eq!(block_data.block_metadata.block_height, 18000000);
    assert_eq!(block_data.finality_status, FinalityStatus::Finalized);
}

#[tokio::test]
async fn test_helpers_and_assertions() {
    // Create test environment
    let storage = helpers::create_test_storage().await;
    let pipeline = helpers::create_test_pipeline().await;
    
    // Add test events to storage
    let events = fixtures::create_test_events(5);
    for event in events {
        storage.store_event(Box::new(event)).await.unwrap();
    }
    
    // Test helper assertions
    helpers::assert_event_count(&storage, 5).await.unwrap();
    helpers::assert_pipeline_stopped(&pipeline).await.unwrap();
}

#[tokio::test]
async fn test_transfer_event_fixture() {
    let transfer_event = fixtures::create_transfer_event(
        "ethereum",
        1000,
        "0x1234567890123456789012345678901234567890",
        "0x0987654321098765432109876543210987654321",
        "1000000000000000000"
    );
    
    assert_eq!(transfer_event.chain, "ethereum");
    assert_eq!(transfer_event.block_number, 1000);
    assert_eq!(transfer_event.event_type, "Transfer");
    
    // Verify transfer data
    let attributes = &transfer_event.attributes;
    assert_eq!(attributes["from"].as_str().unwrap(), "0x1234567890123456789012345678901234567890");
    assert_eq!(attributes["to"].as_str().unwrap(), "0x0987654321098765432109876543210987654321");
    assert_eq!(attributes["amount"].as_str().unwrap(), "1000000000000000000");
}

#[tokio::test]
async fn test_wait_for_condition() {
    let start = std::time::Instant::now();
    
    // Test successful condition
    helpers::wait_for_condition(
        || async { true },
        Duration::from_millis(100)
    ).await.unwrap();
    
    assert!(start.elapsed() < Duration::from_millis(50));
    
    // Test timeout condition
    let result = helpers::wait_for_condition(
        || async { false },
        Duration::from_millis(50)
    ).await;
    
    assert!(result.is_err());
}

#[tokio::test]
async fn test_chain_specific_fixtures() {
    // Test Ethereum fixtures
    let ethereum_events = fixtures::create_ethereum_events(3);
    assert_eq!(ethereum_events.len(), 3);
    assert!(ethereum_events.iter().all(|e| e.chain == "ethereum"));
    assert!(ethereum_events.iter().all(|e| e.tx_hash.starts_with("0x")));
    
    // Test Cosmos fixtures
    let cosmos_events = fixtures::create_cosmos_events(2);
    assert_eq!(cosmos_events.len(), 2);
    assert!(cosmos_events.iter().all(|e| e.chain == "cosmos"));
    assert!(cosmos_events.iter().all(|e| e.tx_hash.starts_with("cosmos_tx_")));
    
    // Test block fixtures
    let ethereum_blocks = fixtures::create_ethereum_blocks(5);
    assert_eq!(ethereum_blocks.len(), 5);
    assert!(ethereum_blocks.iter().all(|b| b.block_hash.starts_with("0x")));
}

#[tokio::test]
async fn test_environment_setup_and_cleanup() {
    // Setup test environment
    let (storage, pipeline, event_bus) = helpers::setup_test_environment().await.unwrap();
    
    // Verify components are created successfully
    assert!(!pipeline.is_running());
    // Storage and event_bus are guaranteed to be valid references in Rust
    
    // Cleanup
    helpers::cleanup_test_environment(storage, pipeline, event_bus).await.unwrap();
}

#[tokio::test]
async fn test_event_filter_creation() {
    let filter = fixtures::create_test_event_filter();
    
    assert!(filter.matches_chain("test-chain"));
    assert!(!filter.matches_chain("other-chain"));
    assert!(filter.matches_block(1500));
    assert!(!filter.matches_block(3000));
}

#[tokio::test]
async fn test_contract_call_event() {
    let contract_call = fixtures::create_contract_call_event(
        "ethereum",
        1001,
        "0xcontract123",
        "transfer",
        serde_json::json!({"to": "0x1234", "value": "1000"})
    );
    
    assert_eq!(contract_call.chain, "ethereum");
    assert_eq!(contract_call.block_number, 1001);
    assert_eq!(contract_call.event_type, "transfer");
    assert_eq!(contract_call.contract_address, Some("0xcontract123".to_string()));
    
    // Verify method call data
    let attributes = &contract_call.attributes;
    assert_eq!(attributes["method"].as_str().unwrap(), "transfer");
    assert_eq!(attributes["args"]["to"].as_str().unwrap(), "0x1234");
    assert_eq!(attributes["args"]["value"].as_str().unwrap(), "1000");
}