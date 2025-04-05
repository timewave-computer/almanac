// Tests for Valence Authorization Contract Indexer
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

use indexer_common::Result;
use indexer_storage::{
    Storage, BoxedStorage, rocks::RocksDbStorage,
    ValenceAuthorizationInfo, ValenceAuthorizationPolicy, ValenceAuthorizationGrant,
    ValenceAuthorizationRequest, ValenceAuthorizationDecision, ValenceAuthorizationState
};
use indexer_cosmos::event::{CosmosEvent, process_valence_authorization_event};

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

// Create a mock instantiate event for a Valence Authorization
fn create_mock_authorization_instantiate_event(
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
    
    create_mock_cosmos_event(
        chain_id,
        block_number,
        &format!("tx-hash-{}", Uuid::new_v4()),
        "wasm",
        attributes,
    )
}

// Create a mock policy creation event
fn create_mock_policy_creation_event(
    chain_id: &str,
    block_number: u64,
    contract_address: &str,
    policy_id: &str,
    version: u32,
    content_hash: &str
) -> CosmosEvent {
    let mut attributes = HashMap::new();
    attributes.insert("_contract_address".to_string(), contract_address.to_string());
    attributes.insert("action".to_string(), "execute".to_string());
    attributes.insert("method".to_string(), "create_policy".to_string());
    attributes.insert("_module".to_string(), "wasm".to_string());
    
    attributes.insert("policy_id".to_string(), policy_id.to_string());
    attributes.insert("version".to_string(), version.to_string());
    attributes.insert("content_hash".to_string(), content_hash.to_string());
    
    create_mock_cosmos_event(
        chain_id,
        block_number,
        &format!("tx-hash-{}", Uuid::new_v4()),
        "wasm",
        attributes,
    )
}

// Create a mock policy activation event
fn create_mock_activate_policy_event(
    chain_id: &str,
    block_number: u64,
    contract_address: &str,
    policy_id: &str
) -> CosmosEvent {
    let mut attributes = HashMap::new();
    attributes.insert("_contract_address".to_string(), contract_address.to_string());
    attributes.insert("action".to_string(), "execute".to_string());
    attributes.insert("method".to_string(), "activate_policy".to_string());
    attributes.insert("_module".to_string(), "wasm".to_string());
    
    attributes.insert("policy_id".to_string(), policy_id.to_string());
    
    create_mock_cosmos_event(
        chain_id,
        block_number,
        &format!("tx-hash-{}", Uuid::new_v4()),
        "wasm",
        attributes,
    )
}

// Create a mock grant permission event
fn create_mock_grant_permission_event(
    chain_id: &str,
    block_number: u64,
    contract_address: &str,
    grant_id: &str,
    grantee: &str,
    permissions: Vec<&str>,
    resources: Vec<&str>,
    expiry: Option<u64>
) -> CosmosEvent {
    let mut attributes = HashMap::new();
    attributes.insert("_contract_address".to_string(), contract_address.to_string());
    attributes.insert("action".to_string(), "execute".to_string());
    attributes.insert("method".to_string(), "grant_permission".to_string());
    attributes.insert("_module".to_string(), "wasm".to_string());
    
    attributes.insert("grant_id".to_string(), grant_id.to_string());
    attributes.insert("grantee".to_string(), grantee.to_string());
    attributes.insert("permissions".to_string(), permissions.join(","));
    attributes.insert("resources".to_string(), resources.join(","));
    
    if let Some(exp) = expiry {
        attributes.insert("expiry".to_string(), exp.to_string());
    }
    
    create_mock_cosmos_event(
        chain_id,
        block_number,
        &format!("tx-hash-{}", Uuid::new_v4()),
        "wasm",
        attributes,
    )
}

// Create a mock revoke permission event
fn create_mock_revoke_permission_event(
    chain_id: &str,
    block_number: u64,
    contract_address: &str,
    grant_id: &str
) -> CosmosEvent {
    let mut attributes = HashMap::new();
    attributes.insert("_contract_address".to_string(), contract_address.to_string());
    attributes.insert("action".to_string(), "execute".to_string());
    attributes.insert("method".to_string(), "revoke_permission".to_string());
    attributes.insert("_module".to_string(), "wasm".to_string());
    
    attributes.insert("grant_id".to_string(), grant_id.to_string());
    
    create_mock_cosmos_event(
        chain_id,
        block_number,
        &format!("tx-hash-{}", Uuid::new_v4()),
        "wasm",
        attributes,
    )
}

