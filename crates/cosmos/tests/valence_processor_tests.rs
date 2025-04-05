// Tests for Valence Processor Contract Indexer
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

use indexer_common::Result;
use indexer_storage::{
    Storage, BoxedStorage, rocks::RocksDbStorage,
    ValenceProcessorInfo, ValenceProcessorConfig, ValenceMessageStatus, ValenceProcessorState
};
use indexer_cosmos::event::{CosmosEvent, process_valence_processor_event};

// Skip tests if not explicitly enabled via environment variable
#[cfg(not(feature = "integration_tests"))]
fn should_run_tests() -> bool {
    std::env::var("RUN_CONTRACT_TESTS").is_ok()
}

// Always run if integration_tests feature is enabled
#[cfg(feature = "integration_tests")]
fn should_run_tests() -> bool {
    true
}

// Helper function to create a mock cosmos event
fn create_mock_cosmos_event(
    chain_id: &str,
    block_number: u64,
    tx_hash: &str,
    event_type: &str,
    attributes: HashMap<String, String>,
) -> CosmosEvent {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    
    CosmosEvent::new(
        Uuid::new_v4().to_string(),
        chain_id.to_string(),
        block_number,
        format!("block-hash-{}", block_number),
        tx_hash.to_string(),
        timestamp,
        event_type.to_string(),
        attributes,
    )
}

// Create a mock instantiate event for a Valence Processor
fn create_mock_processor_instantiate_event(
    chain_id: &str, 
    block_number: u64, 
    contract_address: &str, 
    owner: &str
) -> CosmosEvent {
    let mut attributes = HashMap::new();
    attributes.insert("_contract_address".to_string(), contract_address.to_string());
    attributes.insert("action".to_string(), "instantiate".to_string());
    attributes.insert("owner".to_string(), owner.to_string());
    attributes.insert("_module".to_string(), "wasm".to_string());
    
    // Processor-specific config attributes
    attributes.insert("max_gas_per_message".to_string(), "500000".to_string());
    attributes.insert("message_timeout_blocks".to_string(), "100".to_string());
    attributes.insert("retry_interval_blocks".to_string(), "5".to_string());
    attributes.insert("max_retry_count".to_string(), "3".to_string());
    
    create_mock_cosmos_event(
        chain_id,
        block_number,
        &format!("tx-hash-{}", Uuid::new_v4()),
        "wasm",
        attributes,
    )
}

// Create a mock config update event
fn create_mock_config_update_event(
    chain_id: &str, 
    block_number: u64, 
    contract_address: &str,
    max_gas: Option<&str>,
    timeout_blocks: Option<&str>,
    retry_interval: Option<&str>,
    max_retry: Option<&str>,
    paused: Option<bool>
) -> CosmosEvent {
    let mut attributes = HashMap::new();
    attributes.insert("_contract_address".to_string(), contract_address.to_string());
    attributes.insert("action".to_string(), "execute".to_string());
    attributes.insert("method".to_string(), "update_config".to_string());
    attributes.insert("_module".to_string(), "wasm".to_string());
    
    if let Some(gas) = max_gas {
        attributes.insert("max_gas_per_message".to_string(), gas.to_string());
    }
    
    if let Some(timeout) = timeout_blocks {
        attributes.insert("message_timeout_blocks".to_string(), timeout.to_string());
    }
    
    if let Some(interval) = retry_interval {
        attributes.insert("retry_interval_blocks".to_string(), interval.to_string());
    }
    
    if let Some(retries) = max_retry {
        attributes.insert("max_retry_count".to_string(), retries.to_string());
    }
    
    if let Some(is_paused) = paused {
        attributes.insert("paused".to_string(), is_paused.to_string());
    }
    
    create_mock_cosmos_event(
        chain_id,
        block_number,
        &format!("tx-hash-{}", Uuid::new_v4()),
        "wasm",
        attributes,
    )
}

