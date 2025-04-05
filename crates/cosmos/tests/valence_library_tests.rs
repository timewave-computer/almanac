// Tests for Valence Library Contract Indexer
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

use indexer_common::Result;
use indexer_storage::{
    Storage, BoxedStorage, rocks::RocksDbStorage,
    ValenceLibraryInfo, ValenceLibraryVersion, ValenceLibraryUsage,
    ValenceLibraryApproval, ValenceLibraryState
};
use indexer_cosmos::event::{CosmosEvent, process_valence_library_event};

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

// Create a mock instantiate event for a Valence Library
fn create_mock_library_instantiate_event(
    chain_id: &str, 
    block_number: u64, 
    contract_address: &str, 
    owner: &str,
    library_type: &str
) -> CosmosEvent {
    let mut attributes = HashMap::new();
    attributes.insert("_contract_address".to_string(), contract_address.to_string());
    attributes.insert("action".to_string(), "instantiate".to_string());
    attributes.insert("owner".to_string(), owner.to_string());
    attributes.insert("library_type".to_string(), library_type.to_string());
    attributes.insert("_module".to_string(), "wasm".to_string());
    
    create_mock_cosmos_event(
        chain_id,
        block_number,
        &format!("tx-hash-{}", Uuid::new_v4()),
        "wasm",
        attributes,
    )
}

// Create a mock version creation event
fn create_mock_version_creation_event(
    chain_id: &str,
    block_number: u64,
    contract_address: &str,
    version: u32,
    code_hash: &str,
    features: Vec<&str>
) -> CosmosEvent {
    let mut attributes = HashMap::new();
    attributes.insert("_contract_address".to_string(), contract_address.to_string());
    attributes.insert("action".to_string(), "execute".to_string());
    attributes.insert("method".to_string(), "create_version".to_string());
    attributes.insert("_module".to_string(), "wasm".to_string());
    
    attributes.insert("version".to_string(), version.to_string());
    attributes.insert("code_hash".to_string(), code_hash.to_string());
    attributes.insert("features".to_string(), features.join(","));
    
    create_mock_cosmos_event(
        chain_id,
        block_number,
        &format!("tx-hash-{}", Uuid::new_v4()),
        "wasm",
        attributes,
    )
}

// Create a mock activate version event
fn create_mock_activate_version_event(
    chain_id: &str,
    block_number: u64,
    contract_address: &str,
    version: u32
) -> CosmosEvent {
    let mut attributes = HashMap::new();
    attributes.insert("_contract_address".to_string(), contract_address.to_string());
    attributes.insert("action".to_string(), "execute".to_string());
    attributes.insert("method".to_string(), "activate_version".to_string());
    attributes.insert("_module".to_string(), "wasm".to_string());
    
    attributes.insert("version".to_string(), version.to_string());
    
    create_mock_cosmos_event(
        chain_id,
        block_number,
        &format!("tx-hash-{}", Uuid::new_v4()),
        "wasm",
        attributes,
    )
}

// Create a mock library approval event
fn create_mock_library_approval_event(
    chain_id: &str,
    block_number: u64,
    contract_address: &str,
    account_id: &str
) -> CosmosEvent {
    let mut attributes = HashMap::new();
    attributes.insert("_contract_address".to_string(), contract_address.to_string());
    attributes.insert("action".to_string(), "execute".to_string());
    attributes.insert("method".to_string(), "approve_by_account".to_string());
    attributes.insert("_module".to_string(), "wasm".to_string());
    
    attributes.insert("account_id".to_string(), account_id.to_string());
    
    create_mock_cosmos_event(
        chain_id,
        block_number,
        &format!("tx-hash-{}", Uuid::new_v4()),
        "wasm",
        attributes,
    )
}

