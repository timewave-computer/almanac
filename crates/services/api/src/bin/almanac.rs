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

// Import codegen modules
use indexer_cosmos::codegen::{CosmosCodegenConfig, generate_contract_code as generate_cosmos_contract};
use indexer_ethereum::codegen::{EthereumCodegenConfig, generate_contract_code as generate_ethereum_contract};

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
    /// Cosmos-related commands
    Cosmos {
        #[command(subcommand)]
        command: CosmosCommands,
    },
    /// Ethereum-related commands
    Ethereum {
        #[command(subcommand)]
        command: EthereumCommands,
    },
}

#[derive(Subcommand)]
enum CosmosCommands {
    /// Generate code for interacting with a CosmWasm contract
    GenerateContract {
        /// Path to the CosmWasm message schema JSON file (*_msg.json)
        msg_file: String,
        /// Contract address on the chain
        #[arg(long)]
        address: String,
        /// Chain ID where the contract is deployed
        #[arg(long)]
        chain: String,
        /// Output directory for generated code
        #[arg(long, default_value = "./generated")]
        output_dir: String,
        /// Namespace for generated code
        #[arg(long)]
        namespace: Option<String>,
        /// Comma-separated list of features to generate
        #[arg(long, default_value = "client,storage,api,migrations")]
        features: String,
        /// Preview generated code without writing files
        #[arg(long)]
        dry_run: bool,
        /// Enable verbose output
        #[arg(long, short)]
        verbose: bool,
    },
}

