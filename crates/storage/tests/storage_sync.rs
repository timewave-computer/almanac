use std::{env, sync::Arc};

use tempfile::TempDir;
use anyhow::Result;

use indexer_storage::{
    rocks::RocksStorage,
    postgres::PgStorage,
    sync::SyncManager,
    Storage, StorageProvider,
};
use indexer_core::types::{BlockFinality, BlockStatus};
use sqlx::postgres::PgPoolOptions;
use tokio::sync::Mutex;
use uuid::Uuid;
use chrono::Utc;

// Helper function to get a unique database name
fn get_test_db_name() -> String {
    format!("test_storage_sync_{}", Uuid::new_v4().to_string().replace("-", ""))
}

async fn setup_postgres() -> Result<PgStorage> {
    let database_url = env::var("DATABASE_URL").unwrap_or_else(|_| {
        "postgres://postgres:postgres@localhost:5432/indexer_test".to_string()
    });
    
    println!("Connecting to PostgreSQL at: {}", database_url);
    
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await?;
    
    // Run migrations
    println!("Running PostgreSQL migrations...");
    sqlx::migrate!("./migrations").run(&pool).await?;
    
    Ok(PgStorage::new(pool))
}

fn setup_rocksdb() -> Result<RocksStorage> {
    // Create a temporary directory for RocksDB or use the one from environment
    let rocks_path = match env::var("ROCKSDB_PATH") {
        Ok(path) => {
            println!("Using RocksDB path from environment: {}", path);
            std::path::PathBuf::from(path)
        },
        Err(_) => {
            let dir = TempDir::new()?;
            println!("Using temporary RocksDB path: {:?}", dir.path());
            dir.path().to_path_buf()
        }
    };
    
    let rocks_db = RocksStorage::new(&rocks_path)?;
    Ok(rocks_db)
}

async fn setup_sync_manager() -> Result<SyncManager> {
    let pg_storage = setup_postgres().await?;
    let rocks_storage = setup_rocksdb()?;
    
    let sync_manager = SyncManager::new(
        Arc::new(Mutex::new(rocks_storage)),
        Arc::new(pg_storage),
    );
    
    Ok(sync_manager)
}

// Test data generation
fn generate_test_block(chain_id: &str, block_number: u64) -> (String, BlockStatus) {
    let block_hash = format!("0x{:064x}", block_number);
    let parent_hash = if block_number > 0 {
        format!("0x{:064x}", block_number - 1)
    } else {
        "0x0000000000000000000000000000000000000000000000000000000000000000".to_string()
    };
    
    let timestamp = Utc::now();
    
    let block_status = BlockStatus {
        chain_id: chain_id.to_string(),
        block_number,
        block_hash: block_hash.clone(),
        parent_hash,
        timestamp,
        finality: BlockFinality::Confirmed,
    };
    
    (block_hash, block_status)
}

fn generate_test_transaction(
    chain_id: &str,
    block_number: u64,
    tx_index: u64,
) -> (String, indexer_core::types::Transaction) {
    let tx_hash = format!("0x{:064x}", tx_index);
    let from_address = format!("0x{:040x}", tx_index);
    let to_address = format!("0x{:040x}", tx_index + 1);
    
    let transaction = indexer_core::types::Transaction {
        chain_id: chain_id.to_string(),
        block_number,
        tx_hash: tx_hash.clone(),
        from_address,
        to_address: Some(to_address),
        value: "1000000000000000000".to_string(), // 1 ETH
        data: vec![0, 1, 2, 3],
        timestamp: Utc::now(),
    };
    
    (tx_hash, transaction)
}

fn generate_test_event(
    chain_id: &str,
    block_number: u64,
    tx_hash: &str,
    event_index: u64,
) -> indexer_core::event::Event {
    let contract_address = format!("0x{:040x}", event_index);
    let event_type = "Transfer".to_string();
    let topics = vec![
        format!("0x{:064x}", 1),
        format!("0x{:064x}", 2),
        format!("0x{:064x}", 3),
    ];
    
    indexer_core::event::Event {
        chain_id: chain_id.to_string(),
        block_number,
        transaction_hash: tx_hash.to_string(),
        contract_address,
        log_index: event_index as u32,
        event_type,
        topics,
        data: vec![4, 5, 6, 7],
        timestamp: Utc::now(),
    }
}

