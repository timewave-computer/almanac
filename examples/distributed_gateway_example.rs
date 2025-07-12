/// Example demonstrating the enhanced API gateway with distributed query support
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use indexer_api::{
    AuthState, InMemorySchemaRegistry,
    http_enhanced::{EnhancedHttpConfig, start_enhanced_http_server, RateLimitConfig},
    distributed_gateway::{DistributedQueryRequest, QueryFilterRequest, PaginationRequest, QueryHints},
};
use indexer_core::{
    service::{EventService, BoxedEventService},
    storage::postgres::PostgresStorage,
    coordination::{CoordinationService, NodeInfo, NodeStatus, NodeCapabilities, WorkAssignment, LeaderInfo},
    types::{ChainId, DatabaseConfig},
    Result, Error,
};

use async_trait::async_trait;
use reqwest;
use serde_json::json;
use tracing::{info, Level};
use tracing_subscriber;

/// Mock coordination service for the example
struct MockCoordinationService {
    nodes: Vec<NodeInfo>,
}

#[async_trait]
impl CoordinationService for MockCoordinationService {
    async fn register_node(&self, _info: NodeInfo) -> Result<()> {
        Ok(())
    }
    
    async fn update_node_status(&self, _node_id: &str, _status: NodeStatus) -> Result<()> {
        Ok(())
    }
    
    async fn get_active_nodes(&self) -> Result<Vec<NodeInfo>> {
        Ok(self.nodes.clone())
    }
    
    async fn assign_work(&self, _assignment: WorkAssignment) -> Result<()> {
        Ok(())
    }
    
    async fn get_node_assignments(&self, _node_id: &str) -> Result<Vec<WorkAssignment>> {
        Ok(Vec::new())
    }
    
    async fn complete_assignment(&self, _assignment_id: &str) -> Result<()> {
        Ok(())
    }
    
    async fn elect_leader(&self, _candidate_id: &str) -> Result<bool> {
        Ok(true)
    }
    
    async fn get_leader(&self) -> Result<Option<LeaderInfo>> {
        Ok(None)
    }
    
