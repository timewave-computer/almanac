/// HTTP API integration tests
use std::sync::Arc;

use indexer_api::{
    InMemorySchemaRegistry, ContractSchemaRegistry,
};

#[tokio::test]
async fn test_schema_registry() {
    let registry = InMemorySchemaRegistry::new();
    
    // Test basic registry functionality
    let schemas = registry.get_schemas(None, 10, 0).await.unwrap();
    assert!(schemas.is_empty());
}

#[test]
fn test_api_compilation() {
    // Basic test to verify the API crate compiles correctly
    assert!(true);
} 