// Distributed system performance benchmarks
//
// This module provides comprehensive performance benchmarks for distributed
// indexing, query execution, and cross-node coordination

use anyhow::Result;
use std::time::{Duration, Instant};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, AtomicBool, Ordering};
use std::collections::HashMap;
use tokio::sync::{RwLock, Semaphore};
use futures::future::join_all;
use tracing::{info, debug, warn};
use serde::{Serialize, Deserialize};

use crate::{BenchmarkConfig, fixtures};
use crate::benchmarks::{BenchmarkResults, calculate_latency_statistics};
use crate::distributed::DistributedTestCluster;
use indexer_core::coordination::{NodeCapabilities, WorkType, WorkAssignment};
use indexer_core::distributed_query::{DistributedQuery, QueryResult};
use uuid::Uuid;

/// Distributed benchmark suite
pub struct DistributedBenchmarkSuite {
    cluster: Arc<DistributedTestCluster>,
    config: BenchmarkConfig,
    results: Arc<RwLock<Vec<BenchmarkResults>>>,
}

impl DistributedBenchmarkSuite {
    /// Create a new distributed benchmark suite
    pub async fn new(config: BenchmarkConfig) -> Result<Self> {
        let cluster = DistributedTestCluster::new("benchmark-cluster").await?;
        
        Ok(Self {
            cluster: Arc::new(cluster),
            config,
            results: Arc::new(RwLock::new(Vec::new())),
        })
    }

    /// Run all distributed benchmarks
    pub async fn run_all(&mut self) -> Result<DistributedBenchmarkReport> {
        info!("Starting distributed system benchmarks");
        let suite_start = Instant::now();

        // Setup cluster with different node configurations
        self.setup_benchmark_cluster().await?;

        // Run individual benchmarks
        let mut benchmark_results = Vec::new();

        // Benchmark distributed indexing
        match self.benchmark_distributed_indexing().await {
            Ok(result) => {
                info!("Distributed indexing: {:.2} ops/sec", result.operations_per_second);
                benchmark_results.push(result);
            }
            Err(e) => warn!("Distributed indexing benchmark failed: {}", e),
        }

        // Benchmark distributed queries
        match self.benchmark_distributed_queries().await {
            Ok(result) => {
                info!("Distributed queries: {:.2} ops/sec", result.operations_per_second);
                benchmark_results.push(result);
            }
            Err(e) => warn!("Distributed queries benchmark failed: {}", e),
        }

        // Benchmark node coordination
        match self.benchmark_node_coordination().await {
            Ok(result) => {
                info!("Node coordination: {:.2} ops/sec", result.operations_per_second);
                benchmark_results.push(result);
            }
            Err(e) => warn!("Node coordination benchmark failed: {}", e),
        }

        // Benchmark work distribution
        match self.benchmark_work_distribution().await {
            Ok(result) => {
                info!("Work distribution: {:.2} ops/sec", result.operations_per_second);
                benchmark_results.push(result);
            }
            Err(e) => warn!("Work distribution benchmark failed: {}", e),
        }

        // Benchmark failover scenarios
        match self.benchmark_failover_performance().await {
            Ok(result) => {
                info!("Failover performance: {:.2} ms avg latency", result.average_latency_ms);
                benchmark_results.push(result);
            }
            Err(e) => warn!("Failover benchmark failed: {}", e),
        }

        // Benchmark cross-node data transfer
        match self.benchmark_data_transfer().await {
            Ok(result) => {
                info!("Data transfer: {:.2} MB/sec", result.throughput_mb_per_sec);
                benchmark_results.push(result);
            }
            Err(e) => warn!("Data transfer benchmark failed: {}", e),
        }

        // Store results
        *self.results.write().await = benchmark_results.clone();

        // Generate report
        let report = DistributedBenchmarkReport {
            total_duration: suite_start.elapsed(),
            benchmark_results,
            cluster_configuration: self.get_cluster_configuration().await?,
            performance_summary: self.generate_performance_summary().await?,
        };

        // Cleanup
        self.cluster.shutdown().await?;

        Ok(report)
    }

