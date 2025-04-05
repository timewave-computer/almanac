/// Indexer main entry point
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use async_trait::async_trait;
use clap::{Parser, Subcommand};
use tokio::signal;

use indexer_api::ApiServer;
use indexer_core::event::Event;
use indexer_core::service::{BoxedEventService, EventService, EventServiceRegistry, EventSubscription};
use indexer_core::types::{ApiConfig, ChainId, EventFilter};
use indexer_ethereum::EthereumEventService;
use indexer_cosmos::CosmosEventService;
use indexer_storage::{rocks::RocksStorage, rocks::RocksConfig};
use indexer_storage::migrations::schema::{InMemorySchemaRegistry, ContractSchemaRegistry};
use indexer_storage::migrations::MigrationRegistry;

/// A wrapper around any event service that makes EventType = Box<dyn Event>
struct BoxedEventServiceWrapper<T: EventService + Send + Sync + 'static> {
    inner: Arc<T>,
}

impl<T: EventService + Send + Sync + 'static> BoxedEventServiceWrapper<T> {
    fn new(inner: Arc<T>) -> Self {
        Self { inner }
    }
}

/// Simple event subscription wrapper
struct BoxedEventSubscriptionWrapper {
    inner: Box<dyn EventSubscription>,
}

#[async_trait]
impl EventSubscription for BoxedEventSubscriptionWrapper {
    async fn next(&mut self) -> Option<Box<dyn Event>> {
        self.inner.next().await
    }

    async fn close(&mut self) -> indexer_common::Result<()> {
        self.inner.close().await
    }
}

#[async_trait]
impl<T: EventService + Send + Sync + 'static> EventService for BoxedEventServiceWrapper<T> {
    type EventType = Box<dyn Event>;

    fn chain_id(&self) -> &ChainId {
        self.inner.chain_id()
    }

    async fn get_events(&self, filters: Vec<EventFilter>) -> indexer_common::Result<Vec<Box<dyn Event>>> {
        self.inner.get_events(filters).await
    }
    
    async fn get_events_with_status(&self, filters: Vec<EventFilter>, status: indexer_common::BlockStatus) -> indexer_common::Result<Vec<Box<dyn Event>>> {
        self.inner.get_events_with_status(filters, status).await
    }

    async fn subscribe(&self) -> indexer_common::Result<Box<dyn EventSubscription>> {
        let sub = self.inner.subscribe().await?;
        Ok(Box::new(BoxedEventSubscriptionWrapper { inner: sub }))
    }

    async fn get_latest_block(&self) -> indexer_common::Result<u64> {
        self.inner.get_latest_block().await
    }
    
    async fn get_latest_block_with_status(&self, chain: &str, status: indexer_common::BlockStatus) -> indexer_common::Result<u64> {
        self.inner.get_latest_block_with_status(chain, status).await
    }
}

/// Simple in-memory registry for event services
struct SimpleRegistry {
    services: HashMap<String, BoxedEventService>,
}

impl SimpleRegistry {
    fn new() -> Self {
        Self {
            services: HashMap::new(),
        }
    }
}

impl EventServiceRegistry for SimpleRegistry {
    fn register_service(&mut self, chain_id: ChainId, service: BoxedEventService) {
        self.services.insert(chain_id.0, service);
    }

    fn get_service(&self, chain_id: &str) -> Option<BoxedEventService> {
        self.services.get(chain_id).cloned()
    }

    fn get_services(&self) -> Vec<BoxedEventService> {
        self.services.values().cloned().collect()
    }

