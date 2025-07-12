use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use indexer_core::distributed_query_planner::{DistributedQueryPlanner, QueryPlan};
use indexer_core::coordination::{NodeInfo, NodeCapability, NodeStatus};
use indexer_core::types::{ChainId, EventFilter};
use std::sync::Arc;
use tokio::runtime::Runtime;

fn setup_planner_with_nodes(node_count: usize) -> (Runtime, Arc<DistributedQueryPlanner>) {
    let rt = Runtime::new().unwrap();
    let planner = Arc::new(DistributedQueryPlanner::new());
    
    rt.block_on(async {
        for i in 0..node_count {
            let node = NodeInfo {
                node_id: format!("node{}", i),
                address: format!("127.0.0.1:{}", 8000 + i),
                status: NodeStatus::Active,
                capabilities: vec![
                    NodeCapability::ChainSupport { 
                        chain_id: ChainId(format!("chain{}", i % 3))
                    },
                    NodeCapability::BlockRangeSupport {
                        chain_id: ChainId(format!("chain{}", i % 3)),
                        from_block: i as u64 * 10000,
                        to_block: (i + 1) as u64 * 10000,
                    },
                ],
                heartbeat: std::time::SystemTime::now(),
            };
            planner.register_node(node).await;
        }
    });
    
    (rt, planner)
}

fn benchmark_query_planning(c: &mut Criterion) {
    let mut group = c.benchmark_group("query_planning");
    
    for node_count in [10, 50, 100, 500].iter() {
        let (rt, planner) = setup_planner_with_nodes(*node_count);
        
        group.bench_with_input(
            BenchmarkId::from_parameter(node_count),
            node_count,
            |b, _| {
                b.iter(|| {
                    rt.block_on(async {
                        let filter = EventFilter {
                            chain_ids: Some(vec![ChainId("chain0".to_string())]),
                            block_range: Some((5000, 15000)),
                            ..Default::default()
                        };
                        black_box(planner.plan_query(vec![filter]).await.unwrap())
                    })
                });
            },
        );
    }
    
    group.finish();
}

fn benchmark_parallel_query_execution(c: &mut Criterion) {
    let mut group = c.benchmark_group("parallel_execution");
    
    let (rt, planner) = setup_planner_with_nodes(10);
    
    // Benchmark different numbers of parallel queries
    for query_count in [1, 5, 10, 20, 50].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(query_count),
            query_count,
            |b, &count| {
                b.iter(|| {
                    rt.block_on(async {
                        let mut handles = vec![];
                        
                        for i in 0..count {
                            let planner = planner.clone();
                            let handle = tokio::spawn(async move {
                                let filter = EventFilter {
                                    chain_ids: Some(vec![ChainId(format!("chain{}", i % 3))]),
                                    block_range: Some((i as u64 * 1000, (i + 1) as u64 * 1000)),
                                    ..Default::default()
                                };
                                planner.plan_query(vec![filter]).await.unwrap()
                            });
                            handles.push(handle);
                        }
                        
                        for handle in handles {
                            black_box(handle.await.unwrap());
                        }
                    })
                });
            },
        );
    }
    
    group.finish();
}

fn benchmark_query_caching(c: &mut Criterion) {
    let mut group = c.benchmark_group("query_caching");
    
    let (rt, planner) = setup_planner_with_nodes(50);
    
    // Pre-populate cache
    rt.block_on(async {
        for i in 0..100 {
            let filter = EventFilter {
                chain_ids: Some(vec![ChainId(format!("chain{}", i % 3))]),
                block_range: Some((i as u64 * 100, (i + 1) as u64 * 100)),
                ..Default::default()
            };
            planner.plan_query(vec![filter]).await.unwrap();
        }
    });
    
    // Benchmark cache hits
    group.bench_function("cache_hits", |b| {
        b.iter(|| {
            rt.block_on(async {
                let filter = EventFilter {
                    chain_ids: Some(vec![ChainId("chain0".to_string())]),
                    block_range: Some((0, 100)),
                    ..Default::default()
                };
                black_box(planner.plan_query(vec![filter]).await.unwrap())
            })
        });
    });
    
    // Benchmark cache misses
    group.bench_function("cache_misses", |b| {
        let mut counter = 1000;
        b.iter(|| {
            rt.block_on(async {
                counter += 1;
                let filter = EventFilter {
                    chain_ids: Some(vec![ChainId("chain_new".to_string())]),
                    block_range: Some((counter, counter + 100)),
                    ..Default::default()
                };
                black_box(planner.plan_query(vec![filter]).await.unwrap())
            })
        });
    });
    
    group.finish();
}

fn benchmark_aggregation_strategies(c: &mut Criterion) {
    use indexer_core::distributed_query_planner::AggregationStrategy;
    
    let mut group = c.benchmark_group("aggregation");
    
    // Generate test data
    let data_sizes = vec![100, 1000, 10000, 100000];
    
    for size in data_sizes {
        let data: Vec<Vec<u8>> = (0..size)
            .map(|i| format!("event_{}", i).into_bytes())
            .collect();
        
        group.bench_with_input(
            BenchmarkId::new("merge_all", size),
            &data,
            |b, data| {
                b.iter(|| {
                    let strategy = AggregationStrategy::MergeAll;
                    // Simulate aggregation
                    let mut result = Vec::new();
                    for item in data {
                        result.extend_from_slice(item);
                    }
                    black_box(result)
                });
            },
        );
        
        group.bench_with_input(
            BenchmarkId::new("merge_distinct", size),
            &data,
            |b, data| {
                b.iter(|| {
                    use std::collections::HashSet;
                    let strategy = AggregationStrategy::MergeDistinct;
                    // Simulate deduplication
                    let set: HashSet<_> = data.iter().cloned().collect();
                    let result: Vec<_> = set.into_iter().collect();
                    black_box(result)
                });
            },
        );
    }
    
    group.finish();
}

fn benchmark_node_selection(c: &mut Criterion) {
    let mut group = c.benchmark_group("node_selection");
    
    let (rt, planner) = setup_planner_with_nodes(100);
    
    // Add data locality information
    rt.block_on(async {
        for i in 0..100 {
            planner.update_data_locality(
                &format!("node{}", i),
                vec![format!("contract_{}", i % 10)],
            ).await;
        }
    });
    
    group.bench_function("with_locality", |b| {
        b.iter(|| {
            rt.block_on(async {
                let mut filter = EventFilter {
                    chain_ids: Some(vec![ChainId("chain0".to_string())]),
                    ..Default::default()
                };
                filter.custom_filters.insert(
                    "contract".to_string(),
                    "contract_5".to_string(),
                );
                black_box(planner.plan_query(vec![filter]).await.unwrap())
            })
        });
    });
    
    group.bench_function("without_locality", |b| {
        b.iter(|| {
            rt.block_on(async {
                let filter = EventFilter {
                    chain_ids: Some(vec![ChainId("chain1".to_string())]),
                    ..Default::default()
                };
                black_box(planner.plan_query(vec![filter]).await.unwrap())
            })
        });
    });
    
    group.finish();
}

criterion_group!(
    benches,
    benchmark_query_planning,
    benchmark_parallel_query_execution,
    benchmark_query_caching,
    benchmark_aggregation_strategies,
    benchmark_node_selection
);
criterion_main!(benches);