    /// Setup benchmark cluster with various node types
    async fn setup_benchmark_cluster(&self) -> Result<()> {
        info!("Setting up distributed benchmark cluster");

        // Add indexer nodes
        for i in 0..3 {
            let mut caps = NodeCapabilities::default();
            caps.supported_chains = vec!["ethereum".to_string(), "polygon".to_string()];
            caps.max_concurrent_tasks = 10;
            caps.memory_limit_mb = 1024;
            
            let node_id = self.cluster.add_node(caps).await?;
            self.cluster.start_node(&node_id).await?;
            debug!("Started indexer node {}: {}", i, node_id);
        }

        // Add query nodes
        for i in 0..2 {
            let mut caps = NodeCapabilities::default();
            caps.can_process_queries = true;
            caps.max_concurrent_queries = 50;
            caps.memory_limit_mb = 2048;
            
            let node_id = self.cluster.add_node(caps).await?;
            self.cluster.start_node(&node_id).await?;
            debug!("Started query node {}: {}", i, node_id);
        }

        // Add coordinator node
        let mut coordinator_caps = NodeCapabilities::default();
        coordinator_caps.can_coordinate = true;
        coordinator_caps.memory_limit_mb = 512;
        
        let coordinator_id = self.cluster.add_node(coordinator_caps).await?;
        self.cluster.start_node(&coordinator_id).await?;
        debug!("Started coordinator node: {}", coordinator_id);

        // Wait for cluster to stabilize
        tokio::time::sleep(Duration::from_secs(2)).await;

        Ok(())
    }

    /// Benchmark distributed indexing performance
    async fn benchmark_distributed_indexing(&self) -> Result<BenchmarkResults> {
        info!("Benchmarking distributed indexing");
        let start_time = Instant::now();
        
        let total_blocks = self.config.operations_count;
        let blocks_per_node = total_blocks / 3; // We have 3 indexer nodes
        
        let operations_count = Arc::new(AtomicU64::new(0));
        let errors_count = Arc::new(AtomicU64::new(0));
        let latencies = Arc::new(RwLock::new(Vec::new()));

        // Get active indexer nodes
        let nodes = self.cluster.get_active_nodes().await?;
        let indexer_nodes: Vec<_> = nodes.iter()
            .filter(|n| n.capabilities.supported_chains.contains(&"ethereum".to_string()))
            .collect();

        // Create work assignments
        let mut tasks = Vec::new();
        for (idx, node) in indexer_nodes.iter().enumerate() {
            let start_block = idx as u64 * blocks_per_node;
            let end_block = start_block + blocks_per_node;
            
            let assignment = WorkAssignment {
                assignment_id: Uuid::new_v4().to_string(),
                node_id: node.node_id.clone(),
                chain_id: "ethereum".to_string(),
                work_type: WorkType::BlockIndexing {
                    from_block: start_block,
                    to_block: Some(end_block),
                },
                parameters: HashMap::new(),
                assigned_at: chrono::Utc::now().timestamp() as u64,
            };

            let ops_count = operations_count.clone();
            let err_count = errors_count.clone();
            let lats = latencies.clone();
            
            let task = tokio::spawn(async move {
                for block_num in start_block..end_block {
                    let op_start = Instant::now();
                    
                    // Simulate block indexing
                    let success = simulate_block_indexing_distributed(
                        &assignment.node_id,
                        "ethereum",
                        block_num
                    ).await;
                    
                    let op_latency = op_start.elapsed();
                    lats.write().await.push(op_latency.as_secs_f64() * 1000.0);
                    
                    ops_count.fetch_add(1, Ordering::Relaxed);
                    if !success {
                        err_count.fetch_add(1, Ordering::Relaxed);
                    }
                    
                    // Small delay to simulate realistic processing
                    tokio::time::sleep(Duration::from_micros(100)).await;
                }
            });
            
            tasks.push(task);
        }

        // Wait for all indexing tasks to complete
        join_all(tasks).await;

        let total_duration = start_time.elapsed();
        let total_ops = operations_count.load(Ordering::Relaxed);
        let total_errors = errors_count.load(Ordering::Relaxed);
        
        let latency_vec = latencies.read().await.clone();
        let stats = calculate_latency_statistics(&latency_vec);

        Ok(BenchmarkResults {
            name: "Distributed Indexing".to_string(),
            duration: total_duration,
            operations_per_second: total_ops as f64 / total_duration.as_secs_f64(),
            average_latency_ms: stats.average,
            min_latency_ms: stats.min,
            max_latency_ms: stats.max,
            p50_latency_ms: stats.p50,
            p95_latency_ms: stats.p95,
            p99_latency_ms: stats.p99,
            memory_usage_mb: self.get_cluster_memory_usage().await?,
            cpu_usage_percent: self.get_cluster_cpu_usage().await?,
            error_rate: total_errors as f64 / total_ops as f64,
            throughput_mb_per_sec: calculate_indexing_throughput(total_ops, total_duration),
            metadata: self.collect_metadata("distributed_indexing").await?,
        })
    }

