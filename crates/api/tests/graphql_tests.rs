/// Tests for the GraphQL API
use async_trait::async_trait;
use async_graphql::*;
use std::sync::Arc;
use std::any::Any;
use std::collections::VecDeque;
use std::fmt;
use std::time::SystemTime;
use indexer_core::{
    service::{EventService as CoreEventService, EventSubscription},
    event::Event,
    types::{ChainId, EventFilter},
};
use indexer_pipeline::BlockStatus;
use indexer_api::graphql::create_schema;
use indexer_api::{ContractSchemaRegistry, InMemorySchemaRegistry};
use async_graphql::{Schema, EmptyMutation, EmptySubscription};

use indexer_core::service::{BoxedEventService, EventServiceWrapper};

fn create_wrapped_service(mock_service: MockEventService) -> BoxedEventService {
    Arc::new(EventServiceWrapper::new(Arc::new(mock_service)))
}

// Create a basic mock for the Event trait
#[derive(Clone)]
struct MockEvent {
    id: String,
    chain: String,
    block_number: u64,
    block_hash: String,
    tx_hash: String,
    timestamp: SystemTime,
    event_type: String,
    data: Vec<u8>,
}

impl Event for MockEvent {
    fn id(&self) -> &str {
        &self.id
    }
    
    fn chain(&self) -> &str {
        &self.chain
    }
    
    fn block_number(&self) -> u64 {
        self.block_number
    }
    
    fn block_hash(&self) -> &str {
        &self.block_hash
    }
    
    fn tx_hash(&self) -> &str {
        &self.tx_hash
    }
    
    fn timestamp(&self) -> SystemTime {
        self.timestamp
    }
    
    fn event_type(&self) -> &str {
        &self.event_type
    }
    
    fn raw_data(&self) -> &[u8] {
        &self.data
    }
    
    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl fmt::Debug for MockEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MockEvent")
            .field("id", &self.id)
            .field("chain", &self.chain)
            .field("block_number", &self.block_number)
            .field("event_type", &self.event_type)
            .finish()
    }
}

impl MockEvent {
    fn new(id: &str, chain: &str, event_type: &str) -> Self {
        Self {
            id: id.to_string(),
            chain: chain.to_string(),
            block_number: 1000,
            block_hash: "0x123".to_string(),
            tx_hash: "0xabc".to_string(),
            timestamp: SystemTime::now(),
            event_type: event_type.to_string(),
            data: b"test data".to_vec(),
        }
    }
}

// Create a subscription implementation
struct MockSubscription {
    events: VecDeque<Box<dyn Event>>,
}

impl MockSubscription {
    fn new(events: Vec<Box<dyn Event>>) -> Self {
        Self {
            events: events.into(),
        }
    }
}

#[async_trait]
impl EventSubscription for MockSubscription {
    async fn next(&mut self) -> Option<Box<dyn Event>> {
        self.events.pop_front()
    }
    
    async fn close(&mut self) -> indexer_pipeline::Result<()> {
        Ok(())
    }
}

// Create a mock event service
struct MockEventService {
    chain_id: ChainId,
    latest_block: u64,
}

impl MockEventService {
    fn new(chain_id: String, latest_block: u64) -> Self {
        Self {
            chain_id: ChainId(chain_id),
            latest_block,
        }
    }
    
    fn chain_id(&self) -> &ChainId {
        &self.chain_id
    }
}

#[async_trait]
impl indexer_core::service::EventService for MockEventService {
    type EventType = MockEvent;
    
    fn chain_id(&self) -> &ChainId {
        &self.chain_id
    }
    
    async fn get_events(&self, _filters: Vec<EventFilter>) -> indexer_pipeline::Result<Vec<Box<dyn Event>>> {
        let mut events: Vec<Box<dyn Event>> = Vec::new();
        events.push(Box::new(MockEvent::new("1", &self.chain_id.0, "Transfer")));
        events.push(Box::new(MockEvent::new("2", &self.chain_id.0, "Approval")));
        Ok(events)
    }
    
    async fn get_latest_block(&self) -> indexer_pipeline::Result<u64> {
        Ok(self.latest_block)
    }
    
