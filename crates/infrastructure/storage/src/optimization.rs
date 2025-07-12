// Comprehensive storage optimization system for cross-chain indexer
//
// This module provides smart partitioning by chain and time, compression
// for historical data, optimized indexes, and automatic archival/cleanup.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};

use crate::{Result, Error};

/// Partition strategy for organizing data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PartitionStrategy {
    /// Partition by time periods
    Time {
        /// Duration of each partition
        interval: PartitionInterval,
    },
    /// Partition by chain
    Chain {
        /// Maximum events per partition
        max_events_per_partition: usize,
    },
    /// Partition by both time and chain
    Hybrid {
        /// Time interval for partitioning
        time_interval: PartitionInterval,
        /// Maximum events per chain partition
        max_events_per_chain: usize,
    },
    /// Custom partitioning logic
    Custom {
        /// Custom partition key generator
        partition_key: String,
    },
}

/// Time intervals for partitioning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PartitionInterval {
    /// Partition by hour
    Hourly,
    /// Partition by day
    Daily,
    /// Partition by week
    Weekly,
    /// Partition by month
    Monthly,
    /// Custom interval in seconds
    Custom(u64),
}

impl PartitionInterval {
    /// Get duration in seconds
    pub fn duration_seconds(&self) -> u64 {
        match self {
            PartitionInterval::Hourly => 3600,
            PartitionInterval::Daily => 86400,
            PartitionInterval::Weekly => 604800,
            PartitionInterval::Monthly => 2592000, // 30 days
            PartitionInterval::Custom(seconds) => *seconds,
        }
    }
    
    /// Get partition key for a given timestamp
    pub fn partition_key(&self, timestamp: SystemTime) -> String {
        let epoch_seconds = timestamp
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        
        let interval_seconds = self.duration_seconds();
        let partition_number = epoch_seconds / interval_seconds;
        
        match self {
            PartitionInterval::Hourly => format!("hour_{}", partition_number),
            PartitionInterval::Daily => format!("day_{}", partition_number),
            PartitionInterval::Weekly => format!("week_{}", partition_number),
            PartitionInterval::Monthly => format!("month_{}", partition_number),
            PartitionInterval::Custom(_) => format!("custom_{}", partition_number),
        }
    }
}

/// Compression configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressionConfig {
    /// Compression algorithm to use
    pub algorithm: CompressionAlgorithm,
    /// Compression level (1-9, higher = better compression but slower)
    pub level: u32,
    /// Age threshold for compression (data older than this gets compressed)
    pub age_threshold: Duration,
    /// Whether to compress data in background
    pub background_compression: bool,
    /// Batch size for compression operations
    pub compression_batch_size: usize,
}

/// Compression algorithms
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CompressionAlgorithm {
    /// No compression
    None,
    /// Gzip compression
    Gzip,
    /// LZ4 compression (fast)
    Lz4,
    /// Zstd compression (balanced)
    Zstd,
    /// Brotli compression (high ratio)
    Brotli,
}

/// Index optimization configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexConfig {
    /// Indexes to create/maintain
    pub indexes: Vec<IndexDefinition>,
    /// Whether to analyze query patterns automatically
    pub auto_analyze: bool,
    /// Interval for index maintenance
    pub maintenance_interval: Duration,
    /// Whether to create covering indexes
    pub covering_indexes: bool,
}

/// Index definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexDefinition {
    /// Index name
    pub name: String,
    /// Table name
    pub table: String,
    /// Columns to index
    pub columns: Vec<String>,
    /// Index type
    pub index_type: IndexType,
    /// Whether this is a unique index
    pub unique: bool,
    /// Partial index condition (WHERE clause)
    pub condition: Option<String>,
    /// Include columns for covering index
    pub include_columns: Option<Vec<String>>,
}

/// Types of indexes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IndexType {
    /// B-tree index (default)
    BTree,
    /// Hash index
    Hash,
    /// GIN index (for arrays, JSON)
    Gin,
    /// GiST index (for geometric types)
    Gist,
    /// Partial index
    Partial,
    /// Expression index
    Expression(String),
}

/// Archival configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchivalConfig {
    /// Age threshold for archival
    pub age_threshold: Duration,
    /// Archive storage location
    pub archive_location: ArchiveLocation,
    /// Whether to delete archived data from primary storage
    pub delete_after_archive: bool,
    /// Verification of archived data
    pub verify_archive: bool,
    /// Archival batch size
    pub batch_size: usize,
    /// Compression for archived data
    pub archive_compression: CompressionConfig,
}

