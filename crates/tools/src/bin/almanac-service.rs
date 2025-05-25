/// Almanac Service Management CLI
use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, Subcommand};

use indexer_tools::service::{ServiceManager, ServiceCommand, execute_service_command, load_service_config, generate_default_service_config};
use indexer_tools::config::{ConfigManager, Environment};

#[derive(Parser)]
#[command(name = "almanac-service")]
#[command(author, version, about = "Almanac service management CLI", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Start a service
    Start {
        /// Service name to start
        service: String,
        
        /// Service configuration file
        #[arg(short, long)]
        config: Option<PathBuf>,
    },
    
    /// Stop a service
    Stop {
        /// Service name to stop
        service: String,
    },
    
    /// Restart a service
    Restart {
        /// Service name to restart
        service: String,
    },
    
    /// Show service status
    Status {
        /// Service name (shows all if not specified)
        service: Option<String>,
        
        /// Show detailed health information
        #[arg(long)]
        detailed: bool,
    },
    
    /// List all registered services
    List,
    
    /// Register a service from configuration file
    Register {
        /// Service configuration file
        config_file: PathBuf,
    },
    
    /// Generate a default service configuration file
    Generate {
        /// Output file path
        output: PathBuf,
        
        /// Service name
        #[arg(short, long, default_value = "almanac")]
        name: String,
    },
    
    /// Manage configuration files
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },
}

#[derive(Subcommand)]
enum ConfigAction {
    /// Validate configuration file
    Validate {
        /// Configuration file to validate
        file: PathBuf,
    },
    
    /// Generate default configuration
    Generate {
        /// Output file path
        output: PathBuf,
        
        /// Environment (development, staging, production, test)
        #[arg(short, long, default_value = "development")]
        environment: String,
    },
    
