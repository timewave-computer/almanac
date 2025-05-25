/// Data archival system for long-term storage of historical events
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, Duration};
use serde::{Deserialize, Serialize};
use async_trait::async_trait;
use tokio::fs::{File, create_dir_all};
use tokio::io::AsyncWriteExt;

use crate::event::Event;
use crate::{Result, Error};

/// Archival policies that determine when and how data should be archived
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ArchivalPolicy {
    /// Archive based on age
    TimeBasedAge { older_than: Duration },
    
    /// Archive based on block numbers
    BlockBasedAge { older_than_block: u64 },
    
    /// Archive based on data size
    SizeBased { max_active_size: u64 },
    
    /// Archive based on record count
    CountBased { max_active_records: u64 },
    
    /// Archive based on chain activity
    ChainBased { inactive_chains: Vec<String> },
    
    /// Custom archival criteria
    Custom { criteria: String },
}

/// Archival storage tiers for different data retention needs
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ArchivalTier {
    /// Hot storage - frequently accessed, fast retrieval
    Hot,
    
    /// Warm storage - occasionally accessed, moderate retrieval speed
    Warm,
    
    /// Cold storage - rarely accessed, slower retrieval
    Cold,
    
    /// Frozen storage - archive only, very slow retrieval
    Frozen,
    
    /// Deep archive - long-term retention, very slow and expensive retrieval
    DeepArchive,
}

/// Archival configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchivalConfig {
    /// Archival policy to apply
    pub policy: ArchivalPolicy,
    
    /// Target storage tier
    pub target_tier: ArchivalTier,
    
    /// Base directory for archived data
    pub archive_directory: PathBuf,
    
    /// Whether to compress archived data
    pub compression_enabled: bool,
    
    /// Compression algorithm to use
    pub compression_algorithm: CompressionAlgorithm,
    
    /// Compression level (algorithm-specific)
    pub compression_level: u8,
    
    /// Whether to encrypt archived data
    pub encryption_enabled: bool,
    
    /// Maximum size per archive file
    pub max_archive_file_size: u64,
    
    /// Whether to create metadata indices
    pub create_indices: bool,
    
    /// Retention period for archived data
    pub retention_period: Option<Duration>,
    
    /// Chains to include in archival
    pub included_chains: Option<Vec<String>>,
    
    /// Event types to include in archival
    pub included_event_types: Option<Vec<String>>,
    
    /// Additional archival metadata
    pub metadata: HashMap<String, String>,
}

/// Compression algorithms supported for archival
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum CompressionAlgorithm {
    /// No compression
    None,
    
    /// Gzip compression
    Gzip,
    
    /// LZ4 compression (fast)
    Lz4,
    
    /// Zstandard compression (balanced)
    Zstd,
    
    /// Brotli compression (high compression)
    Brotli,
    
    /// Custom compression algorithm
    Custom { algorithm: String },
}

impl Default for ArchivalConfig {
    fn default() -> Self {
        Self {
            policy: ArchivalPolicy::TimeBasedAge { older_than: Duration::from_secs(30 * 24 * 3600) }, // 30 days
            target_tier: ArchivalTier::Cold,
            archive_directory: PathBuf::from("./archives"),
            compression_enabled: true,
            compression_algorithm: CompressionAlgorithm::Zstd,
            compression_level: 6,
            encryption_enabled: false,
            max_archive_file_size: 1024 * 1024 * 1024, // 1GB
            create_indices: true,
            retention_period: Some(Duration::from_secs(365 * 24 * 3600)), // 1 year
            included_chains: None,
            included_event_types: None,
            metadata: HashMap::new(),
        }
    }
}