    /// Benchmark distributed query performance
    async fn benchmark_distributed_queries(&self) -> Result<BenchmarkResults> {
        info!("Benchmarking distributed queries");
        let start_time = Instant::now();
        
        let total_queries = self.config.operations_count;
        let concurrent_queries = self.config.concurrent_users.min(50);
        
        let operations_count = Arc::new(AtomicU64::new(0));
        let errors_count = Arc::new(AtomicU64::new(0));
        let latencies = Arc::new(RwLock::new(Vec::new()));
        
        // Semaphore to limit concurrent queries
        let semaphore = Arc::new(Semaphore::new(concurrent_queries as usize));

        let mut tasks = Vec::new();
        for i in 0..total_queries {
            let sem = semaphore.clone();
            let ops_count = operations_count.clone();
            let err_count = errors_count.clone();
            let lats = latencies.clone();
            let cluster = self.cluster.clone();
            
            let task = tokio::spawn(async move {
                let _permit = sem.acquire().await.unwrap();
                let op_start = Instant::now();
                
                // Create distributed query
                let query = create_test_distributed_query(i);
                
                // Simulate query execution across nodes
                let result = execute_distributed_query(&cluster, &query).await;
                
                let op_latency = op_start.elapsed();
                lats.write().await.push(op_latency.as_secs_f64() * 1000.0);
                
                ops_count.fetch_add(1, Ordering::Relaxed);
                if result.is_err() {
                    err_count.fetch_add(1, Ordering::Relaxed);
                }
            });
            
            tasks.push(task);
        }

        // Wait for all queries to complete
        join_all(tasks).await;

        let total_duration = start_time.elapsed();
        let total_ops = operations_count.load(Ordering::Relaxed);
        let total_errors = errors_count.load(Ordering::Relaxed);
        
        let latency_vec = latencies.read().await.clone();
        let stats = calculate_latency_statistics(&latency_vec);

        Ok(BenchmarkResults {
            name: "Distributed Queries".to_string(),
            duration: total_duration,
            operations_per_second: total_ops as f64 / total_duration.as_secs_f64(),
            average_latency_ms: stats.average,
            min_latency_ms: stats.min,
            max_latency_ms: stats.max,
            p50_latency_ms: stats.p50,
            p95_latency_ms: stats.p95,
            p99_latency_ms: stats.p99,
            memory_usage_mb: self.get_cluster_memory_usage().await?,
            cpu_usage_percent: self.get_cluster_cpu_usage().await?,
            error_rate: total_errors as f64 / total_ops as f64,
            throughput_mb_per_sec: calculate_query_throughput(total_ops, total_duration),
            metadata: self.collect_metadata("distributed_queries").await?,
        })
    }

