// Example of using the distributed query planner
//
// This example demonstrates how to set up and use the distributed query planner
// with multiple nodes for executing queries across a distributed indexer cluster.

use std::sync::Arc;
use std::collections::HashMap;
use tokio;
use tracing::{info, Level};
use tracing_subscriber;

use indexer_core::{
    Result,
    coordination::{CoordinationService, NodeInfo, NodeCapabilities, NodeStatus},
    distributed_query_planner::{
        DistributedQuery, DistributedQueryPlanner, QueryHints, DataLocality,
        QueryRoutingService, QueryExecutor, SubQuery, QueryResult, QueryExecutionMetadata,
    },
    types::ChainId,
};
use indexer_query::{BaseQuery, QueryFilter, Pagination, Sorting, SortDirection};
use indexer_storage::fdb_coordination::FdbCoordinationService;
use foundationdb::Database;
use async_trait::async_trait;
use serde_json;

/// Example query executor that simulates querying a local node
struct ExampleQueryExecutor {
    node_id: String,
    // In a real implementation, this would have access to local storage
}

#[async_trait]
impl QueryExecutor for ExampleQueryExecutor {
    async fn execute_query<T>(&self, query: SubQuery) -> Result<QueryResult<T>>
    where
        T: serde::de::DeserializeOwned + Send + Sync,
    {
        info!("Node {} executing query with filter: {:?}", self.node_id, query.filter);
        
        // Simulate query execution
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        
        // Return mock results
        Ok(QueryResult {
            node_id: self.node_id.clone(),
            data: Vec::new(), // In real implementation, would contain actual results
            metadata: QueryExecutionMetadata {
                execution_time_ms: 100,
                records_scanned: 1000,
                cache_hit_rate: 0.25,
                warnings: Vec::new(),
            },
        })
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .init();
    
    info!("Starting distributed query example");
    
    // Create FoundationDB connection (this would fail if FDB is not running)
    let db = match Database::default() {
        Ok(db) => db,
        Err(_) => {
            info!("FoundationDB not available, using mock coordination service");
            return run_with_mock_coordination().await;
        }
    };
    
    // Create coordination service
    let coordination = Arc::new(FdbCoordinationService::new(db, "example".to_string()));
    
    // Register some example nodes
    register_example_nodes(&coordination).await?;
    
    // Create distributed query planner
    let query_planner = Arc::new(DistributedQueryPlanner::new(coordination.clone()));
    
    // Update data locality for nodes
    update_data_locality(&query_planner).await;
    
    // Create query routing service
    let routing_service = Arc::new(QueryRoutingService::new(query_planner.clone()));
    
    // Register query executors for each node
    register_query_executors(&routing_service).await;
    
    // Execute some example queries
    execute_example_queries(&routing_service).await?;
    
    Ok(())
}

/// Run example with mock coordination service when FDB is not available
async fn run_with_mock_coordination() -> Result<()> {
    info!("Running with mock coordination service");
    
    // Create mock coordination service
    let coordination = Arc::new(MockCoordinationService::new());
    
    // Register some example nodes
    register_example_nodes(&coordination).await?;
    
    // Create distributed query planner
    let query_planner = Arc::new(DistributedQueryPlanner::new(coordination.clone()));
    
    // Update data locality for nodes
    update_data_locality(&query_planner).await;
    
    // Create and execute example queries
    execute_example_queries_with_planner(&query_planner).await?;
    
    Ok(())
}

/// Register example nodes with the coordination service
async fn register_example_nodes(coordination: &Arc<dyn CoordinationService>) -> Result<()> {
    info!("Registering example nodes");
    
    // Register node 1 - handles Ethereum data
    let node1 = NodeInfo {
        node_id: "node-1".to_string(),
        address: "127.0.0.1:8001".to_string(),
        capabilities: NodeCapabilities {
            supported_chains: vec!["ethereum".to_string()],
            max_throughput: 10000,
            storage_backends: vec!["postgres".to_string(), "rocksdb".to_string()],
            api_endpoints: vec!["rest".to_string()],
        },
        status: NodeStatus::Active,
        last_heartbeat: current_timestamp(),
        metadata: HashMap::new(),
    };
    coordination.register_node(node1).await?;
    
    // Register node 2 - handles Cosmos data
    let node2 = NodeInfo {
        node_id: "node-2".to_string(),
        address: "127.0.0.1:8002".to_string(),
        capabilities: NodeCapabilities {
            supported_chains: vec!["cosmos".to_string(), "osmosis".to_string()],
            max_throughput: 8000,
            storage_backends: vec!["postgres".to_string()],
            api_endpoints: vec!["rest".to_string()],
        },
        status: NodeStatus::Active,
        last_heartbeat: current_timestamp(),
        metadata: HashMap::new(),
    };
    coordination.register_node(node2).await?;
    
    // Register node 3 - handles both chains
    let node3 = NodeInfo {
        node_id: "node-3".to_string(),
        address: "127.0.0.1:8003".to_string(),
        capabilities: NodeCapabilities {
            supported_chains: vec!["ethereum".to_string(), "cosmos".to_string()],
            max_throughput: 15000,
            storage_backends: vec!["postgres".to_string(), "rocksdb".to_string(), "fdb".to_string()],
            api_endpoints: vec!["rest".to_string(), "websocket".to_string()],
        },
        status: NodeStatus::Active,
        last_heartbeat: current_timestamp(),
        metadata: HashMap::new(),
    };
    coordination.register_node(node3).await?;
    
    info!("Registered 3 example nodes");
    Ok(())
}

/// Update data locality information for nodes
async fn update_data_locality(planner: &Arc<DistributedQueryPlanner>) {
    info!("Updating data locality information");
    
    // Node 1 has Ethereum blocks 1-1000000
    planner.update_data_locality(
        "node-1".to_string(),
        DataLocality {
            chains: vec![ChainId::from("ethereum")],
            block_ranges: {
                let mut ranges = HashMap::new();
                ranges.insert(ChainId::from("ethereum"), vec![(1, 1000000)]);
                ranges
            },
            contracts: Some(vec![
                "0x1234567890123456789012345678901234567890".to_string(),
                "0xabcdefabcdefabcdefabcdefabcdefabcdefabcd".to_string(),
            ]),
            cached_queries: vec!["recent_transfers".to_string()],
        },
    ).await;
    
    // Node 2 has Cosmos blocks 1-500000
    planner.update_data_locality(
        "node-2".to_string(),
        DataLocality {
            chains: vec![ChainId::from("cosmos"), ChainId::from("osmosis")],
            block_ranges: {
                let mut ranges = HashMap::new();
                ranges.insert(ChainId::from("cosmos"), vec![(1, 500000)]);
                ranges.insert(ChainId::from("osmosis"), vec![(1, 300000)]);
                ranges
            },
            contracts: None,
            cached_queries: vec!["ibc_transfers".to_string()],
        },
    ).await;
    
    // Node 3 has recent blocks for both chains
    planner.update_data_locality(
        "node-3".to_string(),
        DataLocality {
            chains: vec![ChainId::from("ethereum"), ChainId::from("cosmos")],
            block_ranges: {
                let mut ranges = HashMap::new();
                ranges.insert(ChainId::from("ethereum"), vec![(900000, 1100000)]);
                ranges.insert(ChainId::from("cosmos"), vec![(450000, 550000)]);
                ranges
            },
            contracts: None,
            cached_queries: vec!["cross_chain_messages".to_string()],
        },
    ).await;
    
    info!("Data locality updated for all nodes");
}

/// Register query executors for each node
async fn register_query_executors(routing_service: &Arc<QueryRoutingService>) {
    info!("Registering query executors");
    
    routing_service.register_executor(
        "node-1".to_string(),
        Arc::new(ExampleQueryExecutor { node_id: "node-1".to_string() }),
    ).await;
    
    routing_service.register_executor(
        "node-2".to_string(),
        Arc::new(ExampleQueryExecutor { node_id: "node-2".to_string() }),
    ).await;
    
    routing_service.register_executor(
        "node-3".to_string(),
        Arc::new(ExampleQueryExecutor { node_id: "node-3".to_string() }),
    ).await;
    
    info!("Query executors registered");
}

/// Execute example queries
async fn execute_example_queries(routing_service: &Arc<QueryRoutingService>) -> Result<()> {
    info!("Executing example queries");
    
    // Query 1: Get Ethereum events
    let query1 = DistributedQuery {
        base: BaseQuery {
            filter: Some(QueryFilter {
                chain_ids: Some(vec![ChainId::from("ethereum")]),
                contract_addresses: Some(vec!["0x1234567890123456789012345678901234567890".to_string()]),
                contract_types: None,
                entity_ids: None,
                attributes: None,
                tags: None,
            }),
            pagination: Some(Pagination { limit: 100, offset: 0 }),
            sorting: Some(Sorting {
                field: "block_number".to_string(),
                direction: SortDirection::Descending,
            }),
        },
        hints: QueryHints {
            preferred_nodes: None,
            required_capabilities: None,
            max_parallel: Some(2),
            allow_partial_results: false,
        },
        timeout_ms: 5000,
        use_cache: true,
    };
    
    info!("Executing Ethereum query");
    let results1: Vec<serde_json::Value> = routing_service.execute_query(query1).await?;
    info!("Query 1 returned {} results", results1.len());
    
    // Query 2: Cross-chain query
    let query2 = DistributedQuery {
        base: BaseQuery {
            filter: Some(QueryFilter {
                chain_ids: Some(vec![ChainId::from("ethereum"), ChainId::from("cosmos")]),
                contract_addresses: None,
                contract_types: None,
                entity_ids: None,
                attributes: Some({
                    let mut attrs = HashMap::new();
                    attrs.insert("event_type".to_string(), "transfer".to_string());
                    attrs
                }),
                tags: Some(vec!["cross-chain".to_string()]),
            }),
            pagination: Some(Pagination { limit: 50, offset: 0 }),
            sorting: None,
        },
        hints: QueryHints {
            preferred_nodes: Some(vec!["node-3".to_string()]), // Prefer node that has both chains
            required_capabilities: None,
            max_parallel: Some(3),
            allow_partial_results: true,
        },
        timeout_ms: 10000,
        use_cache: false,
    };
    
    info!("Executing cross-chain query");
    let results2: Vec<serde_json::Value> = routing_service.execute_query(query2).await?;
    info!("Query 2 returned {} results", results2.len());
    
    Ok(())
}

/// Execute example queries using just the planner (for mock mode)
async fn execute_example_queries_with_planner(planner: &Arc<DistributedQueryPlanner>) -> Result<()> {
    info!("Creating query plans");
    
    // Create a simple query
    let query = DistributedQuery {
        base: BaseQuery {
            filter: Some(QueryFilter {
                chain_ids: Some(vec![ChainId::from("ethereum")]),
                contract_addresses: None,
                contract_types: None,
                entity_ids: None,
                attributes: None,
                tags: None,
            }),
            pagination: Some(Pagination { limit: 100, offset: 0 }),
            sorting: None,
        },
        hints: QueryHints::default(),
        timeout_ms: 5000,
        use_cache: true,
    };
    
    // Create query plan
    let plan = planner.create_plan(query).await?;
    
    info!("Created query plan: {}", plan.plan_id);
    info!("Plan will query {} nodes", plan.sub_queries.len());
    for sub_query in &plan.sub_queries {
        info!(
            "  - Node {}: estimated cost {:.2}ms",
            sub_query.node_id,
            sub_query.estimated_cost.estimated_time_ms
        );
    }
    
    Ok(())
}

/// Get current timestamp
fn current_timestamp() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

/// Mock coordination service for testing
struct MockCoordinationService {
    nodes: Arc<tokio::sync::RwLock<Vec<NodeInfo>>>,
}

impl MockCoordinationService {
    fn new() -> Self {
        Self {
            nodes: Arc::new(tokio::sync::RwLock::new(Vec::new())),
        }
    }
}

#[async_trait]
impl CoordinationService for MockCoordinationService {
    async fn register_node(&self, info: NodeInfo) -> Result<()> {
        let mut nodes = self.nodes.write().await;
        nodes.push(info);
        Ok(())
    }
    
