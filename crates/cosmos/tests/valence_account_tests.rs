// Tests for Valence Account Contract Indexer
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

use indexer_common::Result;
use indexer_storage::{
    postgres::PostgresStorage,
    rocks::RocksDbStorage,
    Storage, BoxedStorage, ValenceAccountInfo, ValenceAccountLibrary, ValenceAccountState
};
use indexer_cosmos::event::{CosmosEvent, process_valence_account_event};

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

// Create a mock instantiate event for a Valence Account
fn create_mock_instantiate_event(chain_id: &str, block_number: u64, contract_address: &str, owner: &str) -> CosmosEvent {
    let mut attributes = HashMap::new();
    attributes.insert("_contract_address".to_string(), contract_address.to_string());
    attributes.insert("action".to_string(), "instantiate".to_string());
    attributes.insert("owner".to_string(), owner.to_string());
    attributes.insert("_module".to_string(), "wasm".to_string());
    
    create_mock_cosmos_event(
        chain_id,
        block_number,
        &format!("tx-hash-{}", Uuid::new_v4()),
        "wasm",
        attributes,
    )
}

// Create a mock library approval event
fn create_mock_library_approval_event(chain_id: &str, block_number: u64, contract_address: &str, library_address: &str) -> CosmosEvent {
    let mut attributes = HashMap::new();
    attributes.insert("_contract_address".to_string(), contract_address.to_string());
    attributes.insert("action".to_string(), "execute".to_string());
    attributes.insert("method".to_string(), "approve_library".to_string());
    attributes.insert("library_address".to_string(), library_address.to_string());
    attributes.insert("_module".to_string(), "wasm".to_string());
    
    create_mock_cosmos_event(
        chain_id,
        block_number,
        &format!("tx-hash-{}", Uuid::new_v4()),
        "wasm",
        attributes,
    )
}

// Create a mock ownership transfer event
fn create_mock_ownership_transfer_event(chain_id: &str, block_number: u64, contract_address: &str, new_owner: &str) -> CosmosEvent {
    let mut attributes = HashMap::new();
    attributes.insert("_contract_address".to_string(), contract_address.to_string());
    attributes.insert("action".to_string(), "execute".to_string());
    attributes.insert("method".to_string(), "transfer_ownership".to_string());
    attributes.insert("new_owner".to_string(), new_owner.to_string());
    attributes.insert("_module".to_string(), "wasm".to_string());
    
    create_mock_cosmos_event(
        chain_id,
        block_number,
        &format!("tx-hash-{}", Uuid::new_v4()),
        "wasm",
        attributes,
    )
}

// Create a mock execution event
fn create_mock_execution_event(
    chain_id: &str, 
    block_number: u64, 
    contract_address: &str, 
    executor: &str, 
    payload: &str
) -> CosmosEvent {
    let mut attributes = HashMap::new();
    attributes.insert("_contract_address".to_string(), contract_address.to_string());
    attributes.insert("action".to_string(), "execute".to_string());
    attributes.insert("method".to_string(), "execute".to_string());
    attributes.insert("executor".to_string(), executor.to_string());
    attributes.insert("payload".to_string(), payload.to_string());
    attributes.insert("_module".to_string(), "wasm".to_string());
    
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
    let postgres_url = std::env::var("TEST_POSTGRES_URL")
        .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5432/indexer_test".to_string());
    
    let rocksdb_path = std::env::var("TEST_ROCKSDB_PATH")
        .unwrap_or_else(|_| "/tmp/almanac_test_rocksdb".to_string());
    
    // Create PostgreSQL storage
    let postgres = PostgresStorage::new(&postgres_url).await?;
    
    // Create RocksDB storage
    let rocks = RocksDbStorage::new(&rocksdb_path)?;
    
    // Create a multi-store that uses both
    let storage = Arc::new(rocks) as BoxedStorage;
    
    Ok(storage)
}

