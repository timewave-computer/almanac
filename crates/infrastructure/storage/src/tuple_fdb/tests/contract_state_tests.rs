/// Tests for contract state storage functionality
use std::collections::HashMap;

use super::super::*;
use super::super::contract_state::{
    ContractStateKey, ContractStateValue, ContractStateQuery, ContractStateSummary,
    StorageSlotId, DecodedValue, StateMetadata, ChainStateMetadata, StateChangeType,
    StateProofData, ProofType, VerificationLevel,
};
use super::super::semantic::TrustScope;

// Mock test configuration for in-memory testing
fn test_config() -> TupleFdbConfig {
    TupleFdbConfig {
        cluster_file: None,
        namespace: "test_almanac".to_string(),
        connection_timeout: 10,
        max_retries: 3,
    }
}

/// Create a test contract state key
fn create_test_state_key(block_height: u64) -> ContractStateKey {
    ContractStateKey {
        chain_id: "ethereum".to_string(),
        contract_address: "0x1234567890123456789012345678901234567890".to_string(),
        slot_id: StorageSlotId::Ethereum("0x0000000000000000000000000000000000000000000000000000000000000001".to_string()),
        block_height,
    }
}

/// Create a test contract state value
fn create_test_state_value(block_height: u64, value: &str) -> ContractStateValue {
    let metadata = StateMetadata::new(
        block_height,
        format!("0xblock{:x}", block_height),
        1650000000 + block_height,
        format!("0xtx{:x}", block_height),
        0,
        ChainStateMetadata::Ethereum {
            storage_trie_root: "0xroot123".to_string(),
            account_nonce: 1,
            account_balance: "1000000000000000000".to_string(),
            code_hash: "0xcode123".to_string(),
        },
        StateChangeType::Updated,
    );
    
    let decoded = DecodedValue::UInt(value.to_string());
    
    ContractStateValue::new(
        value.as_bytes().to_vec(),
        metadata,
    ).with_decoded(decoded)
}

#[tokio::test]
#[ignore] // Requires FDB cluster
async fn test_contract_state_storage_and_retrieval() -> Result<()> {
    let storage = TupleFdbStorage::new(test_config()).await?;
    
    // Create test data
    let key = create_test_state_key(1000);
    let value = create_test_state_value(1000, "42");
    let trust_scope = TrustScope::Contract {
        chain_id: "ethereum".to_string(),
        contract_address: "0x1234567890123456789012345678901234567890".to_string(),
    };
    
    // Store the state
    storage.store_contract_state(key.clone(), value.clone(), trust_scope).await?;
    
    // Retrieve the state
    let retrieved = storage.get_contract_state(&key).await?;
    assert!(retrieved.is_some());
    
    let retrieved_value = retrieved.unwrap();
    assert_eq!(retrieved_value.raw_value, value.raw_value);
    assert_eq!(retrieved_value.metadata.block_height, value.metadata.block_height);
    assert_eq!(retrieved_value.decoded_value, value.decoded_value);
    
    println!("✅ Contract state storage and retrieval test passed");
    Ok(())
}

#[tokio::test]
#[ignore] // Requires FDB cluster
async fn test_latest_contract_state() -> Result<()> {
    let storage = TupleFdbStorage::new(test_config()).await?;
    
    let slot_id = StorageSlotId::Ethereum("0x0000000000000000000000000000000000000000000000000000000000000002".to_string());
    let trust_scope = TrustScope::Global;
    
    // Store multiple versions of the same slot
    for height in [1000, 1001, 1002] {
        let key = ContractStateKey {
            chain_id: "ethereum".to_string(),
            contract_address: "0x1234567890123456789012345678901234567890".to_string(),
            slot_id: slot_id.clone(),
            block_height: height,
        };
        let value = create_test_state_value(height, &format!("{}", height * 10));
        
        storage.store_contract_state(key, value, trust_scope.clone()).await?;
    }
    
    // Get latest state
    let latest = storage.get_latest_contract_state(
        "ethereum",
        "0x1234567890123456789012345678901234567890",
        &slot_id,
    ).await?;
    
    assert!(latest.is_some());
    let latest_value = latest.unwrap();
    assert_eq!(latest_value.metadata.block_height, 1002);
    
    // Check decoded value
    if let Some(DecodedValue::UInt(val)) = &latest_value.decoded_value {
        assert_eq!(val, "10020");
    } else {
        panic!("Expected UInt decoded value");
    }
    
    println!("✅ Latest contract state test passed");
    Ok(())
}