/// Archive storage locations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ArchiveLocation {
    /// Local filesystem
    Local { path: String },
    /// S3-compatible storage
    S3 {
        bucket: String,
        region: String,
        access_key: String,
        secret_key: String,
    },
    /// Google Cloud Storage
    Gcs {
        bucket: String,
        credentials_path: String,
    },
    /// Azure Blob Storage
    Azure {
        account: String,
        container: String,
        access_key: String,
    },
}

/// Storage optimization manager
#[derive(Debug)]
pub struct StorageOptimizer {
    /// Partition strategy
    partition_strategy: PartitionStrategy,
    /// Compression configuration
    compression_config: CompressionConfig,
    /// Index configuration
    index_config: IndexConfig,
    /// Archival configuration
    archival_config: Option<ArchivalConfig>,
    /// Partition metadata
    partitions: Arc<RwLock<HashMap<String, PartitionMetadata>>>,
    /// Optimization statistics
    stats: Arc<RwLock<OptimizationStats>>,
}

/// Metadata for a partition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartitionMetadata {
    /// Partition identifier
    pub partition_id: String,
    /// Number of events in partition
    pub event_count: u64,
    /// Size in bytes
    pub size_bytes: u64,
    /// Compressed size (if compressed)
    pub compressed_size_bytes: Option<u64>,
    /// Creation timestamp
    pub created_at: SystemTime,
    /// Last access timestamp
    pub last_accessed: SystemTime,
    /// Whether partition is compressed
    pub is_compressed: bool,
    /// Whether partition is archived
    pub is_archived: bool,
    /// Chains included in this partition
    pub chains: Vec<String>,
    /// Time range covered by partition
    pub time_range: Option<(SystemTime, SystemTime)>,
}

/// Optimization statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationStats {
    /// Total partitions
    pub total_partitions: u64,
    /// Compressed partitions
    pub compressed_partitions: u64,
    /// Archived partitions
    pub archived_partitions: u64,
    /// Total storage savings from compression
    pub compression_savings_bytes: u64,
    /// Total archived data size
    pub archived_data_bytes: u64,
    /// Index maintenance operations
    pub index_maintenance_count: u64,
    /// Last optimization run
    pub last_optimization: SystemTime,
    /// Average query performance improvement
    pub avg_query_improvement_percent: f64,
}

impl StorageOptimizer {
    /// Create a new storage optimizer
    pub fn new(
        partition_strategy: PartitionStrategy,
        compression_config: CompressionConfig,
        index_config: IndexConfig,
        archival_config: Option<ArchivalConfig>,
    ) -> Self {
        Self {
            partition_strategy,
            compression_config,
            index_config,
            archival_config,
            partitions: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(RwLock::new(OptimizationStats {
                total_partitions: 0,
                compressed_partitions: 0,
                archived_partitions: 0,
                compression_savings_bytes: 0,
                archived_data_bytes: 0,
                index_maintenance_count: 0,
                last_optimization: SystemTime::now(),
                avg_query_improvement_percent: 0.0,
            })),
        }
    }
    
    /// Get partition key for an event
    pub fn get_partition_key(&self, chain_id: &str, timestamp: SystemTime) -> String {
        match &self.partition_strategy {
            PartitionStrategy::Time { interval } => {
                interval.partition_key(timestamp)
            }
            PartitionStrategy::Chain { .. } => {
                format!("chain_{}", chain_id)
            }
            PartitionStrategy::Hybrid { time_interval, .. } => {
                let time_key = time_interval.partition_key(timestamp);
                format!("{}_{}", time_key, chain_id)
            }
            PartitionStrategy::Custom { partition_key } => {
                format!("{}_{}", partition_key, chain_id)
            }
        }
    }
    
    /// Create a new partition
    pub async fn create_partition(&self, partition_id: String, chain_id: String) -> Result<()> {
        let mut partitions = self.partitions.write().await;
        
        let metadata = PartitionMetadata {
            partition_id: partition_id.clone(),
            event_count: 0,
            size_bytes: 0,
            compressed_size_bytes: None,
            created_at: SystemTime::now(),
            last_accessed: SystemTime::now(),
            is_compressed: false,
            is_archived: false,
            chains: vec![chain_id],
            time_range: None,
        };
        
        partitions.insert(partition_id, metadata);
        
        // Update stats
        let mut stats = self.stats.write().await;
        stats.total_partitions += 1;
        
        Ok(())
    }
    
