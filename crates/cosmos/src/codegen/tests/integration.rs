//! Integration tests for cosmos contract code generation

use crate::codegen::{CosmosCodegenConfig, parser::*, CosmosContractCodegen};
use indexer_core::Result;
use std::fs;
use tempfile::TempDir;
use std::collections::HashMap;

#[tokio::test]
async fn test_end_to_end_cw20_generation() -> Result<()> {
    // Create temporary directory for output
    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().to_str().unwrap();

    // Create configuration
    let config = CosmosCodegenConfig {
        contract_address: "cosmos1cw20example".to_string(),
        chain_id: "cosmoshub-4".to_string(),
        output_dir: output_path.to_string(),
        features: vec!["client".to_string(), "storage".to_string()],
        dry_run: false,
        namespace: None,
    };

    // Create mock schema manually
    let transfer_variant = MessageVariant {
        name: "Transfer".to_string(),
        description: Some("Transfer tokens".to_string()),
        properties: vec![
            PropertySchema {
                name: "recipient".to_string(),
                type_info: TypeInfo {
                    base_type: "string".to_string(),
                    reference: None,
                    items: None,
                    enum_values: None,
                },
                description: Some("Recipient address".to_string()),
                required: true,
            },
            PropertySchema {
                name: "amount".to_string(),
                type_info: TypeInfo {
                    base_type: "string".to_string(),
                    reference: None,
                    items: None,
                    enum_values: None,
                },
                description: Some("Amount to transfer".to_string()),
                required: true,
            },
        ],
        required: vec!["recipient".to_string(), "amount".to_string()],
    };

    let execute_msg = MessageSchema {
        title: Some("ExecuteMsg".to_string()),
        description: Some("Execute message for CW20 contract".to_string()),
        variants: vec![transfer_variant],
        is_enum: true,
    };

    let balance_variant = MessageVariant {
        name: "Balance".to_string(),
        description: Some("Query balance".to_string()),
        properties: vec![
            PropertySchema {
                name: "address".to_string(),
                type_info: TypeInfo {
                    base_type: "string".to_string(),
                    reference: None,
                    items: None,
                    enum_values: None,
                },
                description: Some("Address to query".to_string()),
                required: true,
            },
        ],
        required: vec!["address".to_string()],
    };

    let query_msg = MessageSchema {
        title: Some("QueryMsg".to_string()),
        description: Some("Query message for CW20 contract".to_string()),
        variants: vec![balance_variant],
        is_enum: true,
    };

    let schema = CosmWasmSchema {
        instantiate_msg: None,
        execute_msg: Some(execute_msg),
        query_msg: Some(query_msg),
        migrate_msg: None,
        events: vec![],
        definitions: HashMap::new(),
    };

    // Generate code
    let codegen = CosmosContractCodegen::new(config);
    codegen.generate_all(&schema).await?;

    // Verify generated files exist
    let client_dir = temp_dir.path().join("client");
    let storage_dir = temp_dir.path().join("storage");
    
    assert!(client_dir.exists(), "Client directory should be created");
    assert!(storage_dir.exists(), "Storage directory should be created");
    
    let client_mod = client_dir.join("mod.rs");
    let storage_mod = storage_dir.join("mod.rs");
    
    assert!(client_mod.exists(), "Client mod.rs should be created");
    assert!(storage_mod.exists(), "Storage mod.rs should be created");

    // Verify generated content contains expected elements
    let client_content = fs::read_to_string(&client_mod).unwrap();
    println!("Debug - client content: {}", &client_content[0..500.min(client_content.len())]);
    
    // Check for key components that should be generated (more flexible checks)
    assert!(client_content.contains("Client"), "Client struct should be generated");
    assert!(client_content.contains("contract"), "Client should reference contract");

    let storage_content = fs::read_to_string(&storage_mod).unwrap();
    println!("Debug - storage content: {}", &storage_content[0..500.min(storage_content.len())]);
    
    assert!(storage_content.contains("Storage"), "Storage struct should be generated");

    println!("✅ End-to-end CW20 generation test passed");
    Ok(())
}

