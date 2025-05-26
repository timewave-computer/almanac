// Main CLI binary for Almanac cross-chain indexer
//
// This binary provides the primary command-line interface for running
// the Almanac indexer with various blockchain clients and storage backends

use clap::{Parser, Subcommand};
use std::sync::Arc;
use tracing::{info, error};
use tracing_subscriber::fmt;

// Import core types
use indexer_core::{Error, Result};
use indexer_storage::{create_postgres_storage, postgres::migrations::PostgresMigrationManager};

// Import blockchain clients with correct names
use indexer_ethereum::EthereumClient;

// Import service management from tools
use indexer_tools::service::ServiceManager;
use indexer_tools::config::{ConfigManager, Environment};

#[derive(Parser)]
#[command(name = "almanac")]
#[command(about = "Almanac cross-chain indexer")]
#[command(version = "0.1.0")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the indexer
    Start {
        /// Configuration file path
        #[arg(short, long, default_value = "config.toml")]
        config: String,
        
        /// Environment to run in
        #[arg(short, long, default_value = "development")]
        env: String,
    },
    /// Stop the indexer
    Stop,
    /// Check indexer status
    Status,
    /// Validate configuration
    ValidateConfig {
        /// Configuration file path
        #[arg(short, long, default_value = "config.toml")]
        config: String,
    },
    /// Initialize database
    InitDb {
        /// Database connection string
        #[arg(short, long)]
        database_url: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    fmt::init();
    
    // Parse command line arguments
    let cli = Cli::parse();

    match cli.command {
        Commands::Start { config, env } => {
            start_indexer(config, env).await?;
        }
        Commands::Stop => {
            stop_indexer().await?;
        }
        Commands::Status => {
            check_status().await?;
        }
        Commands::ValidateConfig { config } => {
            validate_config(config).await?;
        }
        Commands::InitDb { database_url } => {
            init_database(database_url).await?;
        }
    }

    Ok(())
}

async fn start_indexer(config_path: String, env_str: String) -> Result<()> {
    info!("Starting Almanac indexer with config: {}", config_path);
    
    // Parse environment
    let environment = match env_str.as_str() {
        "development" => Environment::Development,
        "staging" => Environment::Staging,
        "production" => Environment::Production,
        "test" => Environment::Test,
        _ => {
            error!("Invalid environment: {}. Use development, staging, production, or test", env_str);
            return Err(Error::generic("Invalid environment"));
        }
    };
    
    // Load configuration
    let config_manager = ConfigManager::load_for_environment(&config_path, environment)
        .map_err(|e| Error::generic(format!("Failed to load config: {}", e)))?;
    
    // Validate configuration
    config_manager.validate()
        .map_err(|e| Error::generic(format!("Invalid config: {:?}", e)))?;
    
    info!("Configuration loaded and validated successfully");
    
    let config = config_manager.config();
    
    // Initialize storage
    let _storage = if let Some(postgres_url) = &config.database.postgres_url {
        create_postgres_storage(postgres_url).await?
    } else {
        return Err(Error::generic("PostgreSQL URL not configured"));
    };
    info!("Storage initialized");
    
    // Initialize blockchain clients based on configuration
    let mut ethereum_clients = Vec::new();
    
    // Initialize Ethereum clients
    for (chain_name, chain_config) in &config.chains {
        // For now, assume all chains are Ethereum-compatible
        // In a real implementation, we'd need to determine chain type
        match EthereumClient::new(chain_config.chain_id.clone(), chain_config.rpc_url.clone()).await {
            Ok(client) => {
                info!("Initialized Ethereum client for chain: {} ({})", chain_name, chain_config.chain_id);
                ethereum_clients.push(Arc::new(client));
            }
            Err(e) => {
                error!("Failed to initialize Ethereum client for chain {}: {}", chain_name, e);
                return Err(Error::generic(format!("Failed to initialize Ethereum client: {}", e)));
            }
        }
    }
    
    info!("All blockchain clients initialized successfully");
    info!("Indexer started successfully");
    
    // Keep the process running
    tokio::signal::ctrl_c().await.map_err(|e| Error::generic(format!("Signal error: {}", e)))?;
    info!("Received shutdown signal, stopping indexer...");
    
    Ok(())
}

