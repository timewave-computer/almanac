// reth_client.rs - Utility for interacting with Ethereum contracts deployed on reth
//
// Purpose: Provides functionality to interact with Valence contracts on a reth Ethereum node
// including querying contract state, sending transactions, and listening for events.

use ethers::{
    providers::{Provider, Http, Ws, Middleware},
    contract::Contract,
    types::{Address, U256, Filter, TransactionRequest, BlockNumber},
    middleware::SignerMiddleware,
    signers::{LocalWallet, Signer},
    abi::Abi,
};
use eyre::{Result, eyre};
use futures::StreamExt;
use hex;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::HashMap,
    fs::{self, File, read_to_string},
    io::BufReader,
    path::{Path, PathBuf},
    sync::Arc,
};
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Path to the reth config file
    #[arg(short, long, default_value = "config/reth/config.json")]
    config_path: String,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Query basic node information
    Info {},
    
    /// Query contract state
    Query {
        /// Contract address to query
        #[arg(short, long)]
        address: String,
        
        /// Contract function to call
        #[arg(short, long)]
        function: String,
        
        /// Function arguments as JSON array
        #[arg(short, long, default_value = "[]")]
        args: String,
    },
    
    /// Send a transaction to a contract
    Send {
        /// Contract address to call
        #[arg(short, long)]
        address: String,
        
        /// Contract function to call
        #[arg(short, long)]
        function: String,
        
        /// Function arguments as JSON array
        #[arg(short, long, default_value = "[]")]
        args: String,
        
        /// Gas limit for transaction
        #[arg(short, long, default_value = "3000000")]
        gas_limit: u64,
        
        /// Value to send with transaction (in wei)
        #[arg(short, long, default_value = "0")]
        value: String,
    },
    
    /// Listen for contract events
    Listen {
        /// Contract address to listen to
        #[arg(short, long)]
        address: String,
        
        /// Event name to listen for
        #[arg(short, long)]
        event: String,
        
        /// Number of blocks to listen for (0 = indefinite)
        #[arg(short, long, default_value = "0")]
        blocks: u64,
    },
}

/// Configuration for the reth node
#[derive(Debug, serde::Deserialize)]
struct RethConfig {
    rpc_url: String,
    ws_url: String,
    private_key: String,
    chain_id: String,
}

/// Helper struct to store contract information
pub struct ContractInfo {
    pub address: Address,
    pub abi: ethers::abi::Abi,
}

/// Main client for interacting with the reth node
pub struct RethClient {
    config: RethConfig,
    provider: Provider<Http>,
    wallet: LocalWallet,
    build_dir: PathBuf,
}

impl RethClient {
    /// Create a new client from the specified config file
    pub async fn new(config_path: &str) -> Result<Self> {
        // Load configuration
        let config_content = fs::read_to_string(config_path)
            .map_err(|e| eyre!("Failed to read config file: {}", e))?;
        
        let config: RethConfig = serde_json::from_str(&config_content)
            .map_err(|e| eyre!("Failed to parse config file: {}", e))?;
        
        // Set up provider
        let provider = Provider::<Http>::try_from(&config.rpc_url)
            .map_err(|e| eyre!("Failed to create provider: {}", e))?;
        
        // Set up wallet with private key
        let private_key = config.private_key.strip_prefix("0x").unwrap_or(&config.private_key);
        let wallet = private_key.parse::<LocalWallet>()
            .map_err(|e| eyre!("Failed to parse private key: {}", e))?
            .with_chain_id(config.chain_id.parse::<u64>().unwrap_or(31337));
        
        // Get build directory
        let current_dir = std::env::current_dir()?;
        let build_dir = current_dir.join("build/valence-contracts/ethereum");
        
        Ok(Self {
            config,
            provider,
            wallet,
            build_dir,
        })
    }
    