#[tokio::test]
#[ignore] // Requires FDB cluster
async fn test_contract_state_query() -> Result<()> {
    let storage = TupleFdbStorage::new(test_config()).await?;
    let trust_scope = TrustScope::Global;
    
    // Store test data for multiple slots and blocks
    let chain_id = "ethereum";
    let contract_address = "0x9876543210987654321098765432109876543210";
    
    for slot_num in 0..3 {
        for height in [2000, 2001, 2002] {
            let key = ContractStateKey {
                chain_id: chain_id.to_string(),
                contract_address: contract_address.to_string(),
                slot_id: StorageSlotId::Ethereum(format!("0x{:064x}", slot_num)),
                block_height: height,
            };
            let value = create_test_state_value(height, &format!("{}", slot_num * 100 + height));
            
            storage.store_contract_state(key, value, trust_scope.clone()).await?;
        }
    }
    
    // Query all states for the contract
    let query = ContractStateQuery {
        chain_id: chain_id.to_string(),
        contract_address: contract_address.to_string(),
        include_zero_values: true,
        include_decoded: true,
        ..Default::default()
    };
    
    let results = storage.query_contract_state(query).await?;
    assert_eq!(results.len(), 9); // 3 slots × 3 blocks
    
    // Query with block range filter
    let query = ContractStateQuery {
        chain_id: chain_id.to_string(),
        contract_address: contract_address.to_string(),
        from_height: Some(2001),
        to_height: Some(2001),
        include_zero_values: true,
        ..Default::default()
    };
    
    let results = storage.query_contract_state(query).await?;
    assert_eq!(results.len(), 3); // 3 slots at block 2001
    
    // Verify all results are from the correct block
    for (key, _value) in results {
        assert_eq!(key.block_height, 2001);
    }
    
    println!("✅ Contract state query test passed");
    Ok(())
}

#[tokio::test]
#[ignore] // Requires FDB cluster
async fn test_contract_summary() -> Result<()> {
    let storage = TupleFdbStorage::new(test_config()).await?;
    let trust_scope = TrustScope::Global;
    
    let chain_id = "polygon";
    let contract_address = "0xabcdefabcdefabcdefabcdefabcdefabcdefabcd";
    
    // Store some contract state
    for i in 0..5 {
        let key = ContractStateKey {
            chain_id: chain_id.to_string(),
            contract_address: contract_address.to_string(),
            slot_id: StorageSlotId::Ethereum(format!("0x{:064x}", i)),
            block_height: 3000 + i,
        };
        let value = create_test_state_value(3000 + i, &format!("{}", i * 10));
        
        storage.store_contract_state(key, value, trust_scope.clone()).await?;
    }
    
    // Get contract summary
    let summary = storage.get_contract_summary(chain_id, contract_address).await?;
    assert!(summary.is_some());
    
    let summary = summary.unwrap();
    assert_eq!(summary.chain_id, chain_id);
    assert_eq!(summary.contract_address, contract_address);
    assert_eq!(summary.total_slots, 5);
    assert_eq!(summary.non_zero_slots, 5);
    assert_eq!(summary.last_updated_height, 3004);
    assert!(summary.total_storage_bytes > 0);
    
    println!("✅ Contract summary test passed");
    Ok(())
}

