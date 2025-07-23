//! Valence domain client integration example
//!
//! This example demonstrates how to use Timewave's Valence domain clients
//! for cross-chain execution within the Almanac ecosystem.

#[cfg(feature = "valence")]
use almanac_chains::valence::{ValenceClientManager, ValenceExecutionRequest};
use tracing::info;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();
    
    info!("Starting Valence domain client integration example");
    
    #[cfg(feature = "valence")]
    {
        // Create Valence client manager
        let mut manager = ValenceClientManager::new();
        
        // In a real implementation, you would register actual domain clients:
        // 
        // #[cfg(feature = "valence-ethereum")]
        // {
        //     use almanac_chains::valence::ethereum::ValenceEthereumClient;
        //     let ethereum_client = ValenceEthereumClient::new(
        //         "https://eth-mainnet.alchemyapi.io/v2/YOUR-API-KEY",
        //         "ethereum".to_string()
        //     ).await?;
        //     manager.register_client("ethereum".to_string(), Box::new(ethereum_client));
        // }
        // 
        // #[cfg(feature = "valence-cosmos")]
        // {
        //     use almanac_chains::valence::cosmos::ValenceCosmosClient;
        //     let cosmos_client = ValenceCosmosClient::new(
        //         "https://rpc-cosmoshub.keplr.app",
        //         "cosmoshub-4".to_string()
        //     ).await?;
        //     manager.register_client("cosmoshub-4".to_string(), Box::new(cosmos_client));
        // }
        //
        // #[cfg(feature = "valence-solana")]
        // {
        //     use almanac_chains::valence::solana::ValenceSolanaClient;
        //     let solana_client = ValenceSolanaClient::new(
        //         "https://api.mainnet-beta.solana.com",
        //         "solana".to_string()
        //     ).await?;
        //     manager.register_client("solana".to_string(), Box::new(solana_client));
        // }
        
        // Demo: Create a cross-chain execution request
        let execution_request = ValenceExecutionRequest {
            target_chain: "ethereum".to_string(),
            contract_address: "0x742d35Cc6634C0532925a3b8D80bE5e4eb16F8d6ef".to_string(),
            function_name: "transfer".to_string(),
            parameters: {
                let mut params = HashMap::new();
                params.insert("to".to_string(), serde_json::json!("0x1234567890123456789012345678901234567890"));
                params.insert("amount".to_string(), serde_json::json!("1000000000000000000")); // 1 ETH in wei
                params
            },
            gas_limit: Some(21000),
            max_fee: Some(20000000000), // 20 gwei
        };
        
        info!("Created execution request for {}: {}", 
              execution_request.target_chain, 
              execution_request.function_name);
        
        // Show supported chains (would show actual chains if clients were registered)
        let supported_chains = manager.supported_chains();
        if supported_chains.is_empty() {
            info!("No Valence domain clients registered (this is expected in the example)");
            info!("To use actual clients, enable the valence features and provide proper RPC endpoints");
        } else {
            info!("Supported chains: {:?}", supported_chains);
        }
        
        // Example of how cross-chain execution would work:
        // match manager.execute_cross_chain(execution_request).await {
        //     Ok(result) => {
        //         info!("Cross-chain execution successful!");
        //         info!("Transaction hash: {}", result.tx_hash);
        //         info!("Status: {:?}", result.status);
        //         info!("Gas used: {}", result.gas_used);
        //     },
        //     Err(e) => {
        //         error!("Cross-chain execution failed: {}", e);
        //     }
        // }
        
        info!("Valence integration example completed");
    }
    
    #[cfg(not(feature = "valence"))]
    {
        info!("Valence feature not enabled. Enable with --features valence");
        info!("Available Valence features:");
        info!("  - valence: Enable all Valence domain clients");
        info!("  - valence-ethereum: Enable Ethereum domain client");
        info!("  - valence-cosmos: Enable Cosmos domain client");
        info!("  - valence-solana: Enable Solana domain client");
    }
    
    Ok(())
}

/// Example of integrating Valence with Almanac storage
#[cfg(all(feature = "valence", feature = "fdb"))]
pub async fn valence_storage_integration_example() -> anyhow::Result<()> {
    use almanac_storage::{StorageManager, FdbConfig};
    use almanac_chains::valence::ValenceExecutionStatus;
    
    info!("Valence + Almanac Storage integration example");
    
    // Create storage manager
    let storage_config = FdbConfig {
        namespace: "valence_example".to_string(),
        ..Default::default()
    };
    
    let storage_manager = StorageManager::new(storage_config).await?;
    let app_state = storage_manager.create_api_state();
    
    // Create Valence manager
    let valence_manager = ValenceClientManager::new();
    
    // In a real implementation, you would:
    // 1. Execute cross-chain operations via Valence
    // 2. Store execution results in Almanac storage
    // 3. Subscribe to cross-chain events
    // 4. Update Merkle trees with cross-chain state changes
    
    info!("Integration setup complete - ready for cross-chain operations");
    
    Ok(())
}

/// Example configuration for different environments
#[cfg(feature = "valence")]
pub mod config_examples {
    use super::*;
    
    /// Development environment configuration
    pub fn dev_config() -> HashMap<String, String> {
        let mut config = HashMap::new();
        
        config.insert("ethereum_rpc".to_string(), "http://localhost:8545".to_string());
        config.insert("cosmos_rpc".to_string(), "http://localhost:26657".to_string());
        config.insert("solana_rpc".to_string(), "http://localhost:8899".to_string());
        
        config
    }
    
    /// Testnet configuration
    pub fn testnet_config() -> HashMap<String, String> {
        let mut config = HashMap::new();
        
        config.insert("ethereum_rpc".to_string(), "https://goerli.infura.io/v3/YOUR-API-KEY".to_string());
        config.insert("cosmos_rpc".to_string(), "https://rpc.sentry-01.theta-testnet.polypore.xyz".to_string());
        config.insert("solana_rpc".to_string(), "https://api.testnet.solana.com".to_string());
        
        config
    }
    
    /// Mainnet configuration  
    pub fn mainnet_config() -> HashMap<String, String> {
        let mut config = HashMap::new();
        
        config.insert("ethereum_rpc".to_string(), "https://eth-mainnet.alchemyapi.io/v2/YOUR-API-KEY".to_string());
        config.insert("cosmos_rpc".to_string(), "https://rpc-cosmoshub.keplr.app".to_string());
        config.insert("solana_rpc".to_string(), "https://api.mainnet-beta.solana.com".to_string());
        
        config
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    #[cfg(feature = "valence")]
    fn test_valence_manager_creation() {
        let manager = ValenceClientManager::new();
        assert_eq!(manager.supported_chains().len(), 0);
    }
    
    #[test]
    #[cfg(feature = "valence")]
    fn test_execution_request_creation() {
        let request = ValenceExecutionRequest {
            target_chain: "ethereum".to_string(),
            contract_address: "0x742d35Cc6634C0532925a3b8D80bE5e4eb16F8d6ef".to_string(),
            function_name: "balanceOf".to_string(),
            parameters: HashMap::new(),
            gas_limit: Some(50000),
            max_fee: Some(10000000000),
        };
        
        assert_eq!(request.target_chain, "ethereum");
        assert_eq!(request.function_name, "balanceOf");
    }
}