    /// Benchmark node coordination overhead
    async fn benchmark_node_coordination(&self) -> Result<BenchmarkResults> {
        info!("Benchmarking node coordination");
        let start_time = Instant::now();
        
        let coordination_ops = self.config.operations_count;
        let operations_count = Arc::new(AtomicU64::new(0));
        let errors_count = Arc::new(AtomicU64::new(0));
        let latencies = Arc::new(RwLock::new(Vec::new()));

        // Test various coordination operations
        for i in 0..coordination_ops {
            let op_start = Instant::now();
            
            match i % 4 {
                0 => {
                    // Leader election
                    let nodes = self.cluster.get_active_nodes().await?;
                    if let Some(node) = nodes.first() {
                        let _ = self.cluster.coordination.elect_leader(&node.node_id).await;
                    }
                }
                1 => {
                    // Heartbeat update
                    let nodes = self.cluster.get_active_nodes().await?;
                    for node in nodes.iter().take(3) {
                        let _ = self.cluster.coordination
                            .update_node_status(&node.node_id, indexer_core::coordination::NodeStatus::Active)
                            .await;
                    }
                }
                2 => {
                    // Work assignment
                    let assignment = WorkAssignment {
                        assignment_id: Uuid::new_v4().to_string(),
                        node_id: format!("node-{}", i % 3),
                        chain_id: "ethereum".to_string(),
                        work_type: WorkType::BlockIndexing {
                            from_block: i as u64,
                            to_block: Some(i as u64 + 100),
                        },
                        parameters: HashMap::new(),
                        assigned_at: chrono::Utc::now().timestamp() as u64,
                    };
                    let _ = self.cluster.coordination.assign_work(assignment).await;
                }
                _ => {
                    // Node discovery
                    let _ = self.cluster.get_active_nodes().await;
                }
            }
            
            let op_latency = op_start.elapsed();
            latencies.write().await.push(op_latency.as_secs_f64() * 1000.0);
            operations_count.fetch_add(1, Ordering::Relaxed);
        }

        let total_duration = start_time.elapsed();
        let total_ops = operations_count.load(Ordering::Relaxed);
        let total_errors = errors_count.load(Ordering::Relaxed);
        
        let latency_vec = latencies.read().await.clone();
        let stats = calculate_latency_statistics(&latency_vec);

        Ok(BenchmarkResults {
            name: "Node Coordination".to_string(),
            duration: total_duration,
            operations_per_second: total_ops as f64 / total_duration.as_secs_f64(),
            average_latency_ms: stats.average,
            min_latency_ms: stats.min,
            max_latency_ms: stats.max,
            p50_latency_ms: stats.p50,
            p95_latency_ms: stats.p95,
            p99_latency_ms: stats.p99,
            memory_usage_mb: self.get_cluster_memory_usage().await?,
            cpu_usage_percent: self.get_cluster_cpu_usage().await?,
            error_rate: total_errors as f64 / total_ops as f64,
            throughput_mb_per_sec: 0.0, // Not applicable for coordination
            metadata: self.collect_metadata("node_coordination").await?,
        })
    }

