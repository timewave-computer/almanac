use almanac_core::{ChainId, BlockStatus};
use almanac_core::types::chains;

#[test]
fn test_chain_id_new() {
    let chain_id = ChainId::new("ethereum");
    assert_eq!(chain_id.as_str(), "ethereum");
    
    let chain_id_string = ChainId::new("cosmos".to_string());
    assert_eq!(chain_id_string.as_str(), "cosmos");
}

#[test]
fn test_chain_id_display() {
    let ethereum = ChainId::new("ethereum");
    assert_eq!(ethereum.to_string(), "ethereum");
    
    let cosmos = ChainId::new("cosmos");
    assert_eq!(cosmos.to_string(), "cosmos");
}

#[test]
fn test_chain_id_from_string() {
    let chain_id: ChainId = "ethereum".into();
    assert_eq!(chain_id.as_str(), "ethereum");
    
    let chain_id: ChainId = "cosmos".to_string().into();
    assert_eq!(chain_id.as_str(), "cosmos");
}

#[test]
fn test_chain_id_serialization() {
    let chain_id = ChainId::new("ethereum");
    let serialized = serde_json::to_string(&chain_id).unwrap();
    assert_eq!(serialized, "\"ethereum\"");
    
    let deserialized: ChainId = serde_json::from_str(&serialized).unwrap();
    assert_eq!(deserialized, chain_id);
}

#[test]
fn test_chain_id_equality() {
    let chain1 = ChainId::new("ethereum");
    let chain2 = ChainId::new("ethereum");
    let chain3 = ChainId::new("cosmos");
    
    assert_eq!(chain1, chain2);
    assert_ne!(chain1, chain3);
}

#[test]
fn test_chain_id_as_str() {
    let chain_id = ChainId::new("ethereum");
    assert_eq!(chain_id.as_str(), "ethereum");
    
    let chain_id = ChainId::new("cosmos");
    assert_eq!(chain_id.as_str(), "cosmos");
}

#[test]
fn test_block_status_variants() {
    let statuses = vec![
        BlockStatus::Pending,
        BlockStatus::Processing,
        BlockStatus::Processed,
        BlockStatus::Failed,
        BlockStatus::Finalized,
        BlockStatus::Reorged,
    ];
    
    // Test Debug implementation
    for status in &statuses {
        assert!(!format!("{:?}", status).is_empty());
    }
    
    // Test Clone implementation
    let cloned = statuses.clone();
    assert_eq!(statuses, cloned);
}

#[test]
fn test_block_status_display() {
    assert_eq!(BlockStatus::Pending.to_string(), "pending");
    assert_eq!(BlockStatus::Processing.to_string(), "processing");
    assert_eq!(BlockStatus::Processed.to_string(), "processed");
    assert_eq!(BlockStatus::Failed.to_string(), "failed");
    assert_eq!(BlockStatus::Finalized.to_string(), "finalized");
    assert_eq!(BlockStatus::Reorged.to_string(), "reorged");
}

#[test]
fn test_block_status_equality() {
    assert_eq!(BlockStatus::Pending, BlockStatus::Pending);
    assert_ne!(BlockStatus::Pending, BlockStatus::Processing);
    assert_eq!(BlockStatus::Finalized, BlockStatus::Finalized);
}

#[test]
fn test_block_status_serialization() {
    let status = BlockStatus::Finalized;
    let serialized = serde_json::to_string(&status).unwrap();
    let deserialized: BlockStatus = serde_json::from_str(&serialized).unwrap();
    assert_eq!(status, deserialized);
}

#[test]
fn test_block_status_in_collections() {
    let mut statuses = [BlockStatus::Finalized, BlockStatus::Pending];
    statuses.sort_by_key(|a| a.to_string());
    assert_eq!(statuses.len(), 2);
}

#[test]
fn test_chain_constants() {
    // Test that chain constants are available
    assert_eq!(chains::ETHEREUM, "ethereum");
    assert_eq!(chains::COSMOS, "cosmoshub");
    assert_eq!(chains::OSMOSIS, "osmosis");
    assert_eq!(chains::NEUTRON, "neutron");
    
    // Test that we can create ChainId from constants
    let ethereum = ChainId::new(chains::ETHEREUM);
    assert_eq!(ethereum.as_str(), "ethereum");
    
    let cosmos = ChainId::new(chains::COSMOS);
    assert_eq!(cosmos.as_str(), "cosmoshub");
}

#[test]
fn test_chain_id_hash() {
    use std::collections::HashMap;
    
    let mut map = HashMap::new();
    let ethereum = ChainId::new("ethereum");
    let cosmos = ChainId::new("cosmos");
    
    map.insert(ethereum.clone(), "Ethereum data");
    map.insert(cosmos.clone(), "Cosmos data");
    
    assert_eq!(map.get(&ethereum), Some(&"Ethereum data"));
    assert_eq!(map.get(&cosmos), Some(&"Cosmos data"));
}

#[test]
fn test_block_status_copy() {
    let status = BlockStatus::Finalized;
    let copied = status;
    
    assert_eq!(status, copied);
    assert_eq!(status, BlockStatus::Finalized);
    assert_eq!(copied, BlockStatus::Finalized);
}