#[tokio::test]
async fn test_complex_message_schema_generation() -> Result<()> {
    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().to_str().unwrap();

    let config = CosmosCodegenConfig {
        contract_address: "cosmos1complex".to_string(),
        chain_id: "osmosis-1".to_string(),
        output_dir: output_path.to_string(),
        features: vec!["client".to_string()],
        dry_run: false,
        namespace: Some("test".to_string()),
    };

    // Create complex schema manually
    let complex_variant = MessageVariant {
        name: "ComplexOperation".to_string(),
        description: Some("Complex operation".to_string()),
        properties: vec![
            PropertySchema {
                name: "nested_data".to_string(),
                type_info: TypeInfo {
                    base_type: "object".to_string(),
                    reference: None,
                    items: None,
                    enum_values: None,
                },
                description: Some("Nested data".to_string()),
                required: true,
            },
            PropertySchema {
                name: "optional_array".to_string(),
                type_info: TypeInfo {
                    base_type: "array".to_string(),
                    reference: None,
                    items: Some(Box::new(TypeInfo {
                        base_type: "integer".to_string(),
                        reference: None,
                        items: None,
                        enum_values: None,
                    })),
                    enum_values: None,
                },
                description: Some("Optional array".to_string()),
                required: false,
            },
        ],
        required: vec!["nested_data".to_string()],
    };

    let execute_msg = MessageSchema {
        title: Some("ExecuteMsg".to_string()),
        description: Some("Complex execute message".to_string()),
        variants: vec![complex_variant],
        is_enum: true,
    };

    let schema = CosmWasmSchema {
        instantiate_msg: None,
        execute_msg: Some(execute_msg),
        query_msg: None,
        migrate_msg: None,
        events: vec![],
        definitions: HashMap::new(),
    };

    let codegen = CosmosContractCodegen::new(config);
    codegen.generate_all(&schema).await?;

    // Verify generation completed without errors
    let client_dir = temp_dir.path().join("client");
    assert!(client_dir.exists(), "Client directory should be created for complex schema");

    println!("✅ Complex message schema generation test passed");
    Ok(())
}

#[tokio::test]
async fn test_dry_run_generation() -> Result<()> {
    let config = CosmosCodegenConfig {
        contract_address: "cosmos1dryrun".to_string(),
        chain_id: "juno-1".to_string(),
        output_dir: "/tmp/should-not-exist".to_string(),
        features: vec!["client".to_string()],
        dry_run: true,
        namespace: None,
    };

    // Create minimal schema manually
    let simple_variant = MessageVariant {
        name: "SimpleMsg".to_string(),
        description: Some("Simple message".to_string()),
        properties: vec![],
        required: vec![],
    };

    let instantiate_msg = MessageSchema {
        title: Some("InstantiateMsg".to_string()),
        description: Some("Instantiate message".to_string()),
        variants: vec![simple_variant],
        is_enum: false,
    };

    let schema = CosmWasmSchema {
        instantiate_msg: Some(instantiate_msg),
        execute_msg: None,
        query_msg: None,
        migrate_msg: None,
        events: vec![],
        definitions: HashMap::new(),
    };

    let codegen = CosmosContractCodegen::new(config);
    codegen.generate_all(&schema).await?;

    // Verify no files were created in dry run
    assert!(!std::path::Path::new("/tmp/should-not-exist").exists(), 
           "No files should be created in dry run mode");

    println!("✅ Dry run generation test passed");
    Ok(())
}

