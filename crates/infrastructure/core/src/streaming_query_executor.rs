use crate::{Error, Result};
use crate::distributed_query_planner::{
    QueryExecutor, QueryResult, SubQuery, QueryExecutionMetadata,
    DistributedQuery, QueryPlan, AggregationStrategy,
};
use async_trait::async_trait;
use futures::{Stream, StreamExt, stream};
use serde::{Deserialize, Serialize};
use std::pin::Pin;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::mpsc;
use tokio::task::JoinSet;
use tracing::{debug, info, warn, error};
use bytes::Bytes;
use tokio_stream::wrappers::ReceiverStream;

/// Streaming query result that yields data in chunks
pub type StreamingQueryResult<T> = Pin<Box<dyn Stream<Item = Result<Vec<T>>> + Send>>;

/// Enhanced query executor with streaming support
pub struct StreamingQueryExecutor {
    /// Connection pool for node communication
    connection_pool: Arc<ConnectionPool>,
    /// Maximum concurrent queries per node
    max_concurrent_per_node: usize,
    /// Chunk size for streaming results
    chunk_size: usize,
    /// Enable compression for network transfers
    enable_compression: bool,
}

impl StreamingQueryExecutor {
    pub fn new(connection_pool: Arc<ConnectionPool>) -> Self {
        Self {
            connection_pool,
            max_concurrent_per_node: 10,
            chunk_size: 1000,
            enable_compression: true,
        }
    }

    /// Execute a query and return a stream of results
    pub fn execute_query_stream<T>(&self, query: SubQuery) -> StreamingQueryResult<T>
    where
        T: serde::de::DeserializeOwned + Send + Sync + 'static,
    {
        let node_id = query.node_id.clone();
        let connection_pool = Arc::clone(&self.connection_pool);
        let chunk_size = self.chunk_size;
        let enable_compression = self.enable_compression;

        Box::pin(stream::try_unfold(
            QueryStreamState::new(query, connection_pool, chunk_size, enable_compression),
            |mut state| async move {
                match state.next_chunk().await {
                    Ok(Some(chunk)) => Ok(Some((chunk, state))),
                    Ok(None) => Ok(None),
                    Err(e) => {
                        error!("Error streaming from node {}: {}", node_id, e);
                        Err(e)
                    }
                }
            },
        ))
    }

    /// Execute multiple queries in parallel with streaming
    pub async fn execute_parallel_streaming<T>(
        &self,
        queries: Vec<SubQuery>,
    ) -> StreamingQueryResult<T>
    where
        T: serde::de::DeserializeOwned + Send + Sync + 'static,
    {
        let (tx, rx) = mpsc::channel(100);
        
        // Create a join set for parallel execution
        let mut join_set = JoinSet::new();
        
        for query in queries {
            let executor = self.clone();
            let tx = tx.clone();
            
            join_set.spawn(async move {
                let mut stream = executor.execute_query_stream::<T>(query);
                
                while let Some(result) = stream.next().await {
                    match result {
                        Ok(chunk) => {
                            if tx.send(Ok(chunk)).await.is_err() {
                                break; // Receiver dropped
                            }
                        }
                        Err(e) => {
                            let _ = tx.send(Err(e)).await;
                            break;
                        }
                    }
                }
            });
        }
        
        // Drop the original sender so the channel closes when all tasks complete
        drop(tx);
        
        // Spawn a task to wait for all queries to complete
        tokio::spawn(async move {
            while let Some(_) = join_set.join_next().await {}
        });
        
        Box::pin(ReceiverStream::new(rx))
    }
}

impl Clone for StreamingQueryExecutor {
    fn clone(&self) -> Self {
        Self {
            connection_pool: Arc::clone(&self.connection_pool),
            max_concurrent_per_node: self.max_concurrent_per_node,
            chunk_size: self.chunk_size,
            enable_compression: self.enable_compression,
        }
    }
}

