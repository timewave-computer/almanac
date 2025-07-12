# Distributed Query Planning

## Overview

The distributed query planning system enables efficient execution of queries across multiple nodes in the Almanac indexer cluster. It analyzes queries, determines optimal routing based on data locality, and aggregates results from multiple nodes.

## Architecture

### Components

1. **DistributedQueryPlanner** - Core query planning engine
   - Analyzes queries and creates execution plans
   - Routes queries based on data locality
   - Manages query caching
   - Estimates query costs

2. **QueryRoutingService** - Query execution orchestrator
   - Manages query executors for each node
   - Executes distributed queries
   - Handles result aggregation

3. **CoordinationService Integration** - Node discovery and management
   - Tracks active nodes and their capabilities
   - Manages work assignments
   - Handles leader election for coordination tasks

### Query Flow

```
1. Client submits query
   ↓
2. Query planner analyzes query
   - Checks cache for existing plans
   - Determines which nodes have relevant data
   - Estimates execution costs
   ↓
3. Create query plan
   - Splits query into sub-queries
   - Assigns sub-queries to nodes
   - Determines aggregation strategy
   ↓
4. Execute sub-queries in parallel
   - Send queries to selected nodes
   - Monitor execution progress
   - Handle timeouts and failures
   ↓
5. Aggregate results
   - Merge results from all nodes
   - Apply deduplication if needed
   - Sort results if requested
   ↓
6. Return results to client
```

## Data Structures

### DistributedQuery

```rust
pub struct DistributedQuery {
    pub base: BaseQuery,        // Filter, pagination, sorting
    pub hints: QueryHints,      // Execution hints
    pub timeout_ms: u64,        // Query timeout
    pub use_cache: bool,        // Whether to use cached results
}
```

### QueryPlan

```rust
pub struct QueryPlan {
    pub plan_id: String,
    pub query: DistributedQuery,
    pub sub_queries: Vec<SubQuery>,
    pub aggregation: AggregationStrategy,
    pub routing_metadata: QueryRoutingMetadata,
    pub cache_key: Option<String>,
}
```

### DataLocality

```rust
pub struct DataLocality {
    pub chains: Vec<ChainId>,
    pub block_ranges: HashMap<ChainId, Vec<(u64, u64)>>,
    pub contracts: Option<Vec<String>>,
    pub cached_queries: Vec<String>,
}
```

## Usage Example

### Basic Query Execution

```rust
use indexer_core::distributed_query_planner::{
    DistributedQuery, DistributedQueryPlanner, QueryHints
};
use indexer_query::{BaseQuery, QueryFilter, Pagination};

// Create query planner
let planner = DistributedQueryPlanner::new(coordination_service);

// Build a distributed query
let query = DistributedQuery {
    base: BaseQuery {
        filter: Some(QueryFilter {
            chain_ids: Some(vec![ChainId::from("ethereum")]),
            contract_addresses: Some(vec!["0x123...".to_string()]),
            ..Default::default()
        }),
        pagination: Some(Pagination { limit: 100, offset: 0 }),
        sorting: None,
    },
    hints: QueryHints::default(),
    timeout_ms: 5000,
    use_cache: true,
};

// Execute query
let plan = planner.create_plan(query).await?;
let results = planner.execute_plan::<Event>(plan).await?;
```

### Advanced Query with Hints

```rust
let query = DistributedQuery {
    base: /* ... */,
    hints: QueryHints {
        preferred_nodes: Some(vec!["node-1".to_string()]),
        required_capabilities: Some(NodeCapabilities {
            supported_chains: vec!["ethereum".to_string()],
            storage_backends: vec!["rocksdb".to_string()],
            ..Default::default()
        }),
        max_parallel: Some(3),
        allow_partial_results: true,
    },
    timeout_ms: 10000,
    use_cache: false,
};
```

### Updating Data Locality

```rust
// Update data locality for a node
planner.update_data_locality(
    "node-1".to_string(),
    DataLocality {
        chains: vec![ChainId::from("ethereum")],
        block_ranges: {
            let mut ranges = HashMap::new();
            ranges.insert(ChainId::from("ethereum"), vec![(1, 1000000)]);
            ranges
        },
        contracts: Some(vec!["0x123...".to_string()]),
        cached_queries: vec!["recent_transfers".to_string()],
    },
).await;
```