    /// Benchmark work distribution efficiency
    async fn benchmark_work_distribution(&self) -> Result<BenchmarkResults> {
        info!("Benchmarking work distribution");
        let start_time = Instant::now();
        
        let total_work_items = self.config.operations_count;
        let operations_count = Arc::new(AtomicU64::new(0));
        let distribution_times = Arc::new(RwLock::new(Vec::new()));
        let load_balance_scores = Arc::new(RwLock::new(Vec::new()));

        // Create work distributor
        use indexer_core::coordination::WorkDistributor;
        let distributor = WorkDistributor::new(
            Box::new(self.cluster.coordination.clone()),
            Default::default(),
        );

        // Distribute work items in batches
        let batch_size = 100;
        let num_batches = total_work_items / batch_size;

        for batch in 0..num_batches {
            let dist_start = Instant::now();
            
            // Create work items
            let mut work_items = Vec::new();
            for i in 0..batch_size {
                let work_idx = batch * batch_size + i;
                work_items.push(WorkType::BlockIndexing {
                    from_block: work_idx as u64 * 100,
                    to_block: Some((work_idx as u64 + 1) * 100),
                });
            }
            
            // Distribute work
            match distributor.distribute_work(work_items).await {
                Ok(assignments) => {
                    // Calculate load balance score
                    let balance_score = calculate_load_balance_score(&assignments);
                    load_balance_scores.write().await.push(balance_score);
                    
                    operations_count.fetch_add(batch_size as u64, Ordering::Relaxed);
                }
                Err(e) => {
                    warn!("Work distribution failed: {}", e);
                }
            }
            
            let dist_time = dist_start.elapsed();
            distribution_times.write().await.push(dist_time.as_secs_f64() * 1000.0);
        }

        let total_duration = start_time.elapsed();
        let total_ops = operations_count.load(Ordering::Relaxed);
        
        let dist_times = distribution_times.read().await.clone();
        let stats = calculate_latency_statistics(&dist_times);
        
        let balance_scores = load_balance_scores.read().await;
        let avg_balance = balance_scores.iter().sum::<f64>() / balance_scores.len() as f64;

        let mut metadata = self.collect_metadata("work_distribution").await?;
        metadata.insert("average_load_balance_score".to_string(), format!("{:.3}", avg_balance));

        Ok(BenchmarkResults {
            name: "Work Distribution".to_string(),
            duration: total_duration,
            operations_per_second: total_ops as f64 / total_duration.as_secs_f64(),
            average_latency_ms: stats.average,
            min_latency_ms: stats.min,
            max_latency_ms: stats.max,
            p50_latency_ms: stats.p50,
            p95_latency_ms: stats.p95,
            p99_latency_ms: stats.p99,
            memory_usage_mb: self.get_cluster_memory_usage().await?,
            cpu_usage_percent: self.get_cluster_cpu_usage().await?,
            error_rate: 0.0,
            throughput_mb_per_sec: 0.0,
            metadata,
        })
    }

    /// Benchmark failover performance
    async fn benchmark_failover_performance(&self) -> Result<BenchmarkResults> {
        info!("Benchmarking failover performance");
        let start_time = Instant::now();
        
        let failover_scenarios = 10;
        let operations_count = Arc::new(AtomicU64::new(0));
        let failover_times = Arc::new(RwLock::new(Vec::new()));
        let recovery_times = Arc::new(RwLock::new(Vec::new()));

        for scenario in 0..failover_scenarios {
            // Get active nodes
            let nodes = self.cluster.get_active_nodes().await?;
            if nodes.len() < 2 {
                warn!("Not enough nodes for failover test");
                continue;
            }

            // Select a node to fail
            let target_node = &nodes[scenario % nodes.len()];
            let failover_start = Instant::now();

            // Simulate node failure
            self.cluster.fail_node(&target_node.node_id).await?;

            // Wait for failure detection
            tokio::time::sleep(Duration::from_millis(500)).await;

            // Check if work was redistributed
            use indexer_core::coordination::HealthMonitor;
            let monitor = HealthMonitor::new(
                Box::new(self.cluster.coordination.clone()),
                Default::default(),
            );
            monitor.check_health().await?;

            let failover_time = failover_start.elapsed();
            failover_times.write().await.push(failover_time.as_secs_f64() * 1000.0);

            // Recover node
            let recovery_start = Instant::now();
            
            // Add new node to replace failed one
            let new_node_id = self.cluster.add_node(NodeCapabilities::default()).await?;
            self.cluster.start_node(&new_node_id).await?;

            let recovery_time = recovery_start.elapsed();
            recovery_times.write().await.push(recovery_time.as_secs_f64() * 1000.0);

            operations_count.fetch_add(1, Ordering::Relaxed);
        }

        let total_duration = start_time.elapsed();
        let total_ops = operations_count.load(Ordering::Relaxed);
        
        let failover_vec = failover_times.read().await.clone();
        let failover_stats = calculate_latency_statistics(&failover_vec);
        
        let recovery_vec = recovery_times.read().await.clone();
        let recovery_stats = calculate_latency_statistics(&recovery_vec);

        let mut metadata = self.collect_metadata("failover").await?;
        metadata.insert("avg_recovery_time_ms".to_string(), format!("{:.2}", recovery_stats.average));

        Ok(BenchmarkResults {
            name: "Failover Performance".to_string(),
            duration: total_duration,
            operations_per_second: total_ops as f64 / total_duration.as_secs_f64(),
            average_latency_ms: failover_stats.average,
            min_latency_ms: failover_stats.min,
            max_latency_ms: failover_stats.max,
            p50_latency_ms: failover_stats.p50,
            p95_latency_ms: failover_stats.p95,
            p99_latency_ms: failover_stats.p99,
            memory_usage_mb: self.get_cluster_memory_usage().await?,
            cpu_usage_percent: self.get_cluster_cpu_usage().await?,
            error_rate: 0.0,
            throughput_mb_per_sec: 0.0,
            metadata,
        })
    }