/// Information about an archived dataset
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchivalInfo {
    /// Unique archive ID
    pub archive_id: String,
    
    /// Archive creation timestamp
    pub created_at: SystemTime,
    
    /// Archival policy used
    pub policy: ArchivalPolicy,
    
    /// Storage tier
    pub tier: ArchivalTier,
    
    /// Archive status
    pub status: ArchivalStatus,
    
    /// Number of events archived
    pub event_count: u64,
    
    /// Original data size (before compression)
    pub original_size: u64,
    
    /// Compressed archive size
    pub compressed_size: u64,
    
    /// Compression ratio achieved
    pub compression_ratio: f64,
    
    /// Archive file paths
    pub file_paths: Vec<PathBuf>,
    
    /// Date range of archived data
    pub date_range: (SystemTime, SystemTime),
    
    /// Block range of archived data
    pub block_range: (u64, u64),
    
    /// Chains included in archive
    pub chains: Vec<String>,
    
    /// Archive checksum for integrity
    pub checksum: String,
    
    /// Configuration used for archival
    pub config: ArchivalConfig,
    
    /// Additional metadata
    pub metadata: HashMap<String, String>,
}

/// Archival operation status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ArchivalStatus {
    /// Archival is in progress
    InProgress,
    
    /// Archival completed successfully
    Completed,
    
    /// Archival failed
    Failed,
    
    /// Archive is being verified
    Verifying,
    
    /// Archive verification failed
    VerificationFailed,
    
    /// Archive is being migrated to different tier
    Migrating,
    
    /// Archive has been deleted
    Deleted,
}

/// Archive retrieval configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetrievalConfig {
    /// Archive ID to retrieve from
    pub archive_id: String,
    
    /// Date range to retrieve (None = all)
    pub date_range: Option<(SystemTime, SystemTime)>,
    
    /// Block range to retrieve (None = all)
    pub block_range: Option<(u64, u64)>,
    
    /// Chains to retrieve (None = all)
    pub chains_filter: Option<Vec<String>>,
    
    /// Event types to retrieve (None = all)
    pub event_types_filter: Option<Vec<String>>,
    
    /// Maximum number of events to retrieve
    pub limit: Option<u64>,
    
    /// Target directory for retrieved data
    pub output_directory: Option<PathBuf>,
    
    /// Whether to decompress retrieved data
    pub decompress: bool,
    
    /// Retrieval options
    pub options: HashMap<String, String>,
}

impl Default for RetrievalConfig {
    fn default() -> Self {
        Self {
            archive_id: String::new(),
            date_range: None,
            block_range: None,
            chains_filter: None,
            event_types_filter: None,
            limit: None,
            output_directory: None,
            decompress: true,
            options: HashMap::new(),
        }
    }
}

/// Archive retrieval information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetrievalInfo {
    /// Unique retrieval operation ID
    pub retrieval_id: String,
    
    /// Archive ID being retrieved
    pub archive_id: String,
    
    /// Retrieval start time
    pub started_at: SystemTime,
    
    /// Current status
    pub status: RetrievalStatus,
    
    /// Total events to retrieve
    pub total_events: u64,
    
    /// Events retrieved so far
    pub retrieved_events: u64,
    
    /// Current retrieval speed (events/second)
    pub retrieval_speed: f64,
    
    /// Estimated completion time
    pub estimated_completion: Option<SystemTime>,
    
    /// Any error messages
    pub error_message: Option<String>,
    
    /// Retrieval configuration
    pub config: RetrievalConfig,
}

/// Archive retrieval status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RetrievalStatus {
    /// Retrieval is in progress
    InProgress,
    
    /// Retrieval completed successfully
    Completed,
    
    /// Retrieval failed
    Failed,
    
    /// Retrieval was cancelled
    Cancelled,
    
    /// Decompressing archive data
    Decompressing,
}

/// Data archival and retrieval operations trait
#[async_trait]
pub trait DataArchival: Send + Sync {
    /// Create an archive based on the given policy
    async fn create_archive(&self, config: ArchivalConfig) -> Result<String>; // Returns archive ID
    
    /// Get archive information
    async fn get_archive_info(&self, archive_id: &str) -> Result<Option<ArchivalInfo>>;
    
    /// List all available archives
    async fn list_archives(&self) -> Result<Vec<ArchivalInfo>>;
    
    /// Delete an archive
    async fn delete_archive(&self, archive_id: &str) -> Result<()>;
    