#[async_trait]
impl QueryExecutor for StreamingQueryExecutor {
    async fn execute_query<T>(&self, query: SubQuery) -> Result<QueryResult<T>>
    where
        T: serde::de::DeserializeOwned + Send + Sync,
    {
        let start_time = Instant::now();
        let node_id = query.node_id.clone();
        
        // Collect all results from the stream
        let mut all_results = Vec::new();
        let mut stream = self.execute_query_stream::<T>(query);
        let mut records_scanned = 0u64;
        
        while let Some(chunk_result) = stream.next().await {
            match chunk_result {
                Ok(chunk) => {
                    records_scanned += chunk.len() as u64;
                    all_results.extend(chunk);
                }
                Err(e) => return Err(e),
            }
        }
        
        let execution_time_ms = start_time.elapsed().as_millis() as u64;
        
        Ok(QueryResult {
            node_id,
            data: all_results,
            metadata: QueryExecutionMetadata {
                execution_time_ms,
                records_scanned,
                cache_hit_rate: 0.0, // Would be tracked by actual implementation
                warnings: Vec::new(),
            },
        })
    }
}

/// State for streaming query execution
struct QueryStreamState {
    query: SubQuery,
    connection_pool: Arc<ConnectionPool>,
    chunk_size: usize,
    enable_compression: bool,
    current_offset: usize,
    is_complete: bool,
}

impl QueryStreamState {
    fn new(
        query: SubQuery,
        connection_pool: Arc<ConnectionPool>,
        chunk_size: usize,
        enable_compression: bool,
    ) -> Self {
        Self {
            query,
            connection_pool,
            chunk_size,
            enable_compression,
            current_offset: 0,
            is_complete: false,
        }
    }

    async fn next_chunk<T>(&mut self) -> Result<Option<Vec<T>>>
    where
        T: serde::de::DeserializeOwned + Send + Sync,
    {
        if self.is_complete {
            return Ok(None);
        }

        // Get connection from pool
        let connection = self.connection_pool
            .get_connection(&self.query.node_id)
            .await?;

        // Create chunk request
        let chunk_request = ChunkRequest {
            query_filter: self.query.filter.clone(),
            offset: self.current_offset,
            limit: self.chunk_size,
            enable_compression: self.enable_compression,
        };

        // Execute chunk query
        let response: ChunkResponse<T> = connection
            .execute_chunk_query(chunk_request)
            .await?;

        // Update state
        self.current_offset += response.data.len();
        self.is_complete = response.is_last_chunk;

        if response.data.is_empty() {
            self.is_complete = true;
            Ok(None)
        } else {
            Ok(Some(response.data))
        }
    }
}

/// Connection pool for managing connections to distributed nodes
pub struct ConnectionPool {
    /// Pool of connections indexed by node ID
    connections: Arc<dashmap::DashMap<String, ConnectionPoolEntry>>,
    /// Maximum connections per node
    max_connections_per_node: usize,
    /// Connection timeout
    connection_timeout: std::time::Duration,
    /// Keep-alive interval
    keep_alive_interval: std::time::Duration,
}

struct ConnectionPoolEntry {
    /// Available connections
    available: Vec<NodeConnection>,
    /// Active connection count
    active_count: usize,
    /// Last access time
    last_access: Instant,
}

impl ConnectionPool {
    pub fn new(max_connections_per_node: usize) -> Self {
        Self {
            connections: Arc::new(dashmap::DashMap::new()),
            max_connections_per_node,
            connection_timeout: std::time::Duration::from_secs(30),
            keep_alive_interval: std::time::Duration::from_secs(60),
        }
    }

    /// Get a connection to a node
    pub async fn get_connection(&self, node_id: &str) -> Result<NodeConnection> {
        // Try to get existing connection
        if let Some(mut entry) = self.connections.get_mut(node_id) {
            if let Some(conn) = entry.available.pop() {
                entry.active_count += 1;
                entry.last_access = Instant::now();
                return Ok(conn);
            }
            
            // Check if we can create a new connection
            if entry.active_count < self.max_connections_per_node {
                entry.active_count += 1;
                entry.last_access = Instant::now();
                drop(entry); // Release lock before creating connection
                return self.create_connection(node_id).await;
            }
        } else {
            // Create new entry and connection
            self.connections.insert(node_id.to_string(), ConnectionPoolEntry {
                available: Vec::new(),
                active_count: 1,
                last_access: Instant::now(),
            });
            return self.create_connection(node_id).await;
        }

        // All connections are in use, wait for one to become available
        // In a real implementation, this would implement proper waiting logic
        Err(Error::Generic("Connection pool exhausted".to_string()))
    }

