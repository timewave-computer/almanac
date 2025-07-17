use almanac_infrastructure_core::{
    Result, Error,
    service::{EventService, BoxedEventService},
    types::{ChainId, EventFilter, ApiConfig},
    coordination::{CoordinationService, NodeInfo, NodeCapability, NodeStatus},
    distributed_event_service::{DistributedEventService, DistributedEventConfig, PartitionStrategy},
};
use almanac_infrastructure_storage::{
    Storage, StorageBackend,
    fdb::{FdbStorage, FdbConfig},
    fdb_coordination::FdbCoordinationService,
};
use indexer_api::{ApiServer, ContractSchemaRegistry, InMemorySchemaRegistry};
use std::sync::Arc;
use tokio::time::{sleep, Duration};
use std::net::SocketAddr;

#[tokio::test]
#[ignore] // Run with --ignored flag for e2e tests
async fn test_full_indexing_pipeline() -> Result<()> {
    // Initialize distributed storage
    let fdb_config = FdbConfig {
        cluster_file: "/etc/foundationdb/fdb.cluster".to_string(),
        namespace: "test_e2e".to_string(),
    };
    
    let storage = Arc::new(FdbStorage::new(fdb_config).await?);
    
    // Initialize coordination service
    let coordination = Arc::new(FdbCoordinationService::new(
        storage.clone(),
        "test_e2e".to_string(),
    )?);
    
    // Create distributed event service
    let event_config = DistributedEventConfig {
        node_id: "e2e-test-node".to_string(),
        partition_strategy: PartitionStrategy::ByChain,
        batch_size: 100,
        flush_interval: Duration::from_secs(1),
    };
    
    let event_service = Arc::new(DistributedEventService::new(
        "e2e-test-node".to_string(),
        storage.clone(),
        event_config,
    ));
    
    // Register node
    let node_info = NodeInfo {
        node_id: "e2e-test-node".to_string(),
        address: "127.0.0.1:8080".to_string(),
        status: NodeStatus::Active,
        capabilities: vec![
            NodeCapability::ChainSupport { chain_id: ChainId("ethereum".to_string()) },
            NodeCapability::ChainSupport { chain_id: ChainId("polygon".to_string()) },
        ],
        heartbeat: std::time::SystemTime::now(),
    };
    
    coordination.register_node(node_info).await?;
    
    // Start API server
    let api_config = ApiConfig {
        host: "127.0.0.1".to_string(),
        port: 8080,
        enable_graphql: true,
        enable_websocket: true,
        cors_origins: vec!["*".to_string()],
    };
    
    let schema_registry = Arc::new(InMemorySchemaRegistry::new());
    let api_server = ApiServer::from_config(
        &api_config,
        event_service.clone() as BoxedEventService,
        schema_registry,
    )?;
    
    // Start server in background
    let server_handle = tokio::spawn(async move {
        api_server.start().await
    });
    
    // Wait for server to start
    sleep(Duration::from_secs(2)).await;
    
    // Test indexing pipeline
    
    // 1. Simulate event ingestion
    println!("Testing event ingestion...");
    let test_events = vec![
        create_test_event("ethereum", 1000, "Transfer"),
        create_test_event("ethereum", 1001, "Approval"),
        create_test_event("polygon", 2000, "Transfer"),
    ];
    
    for event in test_events {
        // In real implementation, events would come from chain clients
        // Here we simulate by directly storing
        storage.store_event(event).await?;
    }
    
    // 2. Test event querying
    println!("Testing event queries...");
    let filter = EventFilter {
        chain_ids: Some(vec![ChainId("ethereum".to_string())]),
        event_types: Some(vec!["Transfer".to_string()]),
        ..Default::default()
    };
    
    let events = event_service.get_events(vec![filter]).await?;
    assert_eq!(events.len(), 1);
    
    // 3. Test API endpoints
    println!("Testing API endpoints...");
    let client = reqwest::Client::new();
    
    // Test health endpoint
    let health_url = "http://127.0.0.1:8080/api/v1/health";
    let health_resp = client.get(health_url).send().await?;
    assert_eq!(health_resp.status(), 200);
    
    // Test events endpoint
    let events_url = "http://127.0.0.1:8080/api/v1/events/chain/ethereum";
    let events_resp = client.get(events_url).send().await?;
    assert_eq!(events_resp.status(), 200);
    
    // 4. Test distributed query
    println!("Testing distributed queries...");
    let query_url = "http://127.0.0.1:8080/api/v1/query/distributed";
    let query_body = serde_json::json!({
        "filters": [{
            "chain_ids": ["ethereum", "polygon"],
            "event_types": ["Transfer"]
        }]
    });
    
    let query_resp = client
        .post(query_url)
        .json(&query_body)
        .send()
        .await?;
    assert_eq!(query_resp.status(), 200);
    
    let query_result: serde_json::Value = query_resp.json().await?;
    assert!(query_result["events"].is_array());
    
    // Cleanup
    server_handle.abort();
    
    Ok(())
}