#[derive(Subcommand)]
enum EthereumCommands {
    /// Generate code for interacting with an Ethereum contract
    GenerateContract {
        /// Path to the contract ABI JSON file
        abi_file: String,
        /// Contract address on the chain
        #[arg(long)]
        address: String,
        /// Chain ID where the contract is deployed
        #[arg(long)]
        chain: String,
        /// Output directory for generated code
        #[arg(long, default_value = "./generated")]
        output_dir: String,
        /// Namespace for generated code
        #[arg(long)]
        namespace: Option<String>,
        /// Comma-separated list of features to generate
        #[arg(long, default_value = "client,storage,api,migrations")]
        features: String,
        /// Preview generated code without writing files
        #[arg(long)]
        dry_run: bool,
        /// Enable verbose output
        #[arg(long, short)]
        verbose: bool,
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
        Commands::Cosmos { command } => {
            handle_cosmos_command(command).await?;
        }
        Commands::Ethereum { command } => {
            handle_ethereum_command(command).await?;
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

async fn handle_cosmos_command(command: CosmosCommands) -> Result<()> {
    match command {
        CosmosCommands::GenerateContract {
            msg_file,
            address,
            chain,
            output_dir,
            namespace,
            features,
            dry_run,
            verbose,
        } => {
            // Validate contract address format
            validate_cosmos_contract_address(&address)?;

            // Validate chain ID
            validate_cosmos_chain_id(&chain)?;

            // Validate message file exists
            validate_cosmos_msg_file(&msg_file).await?;

            let features = features
                .split(',')
                .map(|s| s.trim().to_string())
                .collect();

            let config = CosmosCodegenConfig {
                contract_address: address.clone(),
                chain_id: chain.clone(),
                output_dir: output_dir.clone(),
                namespace,
                features,
                dry_run,
            };

            if verbose {
                println!("Cosmos Contract Code Generation");
                println!("==============================");
                println!("Message file: {}", msg_file);
                println!("Contract address: {}", address);
                println!("Chain ID: {}", chain);
                println!("Output directory: {}", output_dir);
                if let Some(ref ns) = config.namespace {
                    println!("Namespace: {}", ns);
                }
                println!("Features: {}", config.features.join(", "));
                println!("Dry run: {}", dry_run);
                println!();
            }

            if dry_run {
                println!("🔍 Performing dry run - no files will be written");
            } else {
                println!("🚀 Generating cosmos contract code...");
            }

            generate_cosmos_contract(&msg_file, config).await?;

            if !dry_run {
                println!("✅ Code generation completed successfully!");
                println!("📁 Generated files are located in: {}", output_dir);
            }
        }
    }
    Ok(())
}

async fn handle_ethereum_command(command: EthereumCommands) -> Result<()> {
    match command {
        EthereumCommands::GenerateContract {
            abi_file,
            address,
            chain,
            output_dir,
            namespace,
            features,
            dry_run,
            verbose,
        } => {
            // Validate contract address format
            validate_ethereum_contract_address(&address)?;

            // Validate chain ID
            validate_ethereum_chain_id(&chain)?;

            // Validate ABI file exists
            validate_ethereum_abi_file(&abi_file).await?;

            let features = features
                .split(',')
                .map(|s| s.trim().to_string())
                .collect();

            let config = EthereumCodegenConfig {
                contract_address: address.clone(),
                chain_id: chain.clone(),
                output_dir: output_dir.clone(),
                namespace,
                features,
                dry_run,
            };

            if verbose {
                println!("Ethereum Contract Code Generation");
                println!("=================================");
                println!("ABI file: {}", abi_file);
                println!("Contract address: {}", address);
                println!("Chain ID: {}", chain);
                println!("Output directory: {}", output_dir);
                if let Some(ref ns) = config.namespace {
                    println!("Namespace: {}", ns);
                }
                println!("Features: {}", config.features.join(", "));
                println!("Dry run: {}", dry_run);
                println!();
            }

            if dry_run {
                println!("🔍 Performing dry run - no files will be written");
            } else {
                println!("🚀 Generating ethereum contract code...");
            }

            generate_ethereum_contract(&abi_file, config).await?;

            if !dry_run {
                println!("✅ Code generation completed successfully!");
                println!("📁 Generated files are located in: {}", output_dir);
            }
        }
    }
    Ok(())
}

/// Validate cosmos contract address format
fn validate_cosmos_contract_address(address: &str) -> Result<()> {
    if address.is_empty() {
        return Err(Error::generic("Contract address cannot be empty"));
    }

    // Cosmos contract addresses typically start with a chain prefix
    if !address.contains('1') {
        return Err(Error::generic(
            "Contract address should be a valid bech32 address"
        ));
    }

    Ok(())
}

/// Validate cosmos chain ID format
fn validate_cosmos_chain_id(chain_id: &str) -> Result<()> {
    if chain_id.is_empty() {
        return Err(Error::generic("Chain ID cannot be empty"));
    }

    // Basic format validation
    if chain_id.len() < 3 {
        return Err(Error::generic("Chain ID is too short"));
    }

    Ok(())
}

/// Validate cosmos message file exists and is readable
async fn validate_cosmos_msg_file(file_path: &str) -> Result<()> {
    if !tokio::fs::try_exists(file_path).await
        .map_err(|e| Error::generic(format!("Failed to check file existence: {}", e)))? 
    {
        return Err(Error::generic(format!("Message file not found: {}", file_path)));
    }

    // Try to read the file to ensure it's accessible
    let content = tokio::fs::read_to_string(file_path).await
        .map_err(|e| Error::generic(format!("Failed to read message file: {}", e)))?;

    // Basic JSON validation
    let _: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| Error::generic(format!("Invalid JSON in message file: {}", e)))?;

    Ok(())
}

/// Validate ethereum contract address format
fn validate_ethereum_contract_address(address: &str) -> Result<()> {
    if address.is_empty() {
        return Err(Error::generic("Contract address cannot be empty"));
    }

    // Ethereum addresses should start with 0x and be 42 characters long
    if !address.starts_with("0x") || address.len() != 42 {
        return Err(Error::generic(
            "Contract address should be a valid Ethereum address (0x followed by 40 hex characters)"
        ));
    }

    Ok(())
}

/// Validate ethereum chain ID format
fn validate_ethereum_chain_id(chain_id: &str) -> Result<()> {
    if chain_id.is_empty() {
        return Err(Error::generic("Chain ID cannot be empty"));
    }

    // Chain ID should be a valid number
    chain_id.parse::<u64>()
        .map_err(|_| Error::generic("Chain ID must be a valid number"))?;

    Ok(())
}

/// Validate ethereum ABI file exists and is readable
async fn validate_ethereum_abi_file(file_path: &str) -> Result<()> {
    if !tokio::fs::try_exists(file_path).await
        .map_err(|e| Error::generic(format!("Failed to check file existence: {}", e)))? 
    {
        return Err(Error::generic(format!("ABI file not found: {}", file_path)));
    }

    // Try to read the file to ensure it's accessible
    let content = tokio::fs::read_to_string(file_path).await
        .map_err(|e| Error::generic(format!("Failed to read ABI file: {}", e)))?;

    // Basic JSON validation
    let _: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| Error::generic(format!("Invalid JSON in ABI file: {}", e)))?;

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