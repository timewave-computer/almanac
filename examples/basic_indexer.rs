use std::sync::Arc;

use indexer_core::types::ChainId;
use indexer_api::{ApiConfig, ApiServer};
use indexer_storage::rocks::{RocksStorage, RocksConfig};
use indexer_ethereum::EthereumEventService;
use indexer_cosmos::CosmosEventService;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt::init();
    
    // Initialize storage
    let rocks_config = RocksConfig {
        path: "./data/rocks".to_string(),
        create_if_missing: true,
    };
    
    let storage = Arc::new(RocksStorage::new(rocks_config)?);
    
    // Initialize Ethereum service
    let eth_chain_id = ChainId::from("ethereum:mainnet");
    let eth_service = Arc::new(
        EthereumEventService::new(eth_chain_id, "https://mainnet.infura.io/v3/YOUR_INFURA_KEY")
            .await?
    );
    
    // Initialize Cosmos service
    let cosmos_chain_id = ChainId::from("cosmos:hub");
    let cosmos_service = Arc::new(
        CosmosEventService::new(cosmos_chain_id, "https://rpc.cosmos.network:26657")
            .await?
    );
    
    // Collect services
    let event_services = vec![eth_service, cosmos_service];
    
    // Initialize API
    let api_config = ApiConfig {
        host: "127.0.0.1".to_string(),
        port: 8080,
        enable_graphql: true,
        enable_http: true,
    };
    
    let api_server = ApiServer::new(api_config, event_services, storage);
    
    // Start API server
    api_server.start().await?;
    
    // Keep the application running
    tokio::signal::ctrl_c().await?;
    
    println!("Shutting down...");
    
    Ok(())
} 