//! Integration tests for Ethereum contract code generation

use crate::codegen::{EthereumCodegenConfig, EthereumContractCodegen};
use crate::codegen::parser::AbiParser;
use indexer_core::Result;
use std::fs;
use tempfile::TempDir;

/// Real ERC20 contract ABI (USDC) for testing
const ERC20_ABI: &str = r#"[
  {
    "type": "constructor",
    "inputs": [
      {"name": "_name", "type": "string", "internalType": "string"},
      {"name": "_symbol", "type": "string", "internalType": "string"},
      {"name": "_decimals", "type": "uint8", "internalType": "uint8"},
      {"name": "_initialSupply", "type": "uint256", "internalType": "uint256"}
    ]
  },
  {
    "type": "function",
    "name": "name",
    "inputs": [],
    "outputs": [{"name": "", "type": "string", "internalType": "string"}],
    "stateMutability": "view"
  },
  {
    "type": "function",
    "name": "symbol",
    "inputs": [],
    "outputs": [{"name": "", "type": "string", "internalType": "string"}],
    "stateMutability": "view"
  },
  {
    "type": "function",
    "name": "decimals",
    "inputs": [],
    "outputs": [{"name": "", "type": "uint8", "internalType": "uint8"}],
    "stateMutability": "view"
  },
  {
    "type": "function",
    "name": "totalSupply",
    "inputs": [],
    "outputs": [{"name": "", "type": "uint256", "internalType": "uint256"}],
    "stateMutability": "view"
  },
  {
    "type": "function",
    "name": "balanceOf",
    "inputs": [{"name": "account", "type": "address", "internalType": "address"}],
    "outputs": [{"name": "", "type": "uint256", "internalType": "uint256"}],
    "stateMutability": "view"
  },
  {
    "type": "function",
    "name": "transfer",
    "inputs": [
      {"name": "to", "type": "address", "internalType": "address"},
      {"name": "amount", "type": "uint256", "internalType": "uint256"}
    ],
    "outputs": [{"name": "", "type": "bool", "internalType": "bool"}],
    "stateMutability": "nonpayable"
  },
  {
    "type": "function",
    "name": "allowance",
    "inputs": [
      {"name": "owner", "type": "address", "internalType": "address"},
      {"name": "spender", "type": "address", "internalType": "address"}
    ],
    "outputs": [{"name": "", "type": "uint256", "internalType": "uint256"}],
    "stateMutability": "view"
  },
  {
    "type": "function",
    "name": "approve",
    "inputs": [
      {"name": "spender", "type": "address", "internalType": "address"},
      {"name": "amount", "type": "uint256", "internalType": "uint256"}
    ],
    "outputs": [{"name": "", "type": "bool", "internalType": "bool"}],
    "stateMutability": "nonpayable"
  },
  {
    "type": "function",
    "name": "transferFrom",
    "inputs": [
      {"name": "from", "type": "address", "internalType": "address"},
      {"name": "to", "type": "address", "internalType": "address"},
      {"name": "amount", "type": "uint256", "internalType": "uint256"}
    ],
    "outputs": [{"name": "", "type": "bool", "internalType": "bool"}],
    "stateMutability": "nonpayable"
  },
  {
    "type": "event",
    "name": "Transfer",
    "inputs": [
      {"name": "from", "type": "address", "indexed": true, "internalType": "address"},
      {"name": "to", "type": "address", "indexed": true, "internalType": "address"},
      {"name": "value", "type": "uint256", "indexed": false, "internalType": "uint256"}
    ],
    "anonymous": false
  },
  {
    "type": "event",
    "name": "Approval",
    "inputs": [
      {"name": "owner", "type": "address", "indexed": true, "internalType": "address"},
      {"name": "spender", "type": "address", "indexed": true, "internalType": "address"},
      {"name": "value", "type": "uint256", "indexed": false, "internalType": "uint256"}
    ],
    "anonymous": false
  }
]"#;