## API Endpoints

### REST API

```http
POST /query/events
Content-Type: application/json

{
  "filter": {
    "chain_ids": ["ethereum"],
    "contract_addresses": ["0x123..."],
    "block_range": [100000, 200000]
  },
  "pagination": {
    "limit": 100,
    "offset": 0
  },
  "hints": {
    "max_parallel": 3,
    "allow_partial_results": false
  },
  "timeout_ms": 5000,
  "use_cache": true
}
```

### Query Plan Endpoint

```http
POST /query/plan
Content-Type: application/json

{
  "filter": { /* ... */ },
  "hints": { /* ... */ }
}

Response:
{
  "plan_id": "550e8400-e29b-41d4-a716-446655440000",
  "sub_queries": [
    {
      "node_id": "node-1",
      "estimated_cost": {
        "cpu_cost": 25.0,
        "io_cost": 50.0,
        "network_cost": 5.0,
        "estimated_time_ms": 250
      }
    }
  ],
  "aggregation": "MergeDistinct"
}
```

### Data Locality Update

```http
POST /locality/node-1
Content-Type: application/json

{
  "node_id": "node-1",
  "chains": ["ethereum", "polygon"],
  "block_ranges": {
    "ethereum": [[1, 1000000]],
    "polygon": [[1, 500000]]
  },
  "contracts": ["0x123...", "0x456..."]
}
```

## Aggregation Strategies

### MergeAll
Simply concatenates all results from different nodes. Fast but may contain duplicates.

### MergeDistinct
Removes duplicates based on specified key fields. Useful for ensuring unique results.

```rust
AggregationStrategy::MergeDistinct {
    key_fields: vec!["id".to_string()],
}
```

### MergeAndSort
Merges results and applies sorting. Ensures consistent ordering across distributed results.

```rust
AggregationStrategy::MergeAndSort {
    sorting: Sorting {
        field: "block_number".to_string(),
        direction: SortDirection::Descending,
    },
}
```

### Custom
Allows custom aggregation functions for complex use cases.

## Query Caching

The query planner includes a built-in caching layer:

1. **Cache Key Generation** - Based on query parameters
2. **LRU Eviction** - Configurable cache size with LRU eviction
3. **TTL Support** - Optional time-to-live for cached plans
4. **Invalidation** - Automatic invalidation on data updates

```rust
// Configure cache
let planner = DistributedQueryPlanner::new(coordination)
    .with_cache_size(1000)
    .with_cache_ttl(Duration::from_secs(300));
```

## Performance Considerations

### Query Cost Estimation

The planner estimates costs based on:
- Data volume on each node
- Network latency between nodes
- Current node load
- Query complexity

### Optimization Strategies

1. **Data Locality** - Route queries to nodes with relevant data
2. **Load Balancing** - Distribute queries across available nodes
3. **Parallel Execution** - Execute sub-queries concurrently
4. **Result Streaming** - Stream results as they arrive
5. **Partial Results** - Return partial results on timeout

### Monitoring

Key metrics to monitor:
- Query planning time
- Sub-query execution times
- Cache hit rates
- Node utilization
- Network bandwidth usage

## Error Handling

### Partial Failures

When `allow_partial_results` is enabled:
```rust
let results = planner.execute_plan(plan).await?;
// Results may be incomplete but query won't fail
```

### Timeout Handling

Queries respect the configured timeout:
```rust
let query = DistributedQuery {
    timeout_ms: 5000, // 5 second timeout
    // ...
};
```

### Node Failures

The planner automatically handles node failures:
- Removes failed nodes from active set
- Redistributes queries to healthy nodes
- Updates data locality information

## Integration with FDB Coordination

The distributed query planner integrates seamlessly with the FoundationDB coordination service:

```rust
// Create FDB coordination service
let db = Database::default()?;
let coordination = Arc::new(FdbCoordinationService::new(db, "almanac".to_string()));

// Create query planner with FDB coordination
let planner = DistributedQueryPlanner::new(coordination);
```

## Future Enhancements

1. **Query Optimization** - Cost-based query optimization
2. **Adaptive Routing** - Learn from query patterns
3. **Predictive Caching** - Pre-cache frequently used queries
4. **Query Federation** - Join data across different chains
5. **Real-time Updates** - Push query results via WebSocket