    /// Update partition metadata after adding events
    pub async fn update_partition(&self, partition_id: &str, event_count: u64, size_bytes: u64) -> Result<()> {
        let mut partitions = self.partitions.write().await;
        
        if let Some(metadata) = partitions.get_mut(partition_id) {
            metadata.event_count += event_count;
            metadata.size_bytes += size_bytes;
            metadata.last_accessed = SystemTime::now();
        } else {
            return Err(Error::generic(format!("Partition {} not found", partition_id)));
        }
        
        Ok(())
    }
    
    /// Compress old partitions
    pub async fn compress_partitions(&self) -> Result<Vec<String>> {
        let threshold = SystemTime::now() - self.compression_config.age_threshold;
        let mut compressed_partitions = Vec::new();
        let mut partitions = self.partitions.write().await;
        
        for (partition_id, metadata) in partitions.iter_mut() {
            if !metadata.is_compressed && metadata.created_at < threshold {
                // Simulate compression (in real implementation, this would call storage layer)
                let original_size = metadata.size_bytes;
                let compression_ratio = self.get_compression_ratio();
                let compressed_size = (original_size as f64 * compression_ratio) as u64;
                
                metadata.compressed_size_bytes = Some(compressed_size);
                metadata.is_compressed = true;
                
                compressed_partitions.push(partition_id.clone());
                
                // Update stats
                let mut stats = self.stats.write().await;
                stats.compressed_partitions += 1;
                stats.compression_savings_bytes += original_size - compressed_size;
            }
        }
        
        Ok(compressed_partitions)
    }
    
    /// Archive old partitions
    pub async fn archive_partitions(&self) -> Result<Vec<String>> {
        if let Some(ref archival_config) = self.archival_config {
            let threshold = SystemTime::now() - archival_config.age_threshold;
            let mut archived_partitions = Vec::new();
            let mut partitions = self.partitions.write().await;
            
            for (partition_id, metadata) in partitions.iter_mut() {
                if !metadata.is_archived && metadata.last_accessed < threshold {
                    // Simulate archival (in real implementation, this would upload to archive storage)
                    let archive_size = metadata.compressed_size_bytes.unwrap_or(metadata.size_bytes);
                    
                    metadata.is_archived = true;
                    archived_partitions.push(partition_id.clone());
                    
                    // Update stats
                    let mut stats = self.stats.write().await;
                    stats.archived_partitions += 1;
                    stats.archived_data_bytes += archive_size;
                }
            }
            
            Ok(archived_partitions)
        } else {
            Ok(Vec::new())
        }
    }
    
    /// Optimize indexes based on query patterns
    pub async fn optimize_indexes(&self) -> Result<Vec<String>> {
        let mut optimized_indexes = Vec::new();
        
        for index_def in &self.index_config.indexes {
            // Simulate index optimization (in real implementation, this would analyze and optimize indexes)
            optimized_indexes.push(index_def.name.clone());
        }
        
        // Update stats
        let mut stats = self.stats.write().await;
        stats.index_maintenance_count += 1;
        stats.last_optimization = SystemTime::now();
        
        Ok(optimized_indexes)
    }
    
    /// Run full optimization cycle
    pub async fn run_optimization(&self) -> Result<OptimizationReport> {
        let start_time = SystemTime::now();
        
        // Compress old partitions
        let compressed = self.compress_partitions().await?;
        
        // Archive old partitions
        let archived = self.archive_partitions().await?;
        
        // Optimize indexes
        let optimized_indexes = self.optimize_indexes().await?;
        
        // Clean up empty partitions
        let cleaned_partitions = self.cleanup_empty_partitions().await?;
        
        let duration = start_time.elapsed().unwrap_or_default();
        
        Ok(OptimizationReport {
            duration,
            compressed_partitions: compressed,
            archived_partitions: archived,
            optimized_indexes,
            cleaned_partitions,
            total_space_saved: self.calculate_space_saved().await,
        })
    }
    
    /// Cleanup empty or redundant partitions
    pub async fn cleanup_empty_partitions(&self) -> Result<Vec<String>> {
        let mut cleaned_partitions = Vec::new();
        let mut partitions = self.partitions.write().await;
        
        let empty_partitions: Vec<String> = partitions
            .iter()
            .filter(|(_, metadata)| metadata.event_count == 0)
            .map(|(id, _)| id.clone())
            .collect();
        
        for partition_id in empty_partitions {
            partitions.remove(&partition_id);
            cleaned_partitions.push(partition_id);
        }
        
        Ok(cleaned_partitions)
    }
    
