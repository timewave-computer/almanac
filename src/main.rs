/// Indexer main entry point
use std::sync::Arc;

use anyhow::Result;
use clap::{Parser, Subcommand};
use tokio::signal;

use indexer_api::{ApiConfig, ApiServer};
use indexer_core::event::{EventService, EventSubscription};
use indexer_ethereum::EthereumEventService;
use indexer_cosmos::CosmosEventService;
use indexer_storage::{rocks::RocksStorage, rocks::RocksConfig};
use indexer_storage::migrations::MigrationRegistry;

#[derive(Parser)]
#[command(name = "indexer")]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run the indexer service
    Run {
        /// Config file path
        #[arg(short, long, default_value = "config.toml")]
        config: String,
        
        /// API host
        #[arg(long, default_value = "127.0.0.1")]
        api_host: String,
        
        /// API port
        #[arg(long, default_value_t = 8080)]
        api_port: u16,
        
        /// Ethereum RPC URL
        #[arg(long)]
        eth_rpc: Option<String>,
        
        /// Cosmos RPC URL
        #[arg(long)]
        cosmos_rpc: Option<String>,
    },
    
    /// Manage database migrations
    Migrate {
        /// Run database migrations
        #[arg(long)]
        run: bool,
        
        /// List pending migrations
        #[arg(long)]
        list: bool,
        
        /// Rollback a specific migration
        #[arg(long)]
        rollback: Option<String>,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();
    
    // Parse command line arguments
    let cli = Cli::parse();

    match cli.command {
        Commands::Run { config, api_host, api_port, eth_rpc, cosmos_rpc } => {
            // Initialize storage
            let storage_config = RocksConfig {
                path: "./data/rocks".to_string(),
                create_if_missing: true,
            };
            
            let storage = Arc::new(RocksStorage::new(storage_config)?);
            
            // Run migrations
            run_migrations().await?;
            
            // Initialize event services
            let mut event_services: Vec<Arc<dyn EventService>> = Vec::new();
            
            // Add Ethereum service if RPC URL is provided
            if let Some(eth_rpc) = eth_rpc {
                let eth_service = EthereumEventService::new("ethereum", &eth_rpc).await?;
                event_services.push(Arc::new(eth_service));
            }
            
            // Add Cosmos service if RPC URL is provided
            if let Some(cosmos_rpc) = cosmos_rpc {
                let cosmos_service = CosmosEventService::new("cosmos", &cosmos_rpc).await?;
                event_services.push(Arc::new(cosmos_service));
            }
            
            // Initialize API server
            let api_config = ApiConfig {
                host: api_host,
                port: api_port,
                enable_graphql: true,
                enable_http: true,
            };
            
            let api_server = ApiServer::new(
                api_config,
                event_services.clone(),
                storage.clone(),
            );
            
            // Start API server
            let api_handle = tokio::spawn(async move {
                if let Err(e) = api_server.start().await {
                    tracing::error!("API server error: {}", e);
                }
            });
            
            // Set up indexing tasks for each service
            let mut indexing_handles = Vec::new();
            
            for service in event_services.iter() {
                let storage_clone = storage.clone();
                let service_clone = service.clone();
                
                let handle = tokio::spawn(async move {
                    // Subscribe to events
                    let mut subscription = match service_clone.subscribe().await {
                        Ok(sub) => sub,
                        Err(e) => {
                            tracing::error!("Failed to subscribe to events: {}", e);
                            return;
                        }
                    };
                    
                    // Process events
                    loop {
                        match subscription.next().await {
                            Ok(event) => {
                                // Store event
                                if let Err(e) = storage_clone.store_event(event).await {
                                    tracing::error!("Failed to store event: {}", e);
                                }
                            }
                            Err(e) => {
                                tracing::error!("Failed to get next event: {}", e);
                                break;
                            }
                        }
                    }
                });
                
                indexing_handles.push(handle);
            }
            
            // Wait for shutdown signal
            match signal::ctrl_c().await {
                Ok(()) => {
                    tracing::info!("Shutting down gracefully");
                }
                Err(err) => {
                    tracing::error!("Unable to listen for shutdown signal: {}", err);
                }
            }
        },
        
        Commands::Migrate { run, list, rollback } => {
            let registry = create_migration_registry().await?;
            
            if list {
                // List pending migrations
                let pending = registry.get_pending();
                
                if pending.is_empty() {
                    println!("No pending migrations.");
                } else {
                    println!("Pending migrations:");
                    for meta in pending {
                        println!("  {} - {}", meta.id, meta.description);
                    }
                }
            } else if run {
                // Apply migrations
                println!("Running migrations...");
                registry.apply().await?;
                println!("Migrations completed successfully!");
            } else if let Some(id) = rollback {
                // Rollback a specific migration
                println!("Rolling back migration {}...", id);
                registry.rollback(&id).await?;
                println!("Rollback completed successfully!");
            } else {
                // Show help
                println!("Use --list to view pending migrations");
                println!("Use --run to apply pending migrations");
                println!("Use --rollback <id> to rollback a specific migration");
            }
        }
    }
    
    Ok(())
}

/// Create the migration registry with all migrations
async fn create_migration_registry() -> Result<MigrationRegistry> {
    // Create migration registry
    let mut registry = MigrationRegistry::new();
    
    // TODO: Register migrations
    // Example:
    // registry.register(Arc::new(MyMigration::new()));
    
    // Initialize registry
    registry.initialize().await?;
    
    Ok(registry)
}

/// Run all pending migrations
async fn run_migrations() -> Result<()> {
    // Create registry and run migrations
    let registry = create_migration_registry().await?;
    registry.apply().await?;
    
    Ok(())
}