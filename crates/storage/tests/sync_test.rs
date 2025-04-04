use std::sync::Arc;
use tempfile::TempDir;
use indexer_common::Result;

use indexer_storage::rocks::{RocksStorage, RocksConfig};
use indexer_storage::postgres::{PostgresStorage, PostgresConfig};
use indexer_storage::sync::{StorageSynchronizer, SyncConfig};
use indexer_storage::tests::common::create_mock_events;
use indexer_storage::EventFilter;
use indexer_storage::Storage;

/// Test for the synchronization between RocksDB and PostgreSQL
#[tokio::test]
#[ignore] // Ignore this test for now as it requires a running PostgreSQL instance
async fn test_rocks_postgres_sync() -> Result<()> {
    // Set up RocksDB storage
    let rocks_dir = TempDir::new()?;
    let rocks_config = RocksConfig {
        path: rocks_dir.path().to_str().unwrap().to_string(),
        create_if_missing: true,
        cache_size_mb: 64,
    };
    let rocks = Arc::new(RocksStorage::new(rocks_config)?);
    
    // Set up PostgreSQL storage
    // This uses the default 'postgres' database which should already exist
    let pg_config = PostgresConfig {
        url: "postgres://postgres:postgres@localhost:5432/postgres".to_string(),
        max_connections: 5,
        connection_timeout: 30,
    };
    
    let postgres = Arc::new(PostgresStorage::new(pg_config).await?);
    
    // Create sync configuration
    let sync_config = SyncConfig {
        sync_interval_ms: 100, // Short interval for testing
        batch_size: 50,
        chains: vec!["ethereum".to_string(), "cosmos".to_string()],
    };
    
    // Create synchronizer with RocksDB as primary and PostgreSQL as secondary
    let synchronizer = StorageSynchronizer::new_rocks_postgres(
        rocks.clone(), 
        postgres.clone(),
        sync_config
    ).await;
    
    // Create test events for Ethereum
    let eth_events = create_mock_events("ethereum", 100);
    
    // Create test events for Cosmos
    let cosmos_events = create_mock_events("cosmos", 50);
    
    // Store events in RocksDB
    for event in eth_events {
        rocks.store_event(event).await?;
    }
    
    for event in cosmos_events {
        rocks.store_event(event).await?;
    }
    
    // Check that RocksDB has the correct latest blocks
    let rocks_eth_latest = rocks.get_latest_block("ethereum").await?;
    let rocks_cosmos_latest = rocks.get_latest_block("cosmos").await?;
    
    assert_eq!(rocks_eth_latest, 199, "RocksDB should have latest Ethereum block of 199");
    assert_eq!(rocks_cosmos_latest, 149, "RocksDB should have latest Cosmos block of 149");
    
    // PostgreSQL should have no events yet
    let pg_eth_latest = postgres.get_latest_block("ethereum").await.unwrap_or(0);
    let pg_cosmos_latest = postgres.get_latest_block("cosmos").await.unwrap_or(0);
    
    assert_eq!(pg_eth_latest, 0, "PostgreSQL should have no Ethereum blocks initially");
    assert_eq!(pg_cosmos_latest, 0, "PostgreSQL should have no Cosmos blocks initially");
    
    // Start synchronization
    synchronizer.start().await?;
    
    // Wait for synchronization to complete
    // We need to wait enough time for the sync to process all the events
    tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
    
    // Stop synchronization
    synchronizer.stop().await?;
    
    // Check that PostgreSQL now has the same latest blocks
    let pg_eth_latest_after = postgres.get_latest_block("ethereum").await?;
    let pg_cosmos_latest_after = postgres.get_latest_block("cosmos").await?;
    
    assert_eq!(pg_eth_latest_after, rocks_eth_latest, 
        "PostgreSQL should have same Ethereum latest block as RocksDB after sync");
    assert_eq!(pg_cosmos_latest_after, rocks_cosmos_latest, 
        "PostgreSQL should have same Cosmos latest block as RocksDB after sync");
    
    // Verify event counts match
    let eth_filter = EventFilter {
        chain: Some("ethereum".to_string()),
        block_range: None,
        time_range: None,
        event_types: None,
        limit: None,
        offset: None,
    };
    
    let cosmos_filter = EventFilter {
        chain: Some("cosmos".to_string()),
        block_range: None,
        time_range: None,
        event_types: None,
        limit: None,
        offset: None,
    };
    
    let rocks_eth_events = rocks.get_events(vec![eth_filter.clone()]).await?;
    let rocks_cosmos_events = rocks.get_events(vec![cosmos_filter.clone()]).await?;
    
    let pg_eth_events = postgres.get_events(vec![eth_filter]).await?;
    let pg_cosmos_events = postgres.get_events(vec![cosmos_filter]).await?;
    
    assert_eq!(pg_eth_events.len(), rocks_eth_events.len(), 
        "PostgreSQL should have same number of Ethereum events as RocksDB");
    assert_eq!(pg_cosmos_events.len(), rocks_cosmos_events.len(), 
        "PostgreSQL should have same number of Cosmos events as RocksDB");
    
    Ok(())
}

