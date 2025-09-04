use almanac_config::{
    load_config, Config, CosmosChainConfig, EthereumChainConfig, FdbConfig, 
    HistoricalStateConfig, StorageConfig,
};
use std::io::Write;
use tempfile::NamedTempFile;

#[test]
fn test_config_default() {
    let config = Config::default();

    assert_eq!(config.simulation.name, "almanac-default");
    assert_eq!(config.almanac.name, "default");
    assert_eq!(config.almanac.api_port, Some(3000));
    assert_eq!(config.storage.backend, "memory");
    assert!(config.chains.ethereum.is_none());
    assert!(config.chains.cosmos.is_none());
    assert!(config.chains.solana.is_none());
}

#[test]
fn test_storage_config_creation() {
    let storage_config = StorageConfig {
        backend: "fdb".to_string(),
        fdb: Some(FdbConfig {
            cluster_file: Some("/etc/foundationdb/fdb.cluster".to_string()),
            namespace: "almanac".to_string(),
        }),
    };

    assert_eq!(storage_config.backend, "fdb");
    assert!(storage_config.fdb.is_some());
}

#[test]
fn test_fdb_config_creation() {
    let fdb_config = FdbConfig {
        cluster_file: Some("/etc/foundationdb/fdb.cluster".to_string()),
        namespace: "almanac".to_string(),
    };

    assert_eq!(
        fdb_config.cluster_file,
        Some("/etc/foundationdb/fdb.cluster".to_string())
    );
    assert_eq!(fdb_config.namespace, "almanac");
}

#[test]
fn test_ethereum_chain_config_creation() {
    let chain_config = EthereumChainConfig {
        name: "ethereum-mainnet".to_string(),
        rpc_url: "https://mainnet.infura.io/v3/YOUR_PROJECT_ID".to_string(),
        ws_url: Some("wss://mainnet.infura.io/ws/v3/YOUR_PROJECT_ID".to_string()),
        chain_id: 1,
        starting_block: Some(0),
        batch_size: Some(100),
        historical_state: None,
    };

    assert_eq!(chain_config.name, "ethereum-mainnet");
    assert_eq!(chain_config.chain_id, 1);
    assert_eq!(
        chain_config.rpc_url,
        "https://mainnet.infura.io/v3/YOUR_PROJECT_ID"
    );
}

#[test]
fn test_cosmos_chain_config_creation() {
    let chain_config = CosmosChainConfig {
        name: "cosmos-hub".to_string(),
        rpc_url: "https://rpc.cosmos.network".to_string(),
        grpc_url: Some("https://grpc.cosmos.network".to_string()),
        chain_id: "cosmoshub-4".to_string(),
        starting_block: Some(0),
        batch_size: Some(100),
        historical_state: None,
    };

    assert_eq!(chain_config.name, "cosmos-hub");
    assert_eq!(chain_config.chain_id, "cosmoshub-4");
    assert_eq!(chain_config.rpc_url, "https://rpc.cosmos.network");
}

#[test]
fn test_historical_state_config() {
    // Test default configuration
    let default_config = HistoricalStateConfig::default();
    assert_eq!(default_config.blocks_to_keep, 1000);
    assert_eq!(default_config.enable_pruning, true);
    assert_eq!(default_config.pruning_interval, None);
    assert_eq!(default_config.archive_mode, None);

    // Test custom configuration
    let custom_config = HistoricalStateConfig {
        blocks_to_keep: 5000,
        enable_pruning: false,
        pruning_interval: Some(100),
        archive_mode: Some(true),
    };
    assert_eq!(custom_config.blocks_to_keep, 5000);
    assert_eq!(custom_config.enable_pruning, false);
    assert_eq!(custom_config.pruning_interval, Some(100));
    assert_eq!(custom_config.archive_mode, Some(true));

    // Test with chain config
    let chain_config = EthereumChainConfig {
        name: "eth-archive".to_string(),
        rpc_url: "http://localhost:8545".to_string(),
        ws_url: None,
        chain_id: 1,
        starting_block: None,
        batch_size: None,
        historical_state: Some(HistoricalStateConfig {
            blocks_to_keep: 0,
            enable_pruning: false,
            pruning_interval: None,
            archive_mode: Some(true),
        }),
    };
    assert!(chain_config.historical_state.is_some());
    let hist_state = chain_config.historical_state.as_ref().unwrap();
    assert_eq!(hist_state.archive_mode, Some(true));
    assert_eq!(hist_state.enable_pruning, false);
}