#[tokio::test]
async fn test_end_to_end_erc20_generation() -> Result<()> {
    // Create temporary directory for output
    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().to_str().unwrap();

    // Create configuration
    let config = EthereumCodegenConfig {
        contract_address: "0xa0b86a33e6411fbae92b7b5e06e1d4320827cd8".to_string(), // USDC
        chain_id: "1".to_string(), // Ethereum mainnet
        output_dir: output_path.to_string(),
        features: vec!["client".to_string(), "storage".to_string()],
        dry_run: false,
        namespace: None,
    };

    // Parse the ERC20 ABI
    let parser = AbiParser::new();
    let abi = parser.parse_content(ERC20_ABI)?;

    // Verify ABI parsing
    assert!(abi.constructor.is_some(), "Constructor should be parsed");
    assert_eq!(abi.functions.len(), 9, "Should have 9 functions");
    assert_eq!(abi.events.len(), 2, "Should have 2 events");

    // Check specific functions exist
    let function_names: Vec<&str> = abi.functions.iter().map(|f| f.name.as_str()).collect();
    assert!(function_names.contains(&"name"));
    assert!(function_names.contains(&"symbol"));
    assert!(function_names.contains(&"transfer"));
    assert!(function_names.contains(&"balanceOf"));

    // Check events exist
    let event_names: Vec<&str> = abi.events.iter().map(|e| e.name.as_str()).collect();
    assert!(event_names.contains(&"Transfer"));
    assert!(event_names.contains(&"Approval"));

    // Generate code
    let codegen = EthereumContractCodegen::new(config);
    codegen.generate_all(&abi).await?;

    // Verify files were created
    let client_dir = temp_dir.path().join("client");
    let storage_dir = temp_dir.path().join("storage");
    
    assert!(client_dir.exists(), "Client directory should be created");
    assert!(storage_dir.exists(), "Storage directory should be created");

    // Check specific files
    let client_mod = client_dir.join("mod.rs");
    let storage_mod = storage_dir.join("mod.rs");
    
    assert!(client_mod.exists(), "Client mod.rs should be created");
    assert!(storage_mod.exists(), "Storage mod.rs should be created");

    // Verify generated content contains expected elements
    let client_content = fs::read_to_string(&client_mod).unwrap();
    println!("Debug - client content: {}", &client_content[0..500.min(client_content.len())]);
    
    // Check for key components that should be generated
    assert!(client_content.contains("Client"), "Client struct should be generated");
    assert!(client_content.contains("contract"), "Client should reference contract");

    let storage_content = fs::read_to_string(&storage_mod).unwrap();
    println!("Debug - storage content: {}", &storage_content[0..500.min(storage_content.len())]);
    
    assert!(storage_content.contains("Storage"), "Storage struct should be generated");
    assert!(storage_content.contains("pub mod"), "Storage should have module declarations");

    // Check that SQL schema file was created
    let postgres_schema = storage_dir.join("postgres_schema.sql");
    assert!(postgres_schema.exists(), "PostgreSQL schema file should be created");
    
    let sql_content = fs::read_to_string(&postgres_schema).unwrap();
    assert!(sql_content.contains("CREATE TABLE"), "SQL file should contain CREATE TABLE statements");

    println!("✅ End-to-end ERC20 generation test passed");
    Ok(())
}

#[tokio::test]
async fn test_complex_ethereum_contract_generation() -> Result<()> {
    // Complex contract ABI with multiple parameter types
    const COMPLEX_ABI: &str = r#"[
      {
        "type": "function",
        "name": "complexFunction",
        "inputs": [
          {"name": "addresses", "type": "address[]", "internalType": "address[]"},
          {"name": "amounts", "type": "uint256[]", "internalType": "uint256[]"},
          {"name": "data", "type": "bytes", "internalType": "bytes"},
          {"name": "deadline", "type": "uint256", "internalType": "uint256"}
        ],
        "outputs": [
          {"name": "success", "type": "bool", "internalType": "bool"},
          {"name": "returnData", "type": "bytes", "internalType": "bytes"}
        ],
        "stateMutability": "payable"
      },
      {
        "type": "function",
        "name": "tupleFunction",
        "inputs": [
          {
            "name": "params",
            "type": "tuple",
            "internalType": "struct ExampleStruct",
            "components": [
              {"name": "token", "type": "address", "internalType": "address"},
              {"name": "amount", "type": "uint256", "internalType": "uint256"},
              {"name": "deadline", "type": "uint256", "internalType": "uint256"}
            ]
          }
        ],
        "outputs": [],
        "stateMutability": "nonpayable"
      },
      {
        "type": "event",
        "name": "ComplexEvent",
        "inputs": [
          {"name": "user", "type": "address", "indexed": true, "internalType": "address"},
          {"name": "tokens", "type": "address[]", "indexed": false, "internalType": "address[]"},
          {"name": "amounts", "type": "uint256[]", "indexed": false, "internalType": "uint256[]"},
          {"name": "data", "type": "bytes", "indexed": false, "internalType": "bytes"}
        ]
      }
    ]"#;

    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().to_str().unwrap();

    let config = EthereumCodegenConfig {
        contract_address: "0x1234567890123456789012345678901234567890".to_string(),
        chain_id: "1".to_string(),
        output_dir: output_path.to_string(),
        features: vec!["client".to_string(), "api".to_string()],
        dry_run: false,
        namespace: None,
    };

    // Parse the complex ABI
    let parser = AbiParser::new();
    let abi = parser.parse_content(COMPLEX_ABI)?;

    // Verify complex types are parsed correctly
    assert_eq!(abi.functions.len(), 2, "Should have 2 functions");
    assert_eq!(abi.events.len(), 1, "Should have 1 event");

    // Check tuple function
    let tuple_function = abi.functions.iter().find(|f| f.name == "tupleFunction").unwrap();
    assert_eq!(tuple_function.inputs.len(), 1, "Tuple function should have 1 input");
    assert!(tuple_function.inputs[0].param_type.starts_with("tuple"), "Input should be tuple type");

    // Check complex function
    let complex_function = abi.functions.iter().find(|f| f.name == "complexFunction").unwrap();
    assert_eq!(complex_function.inputs.len(), 4, "Complex function should have 4 inputs");
    assert_eq!(complex_function.outputs.len(), 2, "Complex function should have 2 outputs");
    assert!(complex_function.payable, "Complex function should be payable");

    // Generate code
    let codegen = EthereumContractCodegen::new(config);
    codegen.generate_all(&abi).await?;

    // Verify files were created
    let client_dir = temp_dir.path().join("client");
    let api_dir = temp_dir.path().join("api");
    
    assert!(client_dir.exists(), "Client directory should be created");
    assert!(api_dir.exists(), "API directory should be created");

    println!("✅ Complex Ethereum contract generation test passed");
    Ok(())
}