/// Test for partial updates during synchronization
#[tokio::test]
async fn test_sync_partial_updates() -> Result<()> {
    // Set up two RocksDB storage instances for testing
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
    
    // Create sync configuration with a small batch size
    let sync_config = SyncConfig {
        sync_interval_ms: 100,
        batch_size: 20, // Small batch size to test partial updates
        chains: vec!["ethereum".to_string()],
    };
    
    // Create synchronizer for two RocksDB instances using the generic constructor
    let synchronizer = StorageSynchronizer::new_generic(
        primary.clone(), 
        secondary.clone(),
        sync_config
    ).await;
    
    // Create and store a large number of ethereum events in primary
    let eth_events_1 = create_mock_events("ethereum", 50);
    for event in eth_events_1 {
        primary.store_event(event).await?;
    }
    
    // Start synchronization
    synchronizer.start().await?;
    
    // Wait for first batch to sync
    tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
    
    // Check intermediate state - only first batch should be synced
    let primary_latest = primary.get_latest_block("ethereum").await?;
    let secondary_latest = secondary.get_latest_block("ethereum").await?;
    
    assert_eq!(primary_latest, 149, "Primary should have latest block 149");
    
    // We expect only part of the events to be synchronized due to the batch size
    assert!(secondary_latest > 0, "Secondary should have some blocks");
    assert!(secondary_latest < primary_latest, "Secondary should not have all blocks yet");
    
    // Wait for full sync - increase the wait time
    tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
    
    // Now secondary should be caught up
    let secondary_latest_after = secondary.get_latest_block("ethereum").await?;
    let primary_latest_after = primary.get_latest_block("ethereum").await?;
    
    assert_eq!(secondary_latest_after, primary_latest_after, 
        "Secondary should have caught up to primary (after: {} == {})", 
        secondary_latest_after, primary_latest_after);
    
    // Add more events to primary
    let eth_events_2 = create_mock_events("ethereum", 30);
    for event in eth_events_2 {
        primary.store_event(event).await?;
    }
    
    // Wait for additional sync - increase the wait time
    tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
    
    // Stop synchronization
    synchronizer.stop().await?;
    
    // Verify secondary has caught up again
    let primary_latest_final = primary.get_latest_block("ethereum").await?;
    let secondary_latest_final = secondary.get_latest_block("ethereum").await?;
    
    assert_eq!(secondary_latest_final, primary_latest_final, 
        "Secondary should have caught up to primary after new events (final: {} == {})",
        secondary_latest_final, primary_latest_final);
    
    Ok(())
}

