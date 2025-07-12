#[cfg(test)]
mod tests {
    use super::super::distributed_query_planner::*;
    use super::super::coordination::{NodeInfo, NodeCapability, NodeStatus};
    use crate::types::{ChainId, EventFilter};
    use std::collections::HashMap;
    use std::sync::Arc;
    use tokio::sync::RwLock;

    #[tokio::test]
    async fn test_query_distribution_single_chain() {
        let planner = DistributedQueryPlanner::new();
        
        // Register nodes with different chain capabilities
        let node1 = NodeInfo {
            node_id: "node1".to_string(),
            address: "127.0.0.1:8001".to_string(),
            status: NodeStatus::Active,
            capabilities: vec![
                NodeCapability::ChainSupport { chain_id: ChainId("ethereum".to_string()) },
            ],
            heartbeat: std::time::SystemTime::now(),
        };
        
        let node2 = NodeInfo {
            node_id: "node2".to_string(),
            address: "127.0.0.1:8002".to_string(),
            status: NodeStatus::Active,
            capabilities: vec![
                NodeCapability::ChainSupport { chain_id: ChainId("polygon".to_string()) },
            ],
            heartbeat: std::time::SystemTime::now(),
        };
        
        planner.register_node(node1).await;
        planner.register_node(node2).await;
        
        // Create query for Ethereum chain
        let filter = EventFilter {
            chain_ids: Some(vec![ChainId("ethereum".to_string())]),
            ..Default::default()
        };
        
        let plan = planner.plan_query(vec![filter]).await.unwrap();
        
        // Should route to node1 only
        assert_eq!(plan.sub_queries.len(), 1);
        assert_eq!(plan.sub_queries[0].node_id, "node1");
    }

    #[tokio::test]
    async fn test_query_distribution_multi_chain() {
        let planner = DistributedQueryPlanner::new();
        
        // Register nodes
        let node1 = NodeInfo {
            node_id: "node1".to_string(),
            address: "127.0.0.1:8001".to_string(),
            status: NodeStatus::Active,
            capabilities: vec![
                NodeCapability::ChainSupport { chain_id: ChainId("ethereum".to_string()) },
            ],
            heartbeat: std::time::SystemTime::now(),
        };
        
        let node2 = NodeInfo {
            node_id: "node2".to_string(),
            address: "127.0.0.1:8002".to_string(),
            status: NodeStatus::Active,
            capabilities: vec![
                NodeCapability::ChainSupport { chain_id: ChainId("polygon".to_string()) },
            ],
            heartbeat: std::time::SystemTime::now(),
        };
        
        planner.register_node(node1).await;
        planner.register_node(node2).await;
        
        // Create query for both chains
        let filter = EventFilter {
            chain_ids: Some(vec![
                ChainId("ethereum".to_string()),
                ChainId("polygon".to_string()),
            ]),
            ..Default::default()
        };
        
        let plan = planner.plan_query(vec![filter]).await.unwrap();
        
        // Should route to both nodes
        assert_eq!(plan.sub_queries.len(), 2);
        let node_ids: Vec<_> = plan.sub_queries.iter().map(|sq| sq.node_id.as_str()).collect();
        assert!(node_ids.contains(&"node1"));
        assert!(node_ids.contains(&"node2"));
    }

    #[tokio::test]
    async fn test_query_distribution_with_data_locality() {
        let planner = DistributedQueryPlanner::new();
        
        // Register nodes with block range capabilities
        let node1 = NodeInfo {
            node_id: "node1".to_string(),
            address: "127.0.0.1:8001".to_string(),
            status: NodeStatus::Active,
            capabilities: vec![
                NodeCapability::ChainSupport { chain_id: ChainId("ethereum".to_string()) },
                NodeCapability::BlockRangeSupport {
                    chain_id: ChainId("ethereum".to_string()),
                    from_block: 1000,
                    to_block: 5000,
                },
            ],
            heartbeat: std::time::SystemTime::now(),
        };
        
        let node2 = NodeInfo {
            node_id: "node2".to_string(),
            address: "127.0.0.1:8002".to_string(),
            status: NodeStatus::Active,
            capabilities: vec![
                NodeCapability::ChainSupport { chain_id: ChainId("ethereum".to_string()) },
                NodeCapability::BlockRangeSupport {
                    chain_id: ChainId("ethereum".to_string()),
                    from_block: 5001,
                    to_block: 10000,
                },
            ],
            heartbeat: std::time::SystemTime::now(),
        };
        
        planner.register_node(node1).await;
        planner.register_node(node2).await;
        
        // Query for specific block range
        let filter = EventFilter {
            chain_ids: Some(vec![ChainId("ethereum".to_string())]),
            block_range: Some((7000, 8000)),
            ..Default::default()
        };
        
        let plan = planner.plan_query(vec![filter]).await.unwrap();
        
        // Should route to node2 only (which has blocks 5001-10000)
        assert_eq!(plan.sub_queries.len(), 1);
        assert_eq!(plan.sub_queries[0].node_id, "node2");
    }