#[tokio::test]
async fn test_nested_message_schema_generation() -> Result<()> {
    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().to_str().unwrap();

    let config = CosmosCodegenConfig {
        contract_address: "cosmos1nested".to_string(),
        chain_id: "test-1".to_string(),
        output_dir: output_path.to_string(),
        features: vec!["client".to_string(), "storage".to_string()],
        dry_run: false,
        namespace: None,
    };

    // Create deeply nested schema
    let nested_variant = MessageVariant {
        name: "NestedOperation".to_string(),
        description: Some("Operation with nested data".to_string()),
        properties: vec![
            PropertySchema {
                name: "user_info".to_string(),
                type_info: TypeInfo {
                    base_type: "object".to_string(),
                    reference: Some("UserInfo".to_string()),
                    items: None,
                    enum_values: None,
                },
                description: Some("User information".to_string()),
                required: true,
            },
            PropertySchema {
                name: "optional_config".to_string(),
                type_info: TypeInfo {
                    base_type: "object".to_string(),
                    reference: Some("Config".to_string()),
                    items: None,
                    enum_values: None,
                },
                description: Some("Optional configuration".to_string()),
                required: false,
            },
            PropertySchema {
                name: "amounts".to_string(),
                type_info: TypeInfo {
                    base_type: "array".to_string(),
                    reference: None,
                    items: Some(Box::new(TypeInfo {
                        base_type: "string".to_string(),
                        reference: None,
                        items: None,
                        enum_values: None,
                    })),
                    enum_values: None,
                },
                description: Some("Array of amounts".to_string()),
                required: true,
            },
        ],
        required: vec!["user_info".to_string(), "amounts".to_string()],
    };

    let execute_msg = MessageSchema {
        title: Some("ExecuteMsg".to_string()),
        description: Some("Complex nested execute message".to_string()),
        variants: vec![nested_variant],
        is_enum: true,
    };

    let mut definitions = HashMap::new();
    
    // Add UserInfo definition
    definitions.insert("UserInfo".to_string(), TypeDefinition {
        name: "UserInfo".to_string(),
        description: Some("User information structure".to_string()),
        properties: vec![
            PropertySchema {
                name: "address".to_string(),
                type_info: TypeInfo {
                    base_type: "string".to_string(),
                    reference: None,
                    items: None,
                    enum_values: None,
                },
                description: Some("User address".to_string()),
                required: true,
            },
            PropertySchema {
                name: "permissions".to_string(),
                type_info: TypeInfo {
                    base_type: "array".to_string(),
                    reference: None,
                    items: Some(Box::new(TypeInfo {
                        base_type: "string".to_string(),
                        reference: None,
                        items: None,
                        enum_values: Some(vec!["read".to_string(), "write".to_string(), "admin".to_string()]),
                    })),
                    enum_values: None,
                },
                description: Some("User permissions".to_string()),
                required: true,
            },
        ],
        is_enum: false,
        variants: vec![],
    });

    let schema = CosmWasmSchema {
        instantiate_msg: None,
        execute_msg: Some(execute_msg),
        query_msg: None,
        migrate_msg: None,
        events: vec![],
        definitions,
    };

    let codegen = CosmosContractCodegen::new(config);
    codegen.generate_all(&schema).await?;

    // Verify files were created
    let client_dir = temp_dir.path().join("client");
    let storage_dir = temp_dir.path().join("storage");
    
    assert!(client_dir.exists(), "Client directory should be created for nested schema");
    assert!(storage_dir.exists(), "Storage directory should be created for nested schema");

    println!("✅ Nested message schema generation test passed");
    Ok(())
}

#[tokio::test]
async fn test_enum_and_union_schema_generation() -> Result<()> {
    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().to_str().unwrap();

    let config = CosmosCodegenConfig {
        contract_address: "cosmos1enums".to_string(),
        chain_id: "test-1".to_string(),
        output_dir: output_path.to_string(),
        features: vec!["client".to_string()],
        dry_run: false,
        namespace: None,
    };

    // Create schema with enums and unions
    let multi_type_variant = MessageVariant {
        name: "MultiTypeOperation".to_string(),
        description: Some("Operation with various types".to_string()),
        properties: vec![
            PropertySchema {
                name: "status".to_string(),
                type_info: TypeInfo {
                    base_type: "string".to_string(),
                    reference: None,
                    items: None,
                    enum_values: Some(vec!["pending".to_string(), "approved".to_string(), "rejected".to_string()]),
                },
                description: Some("Operation status".to_string()),
                required: true,
            },
            PropertySchema {
                name: "priority".to_string(),
                type_info: TypeInfo {
                    base_type: "integer".to_string(),
                    reference: None,
                    items: None,
                    enum_values: Some(vec!["1".to_string(), "2".to_string(), "3".to_string()]),
                },
                description: Some("Priority level".to_string()),
                required: true,
            },
            PropertySchema {
                name: "metadata".to_string(),
                type_info: TypeInfo {
                    base_type: "object".to_string(),
                    reference: None,
                    items: None,
                    enum_values: None,
                },
                description: Some("Free-form metadata".to_string()),
                required: false,
            },
        ],
        required: vec!["status".to_string(), "priority".to_string()],
    };

    let execute_msg = MessageSchema {
        title: Some("ExecuteMsg".to_string()),
        description: Some("Execute message with enums and unions".to_string()),
        variants: vec![multi_type_variant],
        is_enum: true,
    };

    let schema = CosmWasmSchema {
        instantiate_msg: None,
        execute_msg: Some(execute_msg),
        query_msg: None,
        migrate_msg: None,
        events: vec![],
        definitions: HashMap::new(),
    };

    let codegen = CosmosContractCodegen::new(config);
    codegen.generate_all(&schema).await?;

    // Verify generation completed
    let client_dir = temp_dir.path().join("client");
    assert!(client_dir.exists(), "Client directory should be created for enum schema");

    println!("✅ Enum and union schema generation test passed");
    Ok(())
}