    /// Start data retrieval from archive
    async fn start_retrieval(&self, config: RetrievalConfig) -> Result<String>; // Returns retrieval ID
    
    /// Get retrieval operation status
    async fn get_retrieval_info(&self, retrieval_id: &str) -> Result<Option<RetrievalInfo>>;
    
    /// Cancel a retrieval operation
    async fn cancel_retrieval(&self, retrieval_id: &str) -> Result<()>;
    
    /// Verify archive integrity
    async fn verify_archive(&self, archive_id: &str) -> Result<bool>;
    
    /// Migrate archive to different storage tier
    async fn migrate_archive(&self, archive_id: &str, target_tier: ArchivalTier) -> Result<()>;
    
    /// Apply archival policies automatically
    async fn apply_archival_policies(&self, policies: Vec<ArchivalConfig>) -> Result<Vec<String>>;
    
    /// Clean up expired archives
    async fn cleanup_expired_archives(&self) -> Result<u32>;
    
    /// Estimate archival size and compression
    async fn estimate_archival(&self, config: &ArchivalConfig) -> Result<ArchivalEstimate>;
}

/// Archival size and compression estimate
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchivalEstimate {
    /// Estimated number of events to archive
    pub estimated_events: u64,
    
    /// Estimated original data size
    pub estimated_original_size: u64,
    
    /// Estimated compressed size
    pub estimated_compressed_size: u64,
    
    /// Estimated compression ratio
    pub estimated_compression_ratio: f64,
    
    /// Estimated archival duration
    pub estimated_duration: Duration,
    
    /// Storage tier recommendations
    pub tier_recommendations: Vec<ArchivalTier>,
}

/// Default implementation of data archival
pub struct DefaultDataArchival {
    /// Active archival operations
    archives: std::sync::Arc<tokio::sync::RwLock<HashMap<String, ArchivalInfo>>>,
    
    /// Active retrieval operations
    retrievals: std::sync::Arc<tokio::sync::RwLock<HashMap<String, RetrievalInfo>>>,
    
    /// Base directory for all archival operations
    base_directory: PathBuf,
}

impl DefaultDataArchival {
    /// Create a new data archival manager
    pub fn new(base_directory: PathBuf) -> Self {
        Self {
            archives: std::sync::Arc::new(tokio::sync::RwLock::new(HashMap::new())),
            retrievals: std::sync::Arc::new(tokio::sync::RwLock::new(HashMap::new())),
            base_directory,
        }
    }
    
    /// Generate a unique archive ID
    fn generate_archive_id() -> String {
        format!("archive_{}", uuid::Uuid::new_v4())
    }
    
    /// Generate a unique retrieval ID
    fn generate_retrieval_id() -> String {
        format!("retrieval_{}", uuid::Uuid::new_v4())
    }
    
    /// Calculate checksum for archive integrity
    fn calculate_checksum(&self, data: &[u8]) -> String {
        format!("{:x}", md5::compute(data))
    }
    
    /// Create archive directory structure
    async fn create_archive_directory(&self, archive_id: &str, tier: &ArchivalTier) -> Result<PathBuf> {
        let tier_dir = match tier {
            ArchivalTier::Hot => "hot",
            ArchivalTier::Warm => "warm",
            ArchivalTier::Cold => "cold",
            ArchivalTier::Frozen => "frozen",
            ArchivalTier::DeepArchive => "deep_archive",
        };
        
        let archive_path = self.base_directory.join(tier_dir).join(archive_id);
        create_dir_all(&archive_path).await
            .map_err(|e| Error::Generic(format!("Failed to create archive directory: {}", e)))?;
        Ok(archive_path)
    }
    
    /// Write archive manifest
    async fn write_archive_manifest(&self, archive_info: &ArchivalInfo, archive_path: &Path) -> Result<()> {
        let manifest_path = archive_path.join("manifest.json");
        let manifest_data = serde_json::to_string_pretty(archive_info)
            .map_err(|e| Error::Generic(format!("Failed to serialize manifest: {}", e)))?;
        
        let mut file = File::create(manifest_path).await
            .map_err(|e| Error::Generic(format!("Failed to create manifest file: {}", e)))?;
        
        file.write_all(manifest_data.as_bytes()).await
            .map_err(|e| Error::Generic(format!("Failed to write manifest: {}", e)))?;
        
        Ok(())
    }
    