    #[tokio::test]
    async fn test_query_distribution_with_hints() {
        let planner = DistributedQueryPlanner::new();
        
        // Register multiple nodes
        for i in 1..=3 {
            let node = NodeInfo {
                node_id: format!("node{}", i),
                address: format!("127.0.0.1:800{}", i),
                status: NodeStatus::Active,
                capabilities: vec![
                    NodeCapability::ChainSupport { chain_id: ChainId("ethereum".to_string()) },
                ],
                heartbeat: std::time::SystemTime::now(),
            };
            planner.register_node(node).await;
        }
        
        // Create query with node hint
        let mut filter = EventFilter {
            chain_ids: Some(vec![ChainId("ethereum".to_string())]),
            ..Default::default()
        };
        
        // Add hint to prefer node2
        filter.hints.insert("preferred_node".to_string(), "node2".to_string());
        
        let plan = planner.plan_query(vec![filter]).await.unwrap();
        
        // Should route to node2 based on hint
        assert_eq!(plan.sub_queries.len(), 1);
        assert_eq!(plan.sub_queries[0].node_id, "node2");
    }

    #[tokio::test]
    async fn test_query_distribution_failover() {
        let planner = DistributedQueryPlanner::new();
        
        // Register active and inactive nodes
        let active_node = NodeInfo {
            node_id: "active".to_string(),
            address: "127.0.0.1:8001".to_string(),
            status: NodeStatus::Active,
            capabilities: vec![
                NodeCapability::ChainSupport { chain_id: ChainId("ethereum".to_string()) },
            ],
            heartbeat: std::time::SystemTime::now(),
        };
        
        let inactive_node = NodeInfo {
            node_id: "inactive".to_string(),
            address: "127.0.0.1:8002".to_string(),
            status: NodeStatus::Inactive,
            capabilities: vec![
                NodeCapability::ChainSupport { chain_id: ChainId("ethereum".to_string()) },
            ],
            heartbeat: std::time::SystemTime::now(),
        };
        
        planner.register_node(active_node).await;
        planner.register_node(inactive_node).await;
        
        let filter = EventFilter {
            chain_ids: Some(vec![ChainId("ethereum".to_string())]),
            ..Default::default()
        };
        
        let plan = planner.plan_query(vec![filter]).await.unwrap();
        
        // Should only route to active node
        assert_eq!(plan.sub_queries.len(), 1);
        assert_eq!(plan.sub_queries[0].node_id, "active");
    }

    #[tokio::test]
    async fn test_query_caching() {
        let planner = DistributedQueryPlanner::new();
        
        // Register node
        let node = NodeInfo {
            node_id: "node1".to_string(),
            address: "127.0.0.1:8001".to_string(),
            status: NodeStatus::Active,
            capabilities: vec![
                NodeCapability::ChainSupport { chain_id: ChainId("ethereum".to_string()) },
            ],
            heartbeat: std::time::SystemTime::now(),
        };
        planner.register_node(node).await;
        
        let filter = EventFilter {
            chain_ids: Some(vec![ChainId("ethereum".to_string())]),
            ..Default::default()
        };
        
        // First query - should create new plan
        let plan1 = planner.plan_query(vec![filter.clone()]).await.unwrap();
        
        // Second query - should use cached plan
        let plan2 = planner.plan_query(vec![filter]).await.unwrap();
        
        // Plans should be identical (same query ID indicates cache hit)
        assert_eq!(plan1.query_id, plan2.query_id);
    }
}