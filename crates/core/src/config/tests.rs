#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_almanac_config_default() {
        let config = AlmanacConfig::default();
        assert_eq!(config.indexer.max_concurrent_chains, 4);
        assert_eq!(config.indexer.global_batch_size, 1000);
        assert!(config.indexer.enable_metrics);
        assert!(config.evm_chains.is_empty());
        assert!(config.cosmos_chains.is_empty());
    }
    
    #[test]
    fn test_chain_registry_presets() {
        let registry = ChainRegistry::new();
        
        // Test EVM presets
        let evm_presets = registry.list_evm_presets();
        assert!(evm_presets.contains(&"ethereum"));
        assert!(evm_presets.contains(&"polygon"));
        assert!(evm_presets.contains(&"base"));
        
        // Test Cosmos presets
        let cosmos_presets = registry.list_cosmos_presets();
        assert!(cosmos_presets.contains(&"noble"));
        assert!(cosmos_presets.contains(&"osmosis"));
        assert!(cosmos_presets.contains(&"neutron"));
    }
    
    #[test]
    fn test_enabled_chains() {
        let mut config = AlmanacConfig::default();
        
        // Add EVM chain (enabled)
        config.evm_chains.insert("1".to_string(), EvmChainConfig {
            chain_id: "1".to_string(),
            name: "ethereum".to_string(),
            rpc_url: "https://test.rpc".to_string(),
            network_id: 1,
            native_token: "ETH".to_string(),
            enabled: true,
            backup_rpc_urls: vec![],
            max_gas_price: None,
            confirmation_depth: None,
        });
        
        // Add Cosmos chain (enabled)
        config.cosmos_chains.insert("noble-1".to_string(), CosmosChainConfig {
            chain_id: "noble-1".to_string(),
            name: "noble".to_string(),
            grpc_url: "grpc://test".to_string(),
            prefix: "noble".to_string(),
            denom: "uusdc".to_string(),
            enabled: true,
            backup_grpc_urls: vec![],
            rpc_url: None,
            gas_price: None,
            gas_adjustment: None,
        });
        
        let enabled = config.enabled_chains();
        assert_eq!(enabled.len(), 2);
        assert!(enabled.contains(&"1".to_string()));
        assert!(enabled.contains(&"noble-1".to_string()));
    }
} 