#[tokio::test]
async fn test_account_instantiation() -> Result<()> {
    if !should_run_tests() {
        println!("Skipping Valence Account tests (RUN_CONTRACT_TESTS not set)");
        return Ok(());
    }
    
    // Setup test storage
    let storage = setup_test_storage().await?;
    
    // Test parameters
    let chain_id = "valence-test";
    let contract_address = format!("valence{}", Uuid::new_v4().simple());
    let owner = "cosmos1owner";
    let block_number = 100u64;
    
    // Create a mock instantiation event
    let event = create_mock_instantiate_event(chain_id, block_number, &contract_address, owner);
    let tx_hash = event.tx_hash().to_string();
    
    // Process the event
    process_valence_account_event(&storage, chain_id, &event, &tx_hash).await?;
    
    // Verify the account was created in storage
    let account_id = format!("{}:{}", chain_id, contract_address);
    let account_state = storage.get_valence_account_state(&account_id).await?;
    
    assert!(account_state.is_some(), "Account state should exist after instantiation");
    
    if let Some(state) = account_state {
        assert_eq!(state.account_id, account_id, "Account ID should match");
        assert_eq!(state.chain_id, chain_id, "Chain ID should match");
        assert_eq!(state.address, contract_address, "Contract address should match");
        assert_eq!(state.current_owner, Some(owner.to_string()), "Owner should match");
        assert_eq!(state.last_update_block, block_number, "Last update block should match");
        assert!(state.libraries.is_empty(), "Libraries should be empty initially");
    }
    
    println!("Account instantiation test passed!");
    Ok(())
}

#[tokio::test]
async fn test_library_approval() -> Result<()> {
    if !should_run_tests() {
        return Ok(());
    }
    
    // Setup test storage
    let storage = setup_test_storage().await?;
    
    // Test parameters
    let chain_id = "valence-test";
    let contract_address = format!("valence{}", Uuid::new_v4().simple());
    let owner = "cosmos1owner";
    let library_address = format!("library{}", Uuid::new_v4().simple());
    let block_number = 100u64;
    let approval_block = 101u64;
    
    // First create an account
    let instantiate_event = create_mock_instantiate_event(chain_id, block_number, &contract_address, owner);
    let tx_hash = instantiate_event.tx_hash().to_string();
    process_valence_account_event(&storage, chain_id, &instantiate_event, &tx_hash).await?;
    
    // Now approve a library
    let approval_event = create_mock_library_approval_event(chain_id, approval_block, &contract_address, &library_address);
    let approval_tx = approval_event.tx_hash().to_string();
    process_valence_account_event(&storage, chain_id, &approval_event, &approval_tx).await?;
    
    // Verify the library was approved
    let account_id = format!("{}:{}", chain_id, contract_address);
    let account_state = storage.get_valence_account_state(&account_id).await?;
    
    assert!(account_state.is_some(), "Account state should exist");
    
    if let Some(state) = account_state {
        assert_eq!(state.last_update_block, approval_block, "Last update block should be updated");
        assert_eq!(state.libraries.len(), 1, "Should have one approved library");
        assert!(state.libraries.contains(&library_address), "Library address should be in approved list");
    }
    
    println!("Library approval test passed!");
    Ok(())
}

#[tokio::test]
async fn test_ownership_transfer() -> Result<()> {
    if !should_run_tests() {
        return Ok(());
    }
    
    // Setup test storage
    let storage = setup_test_storage().await?;
    
    // Test parameters
    let chain_id = "valence-test";
    let contract_address = format!("valence{}", Uuid::new_v4().simple());
    let original_owner = "cosmos1owner";
    let new_owner = "cosmos1newowner";
    let block_number = 100u64;
    let transfer_block = 102u64;
    
    // First create an account
    let instantiate_event = create_mock_instantiate_event(chain_id, block_number, &contract_address, original_owner);
    let tx_hash = instantiate_event.tx_hash().to_string();
    process_valence_account_event(&storage, chain_id, &instantiate_event, &tx_hash).await?;
    
    // Now transfer ownership
    let transfer_event = create_mock_ownership_transfer_event(chain_id, transfer_block, &contract_address, new_owner);
    let transfer_tx = transfer_event.tx_hash().to_string();
    process_valence_account_event(&storage, chain_id, &transfer_event, &transfer_tx).await?;
    
    // Verify ownership was transferred
    let account_id = format!("{}:{}", chain_id, contract_address);
    let account_state = storage.get_valence_account_state(&account_id).await?;
    
    assert!(account_state.is_some(), "Account state should exist");
    
    if let Some(state) = account_state {
        assert_eq!(state.last_update_block, transfer_block, "Last update block should be updated");
        assert_eq!(state.current_owner, Some(new_owner.to_string()), "Owner should be updated");
    }
    
    println!("Ownership transfer test passed!");
    Ok(())
}