    /// Show current configuration
    Show {
        /// Configuration file
        #[arg(short, long, default_value = "almanac.toml")]
        file: PathBuf,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();
    
    // Parse command line arguments
    let cli = Cli::parse();

    match cli.command {
        Commands::Start { service, config } => {
            handle_service_start(service, config).await?;
        }
        
        Commands::Stop { service } => {
            handle_service_stop(service).await?;
        }
        
        Commands::Restart { service } => {
            handle_service_restart(service).await?;
        }
        
        Commands::Status { service, detailed } => {
            handle_service_status(service, detailed).await?;
        }
        
        Commands::List => {
            handle_service_list().await?;
        }
        
        Commands::Register { config_file } => {
            handle_service_register(config_file).await?;
        }
        
        Commands::Generate { output, name: _ } => {
            handle_service_generate(output).await?;
        }
        
        Commands::Config { action } => {
            handle_config_command(action).await?;
        }
    }

    Ok(())
}

async fn handle_service_start(service: String, config: Option<PathBuf>) -> Result<()> {
    let manager = ServiceManager::new();
    
    // If a config file is provided, register the service first
    if let Some(config_file) = config {
        let service_config = load_service_config(&config_file)?;
        manager.register_service(service_config)?;
        println!("✅ Service registered from config file: {}", config_file.display());
    }
    
    // Convert to ServiceCommand and execute
    let cmd = ServiceCommand::Start { service };
    execute_service_command(&manager, cmd).await?;
    
    Ok(())
}

async fn handle_service_stop(service: String) -> Result<()> {
    let manager = ServiceManager::new();
    let cmd = ServiceCommand::Stop { service };
    execute_service_command(&manager, cmd).await?;
    
    Ok(())
}

async fn handle_service_restart(service: String) -> Result<()> {
    let manager = ServiceManager::new();
    let cmd = ServiceCommand::Restart { service };
    execute_service_command(&manager, cmd).await?;
    
    Ok(())
}

async fn handle_service_status(service: Option<String>, detailed: bool) -> Result<()> {
    let manager = ServiceManager::new();
    let cmd = ServiceCommand::Status { service: service.clone() };
    execute_service_command(&manager, cmd).await?;
    
    if detailed {
        // Show additional health monitoring information
        let services = if let Some(service_name) = &service {
            vec![manager.get_service_status(service_name)?]
        } else {
            manager.list_services()
        };
        
        for service_info in services {
            println!("\n🔍 Detailed Health Information for '{}':", service_info.config.name);
            if let Some(health_check) = &service_info.config.health_check {
                println!("  Health Check Command: {:?}", health_check.command);
                println!("  Health Check Interval: {:?}", health_check.interval);
                println!("  Health Check Timeout: {:?}", health_check.timeout);
                println!("  Health Check Retries: {}", health_check.retries);
                println!("  Health Check Start Period: {:?}", health_check.start_period);
            }
            
            if let Some(last_check) = service_info.last_health_check {
                println!("  Last Health Check: {:?}", last_check);
            }
            
            println!("  Dependencies: {}", if service_info.config.dependencies.is_empty() {
                "None".to_string()
            } else {
                service_info.config.dependencies.iter()
                    .map(|d| format!("{} (required: {})", d.name, d.required))
                    .collect::<Vec<_>>()
                    .join(", ")
            });
            
            println!("  Auto-recovery: {}", if service_info.config.auto_recovery { "enabled" } else { "disabled" });
            println!("  Restart Policy: {:?}", service_info.config.restart_policy);
            if let Some(max_restarts) = service_info.config.max_restarts {
                println!("  Max Restarts: {}", max_restarts);
            }
        }
    }
    
    Ok(())
}

async fn handle_service_list() -> Result<()> {
    let manager = ServiceManager::new();
    let cmd = ServiceCommand::List;
    execute_service_command(&manager, cmd).await?;
    
    Ok(())
}

async fn handle_service_register(config_file: PathBuf) -> Result<()> {
    let manager = ServiceManager::new();
    let cmd = ServiceCommand::Register { config_file };
    execute_service_command(&manager, cmd).await?;
    
    Ok(())
}

async fn handle_service_generate(output: PathBuf) -> Result<()> {
    generate_default_service_config(&output)?;
    println!("✅ Generated default service configuration: {}", output.display());
    println!("💡 Edit the file to customize the service configuration");
    println!("💡 Use 'almanac-service register --config-file {}' to register the service", output.display());
    
    Ok(())
}

async fn handle_config_command(action: ConfigAction) -> Result<()> {
    match action {
        ConfigAction::Validate { file } => {
            println!("🔍 Validating configuration file: {}", file.display());
            
            match ConfigManager::load_from_file(&file) {
                Ok(manager) => {
                    match manager.validate() {
                        Ok(()) => {
                            println!("✅ Configuration file is valid");
                            println!("📊 Configuration Summary:");
                            let config = manager.config();
                            println!("  Environment: {}", config.environment);
                            println!("  Database Type: {}", config.database.db_type);
                            println!("  API Host: {}:{}", config.api.host, config.api.port);
                            println!("  Chains Configured: {}", config.chains.len());
                            for (name, _) in &config.chains {
                                println!("    - {}", name);
                            }
                            println!("  Logging Level: {}", config.logging.level);
                            println!("  Metrics Enabled: {}", config.monitoring.metrics_enabled);
                        }
                        Err(errors) => {
                            println!("❌ Configuration validation failed:");
                            for error in errors {
                                println!("  - {}", error);
                            }
                            std::process::exit(1);
                        }
                    }
                }
                Err(e) => {
                    println!("❌ Failed to load configuration file: {}", e);
                    std::process::exit(1);
                }
            }
        }
        
        ConfigAction::Generate { output, environment } => {
            let env = match environment.to_lowercase().as_str() {
                "development" | "dev" => Environment::Development,
                "staging" => Environment::Staging,
                "production" | "prod" => Environment::Production,
                "test" => Environment::Test,
                _ => {
                    println!("❌ Invalid environment '{}'. Valid options: development, staging, production, test", environment);
                    std::process::exit(1);
                }
            };
            
            ConfigManager::generate_default_config(&output, env)?;
            println!("✅ Generated default configuration for '{}' environment: {}", environment, output.display());
            println!("💡 Edit the file to customize your configuration");
            println!("💡 Use 'almanac-service config validate --file {}' to validate your changes", output.display());
        }
        
        ConfigAction::Show { file } => {
            println!("📋 Current Configuration from: {}", file.display());
            
            match ConfigManager::load_from_file(&file) {
                Ok(manager) => {
                    let config = manager.config();
                    
                    // Use serde_json for pretty printing
                    match serde_json::to_string_pretty(config) {
                        Ok(json_str) => {
                            println!("{}", json_str);
                        }
                        Err(e) => {
                            println!("❌ Failed to serialize configuration: {}", e);
                        }
                    }
                }
                Err(e) => {
                    println!("❌ Failed to load configuration file: {}", e);
                    println!("💡 Use 'almanac-service config generate --output {}' to create a default configuration", file.display());
                }
            }
        }
    }
    
    Ok(())
} 