#[tokio::test]
async fn test_dry_run_ethereum_generation() -> Result<()> {
    let config = EthereumCodegenConfig {
        contract_address: "0xa0b86a33e6411fbae92b7b5e06e1d4320827cd8".to_string(),
        chain_id: "1".to_string(),
        output_dir: "./test_output".to_string(),
        features: vec!["client".to_string()],
        dry_run: true,
        namespace: None,
    };

    // Parse the ERC20 ABI
    let parser = AbiParser::new();
    let abi = parser.parse_content(ERC20_ABI)?;

    // Generate code in dry run mode
    let codegen = EthereumContractCodegen::new(config);
    codegen.generate_all(&abi).await?;

    // Verify no files were created in dry run mode
    let output_path = std::path::Path::new("./test_output");
    assert!(!output_path.exists(), "No files should be created in dry run mode");

    println!("✅ Dry run Ethereum generation test passed");
    Ok(())
}

#[tokio::test]
async fn test_ethereum_parser_edge_cases() -> Result<()> {
    // Test empty ABI
    const EMPTY_ABI: &str = "[]";
    let parser = AbiParser::new();
    let abi = parser.parse_content(EMPTY_ABI)?;
    
    assert!(abi.constructor.is_none(), "Empty ABI should have no constructor");
    assert!(abi.functions.is_empty(), "Empty ABI should have no functions");
    assert!(abi.events.is_empty(), "Empty ABI should have no events");

    // Test ABI with only events
    const EVENTS_ONLY_ABI: &str = r#"[
      {
        "type": "event",
        "name": "TestEvent",
        "inputs": [
          {"name": "param1", "type": "uint256", "indexed": true},
          {"name": "param2", "type": "string", "indexed": false}
        ]
      }
    ]"#;

    let abi = parser.parse_content(EVENTS_ONLY_ABI)?;
    assert!(abi.constructor.is_none(), "Events-only ABI should have no constructor");
    assert!(abi.functions.is_empty(), "Events-only ABI should have no functions");
    assert_eq!(abi.events.len(), 1, "Should have 1 event");

    // Test ABI with fallback/receive functions
    const FALLBACK_ABI: &str = r#"[
      {
        "type": "fallback",
        "stateMutability": "payable"
      },
      {
        "type": "receive",
        "stateMutability": "payable"
      }
    ]"#;

    let abi = parser.parse_content(FALLBACK_ABI)?;
    assert_eq!(abi.functions.len(), 2, "Should have fallback and receive functions");

    let fallback_function = abi.functions.iter().find(|f| f.function_type == "fallback").unwrap();
    assert!(fallback_function.payable, "Fallback should be payable");

    let receive_function = abi.functions.iter().find(|f| f.function_type == "receive").unwrap();
    assert!(receive_function.payable, "Receive should be payable");

    println!("✅ Ethereum parser edge cases test passed");
    Ok(())
} 