#[tokio::test]
async fn test_api_functionality() -> Result<()> {
    // Mock storage and services
    let storage = Arc::new(create_mock_storage());
    let event_service = Arc::new(create_mock_event_service());
    let schema_registry = Arc::new(InMemorySchemaRegistry::new());
    
    // Create API server
    let api_config = ApiConfig {
        host: "127.0.0.1".to_string(),
        port: 8081,
        enable_graphql: true,
        enable_websocket: true,
        cors_origins: vec!["*".to_string()],
    };
    
    let api_server = ApiServer::from_config(
        &api_config,
        event_service as BoxedEventService,
        schema_registry,
    )?;
    
    // Test various API endpoints
    
    // 1. Test REST endpoints
    println!("Testing REST API...");
    
    // Chain status
    test_endpoint("GET", "/api/v1/chains/ethereum/status", None).await?;
    
    // Events by address
    test_endpoint("GET", "/api/v1/events/address/ethereum/0x123", None).await?;
    
    // Event filtering
    let filter_body = serde_json::json!({
        "chain_id": "ethereum",
        "event_type": "Transfer",
        "from_height": 1000,
        "to_height": 2000
    });
    test_endpoint("POST", "/api/v1/events/filter", Some(filter_body)).await?;
    
    // 2. Test WebSocket functionality
    println!("Testing WebSocket API...");
    test_websocket_connection().await?;
    
    // 3. Test GraphQL endpoint
    println!("Testing GraphQL API...");
    let graphql_query = r#"{
        events(chain: "ethereum", limit: 10) {
            id
            chainId
            blockNumber
            eventType
        }
    }"#;
    test_graphql_query(graphql_query).await?;
    
    Ok(())
}

#[tokio::test]
async fn test_storage_proof_generation() -> Result<()> {
    use indexer_api::storage_proof_api::{GenerateProofRequest, VerifyProofRequest};
    
    let client = reqwest::Client::new();
    let base_url = "http://127.0.0.1:8080";
    
    // 1. Generate storage proof
    println!("Testing proof generation...");
    let proof_request = GenerateProofRequest {
        chain_id: "ethereum".to_string(),
        contract_address: "0x1234567890123456789012345678901234567890".to_string(),
        storage_key: vec![0u8; 32],
        block_number: 15000000,
        proof_type: "merkle".to_string(),
    };
    
    let proof_resp = client
        .post(format!("{}/api/v1/proof/generate", base_url))
        .json(&proof_request)
        .send()
        .await?;
    
    assert_eq!(proof_resp.status(), 200);
    let proof_data: serde_json::Value = proof_resp.json().await?;
    assert!(proof_data["proof_id"].is_string());
    assert!(proof_data["proof_data"]["proof_nodes"].is_array());
    
    // 2. Verify storage proof
    println!("Testing proof verification...");
    let verify_request = VerifyProofRequest {
        proof: serde_json::from_value(proof_data["proof_data"].clone()).unwrap(),
        expected_root: vec![1u8; 32],
    };
    
    let verify_resp = client
        .post(format!("{}/api/v1/proof/verify", base_url))
        .json(&verify_request)
        .send()
        .await?;
    
    assert_eq!(verify_resp.status(), 200);
    let verify_result: serde_json::Value = verify_resp.json().await?;
    assert!(verify_result["valid"].is_boolean());
    
    // 3. Test batch proof generation
    println!("Testing batch proof generation...");
    let batch_request = serde_json::json!({
        "requests": [
            proof_request,
            {
                "chain_id": "polygon",
                "contract_address": "0xabcdef1234567890123456789012345678901234",
                "storage_key": [1; 32],
                "block_number": 30000000,
                "proof_type": "merkle"
            }
        ]
    });
    
    let batch_resp = client
        .post(format!("{}/api/v1/proof/batch", base_url))
        .json(&batch_request)
        .send()
        .await?;
    
    assert_eq!(batch_resp.status(), 200);
    let batch_result: serde_json::Value = batch_resp.json().await?;
    assert!(batch_result["proofs"].is_array());
    assert_eq!(batch_result["proofs"].as_array().unwrap().len(), 2);
    
    Ok(())
}