    async fn release_assignment(&self, _assignment_id: &str) -> Result<()> {
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .init();
    
    info!("Starting distributed gateway example");
    
    // Create mock nodes for the example
    let nodes = vec![
        NodeInfo {
            node_id: "node-1".to_string(),
            address: "10.0.0.1:8080".to_string(),
            capabilities: NodeCapabilities {
                supported_chains: vec!["ethereum".to_string(), "polygon".to_string()],
                max_throughput: 10000,
                storage_backends: vec!["postgres".to_string(), "rocksdb".to_string()],
                api_endpoints: vec!["rest".to_string(), "graphql".to_string(), "websocket".to_string()],
            },
            status: NodeStatus::Active,
            last_heartbeat: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            metadata: HashMap::new(),
        },
        NodeInfo {
            node_id: "node-2".to_string(),
            address: "10.0.0.2:8080".to_string(),
            capabilities: NodeCapabilities {
                supported_chains: vec!["ethereum".to_string(), "arbitrum".to_string()],
                max_throughput: 15000,
                storage_backends: vec!["postgres".to_string()],
                api_endpoints: vec!["rest".to_string(), "graphql".to_string()],
            },
            status: NodeStatus::Active,
            last_heartbeat: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            metadata: HashMap::new(),
        },
        NodeInfo {
            node_id: "node-3".to_string(),
            address: "10.0.0.3:8080".to_string(),
            capabilities: NodeCapabilities {
                supported_chains: vec!["cosmos".to_string(), "osmosis".to_string()],
                max_throughput: 8000,
                storage_backends: vec!["postgres".to_string()],
                api_endpoints: vec!["rest".to_string()],
            },
            status: NodeStatus::Active,
            last_heartbeat: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            metadata: HashMap::new(),
        },
    ];
    
    // Create coordination service
    let coordination = Arc::new(MockCoordinationService { nodes });
    
    // Create storage (would use real database in production)
    let db_config = DatabaseConfig {
        url: "postgresql://user:password@localhost/indexer".to_string(),
        max_connections: 10,
        min_connections: 1,
        connect_timeout: Duration::from_secs(5),
        idle_timeout: Some(Duration::from_secs(300)),
        max_lifetime: Some(Duration::from_secs(3600)),
    };
    
    // For the example, we'll use a mock event service
    let storage = Arc::new(PostgresStorage::new(db_config).await?);
    let event_service: BoxedEventService = Box::new(
        EventService::new(ChainId("ethereum".to_string()), storage)
    );
    
    // Create schema registry
    let schema_registry = Arc::new(InMemorySchemaRegistry::new());
    
    // Create auth state
    let auth_state = AuthState::new(b"example-secret-key");
    
    // Configure the enhanced HTTP server
    let config = EnhancedHttpConfig {
        http_addr: "0.0.0.0:8080".parse().unwrap(),
        enable_distributed: true,
        node_cache_ttl: Duration::from_secs(30),
        rate_limit: RateLimitConfig {
            requests_per_window: 10000,
            window_duration: Duration::from_secs(60),
            sync_threshold: 100,
        },
        enable_cors: true,
        enable_tracing: true,
    };
    
    // Start the server in a background task
    let server_handle = tokio::spawn(async move {
        if let Err(e) = start_enhanced_http_server(
            config,
            event_service,
            schema_registry,
            auth_state,
            coordination,
        ).await {
            eprintln!("Server error: {}", e);
        }
    });
    
    // Wait for server to start
    tokio::time::sleep(Duration::from_secs(2)).await;
    
    // Demonstrate various API endpoints
    demonstrate_api_usage().await?;
    
    // Keep the server running
    info!("Server is running. Press Ctrl+C to stop.");
    server_handle.await.map_err(|e| Error::Generic(e.to_string()))?;
    
    Ok(())
}

/// Demonstrate usage of the distributed gateway API
async fn demonstrate_api_usage() -> Result<()> {
    let client = reqwest::Client::new();
    let base_url = "http://localhost:8080/api/v1";
    
    info!("=== Demonstrating Distributed Gateway API ===");
    
    // 1. List active nodes
    info!("\n1. Listing active nodes:");
    let response = client
        .get(&format!("{}/nodes", base_url))
        .send()
        .await
        .map_err(|e| Error::Generic(e.to_string()))?;
    
    if response.status().is_success() {
        let nodes: serde_json::Value = response.json().await
            .map_err(|e| Error::Generic(e.to_string()))?;
        info!("Active nodes: {}", serde_json::to_string_pretty(&nodes).unwrap());
    }
    
    // 2. Check node health
    info!("\n2. Checking node health:");
    let response = client
        .get(&format!("{}/nodes/node-1/health", base_url))
        .send()
        .await
        .map_err(|e| Error::Generic(e.to_string()))?;
    
    if response.status().is_success() {
        let health: serde_json::Value = response.json().await
            .map_err(|e| Error::Generic(e.to_string()))?;
        info!("Node-1 health: {}", serde_json::to_string_pretty(&health).unwrap());
    }
    
    // 3. Execute a distributed query
    info!("\n3. Executing distributed query across Ethereum nodes:");
    let query_request = json!({
        "filter": {
            "chain_ids": ["ethereum"],
            "from_height": 1000000,
            "to_height": 1001000
        },
        "pagination": {
            "limit": 100,
            "offset": 0
        },
        "hints": {
            "max_parallel": 2,
            "allow_partial_results": false
        },
        "timeout_ms": 30000,
        "use_cache": true
    });
    
    let response = client
        .post(&format!("{}/query/distributed", base_url))
        .json(&query_request)
        .send()
        .await
        .map_err(|e| Error::Generic(e.to_string()))?;
    
    if response.status().is_success() {
        let result: serde_json::Value = response.json().await
            .map_err(|e| Error::Generic(e.to_string()))?;
        info!("Query result metadata: {}", 
            serde_json::to_string_pretty(&result["metadata"]).unwrap());
    }
    
    // 4. Create a query plan without executing
    info!("\n4. Creating query plan for analysis:");
    let response = client
        .post(&format!("{}/query/plan", base_url))
        .json(&query_request)
        .send()
        .await
        .map_err(|e| Error::Generic(e.to_string()))?;
    
    if response.status().is_success() {
        let plan: serde_json::Value = response.json().await
            .map_err(|e| Error::Generic(e.to_string()))?;
        info!("Query plan: {}", serde_json::to_string_pretty(&plan).unwrap());
    }
    
    // 5. Use smart query routing
    info!("\n5. Using smart query routing:");
    let smart_query = json!({
        "chain_id": "ethereum",
        "event_types": ["Transfer", "Approval"],
        "from_height": 1000000,
        "to_height": 1000100,
        "limit": 50,
        "allow_partial": false,
        "use_cache": true
    });
    
    let response = client
        .post(&format!("{}/events/smart", base_url))
        .json(&smart_query)
        .send()
        .await
        .map_err(|e| Error::Generic(e.to_string()))?;
    
    if response.status().is_success() {
        info!("Smart query executed successfully");
    }
    
    // 6. Check gateway metrics
    info!("\n6. Checking gateway metrics:");
    let response = client
        .get(&format!("{}/gateway/metrics", base_url))
        .send()
        .await
        .map_err(|e| Error::Generic(e.to_string()))?;
    
    if response.status().is_success() {
        let metrics: serde_json::Value = response.json().await
            .map_err(|e| Error::Generic(e.to_string()))?;
        info!("Gateway metrics: {}", serde_json::to_string_pretty(&metrics).unwrap());
    }
    
    // 7. Update node data locality
    info!("\n7. Updating node data locality:");
    let locality_update = json!({
        "chains": ["ethereum", "polygon"],
        "block_ranges": {
            "ethereum": [[1000000, 2000000], [3000000, 4000000]],
            "polygon": [[1, 1000000]]
        },
        "contracts": [
            "0x1234567890123456789012345678901234567890",
            "0xabcdefabcdefabcdefabcdefabcdefabcdefabcd"
        ]
    });
    
    let response = client
        .post(&format!("{}/nodes/node-1/locality", base_url))
        .json(&locality_update)
        .send()
        .await
        .map_err(|e| Error::Generic(e.to_string()))?;
    
    if response.status().is_success() {
        info!("Node locality updated successfully");
    }
    
    // 8. Test rate limiting
    info!("\n8. Testing distributed rate limiting:");
    for i in 0..5 {
        let response = client
            .get(&format!("{}/gateway/health", base_url))
            .send()
            .await
            .map_err(|e| Error::Generic(e.to_string()))?;
        
        if let Some(remaining) = response.headers().get("X-RateLimit-Remaining") {
            info!("Request {}: Rate limit remaining: {:?}", i + 1, remaining);
        }
    }
    
    info!("\n=== API demonstration complete ===");
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_mock_coordination_service() {
        // Test that mock service can be created
        let nodes = vec![];
        let _service = MockCoordinationService { nodes };
    }
}