    /// Calculate total space saved by optimizations
    pub async fn calculate_space_saved(&self) -> u64 {
        self.stats.read().await.compression_savings_bytes
    }
    
    /// Get compression ratio based on algorithm
    fn get_compression_ratio(&self) -> f64 {
        match self.compression_config.algorithm {
            CompressionAlgorithm::None => 1.0,
            CompressionAlgorithm::Gzip => 0.3,
            CompressionAlgorithm::Lz4 => 0.5,
            CompressionAlgorithm::Zstd => 0.25,
            CompressionAlgorithm::Brotli => 0.2,
        }
    }
    
    /// Get partition statistics
    pub async fn get_partition_stats(&self) -> HashMap<String, PartitionMetadata> {
        self.partitions.read().await.clone()
    }
    
    /// Get optimization statistics
    pub async fn get_optimization_stats(&self) -> OptimizationStats {
        self.stats.read().await.clone()
    }
    
    /// Get recommended optimizations
    pub async fn get_optimization_recommendations(&self) -> Vec<OptimizationRecommendation> {
        let mut recommendations = Vec::new();
        let partitions = self.partitions.read().await;
        let threshold = SystemTime::now() - self.compression_config.age_threshold;
        
        // Check for uncompressed old partitions
        let uncompressed_old: Vec<_> = partitions
            .iter()
            .filter(|(_, metadata)| !metadata.is_compressed && metadata.created_at < threshold)
            .collect();
        
        if !uncompressed_old.is_empty() {
            recommendations.push(OptimizationRecommendation {
                recommendation_type: RecommendationType::Compression,
                description: format!("Compress {} old partitions to save space", uncompressed_old.len()),
                potential_savings_bytes: uncompressed_old
                    .iter()
                    .map(|(_, metadata)| metadata.size_bytes)
                    .sum::<u64>() * 70 / 100, // Assume 70% compression
                priority: RecommendationPriority::Medium,
            });
        }
        
        // Check for archival candidates
        if let Some(ref archival_config) = self.archival_config {
            let archive_threshold = SystemTime::now() - archival_config.age_threshold;
            let archival_candidates: Vec<_> = partitions
                .iter()
                .filter(|(_, metadata)| !metadata.is_archived && metadata.last_accessed < archive_threshold)
                .collect();
            
            if !archival_candidates.is_empty() {
                recommendations.push(OptimizationRecommendation {
                    recommendation_type: RecommendationType::Archival,
                    description: format!("Archive {} old partitions to reduce storage costs", archival_candidates.len()),
                    potential_savings_bytes: archival_candidates
                        .iter()
                        .map(|(_, metadata)| metadata.compressed_size_bytes.unwrap_or(metadata.size_bytes))
                        .sum(),
                    priority: RecommendationPriority::Low,
                });
            }
        }
        
        recommendations
    }
}

/// Optimization report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationReport {
    /// Duration of optimization run
    pub duration: Duration,
    /// Partitions that were compressed
    pub compressed_partitions: Vec<String>,
    /// Partitions that were archived
    pub archived_partitions: Vec<String>,
    /// Indexes that were optimized
    pub optimized_indexes: Vec<String>,
    /// Partitions that were cleaned up
    pub cleaned_partitions: Vec<String>,
    /// Total space saved in bytes
    pub total_space_saved: u64,
}

/// Optimization recommendation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationRecommendation {
    /// Type of recommendation
    pub recommendation_type: RecommendationType,
    /// Description of the recommendation
    pub description: String,
    /// Potential space savings in bytes
    pub potential_savings_bytes: u64,
    /// Priority of this recommendation
    pub priority: RecommendationPriority,
}

/// Types of optimization recommendations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RecommendationType {
    Compression,
    Archival,
    IndexOptimization,
    PartitionCleanup,
    SchemaOptimization,
}

/// Priority levels for recommendations
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum RecommendationPriority {
    Low,
    Medium,
    High,
    Critical,
}

/// Default configurations for common use cases
impl Default for CompressionConfig {
    fn default() -> Self {
        Self {
            algorithm: CompressionAlgorithm::Zstd,
            level: 3,
            age_threshold: Duration::from_secs(86400 * 7), // 1 week
            background_compression: true,
            compression_batch_size: 1000,
        }
    }
}

