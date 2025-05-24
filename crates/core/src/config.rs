//! Configuration system for multi-chain indexing

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Main configuration for the almanac indexer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlmanacConfig {
    /// Configuration for EVM chains
    pub evm_chains: HashMap<String, EvmChainConfig>,
    /// Configuration for Cosmos chains  
    pub cosmos_chains: HashMap<String, CosmosChainConfig>,
    /// Global indexer settings
    pub indexer: IndexerConfig,
}

/// Configuration for EVM-compatible chains
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvmChainConfig {
    pub chain_id: String,
    pub name: String,
    pub rpc_url: String,
    pub network_id: u64,
    pub native_token: String,
    pub enabled: bool,
    /// Additional RPC endpoints for redundancy
    pub backup_rpc_urls: Vec<String>,
    /// Maximum gas price in gwei
    pub max_gas_price: Option<f64>,
    /// Block confirmation depth
    pub confirmation_depth: Option<u64>,
}

/// Configuration for Cosmos chains
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CosmosChainConfig {
    pub chain_id: String,
    pub name: String,
    pub grpc_url: String,
    pub prefix: String,
    pub denom: String,
    pub enabled: bool,
    /// Additional gRPC endpoints for redundancy
    pub backup_grpc_urls: Vec<String>,
    /// RPC endpoint for tendermint queries
    pub rpc_url: Option<String>,
    /// Gas price for transactions
    pub gas_price: Option<f64>,
    /// Gas adjustment factor
    pub gas_adjustment: Option<f64>,
}

/// Global indexer configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexerConfig {
    pub max_concurrent_chains: usize,
    pub global_batch_size: usize,
    pub enable_metrics: bool,
    /// Metrics server port
    pub metrics_port: Option<u16>,
    /// Enable health checks
    pub enable_health_checks: bool,
    /// Health check interval in seconds
    pub health_check_interval: u64,
}

impl Default for AlmanacConfig {
    fn default() -> Self {
        Self {
            evm_chains: HashMap::new(),
            cosmos_chains: HashMap::new(),
            indexer: IndexerConfig {
                max_concurrent_chains: 4,
                global_batch_size: 1000,
                enable_metrics: true,
                metrics_port: Some(9090),
                enable_health_checks: true,
                health_check_interval: 30,
            },
        }
    }
}

impl AlmanacConfig {
    /// Validate the configuration
    pub fn validate(&self) -> Result<(), ConfigError> {
        // Validate EVM chains
        for (chain_id, config) in &self.evm_chains {
            if chain_id != &config.chain_id {
                return Err(ConfigError::ChainIdMismatch {
                    key: chain_id.clone(),
                    config_chain_id: config.chain_id.clone(),
                });
            }
            
            if config.rpc_url.is_empty() {
                return Err(ConfigError::MissingRpcUrl {
                    chain: chain_id.clone(),
                });
            }
            
            if !config.rpc_url.starts_with("http://") && !config.rpc_url.starts_with("https://") {
                return Err(ConfigError::InvalidRpcUrl {
                    chain: chain_id.clone(),
                    url: config.rpc_url.clone(),
                });
            }
        }
        
        // Validate Cosmos chains
        for (chain_id, config) in &self.cosmos_chains {
            if chain_id != &config.chain_id {
                return Err(ConfigError::ChainIdMismatch {
                    key: chain_id.clone(),
                    config_chain_id: config.chain_id.clone(),
                });
            }
            
            if config.grpc_url.is_empty() {
                return Err(ConfigError::MissingGrpcUrl {
                    chain: chain_id.clone(),
                });
            }
            
            if config.prefix.is_empty() {
                return Err(ConfigError::MissingPrefix {
                    chain: chain_id.clone(),
                });
            }
        }
        
        // Validate indexer config
        if self.indexer.max_concurrent_chains == 0 {
            return Err(ConfigError::InvalidConcurrency);
        }
        
        if self.indexer.global_batch_size == 0 {
            return Err(ConfigError::InvalidBatchSize);
        }
        
        Ok(())
    }
    
    /// Get all enabled chains
    pub fn enabled_chains(&self) -> Vec<String> {
        let mut chains = Vec::new();
        
        chains.extend(
            self.evm_chains
                .iter()
                .filter(|(_, config)| config.enabled)
                .map(|(id, _)| id.clone())
        );
        
        chains.extend(
            self.cosmos_chains
                .iter()
                .filter(|(_, config)| config.enabled)
                .map(|(id, _)| id.clone())
        );
        
        chains
    }
}

/// Configuration errors
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("Chain ID mismatch: key '{key}' does not match config chain_id '{config_chain_id}'")]
    ChainIdMismatch {
        key: String,
        config_chain_id: String,
    },
    #[error("Missing RPC URL for chain '{chain}'")]
    MissingRpcUrl { chain: String },
    #[error("Invalid RPC URL for chain '{chain}': '{url}'")]
    InvalidRpcUrl { chain: String, url: String },
    #[error("Missing gRPC URL for chain '{chain}'")]
    MissingGrpcUrl { chain: String },
    #[error("Missing prefix for chain '{chain}'")]
    MissingPrefix { chain: String },
    #[error("Invalid concurrency setting: must be greater than 0")]
    InvalidConcurrency,
    #[error("Invalid batch size: must be greater than 0")]
    InvalidBatchSize,
}

/// Chain registry for managing supported networks
pub struct ChainRegistry {
    evm_presets: HashMap<String, EvmChainConfig>,
    cosmos_presets: HashMap<String, CosmosChainConfig>,
}