    async fn get_latest_block_with_status(&self, _chain: &str, status: BlockStatus) -> indexer_pipeline::Result<u64> {
        match status {
            BlockStatus::Finalized => Ok(self.latest_block - 50),
            BlockStatus::Safe => Ok(self.latest_block - 20),
            _ => Ok(self.latest_block),
        }
    }
    
    async fn subscribe(&self) -> indexer_pipeline::Result<Box<dyn EventSubscription>> {
        // Create some events for the subscription
        let events = self.get_events(Vec::new()).await?;
        Ok(Box::new(MockSubscription::new(events)))
    }
}

// Test creating the GraphQL schema
#[tokio::test]
async fn test_schema_creation() {
    // Create a mock event service
    let event_service = create_wrapped_service(MockEventService::new("ethereum".to_string(), 1000));
    
    // Create a schema registry
    let schema_registry = Arc::new(InMemorySchemaRegistry::new());
    
    // Create the GraphQL schema
    let schema = create_schema(event_service, schema_registry);
    
    // Check that the schema was created
    let sdl = schema.sdl();
    assert!(sdl.contains("QueryRoot"));
    assert!(sdl.contains("MutationRoot"));
}

// Test health query
#[tokio::test]
async fn test_health_query() {
    // Create a mock event service
    let event_service = create_wrapped_service(MockEventService::new("ethereum".to_string(), 1000));
    
    // Create a schema registry
    let schema_registry = Arc::new(InMemorySchemaRegistry::new());
    
    // Create the GraphQL schema
    let schema = create_schema(event_service, schema_registry);
    
    // Define a GraphQL query
    let query = r#"
    query {
        health {
            healthy
            version
        }
    }
    "#;
    
    // Execute the query
    let res = schema.execute(query).await;
    
    // Check for success
    assert!(res.is_ok());
    
    // Check the result
    let data = res.data.into_json().unwrap();
    assert_eq!(data["health"]["healthy"], true);
    assert!(data["health"]["version"].as_str().is_some());
}

// Test schema registration mutation
#[tokio::test]
async fn test_register_schema_mutation() {
    // Create a mock event service
    let event_service = create_wrapped_service(MockEventService::new("ethereum".to_string(), 1000));
    
    // Create a schema registry
    let schema_registry = Arc::new(InMemorySchemaRegistry::new());
    
    // Create the GraphQL schema
    let schema = create_schema(event_service, schema_registry.clone());
    
    // Define a GraphQL mutation
    let mutation = r#"
    mutation {
        registerContractSchema(input: {
            chainId: "ethereum",
            contractAddress: "0xabc123",
            version: "1.0.0",
            name: "TestToken",
            events: [
                {
                    name: "Transfer",
                    fields: [
                        { name: "from", typeName: "address", indexed: true },
                        { name: "to", typeName: "address", indexed: true },
                        { name: "value", typeName: "uint256", indexed: false }
                    ]
                }
            ],
            functions: []
        }) {
            success
            error
            schemaVersion {
                version
                schema {
                    name
                    chain
                    address
                }
            }
        }
    }
    "#;
    
    // Execute the mutation
    let res = schema.execute(mutation).await;
    
    // Check for success
    assert!(res.is_ok());
    
    // Check the result
    let data = res.data.into_json().unwrap();
    assert_eq!(data["registerContractSchema"]["success"], true);
    assert_eq!(data["registerContractSchema"]["error"], serde_json::Value::Null);
    assert_eq!(data["registerContractSchema"]["schemaVersion"]["version"], "1.0.0");
    assert_eq!(data["registerContractSchema"]["schemaVersion"]["schema"]["name"], "TestToken");
    
    // Check that the schema was actually registered in the registry
    let schema = schema_registry.get_schema("ethereum", "0xabc123", "1.0.0").unwrap();
    assert!(schema.is_some());
    let schema = schema.unwrap();
    assert_eq!(schema.name, "TestToken");
    assert_eq!(schema.events.len(), 1);
    assert_eq!(schema.events[0].name, "Transfer");
}