#[test]
fn test_storage_slot_id_formats() {
    // Test Ethereum slot
    let eth_slot = StorageSlotId::Ethereum("0x1234".to_string());
    let eth_string = eth_slot.as_string();
    assert_eq!(eth_string, "eth:0x1234");
    assert_eq!(StorageSlotId::from_string(&eth_string).unwrap(), eth_slot);
    
    // Test Cosmos slot
    let cosmos_slot = StorageSlotId::Cosmos("store/bank/balances".to_string());
    let cosmos_string = cosmos_slot.as_string();
    assert_eq!(cosmos_string, "cosmos:store/bank/balances");
    assert_eq!(StorageSlotId::from_string(&cosmos_string).unwrap(), cosmos_slot);
    
    // Test Solana slot
    let solana_slot = StorageSlotId::Solana {
        account: "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA".to_string(),
        offset: 32,
        length: 8,
    };
    let solana_string = solana_slot.as_string();
    assert_eq!(solana_string, "solana:TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA:32:8");
    assert_eq!(StorageSlotId::from_string(&solana_string).unwrap(), solana_slot);
    
    // Test Generic slot
    let generic_slot = StorageSlotId::Generic("custom_key_123".to_string());
    let generic_string = generic_slot.as_string();
    assert_eq!(generic_string, "generic:custom_key_123");
    assert_eq!(StorageSlotId::from_string(&generic_string).unwrap(), generic_slot);
    
    println!("✅ Storage slot ID format test passed");
}

#[test]
fn test_decoded_value_zero_detection() {
    // Test numeric zeros
    assert!(DecodedValue::UInt("0".to_string()).is_zero());
    assert!(DecodedValue::Int("0".to_string()).is_zero());
    assert!(!DecodedValue::UInt("1".to_string()).is_zero());
    assert!(!DecodedValue::Int("-1".to_string()).is_zero());
    
    // Test boolean
    assert!(DecodedValue::Bool(false).is_zero());
    assert!(!DecodedValue::Bool(true).is_zero());
    
    // Test address
    assert!(DecodedValue::Address("0x0000000000000000000000000000000000000000".to_string()).is_zero());
    assert!(DecodedValue::Address("".to_string()).is_zero());
    assert!(!DecodedValue::Address("0x1234567890123456789012345678901234567890".to_string()).is_zero());
    
    // Test bytes
    assert!(DecodedValue::Bytes(vec![]).is_zero());
    assert!(DecodedValue::Bytes(vec![0, 0, 0]).is_zero());
    assert!(!DecodedValue::Bytes(vec![1, 0, 0]).is_zero());
    
    // Test collections
    assert!(DecodedValue::Array(vec![]).is_zero());
    assert!(DecodedValue::Struct(HashMap::new()).is_zero());
    assert!(!DecodedValue::Array(vec![DecodedValue::UInt("1".to_string())]).is_zero());
    
    // Test chain-specific types
    assert!(DecodedValue::CosmosAny {
        type_url: "/cosmos.bank.v1beta1.MsgSend".to_string(),
        value: vec![],
    }.is_zero());
    
    assert!(DecodedValue::SolanaAccount {
        lamports: 0,
        data: vec![],
        owner: "11111111111111111111111111111111".to_string(),
        executable: false,
    }.is_zero());
    
    assert!(!DecodedValue::SolanaAccount {
        lamports: 1000,
        data: vec![],
        owner: "11111111111111111111111111111111".to_string(),
        executable: false,
    }.is_zero());
    
    println!("✅ Decoded value zero detection test passed");
}

#[test]
fn test_contract_state_key_tuple_conversion() {
    let key = ContractStateKey {
        chain_id: "ethereum".to_string(),
        contract_address: "0x1234567890123456789012345678901234567890".to_string(),
        slot_id: StorageSlotId::Ethereum("0x0000000000000000000000000000000000000000000000000000000000000001".to_string()),
        block_height: 1000,
    };
    
    let tuple_key = key.to_tuple_key();
    assert_eq!(tuple_key.keyspace, TupleKeyspace::ContractState);
    
    // Test prefix key
    let prefix_key = key.prefix_key();
    assert_eq!(prefix_key.keyspace, TupleKeyspace::ContractState);
    assert_eq!(prefix_key.components.len(), 3); // chain + address + slot (no height)
    
    // Test contract prefix
    let contract_prefix = ContractStateKey::contract_prefix("ethereum", "0x1234567890123456789012345678901234567890");
    assert_eq!(contract_prefix.keyspace, TupleKeyspace::ContractState);
    assert_eq!(contract_prefix.components.len(), 2); // chain + address only
    
    println!("✅ Contract state key tuple conversion test passed");
}