    /// Load contract information based on artifact files
    pub async fn load_contract(&self, address_str: &str) -> Result<ContractInfo> {
        // Parse address
        let address = Address::from_str(address_str)
            .map_err(|_| eyre!("Invalid contract address: {}", address_str))?;
        
        // First attempt to find the contract from deployed artifacts
        let artifacts_dir = self.build_dir.join("artifacts");
        let deployment_info_path = self.build_dir.join("deployment-info.json");
        
        // Find contract ABI from artifacts
        let mut contract_abi = None;
        
        // Try to load from deployment info if it exists
        if deployment_info_path.exists() {
            println!("Using deployment info to find contract ABI");
            let deployment_info: Value = serde_json::from_reader(fs::File::open(deployment_info_path)?)?;
            
            // Scan for contract name that matches the address
            for (name, info) in deployment_info.as_object().unwrap_or(&serde_json::Map::new()) {
                if let Some(deployed_address) = info.get("address") {
                    if deployed_address.as_str().unwrap_or("").to_lowercase() == address_str.to_lowercase() {
                        // Found matching address, try to load ABI
                        let contract_name = format!("{}.json", name);
                        // Search recursively for the contract file
                        contract_abi = self.find_contract_abi(&artifacts_dir, &contract_name).await?;
                        break;
                    }
                }
            }
        }
        
        // If ABI not found from deployment info, try brute force search
        if contract_abi.is_none() {
            println!("Searching for contract ABI in artifact files...");
            // Full recursive scan of artifacts directory
            contract_abi = self.find_contract_abi_by_scanning(&artifacts_dir).await?;
        }
        
        match contract_abi {
            Some(abi) => Ok(ContractInfo { address, abi }),
            None => Err(eyre!("Could not find ABI for contract at address {}", address_str)),
        }
    }
    
    // Helper to find contract ABI by contract name
    async fn find_contract_abi(&self, dir: &Path, contract_name: &str) -> Result<Option<ethers::abi::Abi>> {
        // Walk through all files in directory recursively
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            
            if path.is_dir() {
                // Recursively search subdirectories
                let future = self.find_contract_abi(&path, contract_name);
                if let Some(abi) = Box::pin(future).await? {
                    return Ok(Some(abi));
                }
            } else if path.file_name()
                .and_then(|f| f.to_str())
                .map(|s| s.ends_with(".json"))
                .unwrap_or(false) 
            {
                if path.file_name().unwrap().to_str().unwrap() == contract_name {
                    // Found matching file, try to parse ABI
                    let file_contents = fs::read_to_string(&path)?;
                    let json: Value = serde_json::from_str(&file_contents)?;
                    
                    if let Some(abi_value) = json.get("abi") {
                        let abi: ethers::abi::Abi = serde_json::from_value(abi_value.clone())?;
                        return Ok(Some(abi));
                    }
                }
            }
        }
        
