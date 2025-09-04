//! Configuration management for Almanac
//!
//! This crate provides configuration loading and management functionality
//! for the Almanac indexing system.

use almanac_core::{Error, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Main configuration structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Simulation metadata
    pub simulation: SimulationConfig,
    /// Almanac configuration
    pub almanac: AlmanacConfig,
    /// Storage configuration
    pub storage: StorageConfig,
    /// Chain configurations
    pub chains: ChainConfigurations,
    /// Services configuration
    pub services: ServicesConfig,
}

/// Storage configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    /// Storage backend type
    pub backend: String,
    /// FoundationDB configuration
    pub fdb: Option<FdbConfig>,
}

/// FoundationDB configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FdbConfig {
    /// Cluster file path
    pub cluster_file: Option<String>,
    /// Namespace
    pub namespace: String,
}

/// Simulation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulationConfig {
    /// Simulation name
    pub name: String,
    /// Enabled chains
    pub chains: Vec<String>,
}

/// Almanac configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlmanacConfig {
    /// Almanac instance name
    pub name: String,
    /// API port
    pub api_port: Option<u16>,
    /// API log file
    pub api_log_file: Option<String>,
}

/// Services configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServicesConfig {
    /// FoundationDB service configuration
    pub foundationdb: Option<ServiceConfig>,
    /// Ethereum service configuration
    pub ethereum: Option<ServiceConfig>,
    /// Cosmos service configuration
    pub cosmos: Option<ServiceConfig>,
    /// Solana service configuration
    pub solana: Option<ServiceConfig>,
}

/// Individual service configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceConfig {
    /// Whether service is enabled
    pub enabled: bool,
    /// Node type
    pub node_type: Option<String>,
    /// RPC port
    pub rpc_port: Option<u16>,
    /// WebSocket port
    pub ws_port: Option<u16>,
    /// gRPC port
    pub grpc_port: Option<u16>,
    /// API port
    pub api_port: Option<u16>,
    /// Chain ID
    pub chain_id: Option<String>,
    /// Data directory
    pub data_dir: Option<String>,
    /// Log file
    pub log_file: Option<String>,
    /// Log directory
    pub log_dir: Option<String>,
    /// Cluster file (for FoundationDB)
    pub cluster_file: Option<String>,
}

/// Chain configurations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainConfigurations {
    pub ethereum: Option<Vec<EthereumChainConfig>>,
    pub cosmos: Option<Vec<CosmosChainConfig>>,
    pub solana: Option<Vec<SolanaChainConfig>>,
}

/// Historical state configuration for chain nodes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoricalStateConfig {
    /// Number of blocks to retain in the node's state
    pub blocks_to_keep: u64,
    /// Whether to enable state pruning
    pub enable_pruning: bool,
    /// Custom pruning interval (in blocks)
    pub pruning_interval: Option<u64>,
    /// Archive mode - keeps all historical state (overrides blocks_to_keep)
    pub archive_mode: Option<bool>,
}

/// Ethereum chain configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EthereumChainConfig {
    /// Chain name
    pub name: String,
    /// RPC URL
    pub rpc_url: String,
    /// WebSocket URL
    pub ws_url: Option<String>,
    /// Chain ID
    pub chain_id: u64,
    /// Starting block
    pub starting_block: Option<u64>,
    /// Batch size
    pub batch_size: Option<u64>,
    /// Historical state configuration
    pub historical_state: Option<HistoricalStateConfig>,
}

/// Cosmos chain configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CosmosChainConfig {
    /// Chain name
    pub name: String,
    /// RPC URL
    pub rpc_url: String,
    /// gRPC URL
    pub grpc_url: Option<String>,
    /// Chain ID
    pub chain_id: String,
    /// Starting block
    pub starting_block: Option<u64>,
    /// Batch size
    pub batch_size: Option<u64>,
    /// Historical state configuration
    pub historical_state: Option<HistoricalStateConfig>,
}

/// Solana chain configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SolanaChainConfig {
    /// Chain name
    pub name: String,
    /// RPC URL
    pub rpc_url: String,
    /// WebSocket URL
    pub ws_url: Option<String>,
    /// Starting slot
    pub starting_slot: Option<u64>,
    /// Batch size
    pub batch_size: Option<u64>,
    /// Historical state configuration
    pub historical_state: Option<HistoricalStateConfig>,
}

impl Default for HistoricalStateConfig {
    fn default() -> Self {
        Self {
            blocks_to_keep: 1000,    // Default: keep last 1000 blocks
            enable_pruning: true,    // Default: enable pruning
            pruning_interval: None,  // Use chain default
            archive_mode: None,      // Not archive mode by default
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            simulation: SimulationConfig {
                name: "almanac-default".to_string(),
                chains: vec!["ethereum".to_string()],
            },
            almanac: AlmanacConfig {
                name: "default".to_string(),
                api_port: Some(3000),
                api_log_file: Some("logs/almanac-api.log".to_string()),
            },
            storage: StorageConfig {
                backend: "memory".to_string(),
                fdb: None,
            },
            chains: ChainConfigurations {
                ethereum: None,
                cosmos: None,
                solana: None,
            },
            services: ServicesConfig {
                foundationdb: None,
                ethereum: None,
                cosmos: None,
                solana: None,
            },
        }
    }
}

/// Load configuration from a TOML file
pub fn load_config<P: AsRef<Path>>(path: P) -> Result<Config> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| Error::configuration(format!("Failed to read config file: {}", e)))?;

    toml::from_str(&content)
        .map_err(|e| Error::configuration(format!("Failed to parse config: {}", e)))
}