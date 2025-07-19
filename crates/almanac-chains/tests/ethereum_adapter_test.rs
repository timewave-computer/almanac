//! Integration tests for Ethereum adapter

#[cfg(all(test, feature = "ethereum"))]
mod tests {
    use almanac_chains::{ChainAdapter, ChainConfig, EthereumAdapter};
    use std::env;

    fn get_test_config() -> ChainConfig {
        ChainConfig {
            chain_id: "ethereum-goerli".to_string(),
            rpc_url: env::var("ETHEREUM_TEST_RPC_URL")
                .unwrap_or_else(|_| "https://eth-goerli.g.alchemy.com/v2/demo".to_string()),
            api_key: env::var("ETHEREUM_TEST_API_KEY").ok(),
            confirmations: 6,
            max_retries: 3,
            timeout_secs: 30,
            extra: serde_json::Value::Null,
        }
    }

    #[tokio::test]
    #[ignore] // Run with --ignored flag when RPC is available
    async fn test_ethereum_adapter_connection() {
        let config = get_test_config();
        let adapter = EthereumAdapter::new(config).await;
        
        assert!(adapter.is_ok(), "Failed to create Ethereum adapter");
        
        let adapter = adapter.unwrap();
        assert_eq!(adapter.chain_id(), "ethereum-goerli");
    }

    #[tokio::test]
    #[ignore]
    async fn test_get_latest_block() {
        let config = get_test_config();
        let adapter = EthereumAdapter::new(config).await.unwrap();
        
        let latest_block = adapter.get_latest_block().await;
        assert!(latest_block.is_ok(), "Failed to get latest block");
        
        let block_number = latest_block.unwrap();
        assert!(block_number > 0, "Latest block should be greater than 0");
    }

    #[tokio::test]
    #[ignore]
    async fn test_fetch_block() {
        let config = get_test_config();
        let adapter = EthereumAdapter::new(config).await.unwrap();
        
        // Get a recent block
        let latest = adapter.get_latest_block().await.unwrap();
        let test_block = latest.saturating_sub(10);
        
        let block_data = adapter.fetch_block(test_block).await;
        assert!(block_data.is_ok(), "Failed to fetch block");
        
        let data = block_data.unwrap();
        assert_eq!(data.block_metadata.block_height, test_block);
        assert!(data.block_metadata.timestamp > 0);
    }

    #[tokio::test]
    #[ignore]
    async fn test_finality_status() {
        let config = get_test_config();
        let adapter = EthereumAdapter::new(config).await.unwrap();
        
        let latest = adapter.get_latest_block().await.unwrap();
        
        // Test different finality levels
        let recent_block = latest - 1;
        let safe_block = latest - 10;
        let finalized_block = latest - 100;
        
        let recent_finalized = adapter.is_block_finalized(recent_block).await.unwrap();
        let safe_finalized = adapter.is_block_finalized(safe_block).await.unwrap();
        let old_finalized = adapter.is_block_finalized(finalized_block).await.unwrap();
        
        assert!(!recent_finalized, "Recent block should not be finalized");
        assert!(safe_finalized, "Block with 10 confirmations should be finalized");
        assert!(old_finalized, "Old block should be finalized");
    }

    #[tokio::test]
    #[ignore]
    async fn test_block_metadata_extraction() {
        let config = get_test_config();
        let adapter = EthereumAdapter::new(config).await.unwrap();
        
        let latest = adapter.get_latest_block().await.unwrap();
        let test_block = latest.saturating_sub(10);
        
        let metadata = adapter.get_block_metadata(test_block).await;
        assert!(metadata.is_ok(), "Failed to get block metadata");
        
        let meta = metadata.unwrap();
        assert_eq!(meta.block_height, test_block);
        assert!(!meta.state_root.is_empty());
        assert!(meta.transactions_root.is_some());
        assert!(meta.receipts_root.is_some());
    }

    #[tokio::test]
    async fn test_invalid_rpc_url() {
        let mut config = get_test_config();
        config.rpc_url = "https://invalid.rpc.url".to_string();
        
        let adapter = EthereumAdapter::new(config).await;
        assert!(adapter.is_err(), "Should fail with invalid RPC URL");
    }
}