    /// Check if data matches archival policy
    fn matches_policy(&self, _policy: &ArchivalPolicy, _event: &dyn Event) -> bool {
        // In a real implementation, this would check the event against the policy
        // For now, return true to simulate matching
        true
    }
    
    /// Apply compression to data
    async fn compress_data(&self, data: &[u8], algorithm: &CompressionAlgorithm, _level: u8) -> Result<Vec<u8>> {
        match algorithm {
            CompressionAlgorithm::None => Ok(data.to_vec()),
            CompressionAlgorithm::Gzip => {
                // Simulate compression with simple processing
                // In real implementation, use flate2 or similar
                let mut compressed = Vec::new();
                compressed.extend_from_slice(b"GZIP_HEADER");
                compressed.extend_from_slice(data);
                Ok(compressed)
            }
            CompressionAlgorithm::Lz4 => {
                // Simulate LZ4 compression
                let mut compressed = Vec::new();
                compressed.extend_from_slice(b"LZ4_HEADER");
                compressed.extend_from_slice(data);
                Ok(compressed)
            }
            CompressionAlgorithm::Zstd => {
                // Simulate Zstandard compression
                let mut compressed = Vec::new();
                compressed.extend_from_slice(b"ZSTD_HEADER");
                compressed.extend_from_slice(data);
                Ok(compressed)
            }
            CompressionAlgorithm::Brotli => {
                // Simulate Brotli compression
                let mut compressed = Vec::new();
                compressed.extend_from_slice(b"BROTLI_HEADER");
                compressed.extend_from_slice(data);
                Ok(compressed)
            }
            CompressionAlgorithm::Custom { algorithm: _ } => {
                // For custom algorithms, just pass through for now
                Ok(data.to_vec())
            }
        }
    }
}

#[async_trait]
impl DataArchival for DefaultDataArchival {
    async fn create_archive(&self, config: ArchivalConfig) -> Result<String> {
        let archive_id = Self::generate_archive_id();
        let archive_path = self.create_archive_directory(&archive_id, &config.target_tier).await?;
        
        let mut archive_info = ArchivalInfo {
            archive_id: archive_id.clone(),
            created_at: SystemTime::now(),
            policy: config.policy.clone(),
            tier: config.target_tier.clone(),
            status: ArchivalStatus::InProgress,
            event_count: 0,
            original_size: 0,
            compressed_size: 0,
            compression_ratio: 1.0,
            file_paths: Vec::new(),
            date_range: (SystemTime::now(), SystemTime::now()),
            block_range: (0, 0),
            chains: Vec::new(),
            checksum: String::new(),
            config: config.clone(),
            metadata: config.metadata.clone(),
        };
        
        // Store initial archive info
        {
            let mut archives = self.archives.write().await;
            archives.insert(archive_id.clone(), archive_info.clone());
        }
        
        // In a real implementation, this would:
        // 1. Query events matching the archival policy
        // 2. Compress and write the data to archive files
        // 3. Update archive_info with results
        
        // Simulate successful archival
        archive_info.status = ArchivalStatus::Completed;
        archive_info.event_count = 5000; // Simulated
        archive_info.original_size = 1024 * 1024 * 50; // 50MB
        archive_info.compressed_size = 1024 * 1024 * 15; // 15MB
        archive_info.compression_ratio = archive_info.compressed_size as f64 / archive_info.original_size as f64;
        archive_info.checksum = self.calculate_checksum(b"simulated_archive_data");
        archive_info.chains = vec!["ethereum".to_string(), "polygon".to_string()];
        
        // Write manifest
        self.write_archive_manifest(&archive_info, &archive_path).await?;
        
        // Update stored archive info
        {
            let mut archives = self.archives.write().await;
            archives.insert(archive_id.clone(), archive_info);
        }
        
        Ok(archive_id)
    }
    
