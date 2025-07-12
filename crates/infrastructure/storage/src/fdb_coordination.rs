use async_trait::async_trait;
use foundationdb::{Database, FdbResult, KeySelector, RangeOption, Transaction};
use futures::TryStreamExt;
use indexer_core::coordination::{
    CoordinationService, LeaderInfo, NodeInfo, NodeStatus, WorkAssignment,
};
use indexer_core::Result;
use serde_json;
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::{debug, info, warn};

const NODES_PREFIX: &[u8] = b"/coord/nodes/";
const ASSIGNMENTS_PREFIX: &[u8] = b"/coord/assignments/";
const LEADER_PREFIX: &[u8] = b"/coord/leader";
const NODE_ASSIGNMENTS_PREFIX: &[u8] = b"/coord/node_assignments/";

pub struct FdbCoordinationService {
    db: Database,
    namespace: String,
}

impl FdbCoordinationService {
    pub fn new(db: Database, namespace: String) -> Self {
        Self { db, namespace }
    }

    fn namespace_key(&self, prefix: &[u8], key: &[u8]) -> Vec<u8> {
        let mut full_key = Vec::with_capacity(self.namespace.len() + prefix.len() + key.len() + 2);
        full_key.push(b'/');
        full_key.extend_from_slice(self.namespace.as_bytes());
        full_key.extend_from_slice(prefix);
        full_key.extend_from_slice(key);
        full_key
    }

    fn node_key(&self, node_id: &str) -> Vec<u8> {
        self.namespace_key(NODES_PREFIX, node_id.as_bytes())
    }

    fn assignment_key(&self, assignment_id: &str) -> Vec<u8> {
        self.namespace_key(ASSIGNMENTS_PREFIX, assignment_id.as_bytes())
    }

    fn node_assignment_key(&self, node_id: &str, assignment_id: &str) -> Vec<u8> {
        let key = format!("{}/{}", node_id, assignment_id);
        self.namespace_key(NODE_ASSIGNMENTS_PREFIX, key.as_bytes())
    }

    fn leader_key(&self) -> Vec<u8> {
        self.namespace_key(LEADER_PREFIX, b"")
    }

    async fn transact<F, R>(&self, f: F) -> FdbResult<R>
    where
        F: Fn(Transaction) -> FdbResult<R>,
    {
        self.db.run(f).await
    }

    fn current_timestamp() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
    }
}

#[async_trait]
impl CoordinationService for FdbCoordinationService {
    async fn register_node(&self, mut info: NodeInfo) -> Result<()> {
        info.last_heartbeat = Self::current_timestamp();
        let key = self.node_key(&info.node_id);
        let value = serde_json::to_vec(&info)?;

        self.transact(|mut trx| {
            trx.set(&key, &value);
            Ok(())
        })
        .await?;

        info!(
            "Registered node: {} with capabilities: {:?}",
            info.node_id, info.capabilities.supported_chains
        );

        Ok(())
    }

    async fn update_node_status(&self, node_id: &str, status: NodeStatus) -> Result<()> {
        let key = self.node_key(node_id);

        self.transact(|mut trx| {
            let get_future = trx.get(&key, false);
            
            futures::executor::block_on(async {
                if let Ok(Some(value)) = get_future.await {
                    if let Ok(mut info) = serde_json::from_slice::<NodeInfo>(&value) {
                        info.status = status;
                        info.last_heartbeat = Self::current_timestamp();
                        
                        if let Ok(updated_value) = serde_json::to_vec(&info) {
                            trx.set(&key, &updated_value);
                        }
                    }
                }
            });
            
            Ok(())
        })
        .await?;

        debug!("Updated node {} status to {:?}", node_id, status);

        Ok(())
    }

