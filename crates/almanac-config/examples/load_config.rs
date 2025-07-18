use almanac_config::load_config;

fn main() {
    println!("Testing configuration loading...");
    
    // Test loading simulation configuration
    match load_config("../../config/simulation.toml") {
        Ok(config) => {
            println!("✅ Successfully loaded simulation configuration!");
            println!("   Simulation name: {}", config.simulation.name);
            println!("   Almanac name: {}", config.almanac.name);
            println!("   Storage backend: {}", config.storage.backend);
            
            if let Some(eth_chains) = &config.chains.ethereum {
                println!("   Ethereum chains: {}", eth_chains.len());
                for chain in eth_chains {
                    println!("     - {}: {}", chain.name, chain.rpc_url);
                }
            }
        }
        Err(e) => {
            println!("❌ Failed to load simulation configuration: {}", e);
        }
    }
    
    // Test loading test configuration  
    match load_config("../../config/test.toml") {
        Ok(config) => {
            println!("✅ Successfully loaded test configuration!");
            println!("   Simulation name: {}", config.simulation.name);
            println!("   API port: {:?}", config.almanac.api_port);
        }
        Err(e) => {
            println!("❌ Failed to load test configuration: {}", e);
        }
    }
}