#[tokio::test]
async fn test_account_execution() -> Result<()> {
    if !should_run_tests() {
        return Ok(());
    }
    
    // Setup test storage
    let storage = setup_test_storage().await?;
    
    // Test parameters
    let chain_id = "valence-test";
    let contract_address = format!("valence{}", Uuid::new_v4().simple());
    let owner = "cosmos1owner";
    let executor = "cosmos1executor";
    let payload = "execute_payload_data";
    let block_number = 100u64;
    let execution_block = 103u64;
    
    // First create an account
    let instantiate_event = create_mock_instantiate_event(chain_id, block_number, &contract_address, owner);
    let tx_hash = instantiate_event.tx_hash().to_string();
    process_valence_account_event(&storage, chain_id, &instantiate_event, &tx_hash).await?;
    
    // Now execute with the account
    let execution_event = create_mock_execution_event(chain_id, execution_block, &contract_address, executor, payload);
    let execution_tx = execution_event.tx_hash().to_string();
    process_valence_account_event(&storage, chain_id, &execution_event, &execution_tx).await?;
    
    // We don't have a direct way to query executions from the storage trait,
    // but we can check that the account state is still valid
    let account_id = format!("{}:{}", chain_id, contract_address);
    let account_state = storage.get_valence_account_state(&account_id).await?;
    
    assert!(account_state.is_some(), "Account state should exist");
    
    // Note: In a real implementation, we would add a method to query executions
    // and verify that the execution was properly recorded
    
    println!("Account execution test passed!");
    Ok(())
}

#[tokio::test]
async fn test_complex_account_workflow() -> Result<()> {
    if !should_run_tests() {
        return Ok(());
    }
    
    // Setup test storage
    let storage = setup_test_storage().await?;
    
    // Test parameters
    let chain_id = "valence-test";
    let contract_address = format!("valence{}", Uuid::new_v4().simple());
    let original_owner = "cosmos1owner";
    let new_owner = "cosmos1newowner";
    let library1 = format!("library{}", Uuid::new_v4().simple());
    let library2 = format!("library{}", Uuid::new_v4().simple());
    
    // Create the account at block 100
    let instantiate_event = create_mock_instantiate_event(chain_id, 100, &contract_address, original_owner);
    let tx_hash = instantiate_event.tx_hash().to_string();
    process_valence_account_event(&storage, chain_id, &instantiate_event, &tx_hash).await?;
    
    // Approve first library at block 110
    let library1_event = create_mock_library_approval_event(chain_id, 110, &contract_address, &library1);
    let library1_tx = library1_event.tx_hash().to_string();
    process_valence_account_event(&storage, chain_id, &library1_event, &library1_tx).await?;
    
    // Transfer ownership at block 120
    let transfer_event = create_mock_ownership_transfer_event(chain_id, 120, &contract_address, new_owner);
    let transfer_tx = transfer_event.tx_hash().to_string();
    process_valence_account_event(&storage, chain_id, &transfer_event, &transfer_tx).await?;
    
    // Approve second library at block 130
    let library2_event = create_mock_library_approval_event(chain_id, 130, &contract_address, &library2);
    let library2_tx = library2_event.tx_hash().to_string();
    process_valence_account_event(&storage, chain_id, &library2_event, &library2_tx).await?;
    
    // Execute at block 140
    let execution_event = create_mock_execution_event(chain_id, 140, &contract_address, new_owner, "complex_workflow_test");
    let execution_tx = execution_event.tx_hash().to_string();
    process_valence_account_event(&storage, chain_id, &execution_event, &execution_tx).await?;
    
    // Verify final state
    let account_id = format!("{}:{}", chain_id, contract_address);
    let account_state = storage.get_valence_account_state(&account_id).await?;
    
    assert!(account_state.is_some(), "Account state should exist");
    
    if let Some(state) = account_state {
        assert_eq!(state.current_owner, Some(new_owner.to_string()), "Owner should be updated");
        assert_eq!(state.last_update_block, 140, "Last update block should be 140");
        assert_eq!(state.libraries.len(), 2, "Should have two approved libraries");
        assert!(state.libraries.contains(&library1), "Library 1 should be approved");
        assert!(state.libraries.contains(&library2), "Library 2 should be approved");
    }
    
    println!("Complex account workflow test passed!");
    Ok(())
} 