    /// Benchmark cross-node data transfer
    async fn benchmark_data_transfer(&self) -> Result<BenchmarkResults> {
        info!("Benchmarking cross-node data transfer");
        let start_time = Instant::now();
        
        let transfer_operations = self.config.operations_count / 10; // Fewer but larger transfers
        let operations_count = Arc::new(AtomicU64::new(0));
        let transfer_times = Arc::new(RwLock::new(Vec::new()));
        let bytes_transferred = Arc::new(AtomicU64::new(0));

        // Get active nodes
        let nodes = self.cluster.get_active_nodes().await?;
        if nodes.len() < 2 {
            return Err(anyhow::anyhow!("Not enough nodes for data transfer benchmark"));
        }

        for i in 0..transfer_operations {
            let source_node = &nodes[i % nodes.len()];
            let target_node = &nodes[(i + 1) % nodes.len()];
            
            let transfer_start = Instant::now();
            
            // Create test data
            let data_size_kb = 100 * (i % 10 + 1); // Variable sizes 100KB to 1MB
            let test_data = fixtures::create_test_events((data_size_kb / 10) as usize);
            
            // Simulate data transfer
            let transferred = simulate_node_data_transfer(
                &source_node.node_id,
                &target_node.node_id,
                &test_data,
            ).await?;
            
            let transfer_time = transfer_start.elapsed();
            transfer_times.write().await.push(transfer_time.as_secs_f64() * 1000.0);
            
            bytes_transferred.fetch_add(transferred, Ordering::Relaxed);
            operations_count.fetch_add(1, Ordering::Relaxed);
        }

        let total_duration = start_time.elapsed();
        let total_ops = operations_count.load(Ordering::Relaxed);
        let total_bytes = bytes_transferred.load(Ordering::Relaxed);
        
        let transfer_vec = transfer_times.read().await.clone();
        let stats = calculate_latency_statistics(&transfer_vec);
        
        let throughput_mb_per_sec = (total_bytes as f64 / (1024.0 * 1024.0)) / total_duration.as_secs_f64();

        Ok(BenchmarkResults {
            name: "Data Transfer".to_string(),
            duration: total_duration,
            operations_per_second: total_ops as f64 / total_duration.as_secs_f64(),
            average_latency_ms: stats.average,
            min_latency_ms: stats.min,
            max_latency_ms: stats.max,
            p50_latency_ms: stats.p50,
            p95_latency_ms: stats.p95,
            p99_latency_ms: stats.p99,
            memory_usage_mb: self.get_cluster_memory_usage().await?,
            cpu_usage_percent: self.get_cluster_cpu_usage().await?,
            error_rate: 0.0,
            throughput_mb_per_sec,
            metadata: self.collect_metadata("data_transfer").await?,
        })
    }