#[test]
fn test_config_from_toml() {
    let toml_content = r#"
[simulation]
name = "test-simulation"
chains = ["ethereum", "cosmos"]

[almanac]
name = "test-almanac"
api_port = 3000
api_log_file = "/logs/api.log"

[storage]
backend = "foundationdb"

[storage.fdb]
cluster_file = "/custom/path/fdb.cluster"
namespace = "test"

[services.foundationdb]
enabled = true
cluster_file = "/custom/path/fdb.cluster"
data_dir = "/data/fdb"
log_dir = "/logs/fdb"

[services.ethereum]
enabled = true
node_type = "anvil"
rpc_port = 8545
ws_port = 8546
chain_id = "1337"
data_dir = "/data/ethereum"
log_file = "/logs/ethereum.log"

[[chains.ethereum]]
name = "ethereum-test"
rpc_url = "https://eth-mainnet.alchemyapi.io/v2/YOUR-API-KEY"
ws_url = "wss://eth-mainnet.alchemyapi.io/ws/v2/YOUR-API-KEY"
chain_id = 1
starting_block = 0
batch_size = 100

[[chains.cosmos]]
name = "cosmos-test"
rpc_url = "https://rpc.cosmos.network"
grpc_url = "https://grpc.cosmos.network"
chain_id = "cosmoshub-4"
starting_block = 0
batch_size = 50
"#;

    let mut temp_file = NamedTempFile::new().unwrap();
    writeln!(temp_file, "{}", toml_content).unwrap();

    let config = load_config(temp_file.path()).unwrap();

    assert_eq!(config.simulation.name, "test-simulation");
    assert_eq!(config.almanac.name, "test-almanac");
    assert_eq!(config.almanac.api_port, Some(3000));
    assert_eq!(config.storage.backend, "foundationdb");
    assert!(config.storage.fdb.is_some());
    let fdb_config = config.storage.fdb.as_ref().unwrap();
    assert_eq!(
        fdb_config.cluster_file,
        Some("/custom/path/fdb.cluster".to_string())
    );
    assert_eq!(fdb_config.namespace, "test");

    assert!(config.chains.ethereum.is_some());
    assert!(config.chains.cosmos.is_some());
    let ethereum_chains = config.chains.ethereum.as_ref().unwrap();
    assert_eq!(ethereum_chains.len(), 1);
    assert_eq!(ethereum_chains[0].name, "ethereum-test");
    assert_eq!(ethereum_chains[0].chain_id, 1);

    let cosmos_chains = config.chains.cosmos.as_ref().unwrap();
    assert_eq!(cosmos_chains.len(), 1);
    assert_eq!(cosmos_chains[0].name, "cosmos-test");
    assert_eq!(cosmos_chains[0].chain_id, "cosmoshub-4");
}

#[test]
fn test_config_partial_toml() {
    let toml_content = r#"
[simulation]
name = "partial-test"
chains = ["ethereum"]

[almanac]
name = "partial-almanac"
api_port = 9000

[storage]
backend = "memory"

[services.ethereum]
enabled = true
node_type = "anvil"
rpc_port = 8545

[[chains.ethereum]]
name = "ethereum-partial"
rpc_url = "https://rpc.ethereum.test"
chain_id = 1337
"#;

    let mut temp_file = NamedTempFile::new().unwrap();
    writeln!(temp_file, "{}", toml_content).unwrap();

    let config = load_config(temp_file.path()).unwrap();

    // Should use specified values
    assert_eq!(config.simulation.name, "partial-test");
    assert_eq!(config.almanac.api_port, Some(9000));
    assert_eq!(config.storage.backend, "memory");
    assert!(config.chains.ethereum.is_some());
    let ethereum_chains = config.chains.ethereum.as_ref().unwrap();
    assert_eq!(ethereum_chains.len(), 1);
    assert_eq!(ethereum_chains[0].name, "ethereum-partial");
}

