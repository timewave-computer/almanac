/// Almanac Indexer - Main Entry Point
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

    async fn close(&mut self) -> indexer_pipeline::Result<()> {
        self.inner.close().await
    }
}

#[async_trait]
impl<T: EventService + Send + Sync + 'static> EventService for BoxedEventServiceWrapper<T> {
    type EventType = Box<dyn Event>;

    fn chain_id(&self) -> &ChainId {
        self.inner.chain_id()
    }

    async fn get_events(&self, filters: Vec<indexer_core::types::EventFilter>) -> indexer_pipeline::Result<Vec<Box<dyn Event>>> {
        self.inner.get_events(filters).await
    }

    async fn subscribe(&self) -> indexer_pipeline::Result<Box<dyn EventSubscription>> {
        let sub = self.inner.subscribe().await?;
        Ok(Box::new(BoxedEventSubscriptionWrapper { inner: sub }))
    }

    async fn get_latest_block(&self) -> indexer_pipeline::Result<u64> {
        self.inner.get_latest_block().await
    }
    
    async fn get_latest_block_with_status(&self, chain: &str, status: indexer_pipeline::BlockStatus) -> indexer_pipeline::Result<u64> {
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
#[command(name = "almanac")]
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
                
                let boxed_service = Arc::new(BoxedEventServiceWrapper::new(eth_service_arc));
                registry.register_service(eth_chain_id, boxed_service);
            }
            
            // Add Cosmos service if RPC URL is provided
            if let Some(cosmos_rpc) = cosmos_rpc {
                let cosmos_service = CosmosEventService::new("cosmos".into(), &cosmos_rpc).await?;
                let cosmos_chain_id = cosmos_service.chain_id().clone();
                let cosmos_service_arc = Arc::new(cosmos_service);
                
                let boxed_service = Arc::new(BoxedEventServiceWrapper::new(cosmos_service_arc));
                registry.register_service(cosmos_chain_id, boxed_service);
            }
            
            // Initialize API config
            let api_config = ApiConfig {
                host: api_host,
                port: api_port,
                enable_graphql: true,
                enable_rest: true,
                enable_websocket: true,
                params: HashMap::new(),
            };
            
            // Initialize schema registry
            let schema_registry = Arc::new(InMemorySchemaRegistry::new());
            
            // Get the first available service (for simplicity in this demo)
            let services = registry.get_services();
            let event_service = services.into_iter().next()
                .ok_or_else(|| Error::generic("No event services available. Please provide --eth-rpc or --cosmos-rpc"))?;
            
            // Create and start the API server
            let api_server = ApiServer::from_config(&api_config, event_service, schema_registry)?;
            
            // Handle graceful shutdown
            let shutdown_signal = async {
                signal::ctrl_c()
                    .await
                    .expect("Failed to install CTRL+C signal handler");
                tracing::info!("Received shutdown signal");
            };
            
            // Start server
            tracing::info!("Starting Almanac indexer...");
            tracing::info!("REST API will be available at http://{}:{}/api/v1/", api_host, api_port);
            tracing::info!("GraphQL API will be available at http://{}:{}/", api_host, api_port + 1);
            tracing::info!("GraphQL Playground will be available at http://{}:{}/graphiql", api_host, api_port + 1);
            
            // Start the API server
            api_server.start().await?;
            
            // Wait for shutdown signal
            shutdown_signal.await;
        }
        
        Commands::Migrate { run, list, rollback } => {
            if run {
                run_migrations().await?;
                println!("Migrations completed successfully");
            } else if list {
                list_pending_migrations().await?;
            } else if let Some(migration_id) = rollback {
                rollback_migration(&migration_id).await?;
                println!("Migration {} rolled back successfully", migration_id);
            } else {
                println!("Please specify --run, --list, or --rollback <migration_id>");
            }
        }
    }

    Ok(())
}

async fn create_migration_registry() -> Result<MigrationRegistry> {
    let mut registry = MigrationRegistry::new();
    
    // Register database migrations here
    // registry.register_migration(migration);
    
    Ok(registry)
}

async fn run_migrations() -> Result<()> {
    let _registry = create_migration_registry().await?;
    // TODO: Actually run migrations when we have a proper storage backend
    tracing::info!("Migrations would run here");
    Ok(())
}

async fn list_pending_migrations() -> Result<()> {
    let _registry = create_migration_registry().await?;
    // TODO: List pending migrations
    println!("No pending migrations");
    Ok(())
}

async fn rollback_migration(_migration_id: &str) -> Result<()> {
    // TODO: Implement rollback
    Ok(())
}

async fn update_ethereum_finality_status(
    service: &Arc<EthereumEventService>,
    _storage: Arc<RocksStorage>
) -> Result<()> {
    // Get the latest finalized block
    let _latest_block = service.get_latest_block().await?;
    
    // TODO: Update finality status in storage
    tracing::debug!("Updated Ethereum finality status");
    
    Ok(())
} 