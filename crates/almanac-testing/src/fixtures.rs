//! Test fixtures for Almanac testing

use almanac_core::{UnifiedEvent, EventSource, EventFilter};
use almanac_chains::{BlockMetadata, ChainDataIR, ConsensusMetadata, FinalityStatus, StateChange};
use serde_json::Value;
use std::collections::HashMap;
use std::time::SystemTime;
use uuid::Uuid;

/// Create a collection of test events
pub fn create_test_events(count: usize) -> Vec<UnifiedEvent> {
    (0..count)
        .map(|i| create_test_event_with_id(format!("test-event-{}", i)))
        .collect()
}

/// Create Ethereum-specific test events
pub fn create_ethereum_events(count: usize) -> Vec<UnifiedEvent> {
    (0..count)
        .map(|i| {
            let mut event = create_test_event_with_id(format!("ethereum-event-{}", i));
            event.chain = "ethereum".to_string();
            event.block_number = 1000 + i as u64;
            event.tx_hash = format!("0x{:064x}", i);
            event.contract_address = Some(format!("0x{:040x}", i));
            event
        })
        .collect()
}

/// Create Cosmos-specific test events
pub fn create_cosmos_events(count: usize) -> Vec<UnifiedEvent> {
    (0..count)
        .map(|i| {
            let mut event = create_test_event_with_id(format!("cosmos-event-{}", i));
            event.chain = "cosmos".to_string();
            event.block_number = 2000 + i as u64;
            event.tx_hash = format!("cosmos_tx_{}", i);
            event.contract_address = Some(format!("cosmos{}", i));
            event
        })
        .collect()
}

/// Create a transfer event
pub fn create_transfer_event(
    chain_id: &str,
    block_number: u64,
    from: &str,
    to: &str,
    amount: &str,
) -> UnifiedEvent {
    let mut attributes = HashMap::new();
    attributes.insert("from".to_string(), Value::String(from.to_string()));
    attributes.insert("to".to_string(), Value::String(to.to_string()));
    attributes.insert("amount".to_string(), Value::String(amount.to_string()));

    UnifiedEvent {
        id: Uuid::new_v4().to_string(),
        chain: chain_id.to_string(),
        block_number,
        block_hash: format!("block_hash_{}", block_number),
        tx_hash: format!("tx_hash_{}", block_number),
        timestamp: SystemTime::now(),
        event_type: "Transfer".to_string(),
        contract_address: Some(from.to_string()),
        raw_data: serde_json::to_vec(&attributes).unwrap_or_default(),
        attributes,
        source: EventSource::Chain,
    }
}

/// Create a contract call event
pub fn create_contract_call_event(
    chain_id: &str,
    block_number: u64,
    contract_address: &str,
    method_name: &str,
    args: Value,
) -> UnifiedEvent {
    let mut attributes = HashMap::new();
    attributes.insert("method".to_string(), Value::String(method_name.to_string()));
    attributes.insert("args".to_string(), args);

    UnifiedEvent {
        id: Uuid::new_v4().to_string(),
        chain: chain_id.to_string(),
        block_number,
        block_hash: format!("block_hash_{}", block_number),
        tx_hash: format!("tx_hash_{}", block_number),
        timestamp: SystemTime::now(),
        event_type: method_name.to_string(),
        contract_address: Some(contract_address.to_string()),
        raw_data: serde_json::to_vec(&attributes).unwrap_or_default(),
        attributes,
        source: EventSource::Chain,
    }
}

/// Create test blocks
pub fn create_test_blocks(count: usize) -> Vec<BlockMetadata> {
    (0..count)
        .map(|i| BlockMetadata {
            block_height: 1000 + i as u64,
            block_hash: format!("block_hash_{}", 1000 + i),
            parent_hash: if i == 0 {
                "genesis_hash".to_string()
            } else {
                format!("block_hash_{}", 999 + i)
            },
            timestamp: 1640995200 + i as u64 * 12, // 12 second blocks
            state_root: format!("state_root_{}", 1000 + i),
            receipts_root: Some(format!("receipts_root_{}", 1000 + i)),
            transactions_root: Some(format!("transactions_root_{}", 1000 + i)),
        })
        .collect()
}

/// Create Ethereum test blocks
pub fn create_ethereum_blocks(count: usize) -> Vec<BlockMetadata> {
    create_test_blocks(count)
        .into_iter()
        .enumerate()
        .map(|(i, mut block)| {
            block.block_height = 18000000 + i as u64; // Realistic Ethereum block numbers
            block.block_hash = format!("0x{:064x}", 18000000 + i);
            block.parent_hash = if i == 0 {
                format!("0x{:064x}", 17999999)
            } else {
                format!("0x{:064x}", 17999999 + i)
            };
            block.state_root = format!("0x{:064x}", i + 1000);
            block
        })
        .collect()
}

/// Create a finalized block
pub fn create_finalized_block(chain_id: &str, block_number: u64) -> BlockMetadata {
    BlockMetadata {
        block_height: block_number,
        block_hash: format!("{}_finalized_block_{}", chain_id, block_number),
        parent_hash: format!("{}_block_{}", chain_id, block_number - 1),
        timestamp: 1640995200 + block_number * 12,
        state_root: format!("{}_state_root_{}", chain_id, block_number),
        receipts_root: Some(format!("{}_receipts_root_{}", chain_id, block_number)),
        transactions_root: Some(format!("{}_transactions_root_{}", chain_id, block_number)),
    }
}

