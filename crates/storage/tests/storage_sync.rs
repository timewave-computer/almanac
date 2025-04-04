use std::sync::Arc;

use tempfile::TempDir;
use indexer_common::Result;

use indexer_storage::rocks::{RocksStorage, RocksConfig};
use indexer_storage::Storage;
use indexer_storage::tests::common::{create_mock_event, create_mock_events};

// Test basic storage functionalities needed by synchronization
#[tokio::test]
async fn test_storage_interfaces() -> Result<()> {
    // Create two RocksDB storage instances for testing
    let primary_dir = TempDir::new()?;
    let primary_config = RocksConfig {
        path: primary_dir.path().to_str().unwrap().to_string(),
        create_if_missing: true,
        cache_size_mb: 64,
    };
    let primary = Arc::new(RocksStorage::new(primary_config)?);
    
    let secondary_dir = TempDir::new()?;
    let secondary_config = RocksConfig {
        path: secondary_dir.path().to_str().unwrap().to_string(),
        create_if_missing: true,
        cache_size_mb: 64,
    };
    let secondary = Arc::new(RocksStorage::new(secondary_config)?);
    
    // Create test events
    let ethereum_events = create_mock_events("ethereum", 10);
    let cosmos_events = create_mock_events("cosmos", 5);
    
    // Store events in primary storage
    for event in ethereum_events {
        primary.store_event(event).await?;
    }
    
    for event in cosmos_events {
        primary.store_event(event).await?;
    }
    
    // Check that primary storage has the correct latest blocks
    let eth_primary_latest = primary.get_latest_block("ethereum").await?;
    let cosmos_primary_latest = primary.get_latest_block("cosmos").await?;
    
    assert_eq!(eth_primary_latest, 109, "Primary storage should have latest Ethereum block of 109");
    assert_eq!(cosmos_primary_latest, 104, "Primary storage should have latest Cosmos block of 104");
    
    // Manually copy events from primary to secondary to simulate synchronization
    // Get events from primary
    let eth_filter = indexer_storage::EventFilter {
        chain: Some("ethereum".to_string()),
        block_range: Some((100, 109)),
        time_range: None,
        event_types: None,
        limit: None,
        offset: None,
    };
    
    let cosmos_filter = indexer_storage::EventFilter {
        chain: Some("cosmos".to_string()),
        block_range: Some((100, 104)),
        time_range: None,
        event_types: None,
        limit: None,
        offset: None,
    };
    
    let eth_events = primary.get_events(vec![eth_filter]).await?;
    let cosmos_events = primary.get_events(vec![cosmos_filter]).await?;
    
    // Store events in secondary
    for event in eth_events {
        secondary.store_event(event).await?;
    }
    
    for event in cosmos_events {
        secondary.store_event(event).await?;
    }
    
    // Verify events were copied successfully
    let eth_secondary_latest = secondary.get_latest_block("ethereum").await?;
    let cosmos_secondary_latest = secondary.get_latest_block("cosmos").await?;
    
    assert_eq!(eth_secondary_latest, eth_primary_latest, 
               "Secondary should have the same Ethereum latest block as primary");
    assert_eq!(cosmos_secondary_latest, cosmos_primary_latest, 
               "Secondary should have the same Cosmos latest block as primary");
    
    // Test partial updates by adding more events to primary
    let new_eth_events = create_mock_events("ethereum", 5);
    for (i, _event) in new_eth_events.into_iter().enumerate() {
        // Store events with block numbers 110-114
        let block_num = 110 + i as u64;
        let modified_event = create_mock_event(&format!("eth_event_{}", block_num), "ethereum", block_num);
        primary.store_event(modified_event).await?;
    }
    
    // Verify primary has new events
    let eth_primary_latest = primary.get_latest_block("ethereum").await?;
    assert_eq!(eth_primary_latest, 114, "Primary should now have latest Ethereum block of 114");
    
    // Manually sync new events
    let eth_filter = indexer_storage::EventFilter {
        chain: Some("ethereum".to_string()),
        block_range: Some((110, 114)),
        time_range: None,
        event_types: None,
        limit: None,
        offset: None,
    };
    
    let new_eth_events = primary.get_events(vec![eth_filter]).await?;
    
    // Store new events in secondary
    for event in new_eth_events {
        secondary.store_event(event).await?;
    }
    
    // Verify secondary now has the same latest block
    let eth_secondary_latest = secondary.get_latest_block("ethereum").await?;
    assert_eq!(eth_secondary_latest, eth_primary_latest, 
               "Secondary should have the same latest block as primary after update");
    
    Ok(())
} 