    async fn get_active_nodes(&self) -> Result<Vec<NodeInfo>> {
        let start_key = self.namespace_key(NODES_PREFIX, b"");
        let end_key = self.namespace_key(NODES_PREFIX, b"\xff");

        let range = RangeOption {
            begin: KeySelector::first_greater_or_equal(&start_key),
            end: KeySelector::first_greater_or_equal(&end_key),
            mode: foundationdb::options::StreamingMode::Iterator,
            limit: None,
            reverse: false,
        };

        let results = self
            .transact(|trx| {
                let mut results = Vec::new();
                let iter = trx.get_range(&range, 1, false);
                
                futures::executor::block_on(async {
                    let mut stream = iter;
                    while let Ok(Some(kv)) = stream.try_next().await {
                        results.push(kv);
                    }
                });
                
                Ok(results)
            })
            .await?;

        let mut nodes = Vec::new();
        for kv in results {
            if let Ok(info) = serde_json::from_slice::<NodeInfo>(kv.value()) {
                if matches!(info.status, NodeStatus::Active | NodeStatus::Degraded) {
                    nodes.push(info);
                }
            }
        }

        Ok(nodes)
    }

    async fn assign_work(&self, assignment: WorkAssignment) -> Result<()> {
        let assignment_key = self.assignment_key(&assignment.assignment_id);
        let node_assignment_key = self.node_assignment_key(&assignment.node_id, &assignment.assignment_id);
        let value = serde_json::to_vec(&assignment)?;

        self.transact(|mut trx| {
            // Store the assignment
            trx.set(&assignment_key, &value);
            
            // Create index for node assignments
            trx.set(&node_assignment_key, &assignment.assignment_id.as_bytes());
            
            Ok(())
        })
        .await?;

        info!(
            "Assigned work {} to node {} for chain {}",
            assignment.assignment_id, assignment.node_id, assignment.chain_id
        );

        Ok(())
    }

    async fn get_node_assignments(&self, node_id: &str) -> Result<Vec<WorkAssignment>> {
        let start_key = self.node_assignment_key(node_id, "");
        let end_key = self.node_assignment_key(node_id, "\xff");

        let range = RangeOption {
            begin: KeySelector::first_greater_or_equal(&start_key),
            end: KeySelector::first_greater_or_equal(&end_key),
            mode: foundationdb::options::StreamingMode::Iterator,
            limit: None,
            reverse: false,
        };

        let assignment_ids = self
            .transact(|trx| {
                let mut ids = Vec::new();
                let iter = trx.get_range(&range, 1, false);
                
                futures::executor::block_on(async {
                    let mut stream = iter;
                    while let Ok(Some(kv)) = stream.try_next().await {
                        ids.push(String::from_utf8_lossy(kv.value()).to_string());
                    }
                });
                
                Ok(ids)
            })
            .await?;

        let mut assignments = Vec::new();
        for assignment_id in assignment_ids {
            let key = self.assignment_key(&assignment_id);
            
            let result = self
                .transact(|trx| Ok(trx.get(&key, false)))
                .await?
                .await?;
                
            if let Some(value) = result {
                if let Ok(assignment) = serde_json::from_slice::<WorkAssignment>(&value) {
                    assignments.push(assignment);
                }
            }
        }

        Ok(assignments)
    }

    async fn complete_assignment(&self, assignment_id: &str) -> Result<()> {
        self.transact(|mut trx| {
            let assignment_key = self.assignment_key(assignment_id);
            
            // Get assignment to find node_id
            let get_future = trx.get(&assignment_key, false);
            
            futures::executor::block_on(async {
                if let Ok(Some(value)) = get_future.await {
                    if let Ok(assignment) = serde_json::from_slice::<WorkAssignment>(&value) {
                        let node_assignment_key = self.node_assignment_key(&assignment.node_id, assignment_id);
                        
                        // Remove both keys
                        trx.clear(&assignment_key);
                        trx.clear(&node_assignment_key);
                    }
                }
            });
            
            Ok(())
        })
        .await?;

        info!("Completed assignment: {}", assignment_id);

        Ok(())
    }