    async fn get_archive_info(&self, archive_id: &str) -> Result<Option<ArchivalInfo>> {
        let archives = self.archives.read().await;
        Ok(archives.get(archive_id).cloned())
    }
    
    async fn list_archives(&self) -> Result<Vec<ArchivalInfo>> {
        let archives = self.archives.read().await;
        Ok(archives.values().cloned().collect())
    }
    
    async fn delete_archive(&self, archive_id: &str) -> Result<()> {
        // Mark as deleted
        {
            let mut archives = self.archives.write().await;
            if let Some(archive_info) = archives.get_mut(archive_id) {
                archive_info.status = ArchivalStatus::Deleted;
            }
        }
        
        // In a real implementation, also delete the archive files
        Ok(())
    }
    
    async fn start_retrieval(&self, config: RetrievalConfig) -> Result<String> {
        let retrieval_id = Self::generate_retrieval_id();
        
        let retrieval_info = RetrievalInfo {
            retrieval_id: retrieval_id.clone(),
            archive_id: config.archive_id.clone(),
            started_at: SystemTime::now(),
            status: RetrievalStatus::InProgress,
            total_events: 5000, // Simulated
            retrieved_events: 0,
            retrieval_speed: 0.0,
            estimated_completion: None,
            error_message: None,
            config,
        };
        
        let mut retrievals = self.retrievals.write().await;
        retrievals.insert(retrieval_id.clone(), retrieval_info);
        
        Ok(retrieval_id)
    }
    
    async fn get_retrieval_info(&self, retrieval_id: &str) -> Result<Option<RetrievalInfo>> {
        let retrievals = self.retrievals.read().await;
        Ok(retrievals.get(retrieval_id).cloned())
    }
    
    async fn cancel_retrieval(&self, retrieval_id: &str) -> Result<()> {
        let mut retrievals = self.retrievals.write().await;
        if let Some(retrieval_info) = retrievals.get_mut(retrieval_id) {
            retrieval_info.status = RetrievalStatus::Cancelled;
        }
        Ok(())
    }
    
    async fn verify_archive(&self, archive_id: &str) -> Result<bool> {
        // In a real implementation, this would verify checksums and file integrity
        let archives = self.archives.read().await;
        Ok(archives.contains_key(archive_id))
    }
    
    async fn migrate_archive(&self, archive_id: &str, target_tier: ArchivalTier) -> Result<()> {
        let mut archives = self.archives.write().await;
        if let Some(archive_info) = archives.get_mut(archive_id) {
            archive_info.tier = target_tier;
            archive_info.status = ArchivalStatus::Migrating;
            // In a real implementation, move the files and update paths
        }
        Ok(())
    }
    
    async fn apply_archival_policies(&self, policies: Vec<ArchivalConfig>) -> Result<Vec<String>> {
        let mut archive_ids = Vec::new();
        
        for policy in policies {
            let archive_id = self.create_archive(policy).await?;
            archive_ids.push(archive_id);
        }
        
        Ok(archive_ids)
    }
    
    async fn cleanup_expired_archives(&self) -> Result<u32> {
        let mut cleaned_count = 0u32;
        let current_time = SystemTime::now();
        
        let archive_ids: Vec<String> = {
            let archives = self.archives.read().await;
            archives.iter()
                .filter_map(|(id, info)| {
                    if let Some(retention) = &info.config.retention_period {
                        if current_time.duration_since(info.created_at).unwrap_or_default() > *retention {
                            return Some(id.clone());
                        }
                    }
                    None
                })
                .collect()
        };
        
        for archive_id in archive_ids {
            if self.delete_archive(&archive_id).await.is_ok() {
                cleaned_count += 1;
            }
        }
        
        Ok(cleaned_count)
    }
    
    async fn estimate_archival(&self, _config: &ArchivalConfig) -> Result<ArchivalEstimate> {
        // In a real implementation, this would analyze the data to be archived
        Ok(ArchivalEstimate {
            estimated_events: 10000,
            estimated_original_size: 1024 * 1024 * 100, // 100MB
            estimated_compressed_size: 1024 * 1024 * 30, // 30MB
            estimated_compression_ratio: 0.3,
            estimated_duration: Duration::from_secs(300), // 5 minutes
            tier_recommendations: vec![ArchivalTier::Cold, ArchivalTier::Warm],
        })
    }
}

