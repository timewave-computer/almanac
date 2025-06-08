//! Tests for cosmos contract code generation

use super::*;

mod integration;

#[cfg(test)]
mod cosmos_tests {
    use super::*;
    use tokio;

    #[test]
    fn test_cosmos_codegen_config_creation() {
        let config = CosmosCodegenConfig::default();
        
        assert_eq!(config.output_dir, "./generated");
        assert_eq!(config.features.len(), 4);
        assert!(config.features.contains(&"client".to_string()));
        assert!(config.features.contains(&"storage".to_string()));
        assert!(config.features.contains(&"api".to_string()));
        assert!(config.features.contains(&"migrations".to_string()));
        assert!(!config.dry_run);
    }

    #[test]
    fn test_cosmos_codegen_config_custom() {
        let config = CosmosCodegenConfig {
            contract_address: "cosmos1contract123".to_string(),
            chain_id: "cosmoshub-4".to_string(),
            output_dir: "./my_output".to_string(),
            namespace: Some("my_namespace".to_string()),
            features: vec!["client".to_string()],
            dry_run: true,
        };
        
        assert_eq!(config.contract_address, "cosmos1contract123");
        assert_eq!(config.chain_id, "cosmoshub-4");
        assert_eq!(config.output_dir, "./my_output");
        assert_eq!(config.namespace, Some("my_namespace".to_string()));
        assert_eq!(config.features, vec!["client".to_string()]);
        assert!(config.dry_run);
    }

    #[test]
    fn test_cosmwasm_parser_creation() {
        let _parser = CosmWasmMsgParser::new();
        // Just test that we can create a parser without panicking
    }

    #[test]
    fn test_cosmos_contract_codegen_creation() {
        let config = CosmosCodegenConfig::default();
        let _codegen = CosmosContractCodegen::new(config);
        // Just test that we can create a codegen without panicking
    }

    #[tokio::test]
    async fn test_parse_simple_execute_msg() {
        let parser = CosmWasmMsgParser::new();
        let json_content = r#"
        {
            "title": "ExecuteMsg",
            "type": "object",
            "oneOf": [
                {
                    "type": "object",
                    "required": ["transfer"],
                    "properties": {
                        "transfer": {
                            "type": "object",
                            "required": ["recipient", "amount"],
                            "properties": {
                                "recipient": {
                                    "type": "string"
                                },
                                "amount": {
                                    "type": "string"
                                }
                            }
                        }
                    }
                }
            ]
        }
        "#;

        let result = parser.parse_content(json_content);
        assert!(result.is_ok());
        
        let schema = result.unwrap();
        assert!(schema.execute_msg.is_some());
        
        let execute_msg = schema.execute_msg.unwrap();
        assert_eq!(execute_msg.title, Some("ExecuteMsg".to_string()));
        assert!(execute_msg.is_enum);
        
        let variants = &execute_msg.variants;
        assert_eq!(variants.len(), 1);
        assert!(!variants[0].properties.is_empty());
    }

    #[tokio::test]
    async fn test_parse_simple_query_msg() {
        let parser = CosmWasmMsgParser::new();
        let json_content = r#"
        {
            "title": "QueryMsg",
            "type": "object",
            "oneOf": [
                {
                    "type": "object",
                    "required": ["balance"],
                    "properties": {
                        "balance": {
                            "type": "object",
                            "required": ["address"],
                            "properties": {
                                "address": {
                                    "type": "string"
                                }
                            }
                        }
                    }
                }
            ]
        }
        "#;

        let result = parser.parse_content(json_content);
        assert!(result.is_ok());
        
        let schema = result.unwrap();
        assert!(schema.query_msg.is_some());
        
        let query_msg = schema.query_msg.unwrap();
        assert_eq!(query_msg.title, Some("QueryMsg".to_string()));
        assert!(query_msg.is_enum);
    }

    #[test]
    fn test_template_manager_creation() {
        let result = super::templates::CosmosTemplateManager::new();
        assert!(result.is_ok());
        
        let manager = result.unwrap();
        let templates = manager.available_templates();
        assert!(!templates.is_empty());
    }

    #[tokio::test]
    async fn test_codegen_dry_run() {
        let config = CosmosCodegenConfig {
            contract_address: "cosmos1test123".to_string(),
            chain_id: "test-1".to_string(),
            output_dir: "./test_output".to_string(),
            namespace: None,
            features: vec!["client".to_string()],
            dry_run: true, // This should prevent file writes
        };

        let parser = CosmWasmMsgParser::new();
        let schema = parser.parse_content(r#"{"title": "TestMsg", "type": "object"}"#).unwrap();
        
        let codegen = CosmosContractCodegen::new(config);
        let result = codegen.generate_all(&schema).await;
        
        // Should succeed in dry run mode
        assert!(result.is_ok());
        
        // Verify no actual directory was created
        assert!(!tokio::fs::try_exists("./test_output").await.unwrap_or(true));
    }
} 