    /// Get cluster configuration details
    async fn get_cluster_configuration(&self) -> Result<ClusterConfiguration> {
        let nodes = self.cluster.get_active_nodes().await?;
        
        let mut node_types = HashMap::new();
        let mut total_memory_mb = 0;
        let mut total_concurrent_capacity = 0;

        for node in &nodes {
            let node_type = if node.capabilities.can_coordinate {
                "coordinator"
            } else if node.capabilities.can_process_queries {
                "query"
            } else {
                "indexer"
            };
            
            *node_types.entry(node_type.to_string()).or_insert(0) += 1;
            total_memory_mb += node.capabilities.memory_limit_mb;
            total_concurrent_capacity += node.capabilities.max_concurrent_tasks;
        }

        Ok(ClusterConfiguration {
            total_nodes: nodes.len(),
            node_types,
            total_memory_mb,
            total_concurrent_capacity,
        })
    }

    /// Generate performance summary
    async fn generate_performance_summary(&self) -> Result<PerformanceSummary> {
        let results = self.results.read().await;
        
        let total_operations: u64 = results.iter()
            .map(|r| (r.operations_per_second * r.duration.as_secs_f64()) as u64)
            .sum();
        
        let avg_ops_per_second = results.iter()
            .map(|r| r.operations_per_second)
            .sum::<f64>() / results.len() as f64;
        
        let avg_latency = results.iter()
            .map(|r| r.average_latency_ms)
            .sum::<f64>() / results.len() as f64;
        
        let max_throughput = results.iter()
            .map(|r| r.throughput_mb_per_sec)
            .fold(0.0, f64::max);

        Ok(PerformanceSummary {
            total_operations,
            average_ops_per_second: avg_ops_per_second,
            average_latency_ms: avg_latency,
            max_throughput_mb_per_sec: max_throughput,
        })
    }

    /// Get cluster memory usage
    async fn get_cluster_memory_usage(&self) -> Result<f64> {
        // Simulated memory usage
        Ok(512.0)
    }

    /// Get cluster CPU usage
    async fn get_cluster_cpu_usage(&self) -> Result<f64> {
        // Simulated CPU usage
        Ok(45.0)
    }

    /// Collect metadata for benchmark
    async fn collect_metadata(&self, benchmark_name: &str) -> Result<HashMap<String, String>> {
        let mut metadata = HashMap::new();
        
        let nodes = self.cluster.get_active_nodes().await?;
        metadata.insert("active_nodes".to_string(), nodes.len().to_string());
        metadata.insert("benchmark_name".to_string(), benchmark_name.to_string());
        metadata.insert("timestamp".to_string(), chrono::Utc::now().to_rfc3339());
        
        Ok(metadata)
    }
}

/// Distributed benchmark report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistributedBenchmarkReport {
    pub total_duration: Duration,
    pub benchmark_results: Vec<BenchmarkResults>,
    pub cluster_configuration: ClusterConfiguration,
    pub performance_summary: PerformanceSummary,
}

/// Cluster configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterConfiguration {
    pub total_nodes: usize,
    pub node_types: HashMap<String, usize>,
    pub total_memory_mb: u64,
    pub total_concurrent_capacity: usize,
}

/// Performance summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceSummary {
    pub total_operations: u64,
    pub average_ops_per_second: f64,
    pub average_latency_ms: f64,
    pub max_throughput_mb_per_sec: f64,
}

// Helper functions

/// Simulate distributed block indexing
async fn simulate_block_indexing_distributed(
    node_id: &str,
    chain: &str,
    block_number: u64,
) -> bool {
    // Simulate processing time based on block number
    let processing_time = Duration::from_micros(100 + (block_number % 50));
    tokio::time::sleep(processing_time).await;
    
    // Simulate occasional failures
    block_number % 1000 != 999
}