/// Create a pending block
pub fn create_pending_block(chain_id: &str, block_number: u64) -> BlockMetadata {
    BlockMetadata {
        block_height: block_number,
        block_hash: format!("{}_pending_block_{}", chain_id, block_number),
        parent_hash: format!("{}_block_{}", chain_id, block_number - 1),
        timestamp: 1640995200 + block_number * 12,
        state_root: format!("{}_pending_state_root_{}", chain_id, block_number),
        receipts_root: Some(format!("{}_pending_receipts_root_{}", chain_id, block_number)),
        transactions_root: Some(format!("{}_pending_transactions_root_{}", chain_id, block_number)),
    }
}

/// Create Ethereum chain data
pub fn create_ethereum_chain_data() -> ChainDataIR {
    let block_metadata = create_finalized_block("ethereum", 18000000);
    let state_changes = vec![
        StateChange {
            contract_address: "0x1234567890123456789012345678901234567890".to_string(),
            storage_slot: [1u8; 32],
            previous_value: Some([0u8; 32]),
            new_value: [100u8; 32],
            proof: None,
        }
    ];
    
    let mut extra = HashMap::new();
    extra.insert("gas_limit".to_string(), serde_json::json!("30000000"));
    extra.insert("gas_used".to_string(), serde_json::json!("21000"));
    extra.insert("difficulty".to_string(), serde_json::json!("15000000000000000"));
    
    let consensus_metadata = ConsensusMetadata {
        validator_set_hash: None,
        proposer: Some("0x0000000000000000000000000000000000000000".to_string()),
        round: None,
        signatures: None,
        extra,
    };

    ChainDataIR {
        block_metadata,
        state_changes,
        consensus_metadata,
        finality_status: FinalityStatus::Finalized,
    }
}

/// Create Cosmos chain data
pub fn create_cosmos_chain_data() -> ChainDataIR {
    let block_metadata = create_finalized_block("cosmos", 12345678);
    let state_changes = vec![
        StateChange {
            contract_address: "cosmos1abc123def456ghi789".to_string(),
            storage_slot: [2u8; 32],
            previous_value: Some([0u8; 32]),
            new_value: [200u8; 32],
            proof: None,
        }
    ];
    
    let mut extra = HashMap::new();
    extra.insert("chain_id".to_string(), serde_json::json!("cosmoshub-4"));
    extra.insert("app_hash".to_string(), serde_json::json!("7D1A1C3E5B2F8A9C"));
    
    let consensus_metadata = ConsensusMetadata {
        validator_set_hash: Some("validator_set_hash_123".to_string()),
        proposer: Some("cosmosvaloper1abc123".to_string()),
        round: Some(0),
        signatures: Some(vec!["sig1".to_string(), "sig2".to_string()]),
        extra,
    };

    ChainDataIR {
        block_metadata,
        state_changes,
        consensus_metadata,
        finality_status: FinalityStatus::Finalized,
    }
}

/// Create Solana chain data
pub fn create_solana_chain_data() -> ChainDataIR {
    let block_metadata = create_finalized_block("solana", 150000000);
    let state_changes = vec![
        StateChange {
            contract_address: "11111111111111111111111111111112".to_string(),
            storage_slot: [3u8; 32],
            previous_value: Some([0u8; 32]),
            new_value: [255u8; 32],
            proof: None,
        }
    ];
    
    let mut extra = HashMap::new();
    extra.insert("slot".to_string(), serde_json::json!(150000000));
    extra.insert("bank_hash".to_string(), serde_json::json!("BankHashABC123"));
    
    let consensus_metadata = ConsensusMetadata {
        validator_set_hash: None,
        proposer: Some("SolanaValidator123".to_string()),
        round: None,
        signatures: None,
        extra,
    };

    ChainDataIR {
        block_metadata,
        state_changes,
        consensus_metadata,
        finality_status: FinalityStatus::Finalized,
    }
}

/// Create a test event with specific ID
fn create_test_event_with_id(id: String) -> UnifiedEvent {
    UnifiedEvent {
        id,
        chain: "test-chain".to_string(),
        block_number: 1000,
        block_hash: "test_block_hash".to_string(),
        tx_hash: "test_tx_hash".to_string(),
        timestamp: SystemTime::now(),
        event_type: "TestEvent".to_string(),
        contract_address: Some("test_contract".to_string()),
        raw_data: b"test_data".to_vec(),
        attributes: {
            let mut attrs = HashMap::new();
            attrs.insert("test".to_string(), serde_json::json!("data"));
            attrs
        },
        source: EventSource::Chain,
    }
}

/// Create an arbitrary event for property testing
#[cfg(feature = "proptest")]
pub fn arbitrary_event() -> impl proptest::strategy::Strategy<Value = UnifiedEvent> {
    use proptest::prelude::*;
    use proptest::option;
    
    (
        "[a-z0-9-]{10,20}",
        "[a-z]{3,10}",
        1u64..100000u64,
        "[a-f0-9]{8,64}",
        "[a-f0-9]{8,64}",
        option::of("[a-f0-9]{8,40}"),
        "[A-Z][a-zA-Z]{3,20}",
    ).prop_map(|(id, chain, block_number, block_hash, tx_hash, contract_addr, event_type)| {
        UnifiedEvent {
            id,
            chain,
            block_number,
            block_hash,
            tx_hash,
            timestamp: SystemTime::now(),
            event_type,
            contract_address: contract_addr,
            raw_data: b"test_data".to_vec(),
            attributes: {
                let mut attrs = HashMap::new();
                attrs.insert("test".to_string(), serde_json::json!("data"));
                attrs
            },
            source: EventSource::Chain,
        }
    })
}

/// Create event filter for testing
pub fn create_test_event_filter() -> EventFilter {
    EventFilter::new()
        .with_chain("test-chain")
        .with_block_range(1000, 2000)
        .with_limit(100)
        .with_offset(0)
}