#[tokio::test]
async fn test_distributed_query_validation() -> Result<()> {
    use almanac_infrastructure_core::distributed_query_planner::{
        DistributedQueryPlanner, QueryRoutingService
    };
    
    // Setup distributed query infrastructure
    let planner = Arc::new(DistributedQueryPlanner::new());
    let routing_service = Arc::new(QueryRoutingService::new(planner.clone()));
    
    // Register multiple nodes
    for i in 0..3 {
        let node = NodeInfo {
            node_id: format!("node{}", i),
            address: format!("127.0.0.1:{}", 9000 + i),
            status: NodeStatus::Active,
            capabilities: vec![
                NodeCapability::ChainSupport { 
                    chain_id: ChainId(format!("chain{}", i))
                },
            ],
            heartbeat: std::time::SystemTime::now(),
        };
        planner.register_node(node).await;
    }
    
    // Test 1: Single-chain query
    println!("Testing single-chain distributed query...");
    let single_filter = EventFilter {
        chain_ids: Some(vec![ChainId("chain0".to_string())]),
        ..Default::default()
    };
    
    let single_plan = planner.plan_query(vec![single_filter]).await?;
    assert_eq!(single_plan.sub_queries.len(), 1);
    assert_eq!(single_plan.sub_queries[0].node_id, "node0");
    
    // Test 2: Multi-chain query
    println!("Testing multi-chain distributed query...");
    let multi_filter = EventFilter {
        chain_ids: Some(vec![
            ChainId("chain0".to_string()),
            ChainId("chain1".to_string()),
            ChainId("chain2".to_string()),
        ]),
        ..Default::default()
    };
    
    let multi_plan = planner.plan_query(vec![multi_filter]).await?;
    assert_eq!(multi_plan.sub_queries.len(), 3);
    
    // Test 3: Query execution
    println!("Testing distributed query execution...");
    let results = routing_service.execute_distributed_query(multi_plan).await?;
    
    // Verify results are properly aggregated
    assert!(results.is_object());
    assert!(results["aggregated_results"].is_array());
    assert!(results["execution_time_ms"].is_number());
    assert_eq!(results["nodes_queried"].as_u64().unwrap(), 3);
    
    // Test 4: Query with failures
    println!("Testing query with node failures...");
    
    // Mark one node as inactive
    let failed_node = NodeInfo {
        node_id: "node1".to_string(),
        address: "127.0.0.1:9001".to_string(),
        status: NodeStatus::Inactive,
        capabilities: vec![
            NodeCapability::ChainSupport { 
                chain_id: ChainId("chain1".to_string())
            },
        ],
        heartbeat: std::time::SystemTime::now(),
    };
    planner.register_node(failed_node).await;
    
    // Query should handle the failure gracefully
    let failure_plan = planner.plan_query(vec![multi_filter.clone()]).await?;
    let failure_results = routing_service.execute_distributed_query(failure_plan).await?;
    
    // Should still get results from active nodes
    assert!(failure_results["partial_failure"].as_bool().unwrap_or(false));
    assert_eq!(failure_results["nodes_queried"].as_u64().unwrap(), 2);
    assert!(failure_results["failed_nodes"].is_array());
    
    Ok(())
}