// Test query for contract schema
#[tokio::test]
async fn test_schema_query() {
    // Create a mock event service
    let event_service = create_wrapped_service(MockEventService::new("ethereum".to_string(), 1000));
    
    // Create a schema registry
    let schema_registry = Arc::new(InMemorySchemaRegistry::new());
    
    // Register a schema
    let schema_version = indexer_api::ContractSchemaVersion {
        version: "1.0.0".to_string(),
        schema: indexer_api::ContractSchema {
            chain: "ethereum".to_string(),
            address: "0xdef456".to_string(),
            name: "QueryTest".to_string(),
            events: vec![],
            functions: vec![],
        },
    };
    schema_registry.register_schema(schema_version).unwrap();
    
    // Create the GraphQL schema
    let schema = create_schema(event_service, schema_registry);
    
    // Define a GraphQL query
    let query = r#"
    query {
        contractSchema(chain: "ethereum", address: "0xdef456") {
            name
            chain
            address
        }
    }
    "#;
    
    // Execute the query
    let res = schema.execute(query).await;
    
    // Check for success
    assert!(res.is_ok());
    
    // Check the result
    let data = res.data.into_json().unwrap();
    assert_eq!(data["contractSchema"]["name"], "QueryTest");
    assert_eq!(data["contractSchema"]["chain"], "ethereum");
    assert_eq!(data["contractSchema"]["address"], "0xdef456");
}

// Test context for GraphQL tests
struct GraphQlTestContext {
    schema: async_graphql::Schema<indexer_api::graphql::QueryRoot, indexer_api::graphql::MutationRoot, EmptySubscription>,
}

impl GraphQlTestContext {
    async fn query(&self, query: &str) -> indexer_pipeline::Result<async_graphql::Response> {
        Ok(self.schema.execute(query).await)
    }
}

async fn setup_graphql_test() -> indexer_pipeline::Result<GraphQlTestContext> {
    let event_service = create_wrapped_service(MockEventService::new("ethereum".to_string(), 1000));
    let schema_registry = Arc::new(InMemorySchemaRegistry::new());
    let schema = create_schema(event_service, schema_registry);
    
    Ok(GraphQlTestContext { schema })
}

mod tests {
    use super::*;
    use indexer_core::types::ChainId;
    use serde_json::json;

    #[tokio::test]
    async fn test_query_events() -> indexer_pipeline::Result<()> {
        let context = setup_graphql_test().await?;

        // Test querying events
        let query = r#"
            query {
                events(filter: {chain: "eth"}) {
                    id
                    chain
                    event_type
                }
            }
        "#;

        let response = context.query(query).await?;
        let json = response.data.into_json().unwrap();

        // The most basic check - the response should contain data
        assert!(json.is_object());

        // Now check the events data
        let events = json["events"].as_array().unwrap();
        assert_eq!(events.len(), 2);
        assert_eq!(events[0]["chain"], "eth");
        assert_eq!(events[0]["event_type"], "Transfer");
        assert_eq!(events[1]["chain"], "eth");
        assert_eq!(events[1]["event_type"], "Approval");

        Ok(())
    }

    #[tokio::test]
    async fn test_get_latest_block() -> indexer_pipeline::Result<()> {
        let mock_service = MockEventService::new("eth".into(), 100);
        
        // Test the direct implementation
        assert_eq!(mock_service.get_latest_block().await?, 100);
        assert_eq!(
            mock_service.get_latest_block_with_status("eth", BlockStatus::Finalized).await?,
            50
        );
        assert_eq!(
            mock_service.get_latest_block_with_status("eth", BlockStatus::Safe).await?,
            80
        );
        assert_eq!(
            mock_service.get_latest_block_with_status("eth", BlockStatus::Confirmed).await?,
            100
        );

        // Test through the wrapper
        let wrapped = create_wrapped_service(mock_service);
        assert_eq!(wrapped.get_latest_block().await?, 100);
        assert_eq!(
            wrapped.get_latest_block_with_status("eth", BlockStatus::Finalized).await?,
            50
        );
        assert_eq!(
            wrapped.get_latest_block_with_status("eth", BlockStatus::Safe).await?,
            80
        );
        assert_eq!(
            wrapped.get_latest_block_with_status("eth", BlockStatus::Confirmed).await?,
            100
        );

        Ok(())
    }
} 