impl ChainRegistry {
    /// Create a new chain registry with default presets
    pub fn new() -> Self {
        let mut registry = Self {
            evm_presets: HashMap::new(),
            cosmos_presets: HashMap::new(),
        };
        
        registry.load_default_presets();
        registry
    }
    
    /// Load default chain presets
    fn load_default_presets(&mut self) {
        // EVM chain presets
        self.evm_presets.insert("ethereum".to_string(), EvmChainConfig {
            chain_id: "1".to_string(),
            name: "ethereum".to_string(),
            rpc_url: "https://eth.llamarpc.com".to_string(),
            network_id: 1,
            native_token: "ETH".to_string(),
            enabled: false,
            backup_rpc_urls: vec![
                "https://rpc.ankr.com/eth".to_string(),
                "https://eth-mainnet.public.blastapi.io".to_string(),
            ],
            max_gas_price: Some(200.0),
            confirmation_depth: Some(12),
        });
        
        self.evm_presets.insert("polygon".to_string(), EvmChainConfig {
            chain_id: "137".to_string(),
            name: "polygon".to_string(),
            rpc_url: "https://polygon.llamarpc.com".to_string(),
            network_id: 137,
            native_token: "MATIC".to_string(),
            enabled: false,
            backup_rpc_urls: vec![
                "https://rpc.ankr.com/polygon".to_string(),
                "https://polygon-mainnet.public.blastapi.io".to_string(),
            ],
            max_gas_price: Some(100.0),
            confirmation_depth: Some(50),
        });
        
        self.evm_presets.insert("base".to_string(), EvmChainConfig {
            chain_id: "8453".to_string(),
            name: "base".to_string(),
            rpc_url: "https://base.llamarpc.com".to_string(),
            network_id: 8453,
            native_token: "ETH".to_string(),
            enabled: false,
            backup_rpc_urls: vec![
                "https://mainnet.base.org".to_string(),
                "https://base-mainnet.public.blastapi.io".to_string(),
            ],
            max_gas_price: Some(50.0),
            confirmation_depth: Some(10),
        });
        
        // Cosmos chain presets
        self.cosmos_presets.insert("noble".to_string(), CosmosChainConfig {
            chain_id: "noble-1".to_string(),
            name: "noble".to_string(),
            grpc_url: "grpc.noble.strange.love:443".to_string(),
            prefix: "noble".to_string(),
            denom: "uusdc".to_string(),
            enabled: false,
            backup_grpc_urls: vec![
                "noble-grpc.polkachu.com:21590".to_string(),
            ],
            rpc_url: Some("https://rpc.noble.strange.love".to_string()),
            gas_price: Some(0.1),
            gas_adjustment: Some(1.5),
        });
        
        self.cosmos_presets.insert("osmosis".to_string(), CosmosChainConfig {
            chain_id: "osmosis-1".to_string(),
            name: "osmosis".to_string(),
            grpc_url: "grpc.osmosis.zone:9090".to_string(),
            prefix: "osmo".to_string(),
            denom: "uosmo".to_string(),
            enabled: false,
            backup_grpc_urls: vec![
                "osmosis-grpc.polkachu.com:12590".to_string(),
            ],
            rpc_url: Some("https://rpc.osmosis.zone".to_string()),
            gas_price: Some(0.025),
            gas_adjustment: Some(1.4),
        });
        
        self.cosmos_presets.insert("neutron".to_string(), CosmosChainConfig {
            chain_id: "neutron-1".to_string(),
            name: "neutron".to_string(),
            grpc_url: "grpc-kralum.neutron-1.neutron.org:443".to_string(),
            prefix: "neutron".to_string(),
            denom: "untrn".to_string(),
            enabled: false,
            backup_grpc_urls: vec![
                "neutron-grpc.polkachu.com:18090".to_string(),
            ],
            rpc_url: Some("https://rpc-kralum.neutron-1.neutron.org".to_string()),
            gas_price: Some(0.025),
            gas_adjustment: Some(1.4),
        });
    }
    
    /// Get an EVM chain preset
    pub fn get_evm_preset(&self, name: &str) -> Option<&EvmChainConfig> {
        self.evm_presets.get(name)
    }
    
    /// Get a Cosmos chain preset
    pub fn get_cosmos_preset(&self, name: &str) -> Option<&CosmosChainConfig> {
        self.cosmos_presets.get(name)
    }
    
    /// List all available EVM chain presets
    pub fn list_evm_presets(&self) -> Vec<&str> {
        self.evm_presets.keys().map(|s| s.as_str()).collect()
    }
    
    /// List all available Cosmos chain presets
    pub fn list_cosmos_presets(&self) -> Vec<&str> {
        self.cosmos_presets.keys().map(|s| s.as_str()).collect()
    }
    
    /// Create a new config with preset chains enabled
    pub fn create_config_with_presets(&self, evm_chains: &[&str], cosmos_chains: &[&str]) -> Result<AlmanacConfig, ConfigError> {
        let mut config = AlmanacConfig::default();
        
        // Add EVM chains
        for &chain_name in evm_chains {
            if let Some(preset) = self.get_evm_preset(chain_name) {
                let mut chain_config = preset.clone();
                chain_config.enabled = true;
                config.evm_chains.insert(chain_config.chain_id.clone(), chain_config);
            }
        }
        
        // Add Cosmos chains
        for &chain_name in cosmos_chains {
            if let Some(preset) = self.get_cosmos_preset(chain_name) {
                let mut chain_config = preset.clone();
                chain_config.enabled = true;
                config.cosmos_chains.insert(chain_config.chain_id.clone(), chain_config);
            }
        }
        
        config.validate()?;
        Ok(config)
    }
}

impl Default for ChainRegistry {
    fn default() -> Self {
        Self::new()
    }
} 