// Create a mock message submission event
fn create_mock_submit_message_event(
    chain_id: &str,
    block_number: u64,
    contract_address: &str,
    message_id: &str,
    target_chain: &str,
    sender: &str,
    payload: &str
) -> CosmosEvent {
    let mut attributes = HashMap::new();
    attributes.insert("_contract_address".to_string(), contract_address.to_string());
    attributes.insert("action".to_string(), "execute".to_string());
    attributes.insert("method".to_string(), "submit_message".to_string());
    attributes.insert("_module".to_string(), "wasm".to_string());
    
    attributes.insert("message_id".to_string(), message_id.to_string());
    attributes.insert("target_chain".to_string(), target_chain.to_string());
    attributes.insert("sender".to_string(), sender.to_string());
    attributes.insert("payload".to_string(), payload.to_string());
    
    create_mock_cosmos_event(
        chain_id,
        block_number,
        &format!("tx-hash-{}", Uuid::new_v4()),
        "wasm",
        attributes,
    )
}

// Create a mock message execution event
fn create_mock_execute_message_event(
    chain_id: &str,
    block_number: u64,
    contract_address: &str,
    message_id: &str,
    success: bool,
    gas_used: u64,
    error: Option<&str>
) -> CosmosEvent {
    let mut attributes = HashMap::new();
    attributes.insert("_contract_address".to_string(), contract_address.to_string());
    attributes.insert("action".to_string(), "execute".to_string());
    attributes.insert("method".to_string(), "execute_message".to_string());
    attributes.insert("_module".to_string(), "wasm".to_string());
    
    attributes.insert("message_id".to_string(), message_id.to_string());
    attributes.insert("success".to_string(), success.to_string());
    attributes.insert("gas_used".to_string(), gas_used.to_string());
    
    if let Some(err) = error {
        attributes.insert("error".to_string(), err.to_string());
    }
    
    create_mock_cosmos_event(
        chain_id,
        block_number,
        &format!("tx-hash-{}", Uuid::new_v4()),
        "wasm",
        attributes,
    )
}

// Create a mock message timeout event
fn create_mock_timeout_message_event(
    chain_id: &str,
    block_number: u64,
    contract_address: &str,
    message_id: &str
) -> CosmosEvent {
    let mut attributes = HashMap::new();
    attributes.insert("_contract_address".to_string(), contract_address.to_string());
    attributes.insert("action".to_string(), "execute".to_string());
    attributes.insert("method".to_string(), "timeout_message".to_string());
    attributes.insert("_module".to_string(), "wasm".to_string());
    
    attributes.insert("message_id".to_string(), message_id.to_string());
    
    create_mock_cosmos_event(
        chain_id,
        block_number,
        &format!("tx-hash-{}", Uuid::new_v4()),
        "wasm",
        attributes,
    )
}

// Setup test storage
async fn setup_test_storage() -> Result<BoxedStorage> {
    // Get environment variables for test database connections
    let rocksdb_path = std::env::var("TEST_ROCKSDB_PATH")
        .unwrap_or_else(|_| "/tmp/almanac_test_rocksdb".to_string());
    
    // Create RocksDB storage
    let rocks = RocksDbStorage::new(&rocksdb_path)?;
    
    // Use RocksDB as storage for these tests
    let storage = Arc::new(rocks) as BoxedStorage;
    
    Ok(storage)
}

#[tokio::test]
async fn test_processor_instantiation() -> Result<()> {
    if !should_run_tests() {
        println!("Skipping Valence Processor tests (RUN_CONTRACT_TESTS not set)");
        return Ok(());
    }
    
    // Setup test storage
    let storage = setup_test_storage().await?;
    
    // Test parameters
    let chain_id = "valence-test";
    let contract_address = format!("processor{}", Uuid::new_v4().simple());
    let owner = "cosmos1owner";
    let block_number = 100u64;
    
    // Create a mock instantiation event
    let event = create_mock_processor_instantiate_event(chain_id, block_number, &contract_address, owner);
    let tx_hash = event.tx_hash().to_string();
    
    // Process the event
    process_valence_processor_event(&storage, chain_id, &event, &tx_hash).await?;
    
    // Verify the processor was created in storage
    let processor_id = format!("{}:{}", chain_id, contract_address);
    let processor_state = storage.get_valence_processor_state(&processor_id).await?;
    
    assert!(processor_state.is_some(), "Processor state should exist after instantiation");
    
    if let Some(state) = processor_state {
        assert_eq!(state.processor_id, processor_id, "Processor ID should match");
        assert_eq!(state.chain_id, chain_id, "Chain ID should match");
        assert_eq!(state.address, contract_address, "Contract address should match");
        assert_eq!(state.owner, Some(owner.to_string()), "Owner should match");
        assert_eq!(state.last_update_block, block_number, "Last update block should match");
        
        // Check configuration
        assert!(state.config.is_some(), "Config should be present");
        if let Some(config) = state.config {
            assert_eq!(config.max_gas_per_message, Some(500000), "Max gas should match");
            assert_eq!(config.message_timeout_blocks, Some(100), "Timeout blocks should match");
            assert_eq!(config.retry_interval_blocks, Some(5), "Retry interval should match");
            assert_eq!(config.max_retry_count, Some(3), "Max retry count should match");
            assert_eq!(config.paused, false, "Processor should not be paused initially");
        }
    }
    
    println!("Processor instantiation test passed!");
    Ok(())
}