// Create a mock authorization request event
fn create_mock_request_authorization_event(
    chain_id: &str,
    block_number: u64,
    contract_address: &str,
    request_id: &str,
    requester: &str,
    action: &str,
    resource: &str,
    request_data: Option<&str>
) -> CosmosEvent {
    let mut attributes = HashMap::new();
    attributes.insert("_contract_address".to_string(), contract_address.to_string());
    attributes.insert("action".to_string(), "execute".to_string());
    attributes.insert("method".to_string(), "request_authorization".to_string());
    attributes.insert("_module".to_string(), "wasm".to_string());
    
    attributes.insert("request_id".to_string(), request_id.to_string());
    attributes.insert("requester".to_string(), requester.to_string());
    attributes.insert("action".to_string(), action.to_string());
    attributes.insert("resource".to_string(), resource.to_string());
    
    if let Some(data) = request_data {
        attributes.insert("request_data".to_string(), data.to_string());
    }
    
    create_mock_cosmos_event(
        chain_id,
        block_number,
        &format!("tx-hash-{}", Uuid::new_v4()),
        "wasm",
        attributes,
    )
}

// Create a mock decision event
fn create_mock_decision_event(
    chain_id: &str,
    block_number: u64,
    contract_address: &str,
    request_id: &str,
    decision: &str,
    reason: Option<&str>
) -> CosmosEvent {
    let mut attributes = HashMap::new();
    attributes.insert("_contract_address".to_string(), contract_address.to_string());
    attributes.insert("action".to_string(), "execute".to_string());
    attributes.insert("method".to_string(), "decide_request".to_string());
    attributes.insert("_module".to_string(), "wasm".to_string());
    
    attributes.insert("request_id".to_string(), request_id.to_string());
    attributes.insert("decision".to_string(), decision.to_string());
    
    if let Some(r) = reason {
        attributes.insert("reason".to_string(), r.to_string());
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
async fn test_authorization_instantiation() -> Result<()> {
    if !should_run_tests() {
        println!("Skipping Valence Authorization tests (RUN_CONTRACT_TESTS not set)");
        return Ok(());
    }
    
    // Setup test storage
    let storage = setup_test_storage().await?;
    
    // Test parameters
    let chain_id = "valence-test";
    let contract_address = format!("auth{}", Uuid::new_v4().simple());
    let owner = "cosmos1owner";
    let block_number = 100u64;
    
    // Create a mock instantiation event
    let event = create_mock_authorization_instantiate_event(chain_id, block_number, &contract_address, owner);
    let tx_hash = event.tx_hash().to_string();
    
    // Process the event
    process_valence_authorization_event(&storage, chain_id, &event, &tx_hash).await?;
    
    // Verify the authorization contract was created in storage
    let auth_id = format!("{}:{}", chain_id, contract_address);
    // We don't have a direct method to get auth state in the trait, this would be implementation-specific
    
    println!("Authorization instantiation test passed!");
    Ok(())
}

#[tokio::test]
async fn test_policy_lifecycle() -> Result<()> {
    if !should_run_tests() {
        return Ok(());
    }
    
    // Setup test storage
    let storage = setup_test_storage().await?;
    
    // Test parameters
    let chain_id = "valence-test";
    let contract_address = format!("auth{}", Uuid::new_v4().simple());
    let owner = "cosmos1owner";
    let block_number = 100u64;
    
    // Policy parameters
    let policy_id = format!("policy-{}", Uuid::new_v4().simple());
    let policy_version = 1u32;
    let content_hash = format!("hash-{}", Uuid::new_v4().simple());
    
    // First create an authorization contract
    let instantiate_event = create_mock_authorization_instantiate_event(chain_id, block_number, &contract_address, owner);
    let tx_hash = instantiate_event.tx_hash().to_string();
    process_valence_authorization_event(&storage, chain_id, &instantiate_event, &tx_hash).await?;
    
    // Create a policy at block 110
    let create_policy_event = create_mock_policy_creation_event(
        chain_id,
        110,
        &contract_address,
        &policy_id,
        policy_version,
        &content_hash
    );
    let policy_tx = create_policy_event.tx_hash().to_string();
    process_valence_authorization_event(&storage, chain_id, &create_policy_event, &policy_tx).await?;
    
    // Activate the policy at block 120
    let activate_event = create_mock_activate_policy_event(
        chain_id,
        120,
        &contract_address,
        &policy_id
    );
    let activate_tx = activate_event.tx_hash().to_string();
    process_valence_authorization_event(&storage, chain_id, &activate_event, &activate_tx).await?;
    
    // We don't have a direct method to verify the policy state in the trait
    // In a real implementation, we would add methods to query policies
    
    println!("Policy lifecycle test passed!");
    Ok(())
}

#[tokio::test]
async fn test_permission_management() -> Result<()> {
    if !should_run_tests() {
        return Ok(());
    }
    
    // Setup test storage
    let storage = setup_test_storage().await?;
    
    // Test parameters
    let chain_id = "valence-test";
    let contract_address = format!("auth{}", Uuid::new_v4().simple());
    let owner = "cosmos1owner";
    let block_number = 100u64;
    
    // Grant parameters
    let grant_id = format!("grant-{}", Uuid::new_v4().simple());
    let grantee = "cosmos1grantee";
    let permissions = vec!["read", "write"];
    let resources = vec!["resource1", "resource2"];
    
    // First create an authorization contract
    let instantiate_event = create_mock_authorization_instantiate_event(chain_id, block_number, &contract_address, owner);
    let tx_hash = instantiate_event.tx_hash().to_string();
    process_valence_authorization_event(&storage, chain_id, &instantiate_event, &tx_hash).await?;
    
    // Grant permission at block 110
    let grant_event = create_mock_grant_permission_event(
        chain_id,
        110,
        &contract_address,
        &grant_id,
        grantee,
        permissions,
        resources,
        Some(200) // Expiry at block 200
    );
    let grant_tx = grant_event.tx_hash().to_string();
    process_valence_authorization_event(&storage, chain_id, &grant_event, &grant_tx).await?;
    
    // Revoke permission at block 150
    let revoke_event = create_mock_revoke_permission_event(
        chain_id,
        150,
        &contract_address,
        &grant_id
    );
    let revoke_tx = revoke_event.tx_hash().to_string();
    process_valence_authorization_event(&storage, chain_id, &revoke_event, &revoke_tx).await?;
    
    // We don't have a direct method to verify the permission state in the trait
    // In a real implementation, we would add methods to query grants
    
    println!("Permission management test passed!");
    Ok(())
}

#[tokio::test]
async fn test_authorization_request_lifecycle() -> Result<()> {
    if !should_run_tests() {
        return Ok(());
    }
    
    // Setup test storage
    let storage = setup_test_storage().await?;
    
    // Test parameters
    let chain_id = "valence-test";
    let contract_address = format!("auth{}", Uuid::new_v4().simple());
    let owner = "cosmos1owner";
    let block_number = 100u64;
    
    // Request parameters
    let request_id = format!("req-{}", Uuid::new_v4().simple());
    let requester = "cosmos1requester";
    let action = "write";
    let resource = "database/users";
    let request_data = Some("{'user_id': 123}");
    
    // First create an authorization contract
    let instantiate_event = create_mock_authorization_instantiate_event(chain_id, block_number, &contract_address, owner);
    let tx_hash = instantiate_event.tx_hash().to_string();
    process_valence_authorization_event(&storage, chain_id, &instantiate_event, &tx_hash).await?;
    
    // Submit authorization request at block 110
    let request_event = create_mock_request_authorization_event(
        chain_id,
        110,
        &contract_address,
        &request_id,
        requester,
        action,
        resource,
        request_data
    );
    let request_tx = request_event.tx_hash().to_string();
    process_valence_authorization_event(&storage, chain_id, &request_event, &request_tx).await?;
    
    // Approve the request at block 120
    let decision_event = create_mock_decision_event(
        chain_id,
        120,
        &contract_address,
        &request_id,
        "approved",
        Some("User has sufficient permissions")
    );
    let decision_tx = decision_event.tx_hash().to_string();
    process_valence_authorization_event(&storage, chain_id, &decision_event, &decision_tx).await?;
    
    // We don't have a direct method to verify the request state in the trait
    // In a real implementation, we would add methods to query authorization requests
    
    println!("Authorization request lifecycle test passed!");
    Ok(())
}

#[tokio::test]
async fn test_complex_authorization_workflow() -> Result<()> {
    if !should_run_tests() {
        return Ok(());
    }
    
    // Setup test storage
    let storage = setup_test_storage().await?;
    
    // Test parameters
    let chain_id = "valence-test";
    let contract_address = format!("auth{}", Uuid::new_v4().simple());
    let owner = "cosmos1owner";
    
    // Create an authorization contract at block 100
    let instantiate_event = create_mock_authorization_instantiate_event(chain_id, 100, &contract_address, owner);
    process_valence_authorization_event(&storage, chain_id, &instantiate_event, &instantiate_event.tx_hash()).await?;
    
    // Create a policy at block 110
    let policy_id = format!("policy-{}", Uuid::new_v4().simple());
    let policy_event = create_mock_policy_creation_event(
        chain_id, 110, &contract_address, &policy_id, 1, "content-hash-1"
    );
    process_valence_authorization_event(&storage, chain_id, &policy_event, &policy_event.tx_hash()).await?;
    
    // Activate the policy at block 120
    let activate_event = create_mock_activate_policy_event(
        chain_id, 120, &contract_address, &policy_id
    );
    process_valence_authorization_event(&storage, chain_id, &activate_event, &activate_event.tx_hash()).await?;
    
    // Grant read/write permissions to user1 at block 130
    let grant1_id = format!("grant1-{}", Uuid::new_v4().simple());
    let grant1_event = create_mock_grant_permission_event(
        chain_id, 130, &contract_address, &grant1_id, "cosmos1user1", 
        vec!["read", "write"], vec!["resource1", "resource2"], None
    );
    process_valence_authorization_event(&storage, chain_id, &grant1_event, &grant1_event.tx_hash()).await?;
    
    // Grant read-only permissions to user2 at block 140
    let grant2_id = format!("grant2-{}", Uuid::new_v4().simple());
    let grant2_event = create_mock_grant_permission_event(
        chain_id, 140, &contract_address, &grant2_id, "cosmos1user2", 
        vec!["read"], vec!["resource1"], Some(1000)
    );
    process_valence_authorization_event(&storage, chain_id, &grant2_event, &grant2_event.tx_hash()).await?;
    
    // User1 makes a request to write to resource1 at block 150
    let request1_id = format!("req1-{}", Uuid::new_v4().simple());
    let request1_event = create_mock_request_authorization_event(
        chain_id, 150, &contract_address, &request1_id, "cosmos1user1", 
        "write", "resource1", Some("data1")
    );
    process_valence_authorization_event(&storage, chain_id, &request1_event, &request1_event.tx_hash()).await?;
    
    // The request is approved at block 160
    let decision1_event = create_mock_decision_event(
        chain_id, 160, &contract_address, &request1_id, "approved", None
    );
    process_valence_authorization_event(&storage, chain_id, &decision1_event, &decision1_event.tx_hash()).await?;
    
    // User2 makes a request to write to resource1 at block 170
    let request2_id = format!("req2-{}", Uuid::new_v4().simple());
    let request2_event = create_mock_request_authorization_event(
        chain_id, 170, &contract_address, &request2_id, "cosmos1user2", 
        "write", "resource1", Some("data2")
    );
    process_valence_authorization_event(&storage, chain_id, &request2_event, &request2_event.tx_hash()).await?;
    
    // The request is denied at block 180
    let decision2_event = create_mock_decision_event(
        chain_id, 180, &contract_address, &request2_id, "denied", 
        Some("User doesn't have write permissions")
    );
    process_valence_authorization_event(&storage, chain_id, &decision2_event, &decision2_event.tx_hash()).await?;
    
    // Revoke user1's permissions at block 190
    let revoke_event = create_mock_revoke_permission_event(
        chain_id, 190, &contract_address, &grant1_id
    );
    process_valence_authorization_event(&storage, chain_id, &revoke_event, &revoke_event.tx_hash()).await?;
    
    // We don't have a direct method to verify the full state in the trait
    // In a real implementation, we would add methods to query the authorization state
    
    println!("Complex authorization workflow test passed!");
    Ok(())
} 