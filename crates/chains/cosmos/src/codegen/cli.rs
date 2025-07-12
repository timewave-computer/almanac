//! CLI interface for cosmos contract code generation

use super::{CosmosCodegenConfig, generate_contract_code};
use indexer_core::Result;
use clap::{Arg, ArgMatches, Command};

/// Build the CLI command for cosmos contract generation
pub fn build_generate_contract_command() -> Command {
    Command::new("generate-contract")
        .about("Generate code for interacting with a CosmWasm contract from its message schema")
        .arg(
            Arg::new("msg-file")
                .help("Path to the CosmWasm message schema JSON file (*_msg.json)")
                .required(true)
                .index(1)
        )
        .arg(
            Arg::new("address")
                .long("address")
                .help("Contract address on the chain")
                .required(true)
                .value_name("CONTRACT_ADDRESS")
        )
        .arg(
            Arg::new("chain")
                .long("chain")
                .help("Chain ID where the contract is deployed")
                .required(true)
                .value_name("CHAIN_ID")
        )
        .arg(
            Arg::new("output-dir")
                .long("output-dir")
                .help("Output directory for generated code")
                .default_value("./generated")
                .value_name("PATH")
        )
        .arg(
            Arg::new("namespace")
                .long("namespace")
                .help("Namespace for generated code")
                .value_name("NAMESPACE")
        )
        .arg(
            Arg::new("features")
                .long("features")
                .help("Comma-separated list of features to generate")
                .default_value("client,storage,api,migrations")
                .value_name("FEATURES")
        )
        .arg(
            Arg::new("dry-run")
                .long("dry-run")
                .help("Preview generated code without writing files")
                .action(clap::ArgAction::SetTrue)
        )
        .arg(
            Arg::new("verbose")
                .long("verbose")
                .short('v')
                .help("Enable verbose output")
                .action(clap::ArgAction::SetTrue)
        )
}

/// Handle the cosmos generate-contract command
pub async fn handle_generate_contract_command(matches: &ArgMatches) -> Result<()> {
    let msg_file = matches.get_one::<String>("msg-file")
        .ok_or_else(|| indexer_core::Error::Config("Message file path is required".to_string()))?;

    let contract_address = matches.get_one::<String>("address")
        .ok_or_else(|| indexer_core::Error::Config("Contract address is required".to_string()))?;

    let chain_id = matches.get_one::<String>("chain")
        .ok_or_else(|| indexer_core::Error::Config("Chain ID is required".to_string()))?;

    let output_dir = matches.get_one::<String>("output-dir")
        .map(|s| s.as_str())
        .unwrap_or("./generated");

    let namespace = matches.get_one::<String>("namespace").cloned();

    let features_str = matches.get_one::<String>("features")
        .map(|s| s.as_str())
        .unwrap_or("client,storage,api,migrations");
    
    let features = features_str
        .split(',')
        .map(|s| s.trim().to_string())
        .collect();

    let dry_run = matches.get_flag("dry-run");
    let verbose = matches.get_flag("verbose");

    // Validate contract address format
    validate_contract_address(contract_address)?;

    // Validate chain ID
    validate_chain_id(chain_id)?;

    // Validate message file exists
    validate_msg_file(msg_file).await?;

    let config = CosmosCodegenConfig {
        contract_address: contract_address.clone(),
        chain_id: chain_id.clone(),
        output_dir: output_dir.to_string(),
        namespace,
        features,
        dry_run,
    };

    if verbose {
        println!("Cosmos Contract Code Generation");
        println!("==============================");
        println!("Message file: {}", msg_file);
        println!("Contract address: {}", contract_address);
        println!("Chain ID: {}", chain_id);
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

    generate_contract_code(msg_file, config).await?;

    if !dry_run {
        println!("✅ Code generation completed successfully!");
        println!("📁 Generated files are located in: {}", output_dir);
    }

    Ok(())
}

/// Validate contract address format
fn validate_contract_address(address: &str) -> Result<()> {
    // Basic validation for cosmos contract address format
    if address.is_empty() {
        return Err(indexer_core::Error::Config("Contract address cannot be empty".to_string()));
    }

    // Cosmos contract addresses typically start with a chain prefix
    if !address.contains('1') {
        return Err(indexer_core::Error::Config(
            "Contract address should be a valid bech32 address".to_string()
        ));
    }

    Ok(())
}

/// Validate chain ID format
fn validate_chain_id(chain_id: &str) -> Result<()> {
    if chain_id.is_empty() {
        return Err(indexer_core::Error::Config("Chain ID cannot be empty".to_string()));
    }

    // Basic format validation
    if chain_id.len() < 3 {
        return Err(indexer_core::Error::Config("Chain ID is too short".to_string()));
    }

    Ok(())
}

/// Validate message file exists and is readable
async fn validate_msg_file(file_path: &str) -> Result<()> {
    if !tokio::fs::try_exists(file_path).await
        .map_err(|e| indexer_core::Error::Config(format!("Failed to check file existence: {}", e)))? 
    {
        return Err(indexer_core::Error::Config(format!("Message file not found: {}", file_path)));
    }

    // Try to read the file to ensure it's accessible
    let _content = tokio::fs::read_to_string(file_path).await
        .map_err(|e| indexer_core::Error::Config(format!("Failed to read message file: {}", e)))?;

    // Basic JSON validation
    let _: serde_json::Value = serde_json::from_str(&_content)
        .map_err(|e| indexer_core::Error::Config(format!("Invalid JSON in message file: {}", e)))?;

    Ok(())
}

/// Print help for available features
pub fn print_features_help() {
    println!("Available features:");
    println!("  client     - Generate client code for contract interactions");
    println!("  storage    - Generate storage models and database schemas");
    println!("  api        - Generate REST, GraphQL, and WebSocket APIs");
    println!("  migrations - Generate database migration files");
    println!();
    println!("Example: --features client,storage,api");
} 