// Create a mock library revocation event
fn create_mock_library_revocation_event(
    chain_id: &str,
    block_number: u64,
    contract_address: &str,
    account_id: &str
) -> CosmosEvent {
    let mut attributes = HashMap::new();
    attributes.insert("_contract_address".to_string(), contract_address.to_string());
    attributes.insert("action".to_string(), "execute".to_string());
    attributes.insert("method".to_string(), "revoke_by_account".to_string());
    attributes.insert("_module".to_string(), "wasm".to_string());
    
    attributes.insert("account_id".to_string(), account_id.to_string());
    
    create_mock_cosmos_event(
        chain_id,
        block_number,
        &format!("tx-hash-{}", Uuid::new_v4()),
        "wasm",
        attributes,
    )
}

// Create a mock library usage event
fn create_mock_library_usage_event(
    chain_id: &str,
    block_number: u64,
    contract_address: &str,
    user_address: &str,
    account_id: Option<&str>,
    function_name: Option<&str>,
    gas_used: u64,
    success: bool,
    error: Option<&str>
) -> CosmosEvent {
    let mut attributes = HashMap::new();
    attributes.insert("_contract_address".to_string(), contract_address.to_string());
    attributes.insert("action".to_string(), "execute".to_string());
    attributes.insert("method".to_string(), "execute".to_string());
    attributes.insert("_module".to_string(), "wasm".to_string());
    
    attributes.insert("user_address".to_string(), user_address.to_string());
    attributes.insert("gas_used".to_string(), gas_used.to_string());
    attributes.insert("success".to_string(), success.to_string());
    
    if let Some(account) = account_id {
        attributes.insert("account_id".to_string(), account.to_string());
    }
    
    if let Some(func) = function_name {
        attributes.insert("function_name".to_string(), func.to_string());
    }
    
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
async fn test_library_instantiation() -> Result<()> {
    if !should_run_tests() {
        println!("Skipping Valence Library tests (RUN_CONTRACT_TESTS not set)");
        return Ok(());
    }
    
    // Setup test storage
    let storage = setup_test_storage().await?;
    
    // Test parameters
    let chain_id = "valence-test";
    let contract_address = format!("library{}", Uuid::new_v4().simple());
    let owner = "cosmos1owner";
    let library_type = "swap";
    let block_number = 100u64;
    
    // Create a mock instantiation event
    let event = create_mock_library_instantiate_event(chain_id, block_number, &contract_address, owner, library_type);
    let tx_hash = event.tx_hash().to_string();
    
    // Process the event
    process_valence_library_event(&storage, chain_id, &event, &tx_hash).await?;
    
    // Verify the library was created in storage
    let library_id = format!("{}:{}", chain_id, contract_address);
    let library_state = storage.get_valence_library_state(&library_id).await?;
    
    assert!(library_state.is_some(), "Library state should exist after instantiation");
    
    if let Some(state) = library_state {
        assert_eq!(state.library_id, library_id, "Library ID should match");
        assert_eq!(state.chain_id, chain_id, "Chain ID should match");
        assert_eq!(state.address, contract_address, "Contract address should match");
        assert_eq!(state.library_type, library_type, "Library type should match");
        assert_eq!(state.current_owner, Some(owner.to_string()), "Owner should match");
        assert_eq!(state.current_version, None, "Initial version should be None");
        assert_eq!(state.last_update_block, block_number, "Last update block should match");
        assert!(state.versions.is_empty(), "Versions should be empty initially");
    }
    
    println!("Library instantiation test passed!");
    Ok(())
}

#[tokio::test]
async fn test_version_management() -> Result<()> {
    if !should_run_tests() {
        return Ok(());
    }
    
    // Setup test storage
    let storage = setup_test_storage().await?;
    
    // Test parameters
    let chain_id = "valence-test";
    let contract_address = format!("library{}", Uuid::new_v4().simple());
    let owner = "cosmos1owner";
    let library_type = "messaging";
    let block_number = 100u64;
    
    // Create a library
    let instantiate_event = create_mock_library_instantiate_event(chain_id, block_number, &contract_address, owner, library_type);
    let tx_hash = instantiate_event.tx_hash().to_string();
    process_valence_library_event(&storage, chain_id, &instantiate_event, &tx_hash).await?;
    
    // Create version 1 at block 110
    let version1_event = create_mock_version_creation_event(
        chain_id,
        110,
        &contract_address,
        1,
        "code-hash-1",
        vec!["send", "receive"]
    );
    let version1_tx = version1_event.tx_hash().to_string();
    process_valence_library_event(&storage, chain_id, &version1_event, &version1_tx).await?;
    
    // Activate version 1 at block 120
    let activate1_event = create_mock_activate_version_event(
        chain_id,
        120,
        &contract_address,
        1
    );
    let activate1_tx = activate1_event.tx_hash().to_string();
    process_valence_library_event(&storage, chain_id, &activate1_event, &activate1_tx).await?;
    
    // Create version 2 at block 130
    let version2_event = create_mock_version_creation_event(
        chain_id,
        130,
        &contract_address,
        2,
        "code-hash-2",
        vec!["send", "receive", "broadcast"]
    );
    let version2_tx = version2_event.tx_hash().to_string();
    process_valence_library_event(&storage, chain_id, &version2_event, &version2_tx).await?;
    
    // Verify the library state
    let library_id = format!("{}:{}", chain_id, contract_address);
    let library_state = storage.get_valence_library_state(&library_id).await?;
    
    assert!(library_state.is_some(), "Library state should exist");
    
    if let Some(state) = library_state {
        assert_eq!(state.current_version, Some(1), "Current version should be 1");
        assert_eq!(state.last_update_block, 130, "Last update block should be 130");
        assert_eq!(state.versions.len(), 2, "Should have two versions");
        
        // Check versions
        let versions = storage.get_valence_library_versions(&library_id).await?;
        assert_eq!(versions.len(), 2, "Should have two versions in storage");
        
        // Find version 1
        let v1 = versions.iter().find(|v| v.version == 1).expect("Version 1 should exist");
        assert_eq!(v1.code_hash, "code-hash-1", "Version 1 code hash should match");
        assert!(v1.is_active, "Version 1 should be active");
        assert_eq!(v1.features, vec!["send", "receive"], "Version 1 features should match");
        
        // Find version 2
        let v2 = versions.iter().find(|v| v.version == 2).expect("Version 2 should exist");
        assert_eq!(v2.code_hash, "code-hash-2", "Version 2 code hash should match");
        assert!(!v2.is_active, "Version 2 should not be active yet");
        assert_eq!(v2.features, vec!["send", "receive", "broadcast"], "Version 2 features should match");
    }
    
    // Activate version 2 at block 140
    let activate2_event = create_mock_activate_version_event(
        chain_id,
        140,
        &contract_address,
        2
    );
    let activate2_tx = activate2_event.tx_hash().to_string();
    process_valence_library_event(&storage, chain_id, &activate2_event, &activate2_tx).await?;
    
    // Verify version 2 is now active
    let updated_state = storage.get_valence_library_state(&library_id).await?;
    
    if let Some(state) = updated_state {
        assert_eq!(state.current_version, Some(2), "Current version should be updated to 2");
        assert_eq!(state.last_update_block, 140, "Last update block should be 140");
    }
    
    println!("Version management test passed!");
    Ok(())
}

#[tokio::test]
async fn test_library_approvals() -> Result<()> {
    if !should_run_tests() {
        return Ok(());
    }
    
    // Setup test storage
    let storage = setup_test_storage().await?;
    
    // Test parameters
    let chain_id = "valence-test";
    let contract_address = format!("library{}", Uuid::new_v4().simple());
    let owner = "cosmos1owner";
    let library_type = "bridge";
    let block_number = 100u64;
    
    // Account IDs that will approve the library
    let account1_id = format!("{}:account1", chain_id);
    let account2_id = format!("{}:account2", chain_id);
    
    // Create a library
    let instantiate_event = create_mock_library_instantiate_event(chain_id, block_number, &contract_address, owner, library_type);
    let tx_hash = instantiate_event.tx_hash().to_string();
    process_valence_library_event(&storage, chain_id, &instantiate_event, &tx_hash).await?;
    
    // Create and activate a version
    let version_event = create_mock_version_creation_event(
        chain_id, 110, &contract_address, 1, "code-hash-1", vec!["transfer"]
    );
    process_valence_library_event(&storage, chain_id, &version_event, &version_event.tx_hash()).await?;
    
    let activate_event = create_mock_activate_version_event(
        chain_id, 120, &contract_address, 1
    );
    process_valence_library_event(&storage, chain_id, &activate_event, &activate_event.tx_hash()).await?;
    
    // Account 1 approves the library at block 130
    let approve1_event = create_mock_library_approval_event(
        chain_id, 130, &contract_address, &account1_id
    );
    process_valence_library_event(&storage, chain_id, &approve1_event, &approve1_event.tx_hash()).await?;
    
    // Account 2 approves the library at block 140
    let approve2_event = create_mock_library_approval_event(
        chain_id, 140, &contract_address, &account2_id
    );
    process_valence_library_event(&storage, chain_id, &approve2_event, &approve2_event.tx_hash()).await?;
    
    // Verify approvals
    let library_id = format!("{}:{}", chain_id, contract_address);
    let approvals = storage.get_valence_library_approvals(&library_id).await?;
    
    assert_eq!(approvals.len(), 2, "Should have two approvals");
    
    // Check account 1 approval
    let approval1 = approvals.iter().find(|a| a.account_id == account1_id).expect("Account 1 approval should exist");
    assert_eq!(approval1.library_id, library_id, "Library ID should match");
    assert_eq!(approval1.approved_at_block, 130, "Approval block should match");
    assert!(approval1.is_active, "Approval should be active");
    
    // Check account 2 approval
    let approval2 = approvals.iter().find(|a| a.account_id == account2_id).expect("Account 2 approval should exist");
    assert_eq!(approval2.library_id, library_id, "Library ID should match");
    assert_eq!(approval2.approved_at_block, 140, "Approval block should match");
    assert!(approval2.is_active, "Approval should be active");
    
    // Account 1 revokes approval at block 150
    let revoke_event = create_mock_library_revocation_event(
        chain_id, 150, &contract_address, &account1_id
    );
    process_valence_library_event(&storage, chain_id, &revoke_event, &revoke_event.tx_hash()).await?;
    
    // Verify the revocation
    let updated_approvals = storage.get_valence_library_approvals(&library_id).await?;
    
    // We should still have two records, but one should be inactive
    assert_eq!(updated_approvals.len(), 2, "Should still have two approval records");
    
    // Find account 1's approval
    let updated_approval1 = updated_approvals.iter()
        .find(|a| a.account_id == account1_id)
        .expect("Account 1 approval record should still exist");
    
    assert!(!updated_approval1.is_active, "Account 1 approval should now be inactive");
    assert!(updated_approval1.revoked_at_block.is_some(), "Revocation block should be set");
    assert_eq!(updated_approval1.revoked_at_block.unwrap(), 150, "Revocation block should be 150");
    
    println!("Library approvals test passed!");
    Ok(())
}

#[tokio::test]
async fn test_library_usage() -> Result<()> {
    if !should_run_tests() {
        return Ok(());
    }
    
    // Setup test storage
    let storage = setup_test_storage().await?;
    
    // Test parameters
    let chain_id = "valence-test";
    let contract_address = format!("library{}", Uuid::new_v4().simple());
    let owner = "cosmos1owner";
    let user = "cosmos1user";
    let account_id = format!("{}:account1", chain_id);
    let library_type = "swap";
    let block_number = 100u64;
    
    // Create a library
    let instantiate_event = create_mock_library_instantiate_event(chain_id, block_number, &contract_address, owner, library_type);
    let tx_hash = instantiate_event.tx_hash().to_string();
    process_valence_library_event(&storage, chain_id, &instantiate_event, &tx_hash).await?;
    
    // Create and activate a version
    let version_event = create_mock_version_creation_event(
        chain_id, 110, &contract_address, 1, "code-hash-1", vec!["swap"]
    );
    process_valence_library_event(&storage, chain_id, &version_event, &version_event.tx_hash()).await?;
    
    let activate_event = create_mock_activate_version_event(
        chain_id, 120, &contract_address, 1
    );
    process_valence_library_event(&storage, chain_id, &activate_event, &activate_event.tx_hash()).await?;
    
    // Record successful usage at block 130
    let usage1_event = create_mock_library_usage_event(
        chain_id, 130, &contract_address, user, Some(&account_id), 
        Some("swap_tokens"), 50000, true, None
    );
    process_valence_library_event(&storage, chain_id, &usage1_event, &usage1_event.tx_hash()).await?;
    
    // Record failed usage at block 140
    let usage2_event = create_mock_library_usage_event(
        chain_id, 140, &contract_address, user, Some(&account_id), 
        Some("swap_tokens"), 30000, false, Some("Insufficient liquidity")
    );
    process_valence_library_event(&storage, chain_id, &usage2_event, &usage2_event.tx_hash()).await?;
    
    // Verify usage history
    let library_id = format!("{}:{}", chain_id, contract_address);
    let usage_history = storage.get_valence_library_usage_history(&library_id, None, None).await?;
    
    assert_eq!(usage_history.len(), 2, "Should have two usage records");
    
    // Check successful usage
    let success_usage = usage_history.iter().find(|u| u.success).expect("Successful usage should exist");
    assert_eq!(success_usage.library_id, library_id, "Library ID should match");
    assert_eq!(success_usage.user_address, user, "User address should match");
    assert_eq!(success_usage.account_id, Some(account_id.clone()), "Account ID should match");
    assert_eq!(success_usage.function_name, Some("swap_tokens".to_string()), "Function name should match");
    assert_eq!(success_usage.gas_used, Some(50000), "Gas used should match");
    assert_eq!(success_usage.usage_at_block, 130, "Usage block should match");
    assert!(success_usage.error.is_none(), "Error should be None for successful usage");
    
    // Check failed usage
    let failed_usage = usage_history.iter().find(|u| !u.success).expect("Failed usage should exist");
    assert_eq!(failed_usage.gas_used, Some(30000), "Gas used should match");
    assert_eq!(failed_usage.usage_at_block, 140, "Usage block should match");
    assert!(failed_usage.error.is_some(), "Error should be set for failed usage");
    assert_eq!(failed_usage.error.as_ref().unwrap(), "Insufficient liquidity", "Error message should match");
    
    println!("Library usage test passed!");
    Ok(())
}

#[tokio::test]
async fn test_complex_library_workflow() -> Result<()> {
    if !should_run_tests() {
        return Ok(());
    }
    
    // Setup test storage
    let storage = setup_test_storage().await?;
    
    // Test parameters
    let chain_id = "valence-test";
    let contract_address = format!("library{}", Uuid::new_v4().simple());
    let owner = "cosmos1owner";
    let library_type = "multi-purpose";
    
    // Account and user IDs
    let account1_id = format!("{}:account1", chain_id);
    let account2_id = format!("{}:account2", chain_id);
    let user1 = "cosmos1user1";
    let user2 = "cosmos1user2";
    
    // Comprehensive test scenario:
    // 1. Create library at block 100
    let instantiate_event = create_mock_library_instantiate_event(chain_id, 100, &contract_address, owner, library_type);
    process_valence_library_event(&storage, chain_id, &instantiate_event, &instantiate_event.tx_hash()).await?;
    
    // 2. Create version 1 at block 110
    let version1_event = create_mock_version_creation_event(
        chain_id, 110, &contract_address, 1, "code-hash-1", vec!["basic"]
    );
    process_valence_library_event(&storage, chain_id, &version1_event, &version1_event.tx_hash()).await?;
    
    // 3. Activate version 1 at block 120
    let activate1_event = create_mock_activate_version_event(
        chain_id, 120, &contract_address, 1
    );
    process_valence_library_event(&storage, chain_id, &activate1_event, &activate1_event.tx_hash()).await?;
    
    // 4. Account 1 approves the library at block 130
    let approve1_event = create_mock_library_approval_event(
        chain_id, 130, &contract_address, &account1_id
    );
    process_valence_library_event(&storage, chain_id, &approve1_event, &approve1_event.tx_hash()).await?;
    
    // 5. User 1 uses the library through account 1 at block 140
    let usage1_event = create_mock_library_usage_event(
        chain_id, 140, &contract_address, user1, Some(&account1_id), Some("basic_function"), 40000, true, None
    );
    process_valence_library_event(&storage, chain_id, &usage1_event, &usage1_event.tx_hash()).await?;
    
    // 6. Create version 2 with more features at block 150
    let version2_event = create_mock_version_creation_event(
        chain_id, 150, &contract_address, 2, "code-hash-2", vec!["basic", "advanced"]
    );
    process_valence_library_event(&storage, chain_id, &version2_event, &version2_event.tx_hash()).await?;
    
    // 7. Activate version 2 at block 160
    let activate2_event = create_mock_activate_version_event(
        chain_id, 160, &contract_address, 2
    );
    process_valence_library_event(&storage, chain_id, &activate2_event, &activate2_event.tx_hash()).await?;
    
    // 8. Account 2 approves the library at block 170
    let approve2_event = create_mock_library_approval_event(
        chain_id, 170, &contract_address, &account2_id
    );
    process_valence_library_event(&storage, chain_id, &approve2_event, &approve2_event.tx_hash()).await?;
    
    // 9. User 2 uses the library through account 2 at block 180 (advanced function)
    let usage2_event = create_mock_library_usage_event(
        chain_id, 180, &contract_address, user2, Some(&account2_id), Some("advanced_function"), 75000, true, None
    );
    process_valence_library_event(&storage, chain_id, &usage2_event, &usage2_event.tx_hash()).await?;
    
    // 10. Account 1 revokes approval at block 190
    let revoke_event = create_mock_library_revocation_event(
        chain_id, 190, &contract_address, &account1_id
    );
    process_valence_library_event(&storage, chain_id, &revoke_event, &revoke_event.tx_hash()).await?;
    
    // 11. User 1 tries to use library through account 1 at block 200 but fails due to revoked approval
    let usage3_event = create_mock_library_usage_event(
        chain_id, 200, &contract_address, user1, Some(&account1_id), Some("basic_function"), 
        10000, false, Some("Library not approved by account")
    );
    process_valence_library_event(&storage, chain_id, &usage3_event, &usage3_event.tx_hash()).await?;
    
    // Verify final state
    let library_id = format!("{}:{}", chain_id, contract_address);
    let library_state = storage.get_valence_library_state(&library_id).await?;
    
    assert!(library_state.is_some(), "Library state should exist");
    
    if let Some(state) = library_state {
        // Check basic properties
        assert_eq!(state.library_id, library_id, "Library ID should match");
        assert_eq!(state.current_version, Some(2), "Current version should be 2");
        assert_eq!(state.versions.len(), 2, "Should have two versions");
        assert_eq!(state.last_update_block, 200, "Last update block should be 200");
        
        // Check approvals
        let approvals = storage.get_valence_library_approvals(&library_id).await?;
        assert_eq!(approvals.len(), 2, "Should have two approval records");
        
        // Account 1's approval should be revoked
        let account1_approval = approvals.iter()
            .find(|a| a.account_id == account1_id)
            .expect("Account 1 approval should exist");
        assert!(!account1_approval.is_active, "Account 1 approval should be inactive");
        
        // Account 2's approval should be active
        let account2_approval = approvals.iter()
            .find(|a| a.account_id == account2_id)
            .expect("Account 2 approval should exist");
        assert!(account2_approval.is_active, "Account 2 approval should be active");
        
        // Check usage history
        let usage_history = storage.get_valence_library_usage_history(&library_id, None, None).await?;
        assert_eq!(usage_history.len(), 3, "Should have three usage records");
        
        // Count successful vs failed usages
        let successful_count = usage_history.iter().filter(|u| u.success).count();
        let failed_count = usage_history.iter().filter(|u| !u.success).count();
        
        assert_eq!(successful_count, 2, "Should have two successful usages");
        assert_eq!(failed_count, 1, "Should have one failed usage");
    }
    
    println!("Complex library workflow test passed!");
    Ok(())
} 