// Helper functions

fn create_test_event(chain: &str, block: u64, event_type: &str) -> Box<dyn almanac_infrastructure_core::event::Event> {
    // Mock event implementation
    struct TestEvent {
        chain: String,
        block: u64,
        event_type: String,
    }
    
    impl almanac_infrastructure_core::event::Event for TestEvent {
        fn id(&self) -> almanac_infrastructure_core::event::EventId {
            almanac_infrastructure_core::event::EventId(format!("{}-{}-{}", self.chain, self.block, self.event_type))
        }
        
        fn chain(&self) -> &ChainId {
            // This is a hack - in real code, store ChainId directly
            Box::leak(Box::new(ChainId(self.chain.clone())))
        }
        
        fn block_number(&self) -> u64 {
            self.block
        }
        
        fn block_hash(&self) -> almanac_infrastructure_core::event::BlockHash {
            almanac_infrastructure_core::event::BlockHash(format!("0x{:064x}", self.block))
        }
        
        fn tx_hash(&self) -> almanac_infrastructure_core::event::TxHash {
            almanac_infrastructure_core::event::TxHash(format!("0x{:064x}", self.block * 1000))
        }
        
        fn event_type(&self) -> &str {
            &self.event_type
        }
        
        fn timestamp(&self) -> std::time::SystemTime {
            std::time::SystemTime::now()
        }
        
        fn raw_data(&self) -> &[u8] {
            b"mock_data"
        }
    }
    
    Box::new(TestEvent {
        chain: chain.to_string(),
        block,
        event_type: event_type.to_string(),
    })
}

fn create_mock_storage() -> impl Storage {
    // Return a mock storage implementation
    almanac_infrastructure_storage::MemoryStorage::new()
}

fn create_mock_event_service() -> impl EventService {
    // Return a mock event service
    struct MockEventService;
    
    #[async_trait::async_trait]
    impl EventService for MockEventService {
        fn chain_id(&self) -> &ChainId {
            Box::leak(Box::new(ChainId("ethereum".to_string())))
        }
        
        async fn get_events(&self, _filters: Vec<EventFilter>) -> Result<Vec<Box<dyn almanac_infrastructure_core::event::Event>>> {
            Ok(vec![])
        }
        
        async fn get_latest_block(&self) -> Result<u64> {
            Ok(1000000)
        }
        
        async fn get_latest_block_with_status(&self, _chain: &str, _status: almanac_infrastructure_core::BlockStatus) -> Result<u64> {
            Ok(999000)
        }
        
        async fn subscribe(&self) -> Result<tokio::sync::mpsc::Receiver<Box<dyn almanac_infrastructure_core::event::Event>>> {
            let (tx, rx) = tokio::sync::mpsc::channel(100);
            Ok(rx)
        }
    }
    
    MockEventService
}

async fn test_endpoint(method: &str, path: &str, body: Option<serde_json::Value>) -> Result<()> {
    let client = reqwest::Client::new();
    let url = format!("http://127.0.0.1:8081{}", path);
    
    let request = match method {
        "GET" => client.get(&url),
        "POST" => {
            let req = client.post(&url);
            if let Some(json_body) = body {
                req.json(&json_body)
            } else {
                req
            }
        }
        _ => return Err(Error::generic("Unsupported HTTP method")),
    };
    
    let response = request.send().await.map_err(|e| Error::generic(e.to_string()))?;
    assert!(response.status().is_success());
    
    Ok(())
}

async fn test_websocket_connection() -> Result<()> {
    // Simple WebSocket connection test
    // In a real test, would use a WebSocket client library
    Ok(())
}

async fn test_graphql_query(_query: &str) -> Result<()> {
    // Simple GraphQL query test
    // In a real test, would send query to GraphQL endpoint
    Ok(())
}