#[test]
fn test_config_invalid_toml() {
    let invalid_toml = r#"
[simulation
name = "invalid"
"#;

    let mut temp_file = NamedTempFile::new().unwrap();
    writeln!(temp_file, "{}", invalid_toml).unwrap();

    let result = load_config(temp_file.path());
    assert!(result.is_err());
}

#[test]
fn test_config_nonexistent_file() {
    let result = load_config("/nonexistent/path/config.toml");
    assert!(result.is_err());
}

#[test]
fn test_config_multiple_chains() {
    let toml_content = r#"
[simulation]
name = "multi-chain"
chains = ["ethereum", "cosmos", "solana"]

[almanac]
name = "multi-chain-almanac"
api_port = 8080

[storage]
backend = "memory"

[services.ethereum]
enabled = true

[services.cosmos]
enabled = true

[services.solana]
enabled = true

[[chains.ethereum]]
name = "ethereum-mainnet"
rpc_url = "https://mainnet.infura.io/v3/YOUR_PROJECT_ID"
chain_id = 1

[[chains.cosmos]]
name = "cosmos-hub"
rpc_url = "https://rpc.cosmos.network"
chain_id = "cosmoshub-4"

[[chains.solana]]
name = "solana-mainnet"
rpc_url = "https://api.mainnet-beta.solana.com"
starting_slot = 0
"#;

    let mut temp_file = NamedTempFile::new().unwrap();
    writeln!(temp_file, "{}", toml_content).unwrap();

    let config = load_config(temp_file.path()).unwrap();

    assert!(config.chains.ethereum.is_some());
    assert!(config.chains.cosmos.is_some());
    assert!(config.chains.solana.is_some());

    let ethereum_chains = config.chains.ethereum.as_ref().unwrap();
    assert_eq!(ethereum_chains.len(), 1);
    assert_eq!(ethereum_chains[0].name, "ethereum-mainnet");

    let cosmos_chains = config.chains.cosmos.as_ref().unwrap();
    assert_eq!(cosmos_chains.len(), 1);
    assert_eq!(cosmos_chains[0].name, "cosmos-hub");

    let solana_chains = config.chains.solana.as_ref().unwrap();
    assert_eq!(solana_chains.len(), 1);
    assert_eq!(solana_chains[0].name, "solana-mainnet");
}

#[test]
fn test_config_with_historical_state_toml() {
    let toml_content = r#"
[simulation]
name = "historical-test"
chains = ["ethereum", "cosmos", "solana"]

[almanac]
name = "historical-almanac"
api_port = 8080

[storage]
backend = "memory"

[services]

[[chains.ethereum]]
name = "eth-with-history"
rpc_url = "http://localhost:8545"
chain_id = 1

[chains.ethereum.historical_state]
blocks_to_keep = 10000
enable_pruning = true
pruning_interval = 1000

[[chains.cosmos]]
name = "cosmos-archive"
rpc_url = "http://localhost:26657"
chain_id = "test-chain"

[chains.cosmos.historical_state]
blocks_to_keep = 0        # Ignored when archive_mode = true
archive_mode = true
enable_pruning = false

[[chains.solana]]
name = "solana-minimal"
rpc_url = "http://localhost:8899"

[chains.solana.historical_state]
blocks_to_keep = 500
enable_pruning = true
pruning_interval = 50
"#;

    let mut temp_file = NamedTempFile::new().unwrap();
    writeln!(temp_file, "{}", toml_content).unwrap();

    let config = load_config(temp_file.path()).unwrap();

    // Check Ethereum historical state
    let eth_chains = config.chains.ethereum.as_ref().unwrap();
    assert_eq!(eth_chains[0].name, "eth-with-history");
    let eth_hist = eth_chains[0].historical_state.as_ref().unwrap();
    assert_eq!(eth_hist.blocks_to_keep, 10000);
    assert_eq!(eth_hist.enable_pruning, true);
    assert_eq!(eth_hist.pruning_interval, Some(1000));

    // Check Cosmos archive mode
    let cosmos_chains = config.chains.cosmos.as_ref().unwrap();
    assert_eq!(cosmos_chains[0].name, "cosmos-archive");
    let cosmos_hist = cosmos_chains[0].historical_state.as_ref().unwrap();
    assert_eq!(cosmos_hist.archive_mode, Some(true));
    assert_eq!(cosmos_hist.enable_pruning, false);

    // Check Solana minimal history
    let solana_chains = config.chains.solana.as_ref().unwrap();
    assert_eq!(solana_chains[0].name, "solana-minimal");
    let solana_hist = solana_chains[0].historical_state.as_ref().unwrap();
    assert_eq!(solana_hist.blocks_to_keep, 500);
    assert_eq!(solana_hist.enable_pruning, true);
    assert_eq!(solana_hist.pruning_interval, Some(50));
}
