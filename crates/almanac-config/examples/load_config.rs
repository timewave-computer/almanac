use almanac_config::{load_config, Config};
use std::env;
use std::path::PathBuf;

fn main() {
    println!("Almanac Configuration Loader Example");
    println!("=====================================");
    
    // Find the project root directory
    let project_root = find_project_root();
    println!("Project root: {}", project_root.display());
    
    // Test loading simulation configuration
    let simulation_config_path = project_root.join("config/simulation.toml");
    test_load_config("Simulation", &simulation_config_path);
    
    // Test loading test configuration
    let test_config_path = project_root.join("config/test.toml");
    test_load_config("Test", &test_config_path);
    
    // Test loading development configuration
    let dev_config_path = project_root.join("config/dev.toml");
    test_load_config("Development", &dev_config_path);
    
    // Demonstrate creating a default configuration
    println!("\nDefault Configuration:");
    println!("======================");
    let default_config = Config::default();
    display_config_summary(&default_config);
    
    // Demonstrate configuration validation
    println!("\nConfiguration Validation:");
    println!("=========================");
    validate_config_example(&default_config);
}

fn find_project_root() -> PathBuf {
    // Start from current directory and walk up to find Cargo.toml
    let mut current = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    
    loop {
        if current.join("Cargo.toml").exists() {
            // Check if this is the workspace root by looking for the config directory
            if current.join("config").exists() {
                return current;
            }
        }
        
        if let Some(parent) = current.parent() {
            current = parent.to_path_buf();
        } else {
            // Fallback: assume we're in the crate directory
            return env::current_dir()
                .unwrap_or_else(|_| PathBuf::from("."))
                .join("../..");
        }
    }
}

fn test_load_config(config_name: &str, path: &PathBuf) {
    println!("\nLoading {} Configuration:", config_name);
    println!("   Path: {}", path.display());
    
    match load_config(path) {
        Ok(config) => {
            println!("Successfully loaded {} configuration!", config_name);
            display_config_summary(&config);
        }
        Err(e) => {
            println!("Failed to load {} configuration: {}", config_name, e);
            println!("   This is normal if the file doesn't exist or has invalid format.");
        }
    }
}

fn display_config_summary(config: &Config) {
    println!("   Simulation:");
    println!("      Name: {}", config.simulation.name);
    println!("      Chains: {:?}", config.simulation.chains);
    
    println!("   Almanac:");
    println!("      Name: {}", config.almanac.name);
    println!("      API Port: {:?}", config.almanac.api_port);
    if let Some(log_file) = &config.almanac.api_log_file {
        println!("      API Log File: {}", log_file);
    }
    
    println!("   Storage:");
    println!("      Backend: {}", config.storage.backend);
    if let Some(fdb_config) = &config.storage.fdb {
        println!("      FDB Namespace: {}", fdb_config.namespace);
        if let Some(cluster_file) = &fdb_config.cluster_file {
            println!("      FDB Cluster File: {}", cluster_file);
        }
    }
    
    println!("   Chains:");
    if let Some(eth_chains) = &config.chains.ethereum {
        println!("      Ethereum chains: {}", eth_chains.len());
        for chain in eth_chains {
            println!("        - {}: {} (chain_id: {})", chain.name, chain.rpc_url, chain.chain_id);
        }
    }
    
    if let Some(cosmos_chains) = &config.chains.cosmos {
        println!("      Cosmos chains: {}", cosmos_chains.len());
        for chain in cosmos_chains {
            println!("        - {}: {} (chain_id: {})", chain.name, chain.rpc_url, chain.chain_id);
        }
    }
    
    if let Some(solana_chains) = &config.chains.solana {
        println!("      Solana chains: {}", solana_chains.len());
        for chain in solana_chains {
            println!("        - {}: {}", chain.name, chain.rpc_url);
        }
    }
    
    println!("   Services:");
    if let Some(fdb_service) = &config.services.foundationdb {
        println!("      FoundationDB: enabled={}", fdb_service.enabled);
    }
    if let Some(eth_service) = &config.services.ethereum {
        println!("      Ethereum: enabled={}, type={:?}, port={:?}", 
                 eth_service.enabled, eth_service.node_type, eth_service.rpc_port);
    }
    if let Some(cosmos_service) = &config.services.cosmos {
        println!("      Cosmos: enabled={}, type={:?}, port={:?}", 
                 cosmos_service.enabled, cosmos_service.node_type, cosmos_service.rpc_port);
    }
    if let Some(solana_service) = &config.services.solana {
        println!("      Solana: enabled={}, type={:?}, port={:?}", 
                 solana_service.enabled, solana_service.node_type, solana_service.rpc_port);
    }
}

fn validate_config_example(config: &Config) {
    println!("   Validating configuration structure...");
    
    // Basic validation checks
    if config.simulation.name.is_empty() {
        println!("   Warning: Simulation name is empty");
    } else {
        println!("   Simulation name is valid: {}", config.simulation.name);
    }
    
    if config.simulation.chains.is_empty() {
        println!("   Warning: No chains configured");
    } else {
        println!("   Chains configured: {:?}", config.simulation.chains);
    }
    
    if config.almanac.name.is_empty() {
        println!("   Warning: Almanac name is empty");
    } else {
        println!("   Almanac name is valid: {}", config.almanac.name);
    }
    
    match config.storage.backend.as_str() {
        "memory" | "foundationdb" | "postgres" => {
            println!("   Storage backend is valid: {}", config.storage.backend);
        }
        _ => {
            println!("   Warning: Unknown storage backend: {}", config.storage.backend);
        }
    }
    
    if config.storage.backend == "foundationdb" && config.storage.fdb.is_none() {
        println!("   Warning: FoundationDB backend selected but no FDB config provided");
    }
    
    println!("   Configuration validation complete!");
}