        Ok(None)
    }
    
    // Helper to scan all artifact files for ABIs
    async fn find_contract_abi_by_scanning(&self, dir: &Path) -> Result<Option<ethers::abi::Abi>> {
        // Walk through all files in directory recursively
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            
            if path.is_dir() {
                // Recursively search subdirectories
                let future = self.find_contract_abi_by_scanning(&path);
                if let Some(abi) = Box::pin(future).await? {
                    return Ok(Some(abi));
                }
            } else if path.extension().and_then(|e| e.to_str()) == Some("json") {
                // Found a JSON file, try to parse ABI
                let file_contents = fs::read_to_string(&path)?;
                let json: Value = serde_json::from_str(&file_contents)?;
                
                if let Some(abi_value) = json.get("abi") {
                    let abi: ethers::abi::Abi = serde_json::from_value(abi_value.clone())?;
                    return Ok(Some(abi));
                }
            }
        }
        
        Ok(None)
    }
    
    /// Display basic information about the reth node
    pub async fn show_info(&self) -> Result<()> {
        println!("=== Reth Node Information ===");
        
        // Get basic node information
        let block_number = self.provider.get_block_number().await?;
        let network = self.provider.get_chainid().await?;
        let gas_price = self.provider.get_gas_price().await?;
        
        // Get wallet info
        let address = self.wallet.address();
        let balance = self.provider.get_balance(address, None).await?;
        
        println!("RPC URL: {}", self.config.rpc_url);
        println!("WS URL: {}", self.config.ws_url);
        println!("Network ID: {}", network);
        println!("Current Block: {}", block_number);
        println!("Gas Price: {} gwei", gas_price / U256::exp10(9));
        println!("\n=== Wallet Information ===");
        println!("Address: {}", address);
        println!("Balance: {} ETH", ethers::utils::format_ether(balance));
        
        Ok(())
    }
    
    /// Query contract state
    pub async fn query_contract(&self, address_str: &str, function: &str, args_str: &str) -> Result<()> {
        println!("Querying contract at {}", address_str);
        
        // Load contract info
        let contract_info = self.load_contract(address_str).await?;
        
        // Parse arguments
        let args: Vec<Value> = serde_json::from_str(args_str)?;
        
        // Create contract instance
        let contract = Contract::new(
            contract_info.address,
            contract_info.abi,
            Arc::new(self.provider.clone()),
        );
        
        // Call function
        println!("Calling function: {} with args: {}", function, args_str);
        let result = self.call_contract_function(&contract, function, args).await?;
        
        println!("Result:");
        println!("{}", serde_json::to_string_pretty(&result)?);
        
        Ok(())
    }
    
    /// Helper to call a contract function
    async fn call_contract_function(
        &self, 
        contract: &Contract<Provider<Http>>, 
        function: &str, 
        args: Vec<Value>
    ) -> Result<Value> {
        // Find function in ABI
        let func = contract.abi().function(function)?;
        
        let param_types: Vec<ethers::abi::ParamType> = func.inputs.iter()
            .map(|param| param.kind.clone())
            .collect();
        let tokens = self.json_args_to_tokens(args, &param_types)?;
        
        // Call function - use the correct return type
        let result: Vec<ethers::abi::Token> = contract
            .method::<_, Vec<ethers::abi::Token>>(function, tokens)?
            .call()
            .await?;
        
        // Convert result to JSON
        Ok(self.tokens_to_json(result))
    }
    
    /// Helper to convert JSON args to ABI tokens
    fn json_args_to_tokens(
        &self,
        args: Vec<Value>,
        param_types: &[ethers::abi::ParamType],
    ) -> Result<Vec<ethers::abi::Token>> {
        if args.len() != param_types.len() {
            return Err(eyre!("Argument count mismatch: expected {}, got {}", 
                param_types.len(), args.len()));
        }
        
        let mut tokens = Vec::with_capacity(args.len());
        
        for (arg, param_type) in args.iter().zip(param_types.iter()) {
            tokens.push(self.json_to_token(arg.clone(), param_type)?);
        }
        
        Ok(tokens)
    }
    
    /// Helper to convert a JSON value to an ABI token
    fn json_to_token(&self, value: Value, param_type: &ethers::abi::ParamType) -> Result<ethers::abi::Token> {
        match param_type {
            ethers::abi::ParamType::Address => {
                let addr = value
                    .as_str()
                    .ok_or_else(|| eyre!("Expected string for address"))?;
                Ok(ethers::abi::Token::Address(addr.parse()?))
            },
            ethers::abi::ParamType::Bytes => {
                let bytes = value
                    .as_str()
                    .ok_or_else(|| eyre!("Expected string for bytes"))?;
                let bytes = hex::decode(bytes.strip_prefix("0x").unwrap_or(bytes))?;
                Ok(ethers::abi::Token::Bytes(bytes))
            },
            ethers::abi::ParamType::Int(_) => {
                let num = value
                    .as_str()
                    .ok_or_else(|| eyre!("Expected string for int"))?;
                Ok(ethers::abi::Token::Int(num.parse::<U256>()?))
            },
            ethers::abi::ParamType::Uint(_) => {
                let num = value
                    .as_str()
                    .ok_or_else(|| eyre!("Expected string for uint"))?;
                Ok(ethers::abi::Token::Uint(num.parse::<U256>()?))
            },
            ethers::abi::ParamType::Bool => {
                let b = value
                    .as_bool()
                    .ok_or_else(|| eyre!("Expected boolean"))?;
                Ok(ethers::abi::Token::Bool(b))
            },
            ethers::abi::ParamType::String => {
                let s = value
                    .as_str()
                    .ok_or_else(|| eyre!("Expected string"))?
                    .to_string();
                Ok(ethers::abi::Token::String(s))
            },
            ethers::abi::ParamType::Array(inner) => {
                let arr = value
                    .as_array()
                    .ok_or_else(|| eyre!("Expected array"))?;
                let mut tokens = Vec::with_capacity(arr.len());
                for item in arr {
                    tokens.push(self.json_to_token(item.clone(), inner)?);
                }
                Ok(ethers::abi::Token::Array(tokens))
            },
            ethers::abi::ParamType::FixedArray(inner, size) => {
                let arr = value
                    .as_array()
                    .ok_or_else(|| eyre!("Expected array"))?;
                if arr.len() != *size {
                    return Err(eyre!("Expected array of size {}, got {}", size, arr.len()));
                }
                let mut tokens = Vec::with_capacity(arr.len());
                for item in arr {
                    tokens.push(self.json_to_token(item.clone(), inner)?);
                }
                Ok(ethers::abi::Token::FixedArray(tokens))
            },
            ethers::abi::ParamType::Tuple(inner) => {
                let obj = value
                    .as_object()
                    .ok_or_else(|| eyre!("Expected object for tuple"))?;
                
                if obj.len() != inner.len() {
                    return Err(eyre!("Expected tuple with {} fields, got {}", inner.len(), obj.len()));
                }
                
                let mut tokens = Vec::with_capacity(inner.len());
                for (i, inner_type) in inner.iter().enumerate() {
                    let key = format!("{}", i);
                    let item = obj.get(&key).unwrap_or(&Value::Null);
                    tokens.push(self.json_to_token(item.clone(), inner_type)?);
                }
                
                Ok(ethers::abi::Token::Tuple(tokens))
            },
            _ => Err(eyre!("Unsupported parameter type: {:?}", param_type)),
        }
    }
    
    /// Helper to convert ABI tokens to JSON
    fn tokens_to_json(&self, tokens: Vec<ethers::abi::Token>) -> Value {
        let mut result = Vec::new();
        
        for token in tokens {
            result.push(self.token_to_json(token));
        }
        
        if result.len() == 1 {
            result[0].clone()
        } else {
            Value::Array(result)
        }
    }
    
    /// Helper to convert a single ABI token to JSON
    fn token_to_json(&self, token: ethers::abi::Token) -> Value {
        match token {
            ethers::abi::Token::Address(addr) => json!(format!("{:?}", addr)),
            ethers::abi::Token::Bytes(bytes) => json!(format!("0x{}", hex::encode(&bytes))),
            ethers::abi::Token::Int(num) => json!(format!("{}", num)),
            ethers::abi::Token::Uint(num) => json!(format!("{}", num)),
            ethers::abi::Token::Bool(b) => json!(b),
            ethers::abi::Token::String(s) => json!(s),
            ethers::abi::Token::Array(tokens) => {
                let mut result = Vec::new();
                for token in tokens {
                    result.push(self.token_to_json(token));
                }
                Value::Array(result)
            },
            ethers::abi::Token::FixedArray(tokens) => {
                let mut result = Vec::new();
                for token in tokens {
                    result.push(self.token_to_json(token));
                }
                Value::Array(result)
            },
            ethers::abi::Token::Tuple(tokens) => {
                let mut result = serde_json::Map::new();
                for (i, token) in tokens.iter().enumerate() {
                    result.insert(format!("{}", i), self.token_to_json(token.clone()));
                }
                Value::Object(result)
            },
            _ => Value::Null,
        }
    }
    
    /// Send a transaction to a contract
    pub async fn send_transaction(
        &self, 
        address_str: &str, 
        function: &str, 
        args_str: &str, 
        gas_limit: u64, 
        value_str: &str
    ) -> Result<()> {
        println!("Sending transaction to contract at {}", address_str);
        
        // Load contract info
        let contract_info = self.load_contract(address_str).await?;
        
        // Parse arguments
        let args: Vec<Value> = serde_json::from_str(args_str)?;
        
        // Parse value
        let value = U256::from_dec_str(value_str)?;
        
        // Create contract instance with signer
        let client = SignerMiddleware::new(
            self.provider.clone(),
            self.wallet.clone(),
        );
        
        let contract = Contract::new(
            contract_info.address,
            contract_info.abi,
            Arc::new(client.clone()),
        );
        
        // Create function call data
        let func = contract.abi().function(function)?;
        
        // Convert JSON args to ethers::abi::Token
        let param_types: Vec<ethers::abi::ParamType> = func.inputs.iter()
            .map(|param| param.kind.clone())
            .collect();
        let tokens = self.json_args_to_tokens(args, &param_types)?;
        
        // Call function
        println!("Sending transaction: {}({}) with gas limit: {}, value: {}", 
            function, args_str, gas_limit, value_str);
        
        let tx_request = TransactionRequest::new()
            .to(contract_info.address)
            .value(value)
            .gas(gas_limit);
        
        let data = func.encode_input(&tokens)?;
        let tx_request = tx_request.data(data);
        
        let pending_tx = client.send_transaction(tx_request, None).await?;
        
        println!("Transaction sent: {}", pending_tx.tx_hash());
        
        // Wait for confirmation
        println!("Waiting for transaction to be mined...");
        let receipt = pending_tx.await?;
        
        if let Some(receipt) = receipt {
            if let Some(block_number) = receipt.block_number {
                println!("Transaction mined in block: {}", block_number);
            }
            if let Some(gas_used) = receipt.gas_used {
                println!("Gas used: {}", gas_used);
            }
            
            if let Some(status) = receipt.status {
                if status.as_u64() == 1 {
                    println!("Transaction succeeded!");
                } else {
                    println!("Transaction failed with status: {}", status);
                }
            }
        } else {
            println!("Transaction not mined yet");
        }
        
        Ok(())
    }
    
    /// Listen for contract events
    pub async fn listen_for_events(&self, address_str: &str, event_name: &str, blocks: u64) -> Result<()> {
        println!("Listening for {} events from contract at {}", event_name, address_str);
        
        // Load contract info
        let contract_info = self.load_contract(address_str).await?;
        
        // Create websocket provider for event streaming
        let ws_provider = Provider::<Ws>::connect(&self.config.ws_url).await?;
        
        // Create contract instance
        let contract = Contract::new(
            contract_info.address,
            contract_info.abi,
            Arc::new(ws_provider.clone()),
        );
        
        // Find event by name
        let event = contract.abi().event(event_name)?;
        
        // Determine from and to blocks
        let current_block = ws_provider.get_block_number().await?;
        let from_block = current_block;
        let to_block: BlockNumber = if blocks == 0 { 
            BlockNumber::Latest
        } else { 
            BlockNumber::Number(ethers::types::U64::from(current_block + blocks))
        };
        
        println!("Listening from block {} to {}", from_block, 
            if blocks == 0 { "latest".to_string() } else { to_block.to_string() });
        
        // Create filter
        let filter = Filter::new()
            .address(contract_info.address.into())
            .topic0(event.signature())
            .from_block(from_block)
            .to_block(to_block);
        
        // Stream events
        let mut stream = ws_provider.subscribe_logs(&filter).await?;
        
        println!("Waiting for events...");
        println!("Press Ctrl+C to stop");
        
        while let Some(log) = stream.next().await {
            println!("\nEvent detected in block {}:", log.block_number.unwrap_or_default());
            
            // Try to parse the event
            match event.parse_log(log.clone().into()) {
                Ok(parsed) => {
                    println!("Event: {}", event_name);
                    
                    // Print each parameter
                    for param in parsed.params {
                        println!("  {}: {}", param.name, self.token_to_json(param.value));
                    }
                },
                Err(e) => {
                    println!("Failed to parse event: {}", e);
                    println!("Raw log: {:?}", log);
                }
            }
        }
        
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    
    // Create client
    let client = RethClient::new(&cli.config_path).await?;
    
    // Process command
    match cli.command {
        Commands::Info {} => {
            client.show_info().await?;
        },
        Commands::Query { address, function, args } => {
            client.query_contract(&address, &function, &args).await?;
        },
        Commands::Send { address, function, args, gas_limit, value } => {
            client.send_transaction(&address, &function, &args, gas_limit, &value).await?;
        },
        Commands::Listen { address, event, blocks } => {
            client.listen_for_events(&address, &event, blocks).await?;
        },
    }
    
    Ok(())
} 