#[tokio::test]
async fn test_storage_sync_blocks() -> Result<()> {
    println!("Setting up sync manager for block test...");
    let sync_manager = setup_sync_manager().await?;
    let chain_id = "ethereum";
    
    println!("Storing blocks in both databases...");
    // Store blocks in both databases
    for block_number in 1..=10 {
        let (block_hash, block_status) = generate_test_block(chain_id, block_number);
        sync_manager.store_block_status(&block_status).await?;
        
        // Verify the block is stored in RocksDB
        let rocks_result = sync_manager
            .get_rocks()
            .lock()
            .await
            .get_block_status(chain_id, block_number)
            .await?;
        
        assert_eq!(rocks_result.unwrap().block_hash, block_hash);
        
        // Verify the block is stored in PostgreSQL
        let pg_result = sync_manager
            .get_postgres()
            .get_block_status(chain_id, block_number)
            .await?;
        
        assert_eq!(pg_result.unwrap().block_hash, block_hash);
    }
    
    // Test getting latest block number
    println!("Testing latest block retrieval...");
    let latest_block = sync_manager.get_latest_block_number(chain_id).await?;
    assert_eq!(latest_block, Some(10));
    
    Ok(())
}

#[tokio::test]
async fn test_storage_sync_transactions() -> Result<()> {
    println!("Setting up sync manager for transaction test...");
    let sync_manager = setup_sync_manager().await?;
    let chain_id = "ethereum";
    
    // First, store a block
    println!("Storing a test block...");
    let (block_hash, block_status) = generate_test_block(chain_id, 1);
    sync_manager.store_block_status(&block_status).await?;
    
    // Store transactions for the block
    println!("Storing transactions...");
    for tx_index in 1..=5 {
        let (tx_hash, transaction) = generate_test_transaction(chain_id, 1, tx_index);
        sync_manager.store_transaction(&transaction).await?;
        
        // Verify the transaction is stored in RocksDB
        let rocks_result = sync_manager
            .get_rocks()
            .lock()
            .await
            .get_transaction(chain_id, &tx_hash)
            .await?;
        
        assert_eq!(rocks_result.unwrap().tx_hash, tx_hash);
        
        // Verify the transaction is stored in PostgreSQL
        let pg_result = sync_manager
            .get_postgres()
            .get_transaction(chain_id, &tx_hash)
            .await?;
        
        assert_eq!(pg_result.unwrap().tx_hash, tx_hash);
    }
    
    // Test getting transactions for a block
    println!("Testing transaction retrieval for block...");
    let transactions = sync_manager.get_transactions_for_block(chain_id, 1).await?;
    assert_eq!(transactions.len(), 5);
    
    Ok(())
}

#[tokio::test]
async fn test_storage_sync_events() -> Result<()> {
    println!("Setting up sync manager for event test...");
    let sync_manager = setup_sync_manager().await?;
    let chain_id = "ethereum";
    
    // First, store a block
    println!("Storing a test block...");
    let (block_hash, block_status) = generate_test_block(chain_id, 1);
    sync_manager.store_block_status(&block_status).await?;
    
    // Store a transaction
    println!("Storing a test transaction...");
    let (tx_hash, transaction) = generate_test_transaction(chain_id, 1, 1);
    sync_manager.store_transaction(&transaction).await?;
    
    // Store events for the transaction
    println!("Storing events...");
    for event_index in 1..=3 {
        let event = generate_test_event(chain_id, 1, &tx_hash, event_index);
        sync_manager.store_event(&event).await?;
        
        // Verify the event is stored in RocksDB
        let rocks_result = sync_manager
            .get_rocks()
            .lock()
            .await
            .get_events_for_transaction(chain_id, &tx_hash)
            .await?;
        
        assert!(rocks_result.len() >= event_index as usize);
        
        // Verify the event is stored in PostgreSQL
        let pg_result = sync_manager
            .get_postgres()
            .get_events_for_transaction(chain_id, &tx_hash)
            .await?;
        
        assert!(pg_result.len() >= event_index as usize);
    }
    
    // Test getting events for a transaction
    println!("Testing event retrieval for transaction...");
    let events = sync_manager
        .get_events_for_transaction(chain_id, &tx_hash)
        .await?;
    assert_eq!(events.len(), 3);
    
    // Test getting events for a block
    println!("Testing event retrieval for block...");
    let events = sync_manager.get_events_for_block(chain_id, 1).await?;
    assert_eq!(events.len(), 3);
    
    Ok(())
}