    async fn elect_leader(&self, candidate_id: &str) -> Result<bool> {
        let leader_key = self.leader_key();
        let now = Self::current_timestamp();

        let elected = self
            .transact(|mut trx| {
                let get_future = trx.get(&leader_key, false);
                
                let mut can_elect = false;
                
                futures::executor::block_on(async {
                    match get_future.await {
                        Ok(Some(value)) => {
                            if let Ok(current_leader) = serde_json::from_slice::<LeaderInfo>(&value) {
                                // Check if lease expired
                                if now > current_leader.elected_at + current_leader.lease_duration {
                                    can_elect = true;
                                }
                            }
                        }
                        Ok(None) => {
                            // No current leader
                            can_elect = true;
                        }
                        Err(_) => {}
                    }
                });
                
                if can_elect {
                    let leader_info = LeaderInfo {
                        node_id: candidate_id.to_string(),
                        term: now / 300, // 5-minute terms
                        elected_at: now,
                        lease_duration: 300, // 5 minutes
                    };
                    
                    if let Ok(value) = serde_json::to_vec(&leader_info) {
                        trx.set(&leader_key, &value);
                    }
                }
                
                Ok(can_elect)
            })
            .await?;

        if elected {
            info!("Node {} elected as leader", candidate_id);
        }

        Ok(elected)
    }

    async fn get_leader(&self) -> Result<Option<LeaderInfo>> {
        let leader_key = self.leader_key();

        let result = self
            .transact(|trx| Ok(trx.get(&leader_key, false)))
            .await?
            .await?;

        match result {
            Some(value) => {
                let leader = serde_json::from_slice(&value)?;
                Ok(Some(leader))
            }
            None => Ok(None),
        }
    }

    async fn release_assignment(&self, assignment_id: &str) -> Result<()> {
        self.transact(|mut trx| {
            let assignment_key = self.assignment_key(assignment_id);
            
            // Get assignment to find node_id
            let get_future = trx.get(&assignment_key, false);
            
            futures::executor::block_on(async {
                if let Ok(Some(value)) = get_future.await {
                    if let Ok(assignment) = serde_json::from_slice::<WorkAssignment>(&value) {
                        let node_assignment_key = self.node_assignment_key(&assignment.node_id, assignment_id);
                        
                        // Remove from node assignments index
                        trx.clear(&node_assignment_key);
                        
                        // Don't remove the assignment itself - it can be reassigned
                        // Just update its status or reassign it
                    }
                }
            });
            
            Ok(())
        })
        .await?;

        warn!("Released assignment: {}", assignment_id);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use indexer_core::coordination::{NodeCapabilities, WorkType};

    #[tokio::test]
    async fn test_coordination_basic_operations() {
        // Skip if FDB not available
        let db = match Database::default() {
            Ok(db) => db,
            Err(_) => {
                eprintln!("FoundationDB not available, skipping test");
                return;
            }
        };

        let coord = FdbCoordinationService::new(db, "test".to_string());

        // Register a node
        let node = NodeInfo {
            node_id: "test-node-1".to_string(),
            address: "127.0.0.1:8080".to_string(),
            capabilities: NodeCapabilities {
                supported_chains: vec!["ethereum".to_string()],
                max_throughput: 1000,
                storage_backends: vec!["fdb".to_string()],
                api_endpoints: vec!["rest".to_string()],
            },
            status: NodeStatus::Active,
            last_heartbeat: 0,
            metadata: std::collections::HashMap::new(),
        };

        coord.register_node(node.clone()).await.unwrap();

        // Get active nodes
        let nodes = coord.get_active_nodes().await.unwrap();
        assert_eq!(nodes.len(), 1);
        assert_eq!(nodes[0].node_id, "test-node-1");

        // Assign work
        let assignment = WorkAssignment {
            assignment_id: "work-1".to_string(),
            node_id: "test-node-1".to_string(),
            chain_id: "ethereum".to_string(),
            work_type: WorkType::BlockIndexing {
                from_block: 100,
                to_block: Some(200),
            },
            parameters: std::collections::HashMap::new(),
            assigned_at: FdbCoordinationService::current_timestamp(),
        };

        coord.assign_work(assignment).await.unwrap();

        // Get node assignments
        let assignments = coord.get_node_assignments("test-node-1").await.unwrap();
        assert_eq!(assignments.len(), 1);
        assert_eq!(assignments[0].assignment_id, "work-1");

        // Complete assignment
        coord.complete_assignment("work-1").await.unwrap();

        let assignments = coord.get_node_assignments("test-node-1").await.unwrap();
        assert_eq!(assignments.len(), 0);
    }
}