#[tokio::test]
async fn test_config_update() -> Result<()> {
    if !should_run_tests() {
        return Ok(());
    }
    
    // Setup test storage
    let storage = setup_test_storage().await?;
    
    // Test parameters
    let chain_id = "valence-test";
    let contract_address = format!("processor{}", Uuid::new_v4().simple());
    let owner = "cosmos1owner";
    let block_number = 100u64;
    let update_block = 110u64;
    
    // First create a processor
    let instantiate_event = create_mock_processor_instantiate_event(chain_id, block_number, &contract_address, owner);
    let tx_hash = instantiate_event.tx_hash().to_string();
    process_valence_processor_event(&storage, chain_id, &instantiate_event, &tx_hash).await?;
    
    // Now update its configuration
    let update_event = create_mock_config_update_event(
        chain_id, 
        update_block, 
        &contract_address,
        Some("1000000"),  // max_gas
        Some("200"),      // timeout_blocks
        Some("10"),       // retry_interval
        Some("5"),        // max_retry
        Some(true)        // paused
    );
    let update_tx = update_event.tx_hash().to_string();
    process_valence_processor_event(&storage, chain_id, &update_event, &update_tx).await?;
    
    // Verify the config was updated
    let processor_id = format!("{}:{}", chain_id, contract_address);
    let processor_state = storage.get_valence_processor_state(&processor_id).await?;
    
    assert!(processor_state.is_some(), "Processor state should exist");
    
    if let Some(state) = processor_state {
        assert_eq!(state.last_update_block, update_block, "Last update block should be updated");
        
        // Check updated configuration
        assert!(state.config.is_some(), "Config should be present");
        if let Some(config) = state.config {
            assert_eq!(config.max_gas_per_message, Some(1000000), "Max gas should be updated");
            assert_eq!(config.message_timeout_blocks, Some(200), "Timeout blocks should be updated");
            assert_eq!(config.retry_interval_blocks, Some(10), "Retry interval should be updated");
            assert_eq!(config.max_retry_count, Some(5), "Max retry count should be updated");
            assert_eq!(config.paused, true, "Processor should be paused");
        }
    }
    
    println!("Config update test passed!");
    Ok(())
}

#[tokio::test]
async fn test_message_workflow() -> Result<()> {
    if !should_run_tests() {
        return Ok(());
    }
    
    // Setup test storage
    let storage = setup_test_storage().await?;
    
    // Test parameters
    let chain_id = "valence-test";
    let contract_address = format!("processor{}", Uuid::new_v4().simple());
    let owner = "cosmos1owner";
    let block_number = 100u64;
    
    // Create message parameters
    let message_id = format!("msg-{}", Uuid::new_v4().simple());
    let target_chain = "ethereum-1";
    let sender = "cosmos1sender";
    let payload = "test_payload_data";
    
    // First create a processor
    let instantiate_event = create_mock_processor_instantiate_event(chain_id, block_number, &contract_address, owner);
    let tx_hash = instantiate_event.tx_hash().to_string();
    process_valence_processor_event(&storage, chain_id, &instantiate_event, &tx_hash).await?;
    
    // Submit a message at block 110
    let submit_event = create_mock_submit_message_event(
        chain_id,
        110,
        &contract_address,
        &message_id,
        target_chain,
        sender,
        payload
    );
    let submit_tx = submit_event.tx_hash().to_string();
    process_valence_processor_event(&storage, chain_id, &submit_event, &submit_tx).await?;
    
    // Execute the message successfully at block 120
    let execute_event = create_mock_execute_message_event(
        chain_id,
        120,
        &contract_address,
        &message_id,
        true,   // success
        250000, // gas_used
        None    // no error
    );
    let execute_tx = execute_event.tx_hash().to_string();
    process_valence_processor_event(&storage, chain_id, &execute_event, &execute_tx).await?;
    
    // Verify the processor state reflects the message lifecycle
    let processor_id = format!("{}:{}", chain_id, contract_address);
    let processor_state = storage.get_valence_processor_state(&processor_id).await?;
    
    assert!(processor_state.is_some(), "Processor state should exist");
    
    if let Some(state) = processor_state {
        assert_eq!(state.last_update_block, 120, "Last update block should reflect execution");
        assert_eq!(state.completed_message_count, 1, "Should have 1 completed message");
        assert_eq!(state.pending_message_count, 0, "Should have 0 pending messages");
        assert_eq!(state.failed_message_count, 0, "Should have 0 failed messages");
    }
    
    println!("Message workflow test passed!");
    Ok(())
}