#[tokio::test]
async fn test_storage_sync_block_finality() -> Result<()> {
    println!("Setting up sync manager for block finality test...");
    let sync_manager = setup_sync_manager().await?;
    let chain_id = "ethereum";
    
    // Store a block with Confirmed status
    println!("Storing block with Confirmed status...");
    let (block_hash, mut block_status) = generate_test_block(chain_id, 1);
    block_status.finality = BlockFinality::Confirmed;
    sync_manager.store_block_status(&block_status).await?;
    
    // Verify the block has Confirmed status
    let status = sync_manager
        .get_block_status(chain_id, 1)
        .await?
        .unwrap();
    assert_eq!(status.finality, BlockFinality::Confirmed);
    
    // Update the block to Safe status
    println!("Updating block to Safe status...");
    block_status.finality = BlockFinality::Safe;
    sync_manager.store_block_status(&block_status).await?;
    
    // Verify the block has Safe status
    let status = sync_manager
        .get_block_status(chain_id, 1)
        .await?
        .unwrap();
    assert_eq!(status.finality, BlockFinality::Safe);
    
    // Update the block to Finalized status
    println!("Updating block to Finalized status...");
    block_status.finality = BlockFinality::Finalized;
    sync_manager.store_block_status(&block_status).await?;
    
    // Verify the block has Finalized status
    let status = sync_manager
        .get_block_status(chain_id, 1)
        .await?
        .unwrap();
    assert_eq!(status.finality, BlockFinality::Finalized);
    
    Ok(())
}

#[tokio::test]
async fn test_storage_sync_chain_reorg() -> Result<()> {
    println!("Setting up sync manager for chain reorganization test...");
    let sync_manager = setup_sync_manager().await?;
    let chain_id = "ethereum";
    
    // Store blocks in the original chain
    println!("Storing original chain blocks...");
    for block_number in 1..=5 {
        let (block_hash, block_status) = generate_test_block(chain_id, block_number);
        sync_manager.store_block_status(&block_status).await?;
        
        // Add transactions and events to each block
        let (tx_hash, transaction) = generate_test_transaction(chain_id, block_number, 1);
        sync_manager.store_transaction(&transaction).await?;
        
        let event = generate_test_event(chain_id, block_number, &tx_hash, 1);
        sync_manager.store_event(&event).await?;
    }
    
    // Simulate a chain reorganization by replacing blocks 3-5
    println!("Simulating chain reorganization for blocks 3-5...");
    for block_number in 3..=5 {
        // Create a block with the same number but different hash (simulating the reorg)
        let (mut block_hash, mut block_status) = generate_test_block(chain_id, block_number);
        block_hash = format!("0xreorg{:060x}", block_number); // Different hash
        block_status.block_hash = block_hash.clone();
        
        // Store the new block
        sync_manager.store_block_status(&block_status).await?;
        
        // Verify the block hash has been updated in both storages
        let rocks_result = sync_manager
            .get_rocks()
            .lock()
            .await
            .get_block_status(chain_id, block_number)
            .await?;
        
        assert_eq!(rocks_result.unwrap().block_hash, block_hash);
        
        let pg_result = sync_manager
            .get_postgres()
            .get_block_status(chain_id, block_number)
            .await?;
        
        assert_eq!(pg_result.unwrap().block_hash, block_hash);
    }
    
    Ok(())
}

#[tokio::test]
async fn test_storage_sync_multi_chain() -> Result<()> {
    println!("Setting up sync manager for multi-chain test...");
    let sync_manager = setup_sync_manager().await?;
    let ethereum_chain_id = "ethereum";
    let cosmos_chain_id = "cosmos";
    
    // Store blocks for Ethereum
    println!("Storing Ethereum blocks...");
    for block_number in 1..=3 {
        let (block_hash, block_status) = generate_test_block(ethereum_chain_id, block_number);
        sync_manager.store_block_status(&block_status).await?;
    }
    
    // Store blocks for Cosmos
    println!("Storing Cosmos blocks...");
    for block_number in 1..=3 {
        let (block_hash, block_status) = generate_test_block(cosmos_chain_id, block_number);
        sync_manager.store_block_status(&block_status).await?;
    }
    
    // Verify latest block numbers for both chains
    println!("Verifying latest block numbers...");
    let ethereum_latest = sync_manager
        .get_latest_block_number(ethereum_chain_id)
        .await?;
    let cosmos_latest = sync_manager
        .get_latest_block_number(cosmos_chain_id)
        .await?;
    
    assert_eq!(ethereum_latest, Some(3));
    assert_eq!(cosmos_latest, Some(3));
    
    // Verify blocks for Ethereum
    println!("Verifying Ethereum blocks...");
    for block_number in 1..=3 {
        let block = sync_manager
            .get_block_status(ethereum_chain_id, block_number)
            .await?
            .unwrap();
        assert_eq!(block.chain_id, ethereum_chain_id);
    }
    
    // Verify blocks for Cosmos
    println!("Verifying Cosmos blocks...");
    for block_number in 1..=3 {
        let block = sync_manager
            .get_block_status(cosmos_chain_id, block_number)
            .await?
            .unwrap();
        assert_eq!(block.chain_id, cosmos_chain_id);
    }
    
    Ok(())
} 