async fn stop_indexer() -> Result<()> {
    info!("Stopping Almanac indexer");
    
    // Create service manager
    let service_manager = ServiceManager::new();
    
    // Try to stop the almanac service
    match service_manager.stop_service("almanac").await {
        Ok(_) => info!("Indexer stopped successfully"),
        Err(e) => {
            error!("Failed to stop indexer: {}", e);
            return Err(Error::generic(format!("Failed to stop indexer: {}", e)));
        }
    }
    
    Ok(())
}

async fn check_status() -> Result<()> {
    info!("Checking Almanac indexer status");
    
    // Create service manager
    let service_manager = ServiceManager::new();
    
    // Check service status
    match service_manager.get_service_status("almanac") {
        Ok(status) => {
            println!("Indexer status: {:?}", status.status);
            if let Some(pid) = status.pid {
                println!("Process ID: {}", pid);
            }
            if let Some(uptime) = status.uptime() {
                println!("Uptime: {:?}", uptime);
            }
            println!("Healthy: {}", status.healthy);
            println!("Restart count: {}", status.restart_count);
        }
        Err(e) => {
            error!("Failed to check status: {}", e);
            return Err(Error::generic(format!("Failed to check status: {}", e)));
        }
    }
    
    Ok(())
}

async fn validate_config(config_path: String) -> Result<()> {
    info!("Validating configuration file: {}", config_path);
    
    // Try to load and validate config for all environments
    for env in [Environment::Development, Environment::Staging, Environment::Production, Environment::Test] {
        match ConfigManager::load_for_environment(&config_path, env.clone()) {
            Ok(config_manager) => {
                match config_manager.validate() {
                    Ok(_) => info!("Configuration valid for environment: {:?}", env),
                    Err(e) => {
                        error!("Configuration invalid for environment {:?}: {:?}", env, e);
                        return Err(Error::generic(format!("Invalid config for {:?}: {:?}", env, e)));
                    }
                }
            }
            Err(e) => {
                error!("Failed to load config for environment {:?}: {}", env, e);
                return Err(Error::generic(format!("Failed to load config for {:?}: {}", env, e)));
            }
        }
    }
    
    info!("Configuration validation completed successfully");
    Ok(())
}

async fn init_database(database_url: String) -> Result<()> {
    info!("Initializing database with URL: {}", mask_password(&database_url));
    
    // Try to create storage connection to test database
    match create_postgres_storage(&database_url).await {
        Ok(_) => {
            info!("Database connection successful");
            
            // Run migrations
            let migrations_path = "./crates/storage/migrations";
            let migration_manager = PostgresMigrationManager::new(&database_url, migrations_path);
            
            match migration_manager.migrate().await {
                Ok(_) => {
                    info!("✅ Database migrations completed successfully");
                }
                Err(e) => {
                    error!("❌ Migration failed: {}", e);
                    return Err(Error::generic(format!("Migration failed: {}", e)));
                }
            }
            
            info!("Database initialization completed");
        }
        Err(e) => {
            error!("Failed to connect to database: {}", e);
            return Err(Error::generic(format!("Database initialization failed: {}", e)));
        }
    }
    
    Ok(())
}

fn mask_password(url: &str) -> String {
    // Simple password masking for display
    if let Some(at_pos) = url.find('@') {
        if let Some(colon_pos) = url[..at_pos].rfind(':') {
            let mut masked = url.to_string();
            masked.replace_range(colon_pos + 1..at_pos, "****");
            return masked;
        }
    }
    url.to_string()
} 