#[tokio::test]
async fn test_message_failure_and_timeout() -> Result<()> {
    if !should_run_tests() {
        return Ok(());
    }
    
    // Setup test storage
    let storage = setup_test_storage().await?;
    
    // Test parameters
    let chain_id = "valence-test";
    let contract_address = format!("processor{}", Uuid::new_v4().simple());
    let owner = "cosmos1owner";
    let block_number = 100u64;
    
    // Create message parameters
    let message1_id = format!("msg1-{}", Uuid::new_v4().simple());
    let message2_id = format!("msg2-{}", Uuid::new_v4().simple());
    let target_chain = "ethereum-1";
    let sender = "cosmos1sender";
    let payload = "test_payload_data";
    
    // First create a processor
    let instantiate_event = create_mock_processor_instantiate_event(chain_id, block_number, &contract_address, owner);
    let tx_hash = instantiate_event.tx_hash().to_string();
    process_valence_processor_event(&storage, chain_id, &instantiate_event, &tx_hash).await?;
    
    // Submit first message at block 110
    let submit1_event = create_mock_submit_message_event(
        chain_id,
        110,
        &contract_address,
        &message1_id,
        target_chain,
        sender,
        payload
    );
    let submit1_tx = submit1_event.tx_hash().to_string();
    process_valence_processor_event(&storage, chain_id, &submit1_event, &submit1_tx).await?;
    
    // Submit second message at block 115
    let submit2_event = create_mock_submit_message_event(
        chain_id,
        115,
        &contract_address,
        &message2_id,
        target_chain,
        sender,
        payload
    );
    let submit2_tx = submit2_event.tx_hash().to_string();
    process_valence_processor_event(&storage, chain_id, &submit2_event, &submit2_tx).await?;
    
    // Execute the first message with failure at block 120
    let fail_event = create_mock_execute_message_event(
        chain_id,
        120,
        &contract_address,
        &message1_id,
        false,              // failure
        350000,             // gas_used
        Some("Out of gas")  // error message
    );
    let fail_tx = fail_event.tx_hash().to_string();
    process_valence_processor_event(&storage, chain_id, &fail_event, &fail_tx).await?;
    
    // Timeout the second message at block 220
    let timeout_event = create_mock_timeout_message_event(
        chain_id,
        220,
        &contract_address,
        &message2_id
    );
    let timeout_tx = timeout_event.tx_hash().to_string();
    process_valence_processor_event(&storage, chain_id, &timeout_event, &timeout_tx).await?;
    
    // Verify the processor state reflects both failed and timed out messages
    let processor_id = format!("{}:{}", chain_id, contract_address);
    let processor_state = storage.get_valence_processor_state(&processor_id).await?;
    
    assert!(processor_state.is_some(), "Processor state should exist");
    
    if let Some(state) = processor_state {
        assert_eq!(state.last_update_block, 220, "Last update block should reflect timeout");
        assert_eq!(state.completed_message_count, 0, "Should have 0 completed messages");
        assert_eq!(state.pending_message_count, 0, "Should have 0 pending messages");
        assert_eq!(state.failed_message_count, 2, "Should have 2 failed messages");
    }
    
    println!("Message failure and timeout test passed!");
    Ok(())
}