    fn remove_service(&mut self, chain_id: &str) -> Option<BoxedEventService> {
        self.services.remove(chain_id)
    }
}

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
        Commands::Run { config: _, api_host, api_port, eth_rpc, cosmos_rpc } => {
            // Initialize storage
            let storage_config = RocksConfig {
                path: "./data/rocks".to_string(),
                create_if_missing: true,
                cache_size_mb: 128,
            };
            
            let storage = Arc::new(RocksStorage::new(storage_config)?);
            
            // Run migrations
            run_migrations().await?;
            
            // Initialize service registry
            let mut registry = SimpleRegistry::new();
            
            // Add Ethereum service if RPC URL is provided
            if let Some(eth_rpc) = eth_rpc {
                let eth_service = EthereumEventService::new("ethereum".into(), &eth_rpc).await?;
                let eth_chain_id = eth_service.chain_id().clone();
                let eth_service_arc = Arc::new(eth_service);
                
                // Clone for background task
                let eth_service_for_task = eth_service_arc.clone();
                let storage_clone = storage.clone();
                
                // Set up a background task to track block finality status
                tokio::spawn(async move {
                    let interval = Duration::from_secs(30);
                    loop {
                        tokio::time::sleep(interval).await;
                        
                        if let Err(e) = update_ethereum_finality_status(&eth_service_for_task, storage_clone.clone()).await {
                            tracing::error!("Failed to update Ethereum finality status: {}", e);
                        }
                    }
                });
                
                // Wrap into BoxedEventService with our custom wrapper
                let wrapper = BoxedEventServiceWrapper::new(eth_service_arc);
                let boxed_service: BoxedEventService = Arc::new(wrapper);
                
                registry.register_service(eth_chain_id, boxed_service);
            }
            
            // Add Cosmos service if RPC URL is provided
            if let Some(cosmos_rpc) = cosmos_rpc {
                let cosmos_service = CosmosEventService::new("cosmos".into(), &cosmos_rpc).await?;
                let cosmos_chain_id = cosmos_service.chain_id().clone();
                
                // Note: Cosmos doesn't need finality monitoring due to instant finality
                let cosmos_service_arc = Arc::new(cosmos_service);
                
                // Wrap into BoxedEventService with our custom wrapper
                let wrapper = BoxedEventServiceWrapper::new(cosmos_service_arc);
                let boxed_service: BoxedEventService = Arc::new(wrapper);
                
                registry.register_service(cosmos_chain_id, boxed_service);
            }
            
            // Choose first service for API (in a real app, you'd handle this better)
            let services = registry.get_services();
            
            if let Some(service) = services.first() {
                // Initialize schema registry
                let schema_registry = Arc::new(InMemorySchemaRegistry::new());
                
                // Initialize API server
                let api_config = ApiConfig {
                    host: api_host,
                    port: api_port,
                    enable_graphql: true,
                    enable_rest: true,
                    enable_websocket: true,
                    params: HashMap::new(), // Empty parameters
                };
                
                let api_server = ApiServer::from_config(&api_config, service.clone(), schema_registry.clone())
                    .expect("Failed to create API server");
                
                // Start API server
                tokio::spawn(async move {
                    if let Err(e) = api_server.start().await {
                        tracing::error!("API server error: {}", e);
                    }
                });
                
                // Set up indexing tasks for each service
                let mut indexing_handles = Vec::new();
                
                for service in services.iter() {
                    let service_clone = service.clone();
                    
                    let handle = tokio::spawn(async move {
                        // Subscribe to events
                        match service_clone.subscribe().await {
                            Ok(mut subscription) => {
                                // Process events
                                while let Some(event) = subscription.next().await {
                                    tracing::info!("Received event: {:?}", event);
                                    
                                    // In a real implementation, you'd store the event in storage
                                    // if let Err(e) = storage.store_event(event).await {
                                    //     tracing::error!("Failed to store event: {}", e);
                                    // }
                                }
                            }
                            Err(e) => {
                                tracing::error!("Failed to subscribe to events: {}", e);
                            }
                        };
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
            } else {
                tracing::error!("No event services configured. Please provide at least one RPC URL.");
                return Ok(());
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
    let registry = MigrationRegistry::new();
    
    // TODO: Register migrations
    // Example:
    // registry.register(Arc::new(MyMigration::new()));
    
    // Initialize registry
    // registry.initialize().await?;
    
    Ok(registry)
}

/// Run all pending migrations
async fn run_migrations() -> Result<()> {
    // Create registry and run migrations
    let _registry = create_migration_registry().await?;
    // registry.apply().await?;
    
    Ok(())
}

/// Update Ethereum finality status
async fn update_ethereum_finality_status(
    service: &Arc<EthereumEventService>,
    _storage: Arc<RocksStorage>
) -> Result<()> {
    // Get latest block
    let latest_block = service.get_latest_block().await?;
    
    // In a real implementation, you would:
    // 1. Get finality status for different block heights
    // 2. Update the storage with new block status
    // 3. Handle reorgs if needed
    
    tracing::info!("Updated Ethereum finality status, latest block: {}", latest_block);
    
    Ok(())
}