impl Default for IndexConfig {
    fn default() -> Self {
        Self {
            indexes: vec![
                // Common indexes for event data
                IndexDefinition {
                    name: "idx_events_chain_block".to_string(),
                    table: "events".to_string(),
                    columns: vec!["chain_id".to_string(), "block_number".to_string()],
                    index_type: IndexType::BTree,
                    unique: false,
                    condition: None,
                    include_columns: Some(vec!["event_type".to_string(), "timestamp".to_string()]),
                },
                IndexDefinition {
                    name: "idx_events_timestamp".to_string(),
                    table: "events".to_string(),
                    columns: vec!["timestamp".to_string()],
                    index_type: IndexType::BTree,
                    unique: false,
                    condition: None,
                    include_columns: None,
                },
                IndexDefinition {
                    name: "idx_events_tx_hash".to_string(),
                    table: "events".to_string(),
                    columns: vec!["transaction_hash".to_string()],
                    index_type: IndexType::Hash,
                    unique: false,
                    condition: None,
                    include_columns: None,
                },
            ],
            auto_analyze: true,
            maintenance_interval: Duration::from_secs(86400), // Daily
            covering_indexes: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_partition_strategy() {
        let time_strategy = PartitionStrategy::Time {
            interval: PartitionInterval::Daily,
        };
        
        let optimizer = StorageOptimizer::new(
            time_strategy,
            CompressionConfig::default(),
            IndexConfig::default(),
            None,
        );
        
        let timestamp = SystemTime::now();
        let partition_key = optimizer.get_partition_key("ethereum", timestamp);
        assert!(partition_key.starts_with("day_"));
    }
    
    #[tokio::test]
    async fn test_partition_creation() {
        let optimizer = StorageOptimizer::new(
            PartitionStrategy::Chain { max_events_per_partition: 10000 },
            CompressionConfig::default(),
            IndexConfig::default(),
            None,
        );
        
        optimizer.create_partition("test_partition".to_string(), "ethereum".to_string()).await.unwrap();
        
        let stats = optimizer.get_optimization_stats().await;
        assert_eq!(stats.total_partitions, 1);
    }
    
    #[tokio::test]
    async fn test_compression() {
        let mut compression_config = CompressionConfig::default();
        compression_config.age_threshold = Duration::from_secs(0); // Immediate compression for test
        
        let optimizer = StorageOptimizer::new(
            PartitionStrategy::Chain { max_events_per_partition: 10000 },
            compression_config,
            IndexConfig::default(),
            None,
        );
        
        // Create a partition
        optimizer.create_partition("test_partition".to_string(), "ethereum".to_string()).await.unwrap();
        optimizer.update_partition("test_partition", 100, 10000).await.unwrap();
        
        // Compress partitions
        let compressed = optimizer.compress_partitions().await.unwrap();
        assert_eq!(compressed.len(), 1);
        
        let stats = optimizer.get_optimization_stats().await;
        assert_eq!(stats.compressed_partitions, 1);
        assert!(stats.compression_savings_bytes > 0);
    }
    
    #[tokio::test]
    async fn test_optimization_recommendations() {
        // Use compression config with immediate threshold for testing
        let mut compression_config = CompressionConfig::default();
        compression_config.age_threshold = Duration::from_secs(0);
        
        let optimizer = StorageOptimizer::new(
            PartitionStrategy::Chain { max_events_per_partition: 10000 },
            compression_config,
            IndexConfig::default(),
            None,
        );
        
        // Create an old partition (eligible for compression due to 0-second threshold)
        optimizer.create_partition("old_partition".to_string(), "ethereum".to_string()).await.unwrap();
        optimizer.update_partition("old_partition", 100, 10000).await.unwrap();
        
        let recommendations = optimizer.get_optimization_recommendations().await;
        assert!(!recommendations.is_empty());
        
        // Should recommend compression
        assert!(recommendations.iter().any(|r| matches!(r.recommendation_type, RecommendationType::Compression)));
    }
    
    #[test]
    fn test_partition_interval() {
        let daily = PartitionInterval::Daily;
        assert_eq!(daily.duration_seconds(), 86400);
        
        let timestamp = UNIX_EPOCH + Duration::from_secs(86400 * 2 + 3600); // 2 days + 1 hour
        let key = daily.partition_key(timestamp);
        assert_eq!(key, "day_2");
    }
} 