#[test]
fn test_state_metadata_creation() {
    let metadata = StateMetadata::new(
        1000,
        "0xblock1000".to_string(),
        1650000000,
        "0xtx1000".to_string(),
        5,
        ChainStateMetadata::Ethereum {
            storage_trie_root: "0xroot".to_string(),
            account_nonce: 10,
            account_balance: "1000000000000000000".to_string(),
            code_hash: "0xcode".to_string(),
        },
        StateChangeType::Created,
    );
    
    assert_eq!(metadata.block_height, 1000);
    assert_eq!(metadata.block_hash, "0xblock1000");
    assert_eq!(metadata.tx_hash, "0xtx1000");
    assert_eq!(metadata.tx_index, 5);
    assert_eq!(metadata.change_type, StateChangeType::Created);
    assert_eq!(metadata.version, 1);
    assert!(metadata.indexed_at > 0);
    
    // Test recent block detection
    assert!(metadata.is_recent(1005, 10)); // Within threshold
    assert!(!metadata.is_recent(1020, 10)); // Outside threshold
    
    println!("✅ State metadata creation test passed");
}

#[test]
fn test_contract_state_value_operations() {
    let metadata = StateMetadata::new(
        1000,
        "0xblock".to_string(),
        1650000000,
        "0xtx".to_string(),
        0,
        ChainStateMetadata::Ethereum {
            storage_trie_root: "0xroot".to_string(),
            account_nonce: 1,
            account_balance: "1000000000000000000".to_string(),
            code_hash: "0xcode".to_string(),
        },
        StateChangeType::Updated,
    );
    
    // Test zero value
    let zero_value = ContractStateValue::new(
        vec![0, 0, 0, 0],
        metadata.clone(),
    ).with_decoded(DecodedValue::UInt("0".to_string()));
    
    assert!(zero_value.is_zero_value());
    
    // Test non-zero value
    let non_zero_value = ContractStateValue::new(
        vec![1, 2, 3, 4],
        metadata.clone(),
    ).with_decoded(DecodedValue::UInt("42".to_string()));
    
    assert!(!non_zero_value.is_zero_value());
    
    // Test with proof
    let proof_data = StateProofData {
        proof: vec![1, 2, 3, 4, 5],
        root_hash: "0xproof_root".to_string(),
        proof_type: ProofType::EthereumMPT,
        verification_data: None,
    };
    
    let value_with_proof = non_zero_value.with_proof(proof_data);
    assert!(value_with_proof.proof_data.is_some());
    assert!(value_with_proof.storage_size() > non_zero_value.storage_size());
    
    println!("✅ Contract state value operations test passed");
}

#[tokio::test]
#[ignore] // Requires FDB cluster
async fn test_multi_chain_contract_state() -> Result<()> {
    let storage = TupleFdbStorage::new(test_config()).await?;
    let trust_scope = TrustScope::Global;
    
    // Store state for multiple chains
    let chains = vec![
        ("ethereum", "0x1111111111111111111111111111111111111111"),
        ("polygon", "0x2222222222222222222222222222222222222222"),
        ("arbitrum", "0x3333333333333333333333333333333333333333"),
    ];
    
    for (chain_id, contract_address) in &chains {
        let key = ContractStateKey {
            chain_id: chain_id.to_string(),
            contract_address: contract_address.to_string(),
            slot_id: StorageSlotId::Ethereum("0x0000000000000000000000000000000000000000000000000000000000000001".to_string()),
            block_height: 5000,
        };
        
        let value = create_test_state_value(5000, &format!("{}000", chain_id.len()));
        storage.store_contract_state(key, value, trust_scope.clone()).await?;
    }
    
    // Query each chain independently
    for (chain_id, contract_address) in &chains {
        let query = ContractStateQuery {
            chain_id: chain_id.to_string(),
            contract_address: contract_address.to_string(),
            include_zero_values: true,
            ..Default::default()
        };
        
        let results = storage.query_contract_state(query).await?;
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0.chain_id, *chain_id);
        assert_eq!(results[0].0.contract_address, *contract_address);
    }
    
    println!("✅ Multi-chain contract state test passed");
    Ok(())
} 