    /// Return a connection to the pool
    pub fn return_connection(&self, node_id: String, connection: NodeConnection) {
        if let Some(mut entry) = self.connections.get_mut(&node_id) {
            entry.active_count = entry.active_count.saturating_sub(1);
            entry.available.push(connection);
        }
    }

    /// Create a new connection to a node
    async fn create_connection(&self, node_id: &str) -> Result<NodeConnection> {
        // In a real implementation, this would establish actual network connection
        Ok(NodeConnection {
            node_id: node_id.to_string(),
            client: Arc::new(MockNodeClient),
        })
    }

    /// Periodic cleanup of idle connections
    pub async fn cleanup_idle_connections(&self) {
        let idle_timeout = std::time::Duration::from_secs(300); // 5 minutes
        let now = Instant::now();

        self.connections.retain(|_, entry| {
            now.duration_since(entry.last_access) < idle_timeout || entry.active_count > 0
        });
    }
}

/// Connection to a distributed node
pub struct NodeConnection {
    node_id: String,
    client: Arc<dyn NodeClient>,
}

impl NodeConnection {
    /// Execute a chunk query on this connection
    async fn execute_chunk_query<T>(&self, request: ChunkRequest) -> Result<ChunkResponse<T>>
    where
        T: serde::de::DeserializeOwned,
    {
        self.client.query_chunk(request).await
    }
}

/// Trait for node client implementations
#[async_trait]
trait NodeClient: Send + Sync {
    /// Query a chunk of data
    async fn query_chunk<T>(&self, request: ChunkRequest) -> Result<ChunkResponse<T>>
    where
        T: serde::de::DeserializeOwned;
}

/// Mock implementation for testing
struct MockNodeClient;

#[async_trait]
impl NodeClient for MockNodeClient {
    async fn query_chunk<T>(&self, request: ChunkRequest) -> Result<ChunkResponse<T>>
    where
        T: serde::de::DeserializeOwned,
    {
        // Mock implementation returns empty results
        Ok(ChunkResponse {
            data: Vec::new(),
            is_last_chunk: true,
            total_records: Some(0),
        })
    }
}

/// Request for querying a chunk of data
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ChunkRequest {
    query_filter: indexer_query::QueryFilter,
    offset: usize,
    limit: usize,
    enable_compression: bool,
}

/// Response containing a chunk of data
#[derive(Debug, Serialize, Deserialize)]
struct ChunkResponse<T> {
    data: Vec<T>,
    is_last_chunk: bool,
    total_records: Option<usize>,
}

/// Parallel query orchestrator that manages distributed execution
pub struct ParallelQueryOrchestrator {
    executor: StreamingQueryExecutor,
    max_parallel_queries: usize,
    aggregation_buffer_size: usize,
}

impl ParallelQueryOrchestrator {
    pub fn new(executor: StreamingQueryExecutor) -> Self {
        Self {
            executor,
            max_parallel_queries: 50,
            aggregation_buffer_size: 10000,
        }
    }

    /// Execute a query plan with parallel streaming
    pub async fn execute_plan_streaming<T>(
        &self,
        plan: QueryPlan,
    ) -> StreamingQueryResult<T>
    where
        T: serde::de::DeserializeOwned + Send + Sync + 'static,
    {
        info!("Executing query plan {} with {} sub-queries", 
              plan.plan_id, plan.sub_queries.len());

        // Execute sub-queries in parallel with concurrency limit
        let chunks = plan.sub_queries.chunks(self.max_parallel_queries);
        let (tx, rx) = mpsc::channel(100);

        for chunk in chunks {
            let executor = self.executor.clone();
            let queries = chunk.to_vec();
            let aggregation = plan.aggregation.clone();
            let tx = tx.clone();

            tokio::spawn(async move {
                let stream = executor.execute_parallel_streaming::<T>(queries).await;
                tokio::pin!(stream);

                while let Some(result) = stream.next().await {
                    if tx.send(result).await.is_err() {
                        break; // Receiver dropped
                    }
                }
            });
        }

        drop(tx);

        // Apply aggregation if needed
        match plan.aggregation {
            AggregationStrategy::MergeAll => {
                Box::pin(ReceiverStream::new(rx))
            }
            _ => {
                // For other strategies, we need to buffer and aggregate
                Box::pin(self.create_aggregated_stream(rx, plan.aggregation))
            }
        }
    }