    async fn update_node_status(&self, node_id: &str, status: NodeStatus) -> Result<()> {
        let mut nodes = self.nodes.write().await;
        if let Some(node) = nodes.iter_mut().find(|n| n.node_id == node_id) {
            node.status = status;
        }
        Ok(())
    }
    
    async fn get_active_nodes(&self) -> Result<Vec<NodeInfo>> {
        let nodes = self.nodes.read().await;
        Ok(nodes.iter()
            .filter(|n| matches!(n.status, NodeStatus::Active | NodeStatus::Degraded))
            .cloned()
            .collect())
    }
    
    async fn assign_work(&self, _assignment: indexer_core::coordination::WorkAssignment) -> Result<()> {
        Ok(())
    }
    
    async fn get_node_assignments(&self, _node_id: &str) -> Result<Vec<indexer_core::coordination::WorkAssignment>> {
        Ok(Vec::new())
    }
    
    async fn complete_assignment(&self, _assignment_id: &str) -> Result<()> {
        Ok(())
    }
    
    async fn elect_leader(&self, _candidate_id: &str) -> Result<bool> {
        Ok(true)
    }
    
    async fn get_leader(&self) -> Result<Option<indexer_core::coordination::LeaderInfo>> {
        Ok(None)
    }
    
    async fn release_assignment(&self, _assignment_id: &str) -> Result<()> {
        Ok(())
    }
}