/// Test for chain-specific synchronization
#[tokio::test]
async fn test_sync_chain_specific() -> Result<()> {
    // Set up two RocksDB storage instances
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
    
    // Configure sync for multiple chains
    let sync_config = SyncConfig {
        sync_interval_ms: 100,
        batch_size: 100,
        chains: vec!["ethereum".to_string(), "cosmos".to_string()],
    };
    
    // Create synchronizer using the generic constructor
    let synchronizer = StorageSynchronizer::new_generic(
        primary.clone(), 
        secondary.clone(),
        sync_config
    ).await;
    
    // Create and store different events for each chain
    let eth_events = create_mock_events("ethereum", 30);
    let cosmos_events = create_mock_events("cosmos", 20);
    
    for event in eth_events {
        primary.store_event(event).await?;
    }
    
    for event in cosmos_events {
        primary.store_event(event).await?;
    }
    
    // Start synchronization
    synchronizer.start().await?;
    
    // Wait for sync to complete
    tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
    
    // Stop synchronization
    synchronizer.stop().await?;
    
    // Verify that both chains were synchronized correctly
    let primary_eth_latest = primary.get_latest_block("ethereum").await?;
    let primary_cosmos_latest = primary.get_latest_block("cosmos").await?;
    
    let secondary_eth_latest = secondary.get_latest_block("ethereum").await?;
    let secondary_cosmos_latest = secondary.get_latest_block("cosmos").await?;
    
    assert_eq!(secondary_eth_latest, primary_eth_latest, 
        "Secondary should have same Ethereum latest block as primary");
    assert_eq!(secondary_cosmos_latest, primary_cosmos_latest, 
        "Secondary should have same Cosmos latest block as primary");
    
    Ok(())
}

/// Test for chain-specific synchronization with PostgreSQL to RocksDB
#[tokio::test]
#[ignore] // Ignore this test for now as it requires a running PostgreSQL instance
async fn test_sync_chain_specific_postgres_rocks() -> Result<()> {
    // Set up RocksDB storage instance
    let rocks_dir = TempDir::new()?;
    let rocks_config = RocksConfig {
        path: rocks_dir.path().to_str().unwrap().to_string(),
        create_if_missing: true,
        cache_size_mb: 64,
    };
    let rocks = Arc::new(RocksStorage::new(rocks_config)?);
    
    // Set up PostgreSQL storage
    let pg_config = PostgresConfig {
        url: "postgres://postgres:postgres@localhost:5432/postgres".to_string(),
        max_connections: 5,
        connection_timeout: 30,
    };
    
    let postgres = Arc::new(PostgresStorage::new(pg_config).await?);
    
    // Configure sync for multiple chains
    let sync_config = SyncConfig {
        sync_interval_ms: 100,
        batch_size: 100,
        chains: vec!["ethereum".to_string(), "cosmos".to_string()],
    };
    
    // Create synchronizer with generic storages
    // Using PostgreSQL as primary and RocksDB as secondary
    let synchronizer = StorageSynchronizer::new_generic(
        postgres.clone(), 
        rocks.clone(),
        sync_config
    ).await;
    
    // Create and store different events for each chain
    let eth_events = create_mock_events("ethereum", 30);
    let cosmos_events = create_mock_events("cosmos", 20);
    
    for event in eth_events {
        postgres.store_event(event).await?;
    }
    
    for event in cosmos_events {
        postgres.store_event(event).await?;
    }
    
    // Start synchronization
    synchronizer.start().await?;
    
    // Wait for sync to complete
    tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
    
    // Stop synchronization
    synchronizer.stop().await?;
    
    // Verify that both chains were synchronized correctly
    let postgres_eth_latest = postgres.get_latest_block("ethereum").await?;
    let postgres_cosmos_latest = postgres.get_latest_block("cosmos").await?;
    
    let rocks_eth_latest = rocks.get_latest_block("ethereum").await?;
    let rocks_cosmos_latest = rocks.get_latest_block("cosmos").await?;
    
    assert_eq!(rocks_eth_latest, postgres_eth_latest, 
        "RocksDB should have same Ethereum latest block as PostgreSQL");
    assert_eq!(rocks_cosmos_latest, postgres_cosmos_latest, 
        "RocksDB should have same Cosmos latest block as PostgreSQL");
    
    Ok(())
} 