#[tokio::test]
async fn test_complex_processor_workflow() -> Result<()> {
    if !should_run_tests() {
        return Ok(());
    }
    
    // Setup test storage
    let storage = setup_test_storage().await?;
    
    // Test parameters
    let chain_id = "valence-test";
    let contract_address = format!("processor{}", Uuid::new_v4().simple());
    let owner = "cosmos1owner";
    
    // Create three different message IDs
    let message1_id = format!("msg1-{}", Uuid::new_v4().simple());
    let message2_id = format!("msg2-{}", Uuid::new_v4().simple());
    let message3_id = format!("msg3-{}", Uuid::new_v4().simple());
    
    let target_chain = "ethereum-1";
    let sender = "cosmos1sender";
    
    // Comprehensive test scenario:
    // 1. Instantiate processor at block 100
    let instantiate_event = create_mock_processor_instantiate_event(chain_id, 100, &contract_address, owner);
    let tx_hash = instantiate_event.tx_hash().to_string();
    process_valence_processor_event(&storage, chain_id, &instantiate_event, &tx_hash).await?;
    
    // 2. Update config at block 110
    let update_event = create_mock_config_update_event(
        chain_id, 110, &contract_address,
        Some("750000"), Some("150"), Some("7"), Some("4"), None
    );
    let update_tx = update_event.tx_hash().to_string();
    process_valence_processor_event(&storage, chain_id, &update_event, &update_tx).await?;
    
    // 3. Submit three messages at blocks 120, 125, 130
    let submit1_event = create_mock_submit_message_event(
        chain_id, 120, &contract_address, &message1_id, target_chain, sender, "payload1"
    );
    process_valence_processor_event(&storage, chain_id, &submit1_event, &submit1_event.tx_hash()).await?;
    
    let submit2_event = create_mock_submit_message_event(
        chain_id, 125, &contract_address, &message2_id, target_chain, sender, "payload2"
    );
    process_valence_processor_event(&storage, chain_id, &submit2_event, &submit2_event.tx_hash()).await?;
    
    let submit3_event = create_mock_submit_message_event(
        chain_id, 130, &contract_address, &message3_id, target_chain, sender, "payload3"
    );
    process_valence_processor_event(&storage, chain_id, &submit3_event, &submit3_event.tx_hash()).await?;
    
    // 4. Successfully execute message 1 at block 140
    let execute1_event = create_mock_execute_message_event(
        chain_id, 140, &contract_address, &message1_id, true, 200000, None
    );
    process_valence_processor_event(&storage, chain_id, &execute1_event, &execute1_event.tx_hash()).await?;
    
    // 5. Fail to execute message 2 at block 150
    let fail_event = create_mock_execute_message_event(
        chain_id, 150, &contract_address, &message2_id, false, 400000, Some("Contract reverted")
    );
    process_valence_processor_event(&storage, chain_id, &fail_event, &fail_event.tx_hash()).await?;
    
    // 6. Timeout message 3 at block 300
    let timeout_event = create_mock_timeout_message_event(
        chain_id, 300, &contract_address, &message3_id
    );
    process_valence_processor_event(&storage, chain_id, &timeout_event, &timeout_event.tx_hash()).await?;
    
    // 7. Pause the processor at block 350
    let pause_event = create_mock_config_update_event(
        chain_id, 350, &contract_address, None, None, None, None, Some(true)
    );
    process_valence_processor_event(&storage, chain_id, &pause_event, &pause_event.tx_hash()).await?;
    
    // Verify final processor state
    let processor_id = format!("{}:{}", chain_id, contract_address);
    let processor_state = storage.get_valence_processor_state(&processor_id).await?;
    
    assert!(processor_state.is_some(), "Processor state should exist");
    
    if let Some(state) = processor_state {
        // Verify config
        assert!(state.config.is_some(), "Config should be present");
        if let Some(config) = state.config {
            assert_eq!(config.max_gas_per_message, Some(750000), "Max gas should be 750000");
            assert_eq!(config.message_timeout_blocks, Some(150), "Timeout blocks should be 150");
            assert_eq!(config.retry_interval_blocks, Some(7), "Retry interval should be 7");
            assert_eq!(config.max_retry_count, Some(4), "Max retry count should be 4");
            assert_eq!(config.paused, true, "Processor should be paused");
        }
        
        // Verify message counts
        assert_eq!(state.completed_message_count, 1, "Should have 1 completed message");
        assert_eq!(state.pending_message_count, 0, "Should have 0 pending messages");
        assert_eq!(state.failed_message_count, 2, "Should have 2 failed messages");
        
        // Verify block number
        assert_eq!(state.last_update_block, 350, "Last update block should be 350");
    }
    
    println!("Complex processor workflow test passed!");
    Ok(())
} 