/// Archival policy manager for automated archival
pub struct ArchivalPolicyManager {
    archival: std::sync::Arc<dyn DataArchival>,
    policies: HashMap<String, ArchivalConfig>,
}

impl ArchivalPolicyManager {
    /// Create a new archival policy manager
    pub fn new(archival: std::sync::Arc<dyn DataArchival>) -> Self {
        Self {
            archival,
            policies: HashMap::new(),
        }
    }
    
    /// Add an archival policy
    pub fn add_policy(&mut self, policy_name: String, config: ArchivalConfig) {
        self.policies.insert(policy_name, config);
    }
    
    /// Remove an archival policy
    pub fn remove_policy(&mut self, policy_name: &str) {
        self.policies.remove(policy_name);
    }
    
    /// Get all policies
    pub fn get_policies(&self) -> Vec<(String, ArchivalConfig)> {
        self.policies.iter()
            .map(|(name, config)| (name.clone(), config.clone()))
            .collect()
    }
    
    /// Apply all policies
    pub async fn apply_all_policies(&self) -> Result<HashMap<String, String>> {
        let mut results = HashMap::new();
        
        for (policy_name, config) in &self.policies {
            match self.archival.create_archive(config.clone()).await {
                Ok(archive_id) => {
                    results.insert(policy_name.clone(), archive_id);
                }
                Err(_) => {
                    // Log error but continue with other policies
                    continue;
                }
            }
        }
        
        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{UNIX_EPOCH, Duration};
    
    #[tokio::test]
    async fn test_archival_creation() {
        let temp_dir = std::env::temp_dir().join("test_archival");
        let archival = DefaultDataArchival::new(temp_dir);
        
        let config = ArchivalConfig::default();
        let archive_id = archival.create_archive(config).await.unwrap();
        
        let info = archival.get_archive_info(&archive_id).await.unwrap();
        assert!(info.is_some());
        assert_eq!(info.unwrap().status, ArchivalStatus::Completed);
    }
    
    #[tokio::test]
    async fn test_archive_listing() {
        let temp_dir = std::env::temp_dir().join("test_archive_listing");
        let archival = DefaultDataArchival::new(temp_dir);
        
        let config1 = ArchivalConfig::default();
        let config2 = ArchivalConfig {
            target_tier: ArchivalTier::Hot,
            ..Default::default()
        };
        
        let _archive_id1 = archival.create_archive(config1).await.unwrap();
        let _archive_id2 = archival.create_archive(config2).await.unwrap();
        
        let archives = archival.list_archives().await.unwrap();
        assert_eq!(archives.len(), 2);
    }
    
    #[tokio::test]
    async fn test_archive_deletion() {
        let temp_dir = std::env::temp_dir().join("test_archive_deletion");
        let archival = DefaultDataArchival::new(temp_dir);
        
        let config = ArchivalConfig::default();
        let archive_id = archival.create_archive(config).await.unwrap();
        
        // Verify archive exists
        let info = archival.get_archive_info(&archive_id).await.unwrap();
        assert!(info.is_some());
        
        // Delete archive
        archival.delete_archive(&archive_id).await.unwrap();
        
        // Verify archive is marked as deleted
        let info = archival.get_archive_info(&archive_id).await.unwrap();
        assert!(info.is_some());
        assert_eq!(info.unwrap().status, ArchivalStatus::Deleted);
    }
    
    #[tokio::test]
    async fn test_retrieval_operations() {
        let temp_dir = std::env::temp_dir().join("test_retrieval");
        let archival = DefaultDataArchival::new(temp_dir);
        
        // Create an archive first
        let archive_config = ArchivalConfig::default();
        let archive_id = archival.create_archive(archive_config).await.unwrap();
        
        // Start retrieval
        let retrieval_config = RetrievalConfig {
            archive_id: archive_id.clone(),
            ..Default::default()
        };
        let retrieval_id = archival.start_retrieval(retrieval_config).await.unwrap();
        
        // Check retrieval status
        let retrieval_info = archival.get_retrieval_info(&retrieval_id).await.unwrap();
        assert!(retrieval_info.is_some());
        assert_eq!(retrieval_info.unwrap().status, RetrievalStatus::InProgress);
        
        // Cancel retrieval
        archival.cancel_retrieval(&retrieval_id).await.unwrap();
        
        let retrieval_info = archival.get_retrieval_info(&retrieval_id).await.unwrap();
        assert!(retrieval_info.is_some());
        assert_eq!(retrieval_info.unwrap().status, RetrievalStatus::Cancelled);
    }
    
    #[tokio::test]
    async fn test_archive_verification() {
        let temp_dir = std::env::temp_dir().join("test_verification");
        let archival = DefaultDataArchival::new(temp_dir);
        
        let config = ArchivalConfig::default();
        let archive_id = archival.create_archive(config).await.unwrap();
        
        // Verify archive
        let is_valid = archival.verify_archive(&archive_id).await.unwrap();
        assert!(is_valid);
        
        // Verify non-existent archive
        let is_valid = archival.verify_archive("non_existent").await.unwrap();
        assert!(!is_valid);
    }
    
    #[tokio::test]
    async fn test_archive_migration() {
        let temp_dir = std::env::temp_dir().join("test_migration");
        let archival = DefaultDataArchival::new(temp_dir);
        
        let config = ArchivalConfig {
            target_tier: ArchivalTier::Cold,
            ..Default::default()
        };
        let archive_id = archival.create_archive(config).await.unwrap();
        
        // Migrate to hot tier
        archival.migrate_archive(&archive_id, ArchivalTier::Hot).await.unwrap();
        
        let info = archival.get_archive_info(&archive_id).await.unwrap();
        assert!(info.is_some());
        assert_eq!(info.unwrap().tier, ArchivalTier::Hot);
    }
    
    #[tokio::test]
    async fn test_archival_estimation() {
        let temp_dir = std::env::temp_dir().join("test_estimation");
        let archival = DefaultDataArchival::new(temp_dir);
        
        let config = ArchivalConfig::default();
        let estimate = archival.estimate_archival(&config).await.unwrap();
        
        assert!(estimate.estimated_events > 0);
        assert!(estimate.estimated_original_size > 0);
        assert!(estimate.estimated_compressed_size > 0);
        assert!(estimate.estimated_compression_ratio > 0.0);
    }
    
    #[tokio::test]
    async fn test_archival_policy_manager() {
        let temp_dir = std::env::temp_dir().join("test_policy_manager");
        let archival = std::sync::Arc::new(DefaultDataArchival::new(temp_dir));
        let mut manager = ArchivalPolicyManager::new(archival);
        
        let config = ArchivalConfig::default();
        manager.add_policy("daily_archive".to_string(), config);
        
        let policies = manager.get_policies();
        assert_eq!(policies.len(), 1);
        assert_eq!(policies[0].0, "daily_archive");
        
        manager.remove_policy("daily_archive");
        let policies = manager.get_policies();
        assert_eq!(policies.len(), 0);
    }
    
    #[test]
    fn test_archival_config_defaults() {
        let config = ArchivalConfig::default();
        assert_eq!(config.target_tier, ArchivalTier::Cold);
        assert!(config.compression_enabled);
        assert_eq!(config.compression_algorithm, CompressionAlgorithm::Zstd);
        assert_eq!(config.compression_level, 6);
        assert!(!config.encryption_enabled);
        assert!(config.create_indices);
    }
    
    #[test]
    fn test_compression_algorithms() {
        let algorithms = vec![
            CompressionAlgorithm::None,
            CompressionAlgorithm::Gzip,
            CompressionAlgorithm::Lz4,
            CompressionAlgorithm::Zstd,
            CompressionAlgorithm::Brotli,
        ];
        
        for algorithm in algorithms {
            assert!(!format!("{:?}", algorithm).is_empty());
        }
    }
} 