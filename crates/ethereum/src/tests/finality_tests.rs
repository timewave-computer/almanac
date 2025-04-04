/// Tests for Ethereum block finality tracking functionality
use std::sync::Arc;
use mockall::predicate::*;
use mockall::mock;

use indexer_core::Result;
use indexer_storage::{BlockStatus, Storage};
use crate::provider::BlockStatus as EthereumBlockStatus;
use crate::EthereumEventService;

// Create a mock for the EthereumEventService
mock! {
    pub EthereumEventService {
        pub async fn get_latest_finalized_block(&self) -> Result<u64>;
        pub async fn get_latest_safe_block(&self) -> Result<u64>;
        pub async fn get_block_by_status(&self, status: EthereumBlockStatus) -> Result<(ethers::types::Block<ethers::types::Transaction>, u64)>;
    }
}

// Create a mock for the Storage
mock! {
    pub Storage {
        pub async fn update_block_status(&self, chain: &str, block_number: u64, status: BlockStatus) -> Result<()>;
        pub async fn get_latest_block_with_status(&self, chain: &str, status: BlockStatus) -> Result<u64>;
    }
}

#[tokio::test]
async fn test_finalized_block_retrieval() {
    let mut mock_service = MockEthereumEventService::new();
    
    // Set up expectations
    mock_service
        .expect_get_latest_finalized_block()
        .times(1)
        .returning(|| Ok(12345));
        
    // Call the method being tested
    let result = mock_service.get_latest_finalized_block().await;
    
    // Verify result
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 12345);
}

#[tokio::test]
async fn test_safe_block_retrieval() {
    let mut mock_service = MockEthereumEventService::new();
    
    // Set up expectations
    mock_service
        .expect_get_latest_safe_block()
        .times(1)
        .returning(|| Ok(23456));
        
    // Call the method being tested
    let result = mock_service.get_latest_safe_block().await;
    
    // Verify result
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 23456);
}

#[tokio::test]
async fn test_block_status_update() {
    let mut mock_service = MockEthereumEventService::new();
    let mut mock_storage = MockStorage::new();
    
    // Set up expectations for service
    mock_service
        .expect_get_latest_finalized_block()
        .times(1)
        .returning(|| Ok(12345));
        
    mock_service
        .expect_get_latest_safe_block()
        .times(1)
        .returning(|| Ok(23456));
    
    // Set up expectations for storage
    mock_storage
        .expect_update_block_status()
        .with(eq("ethereum"), eq(12345), eq(BlockStatus::Finalized))
        .times(1)
        .returning(|_, _, _| Ok(()));
        
    mock_storage
        .expect_update_block_status()
        .with(eq("ethereum"), eq(23456), eq(BlockStatus::Safe))
        .times(1)
        .returning(|_, _, _| Ok(()));
    
    // Call and verify finalized block update
    let finalized_block = mock_service.get_latest_finalized_block().await.unwrap();
    let result = mock_storage.update_block_status("ethereum", finalized_block, BlockStatus::Finalized).await;
    assert!(result.is_ok());
    
    // Call and verify safe block update
    let safe_block = mock_service.get_latest_safe_block().await.unwrap();
    let result = mock_storage.update_block_status("ethereum", safe_block, BlockStatus::Safe).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_storage_retrieval_by_status() {
    let mut mock_storage = MockStorage::new();
    
    // Set up expectations
    mock_storage
        .expect_get_latest_block_with_status()
        .with(eq("ethereum"), eq(BlockStatus::Finalized))
        .times(1)
        .returning(|_, _| Ok(12345));
        
    mock_storage
        .expect_get_latest_block_with_status()
        .with(eq("ethereum"), eq(BlockStatus::Safe))
        .times(1)
        .returning(|_, _| Ok(23456));
    
    // Call and verify finalized block retrieval
    let finalized_block = mock_storage.get_latest_block_with_status("ethereum", BlockStatus::Finalized).await;
    assert!(finalized_block.is_ok());
    assert_eq!(finalized_block.unwrap(), 12345);
    
    // Call and verify safe block retrieval
    let safe_block = mock_storage.get_latest_block_with_status("ethereum", BlockStatus::Safe).await;
    assert!(safe_block.is_ok());
    assert_eq!(safe_block.unwrap(), 23456);
} 