/// Create test distributed query
fn create_test_distributed_query(index: u32) -> DistributedQuery {
    let query_types = ["events", "blocks", "aggregation"];
    let query_type = query_types[index as usize % query_types.len()];
    
    DistributedQuery {
        id: Uuid::new_v4().to_string(),
        query_text: format!("SELECT * FROM {} WHERE index = {}", query_type, index),
        query_type: query_type.to_string(),
        filters: vec![],
        limit: Some(100),
        offset: None,
    }
}

/// Execute distributed query
async fn execute_distributed_query(
    cluster: &DistributedTestCluster,
    query: &DistributedQuery,
) -> Result<Vec<QueryResult>> {
    // Simulate query execution
    tokio::time::sleep(Duration::from_millis(5)).await;
    
    // Return mock results
    Ok(vec![
        QueryResult {
            id: Uuid::new_v4().to_string(),
            data: serde_json::json!({"query_id": query.id}),
            timestamp: chrono::Utc::now().timestamp() as u64,
        }
    ])
}

/// Calculate load balance score
fn calculate_load_balance_score(assignments: &[WorkAssignment]) -> f64 {
    if assignments.is_empty() {
        return 0.0;
    }
    
    // Count assignments per node
    let mut node_counts: HashMap<String, usize> = HashMap::new();
    for assignment in assignments {
        *node_counts.entry(assignment.node_id.clone()).or_insert(0) += 1;
    }
    
    let counts: Vec<usize> = node_counts.values().cloned().collect();
    let avg = counts.iter().sum::<usize>() as f64 / counts.len() as f64;
    let variance = counts.iter()
        .map(|&c| (c as f64 - avg).powi(2))
        .sum::<f64>() / counts.len() as f64;
    
    // Return inverse of coefficient of variation (higher is better)
    if avg > 0.0 {
        1.0 / (1.0 + (variance.sqrt() / avg))
    } else {
        0.0
    }
}

/// Simulate node data transfer
async fn simulate_node_data_transfer(
    source: &str,
    target: &str,
    data: &[Box<dyn indexer_core::types::Event>],
) -> Result<u64> {
    // Calculate data size (approximate)
    let bytes_per_event = 1024; // 1KB per event
    let total_bytes = data.len() as u64 * bytes_per_event;
    
    // Simulate transfer time based on size
    let transfer_time_ms = (total_bytes / 1024) / 100; // 100MB/s simulated speed
    tokio::time::sleep(Duration::from_millis(transfer_time_ms)).await;
    
    Ok(total_bytes)
}

/// Calculate indexing throughput
fn calculate_indexing_throughput(operations: u64, duration: Duration) -> f64 {
    // Assume each block is ~100KB
    let total_mb = (operations * 100) as f64 / 1024.0;
    total_mb / duration.as_secs_f64()
}

/// Calculate query throughput
fn calculate_query_throughput(operations: u64, duration: Duration) -> f64 {
    // Assume each query result is ~10KB
    let total_mb = (operations * 10) as f64 / 1024.0;
    total_mb / duration.as_secs_f64()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_balance_score() {
        // Perfect balance
        let assignments = vec![
            WorkAssignment {
                assignment_id: "1".to_string(),
                node_id: "node1".to_string(),
                chain_id: "eth".to_string(),
                work_type: WorkType::BlockIndexing { from_block: 0, to_block: Some(100) },
                parameters: HashMap::new(),
                assigned_at: 0,
            },
            WorkAssignment {
                assignment_id: "2".to_string(),
                node_id: "node2".to_string(),
                chain_id: "eth".to_string(),
                work_type: WorkType::BlockIndexing { from_block: 100, to_block: Some(200) },
                parameters: HashMap::new(),
                assigned_at: 0,
            },
        ];
        
        let score = calculate_load_balance_score(&assignments);
        assert!(score > 0.9); // Should be close to 1.0 for perfect balance
    }

    #[tokio::test]
    async fn test_distributed_query_creation() {
        let query = create_test_distributed_query(0);
        assert_eq!(query.query_type, "events");
        assert!(query.limit.is_some());
    }
}