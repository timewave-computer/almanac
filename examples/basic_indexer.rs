// Example of a basic cross-chain indexer
// Note: This is a simplified example and not meant for production use
// Important: The codebase currently has compilation issues with the storage implementations.
// This example uses MemoryStorage which avoids the problematic RocksDB and PostgreSQL implementations.

use std::sync::Arc;

use indexer_core::types::ChainId;
use indexer_api::{ApiServer, ApiConfig};
use indexer_storage::memory::MemoryStorage;
use indexer_ethereum::{EthereumEventService, EthereumEventServiceConfig};
use indexer_cosmos::{CosmosEventService, CosmosEventServiceConfig};

fn main() {
    println!("Note: This example cannot be run directly due to incomplete implementations in the codebase");
    println!("Please refer to the examples/README.md file for guidance on using the code examples");
    println!();
    println!("The code below shows how to create and configure an Almanac indexer");
    println!("when all the required implementations are available:");
    println!();
    println!("1. Create a storage implementation (MemoryStorage is recommended for testing)");
    println!("2. Configure and create chain-specific event services (e.g., Ethereum, Cosmos)");
    println!("3. Initialize the API server with the storage and event services");
    println!("4. Start the API server and handle shutdown gracefully");
    
    std::process::exit(0); // Exit with success code since this is expected behavior
}

// The code below shows how the implementation would work
// when the codebase is complete
#[allow(dead_code)]
async fn sample_implementation() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt::init();
    
    // Use memory storage for the example to avoid RocksDB/Postgres issues
    println!("Initializing MemoryStorage...");
    let storage = Arc::new(MemoryStorage::new());
    
    // Initialize Ethereum service with placeholder config
    // Replace YOUR_INFURA_KEY with a valid API key in production
    println!("Configuring Ethereum service...");
    let eth_config = EthereumEventServiceConfig {
        chain_id: "ethereum:mainnet".to_string(),
        rpc_url: "https://mainnet.infura.io/v3/YOUR_INFURA_KEY".to_string(),
        use_websocket: false,
        confirmation_blocks: 12,
        max_batch_size: 100,
        poll_interval_ms: 5000,
        max_parallel_requests: 5,
    };
    
    println!("Creating Ethereum service...");
    let eth_service = Arc::new(
        EthereumEventService::new(eth_config).await?
    );
    
    // Initialize Cosmos service with placeholder config
    println!("Configuring Cosmos service...");
    let cosmos_config = CosmosEventServiceConfig {
        chain_id: "cosmos:hub".to_string(),
        rpc_url: "https://rpc.cosmos.network:26657".to_string(),
        confirmation_blocks: 6,
        max_batch_size: 10,
        poll_interval_ms: 5000,
        max_parallel_requests: 3,
        valence_account_code_ids: Vec::new(),
        valence_processor_code_ids: Vec::new(),
        valence_authorization_code_ids: Vec::new(),
        valence_library_code_ids: Vec::new(),
    };
    
    println!("Creating Cosmos service...");
    let cosmos_service = Arc::new(
        CosmosEventService::new(cosmos_config, storage.clone()).await?
    );
    
    // Collect services
    let event_services = vec![eth_service, cosmos_service];
    
    // Initialize API with sensible defaults
    println!("Configuring API server...");
    let api_config = ApiConfig {
        host: "127.0.0.1".to_string(),
        port: 8080,
        enable_graphql: true,
        enable_http: true,
    };
    
    println!("Creating API server...");
    let api_server = ApiServer::new(api_config, event_services, storage);
    
    // Start API server
    println!("Starting API server...");
    api_server.start().await?;
    
    println!("Indexer API server running at http://127.0.0.1:8080");
    println!("Press Ctrl+C to shut down");
    
    // Keep the application running
    tokio::signal::ctrl_c().await?;
    
    println!("Shutting down...");
    
    Ok(())
} 