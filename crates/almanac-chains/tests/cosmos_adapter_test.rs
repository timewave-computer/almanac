//! Integration tests for Cosmos adapter

#[cfg(all(test, feature = "cosmos"))]
mod tests {
    use almanac_chains::{ChainAdapter, ChainConfig, CosmosAdapter};
    use std::env;

    fn get_test_config() -> ChainConfig {
        ChainConfig {
            chain_id: "cosmoshub-testnet".to_string(),
            rpc_url: env::var("COSMOS_TEST_RPC_URL")
                .unwrap_or_else(|_| "https://rpc.testnet.cosmos.network".to_string()),
            api_key: None,
            confirmations: 2,
            max_retries: 3,
            timeout_secs: 30,
            extra: serde_json::Value::Null,
        }
    }

    #[tokio::test]
    #[ignore] // Run with --ignored flag when RPC is available
    async fn test_cosmos_adapter_connection() {
        let config = get_test_config();
        let adapter = CosmosAdapter::new(config).await;

        assert!(adapter.is_ok(), "Failed to create Cosmos adapter");

        let adapter = adapter.unwrap();
        assert_eq!(adapter.chain_id(), "cosmoshub-testnet");
    }

    #[tokio::test]
    #[ignore]
    async fn test_get_latest_block() {
        let config = get_test_config();
        let adapter = CosmosAdapter::new(config).await.unwrap();

        let latest_block = adapter.get_latest_block().await;
        assert!(latest_block.is_ok(), "Failed to get latest block");

        let block_number = latest_block.unwrap();
        assert!(block_number > 0, "Latest block should be greater than 0");
    }

    #[tokio::test]
    #[ignore]
    async fn test_fetch_block() {
        let config = get_test_config();
        let adapter = CosmosAdapter::new(config).await.unwrap();

        // Get a recent block
        let latest = adapter.get_latest_block().await.unwrap();
        let test_block = latest.saturating_sub(5);

        let block_data = adapter.fetch_block(test_block).await;
        assert!(block_data.is_ok(), "Failed to fetch block");

        let data = block_data.unwrap();
        assert_eq!(data.block_metadata.block_height, test_block);
        assert!(data.block_metadata.timestamp > 0);
        assert!(data.consensus_metadata.validator_set_hash.is_some());
    }

    #[tokio::test]
    #[ignore]
    async fn test_finality_status() {
        let config = get_test_config();
        let adapter = CosmosAdapter::new(config).await.unwrap();

        let latest = adapter.get_latest_block().await.unwrap();

        // Test Cosmos instant finality
        let recent_block = latest - 1;
        let finalized_block = latest - 3;

        let recent_finalized = adapter.is_block_finalized(recent_block).await.unwrap();
        let old_finalized = adapter.is_block_finalized(finalized_block).await.unwrap();

        // Cosmos achieves finality quickly - recent blocks should be finalized
        assert!(recent_finalized, "Cosmos has fast finality");
        assert!(
            old_finalized,
            "Block with 3 confirmations should be finalized"
        );
    }

    #[tokio::test]
    #[ignore]
    async fn test_block_metadata_extraction() {
        let config = get_test_config();
        let adapter = CosmosAdapter::new(config).await.unwrap();

        let latest = adapter.get_latest_block().await.unwrap();
        let test_block = latest.saturating_sub(5);

        let metadata = adapter.get_block_metadata(test_block).await;
        assert!(metadata.is_ok(), "Failed to get block metadata");

        let meta = metadata.unwrap();
        // Chain ID is not part of BlockMetadata, remove this assertion
        // assert_eq!(meta.chain_id, "cosmoshub-testnet");
        assert_eq!(meta.block_height, test_block);
        assert!(!meta.state_root.is_empty()); // app_hash
        assert!(meta.transactions_root.is_some()); // data_hash
        assert!(meta.receipts_root.is_none()); // Not applicable for Cosmos
    }

    #[tokio::test]
    #[ignore]
    async fn test_state_change_extraction() {
        let config = get_test_config();
        let adapter = CosmosAdapter::new(config).await.unwrap();

        let latest = adapter.get_latest_block().await.unwrap();
        let test_block = latest.saturating_sub(5);

        let state_changes = adapter.extract_state_changes(test_block).await;
        assert!(state_changes.is_ok(), "Failed to extract state changes");

        // State changes depend on block content
        let _changes = state_changes.unwrap();
        // Even empty blocks are valid - length is always non-negative
        // State changes retrieved successfully
    }

    #[tokio::test]
    async fn test_invalid_rpc_url() {
        let mut config = get_test_config();
        config.rpc_url = "https://invalid.rpc.url".to_string();

        let adapter = CosmosAdapter::new(config).await;
        assert!(adapter.is_err(), "Should fail with invalid RPC URL");
    }
}