    /// Create an aggregated stream based on the aggregation strategy
    fn create_aggregated_stream<T>(
        &self,
        mut rx: mpsc::Receiver<Result<Vec<T>>>,
        strategy: AggregationStrategy,
    ) -> impl Stream<Item = Result<Vec<T>>>
    where
        T: serde::de::DeserializeOwned + serde::Serialize + Send + Sync + 'static,
    {
        let buffer_size = self.aggregation_buffer_size;

        stream::try_unfold(
            (rx, Vec::new(), strategy),
            move |(mut rx, mut buffer, strategy)| async move {
                while let Some(result) = rx.recv().await {
                    match result {
                        Ok(chunk) => {
                            buffer.extend(chunk);
                            
                            if buffer.len() >= buffer_size {
                                let aggregated = apply_aggregation(&buffer, &strategy)?;
                                buffer.clear();
                                return Ok(Some((aggregated, (rx, buffer, strategy))));
                            }
                        }
                        Err(e) => return Err(e),
                    }
                }

                // Process remaining buffer
                if !buffer.is_empty() {
                    let aggregated = apply_aggregation(&buffer, &strategy)?;
                    Ok(Some((aggregated, (rx, Vec::new(), strategy))))
                } else {
                    Ok(None)
                }
            },
        )
    }
}

/// Apply aggregation strategy to a buffer of results
fn apply_aggregation<T>(
    buffer: &[T],
    strategy: &AggregationStrategy,
) -> Result<Vec<T>>
where
    T: serde::de::DeserializeOwned + serde::Serialize + Clone,
{
    match strategy {
        AggregationStrategy::MergeAll => Ok(buffer.to_vec()),
        AggregationStrategy::MergeDistinct { key_fields } => {
            // Simple deduplication based on serialization
            let mut seen = std::collections::HashSet::new();
            let mut result = Vec::new();
            
            for item in buffer {
                let key = serde_json::to_string(item)?;
                if seen.insert(key) {
                    result.push(item.clone());
                }
            }
            
            Ok(result)
        }
        AggregationStrategy::MergeAndSort { sorting } => {
            // In a real implementation, would sort based on the field
            Ok(buffer.to_vec())
        }
        AggregationStrategy::Custom { .. } => {
            Err(Error::Generic("Custom aggregation not implemented".to_string()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::distributed_query_planner::DataLocality;
    use indexer_query::QueryFilter;

    #[tokio::test]
    async fn test_streaming_executor() {
        let pool = Arc::new(ConnectionPool::new(10));
        let executor = StreamingQueryExecutor::new(pool);

        let query = SubQuery {
            node_id: "test_node".to_string(),
            filter: QueryFilter {
                chain_ids: None,
                contract_addresses: None,
                contract_types: None,
                entity_ids: None,
                attributes: None,
                tags: None,
            },
            pagination: None,
            estimated_cost: Default::default(),
            locality: DataLocality {
                chains: Vec::new(),
                block_ranges: Default::default(),
                contracts: None,
                cached_queries: Vec::new(),
            },
        };

        let mut stream = executor.execute_query_stream::<serde_json::Value>(query);
        
        // Should complete without error
        while let Some(result) = stream.next().await {
            assert!(result.is_ok());
        }
    }

    #[tokio::test]
    async fn test_connection_pool() {
        let pool = ConnectionPool::new(2);
        
        // Should be able to get connections up to the limit
        let conn1 = pool.get_connection("node1").await.unwrap();
        let conn2 = pool.get_connection("node1").await.unwrap();
        
        // Return one connection
        pool.return_connection("node1".to_string(), conn1);
        
        // Should be able to get it again
        let _conn3 = pool.get_connection("node1").await.unwrap();
    }
}