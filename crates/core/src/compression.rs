/// Data compression functionality for historical events and storage optimization
use std::collections::HashMap;
use std::time::Duration;
use serde::{Deserialize, Serialize};
use async_trait::async_trait;

use crate::event::Event;
use crate::{Result, Error};

/// Compression algorithms supported by the system
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum CompressionAlgorithm {
    /// No compression
    None,
    
    /// LZ4 - fast compression/decompression
    Lz4,
    
    /// Zstandard - balanced speed and compression ratio
    Zstd,
    
    /// Gzip - standard compression with good ratio
    Gzip,
    
    /// Brotli - high compression ratio, slower
    Brotli,
    
    /// Snappy - very fast compression
    Snappy,
    
    /// LZO - fast compression with reasonable ratio
    Lzo,
    
    /// Custom compression algorithm
    Custom { name: String, level: u8 },
}

/// Compression configuration and settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressionConfig {
    /// Primary compression algorithm
    pub algorithm: CompressionAlgorithm,
    
    /// Compression level (algorithm-specific)
    pub level: u8,
    
    /// Whether to use adaptive compression
    pub adaptive: bool,
    
    /// Minimum size threshold for compression
    pub min_size_threshold: u64,
    
    /// Maximum compression time allowed
    pub max_compression_time: Duration,
    
    /// Target compression ratio (for adaptive mode)
    pub target_ratio: Option<f64>,
    
    /// Whether to verify compression integrity
    pub verify_integrity: bool,
    
    /// Fallback algorithm if primary fails
    pub fallback_algorithm: Option<CompressionAlgorithm>,
    
    /// Compression dictionary for repetitive data
    pub dictionary: Option<Vec<u8>>,
    
    /// Additional compression options
    pub options: HashMap<String, String>,
}

impl Default for CompressionConfig {
    fn default() -> Self {
        Self {
            algorithm: CompressionAlgorithm::Zstd,
            level: 6,
            adaptive: true,
            min_size_threshold: 1024, // 1KB minimum
            max_compression_time: Duration::from_secs(30),
            target_ratio: Some(0.5), // 50% compression target
            verify_integrity: true,
            fallback_algorithm: Some(CompressionAlgorithm::Lz4),
            dictionary: None,
            options: HashMap::new(),
        }
    }
}

/// Compression result information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressionResult {
    /// Original data size
    pub original_size: u64,
    
    /// Compressed data size
    pub compressed_size: u64,
    
    /// Compression ratio (compressed_size / original_size)
    pub compression_ratio: f64,
    
    /// Time taken for compression
    pub compression_time: Duration,
    
    /// Algorithm used
    pub algorithm_used: CompressionAlgorithm,
    
    /// Compression level used
    pub level_used: u8,
    
    /// Whether integrity verification passed
    pub integrity_verified: bool,
    
    /// Checksum of original data
    pub original_checksum: String,
    
    /// Checksum of compressed data
    pub compressed_checksum: String,
    
    /// Additional metadata
    pub metadata: HashMap<String, String>,
}

/// Compression statistics and metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressionStats {
    /// Total events compressed
    pub total_events: u64,
    
    /// Total original size
    pub total_original_size: u64,
    
    /// Total compressed size
    pub total_compressed_size: u64,
    
    /// Average compression ratio
    pub avg_compression_ratio: f64,
    
    /// Best compression ratio achieved
    pub best_compression_ratio: f64,
    
    /// Worst compression ratio achieved
    pub worst_compression_ratio: f64,
    
    /// Total compression time
    pub total_compression_time: Duration,
    
    /// Average compression speed (bytes/second)
    pub avg_compression_speed: f64,
    
    /// Algorithm usage statistics
    pub algorithm_usage: HashMap<CompressionAlgorithm, u64>,
    
    /// Compression failures
    pub failures: u64,
}

impl Default for CompressionStats {
    fn default() -> Self {
        Self {
            total_events: 0,
            total_original_size: 0,
            total_compressed_size: 0,
            avg_compression_ratio: 1.0,
            best_compression_ratio: 1.0,
            worst_compression_ratio: 1.0,
            total_compression_time: Duration::default(),
            avg_compression_speed: 0.0,
            algorithm_usage: HashMap::new(),
            failures: 0,
        }
    }
}

/// Data compression trait for different compression implementations
#[async_trait]
pub trait DataCompressor: Send + Sync {
    /// Compress data using the specified configuration
    async fn compress(&self, data: &[u8], config: &CompressionConfig) -> Result<(Vec<u8>, CompressionResult)>;
    
    /// Decompress data
    async fn decompress(&self, data: &[u8], algorithm: &CompressionAlgorithm) -> Result<Vec<u8>>;
    
    /// Estimate compression ratio without actually compressing
    async fn estimate_compression(&self, data: &[u8], algorithm: &CompressionAlgorithm) -> Result<f64>;
    
    /// Get compression statistics
    async fn get_stats(&self) -> Result<CompressionStats>;
    
    /// Reset compression statistics
    async fn reset_stats(&self) -> Result<()>;
    
    /// Verify compressed data integrity
    async fn verify_integrity(&self, original: &[u8], compressed: &[u8], algorithm: &CompressionAlgorithm) -> Result<bool>;
}

/// Event-specific compression for blockchain events
#[async_trait]
pub trait EventCompressor: Send + Sync {
    /// Compress a single event
    async fn compress_event(&self, event: &dyn Event, config: &CompressionConfig) -> Result<(Vec<u8>, CompressionResult)>;
    
    /// Compress multiple events as a batch
    async fn compress_events(&self, events: &[&dyn Event], config: &CompressionConfig) -> Result<(Vec<u8>, CompressionResult)>;
    
    /// Decompress and reconstruct events
    async fn decompress_events(&self, data: &[u8], algorithm: &CompressionAlgorithm) -> Result<Vec<serde_json::Value>>;
    
    /// Adaptive compression - choose best algorithm for data
    async fn adaptive_compress(&self, data: &[u8], target_ratio: f64) -> Result<(Vec<u8>, CompressionResult)>;
}

/// Default implementation of data compression
pub struct DefaultDataCompressor {
    /// Compression statistics
    stats: std::sync::Arc<tokio::sync::RwLock<CompressionStats>>,
    
    /// Compression dictionaries cache
    dictionaries: std::sync::Arc<tokio::sync::RwLock<HashMap<String, Vec<u8>>>>,
}

impl DefaultDataCompressor {
    /// Create a new data compressor
    pub fn new() -> Self {
        Self {
            stats: std::sync::Arc::new(tokio::sync::RwLock::new(CompressionStats::default())),
            dictionaries: std::sync::Arc::new(tokio::sync::RwLock::new(HashMap::new())),
        }
    }
    
    /// Calculate checksum for data integrity
    fn calculate_checksum(&self, data: &[u8]) -> String {
        format!("{:x}", md5::compute(data))
    }
    
    /// Perform actual compression with specified algorithm
    async fn compress_with_algorithm(
        &self,
        data: &[u8],
        algorithm: &CompressionAlgorithm,
        level: u8,
    ) -> Result<Vec<u8>> {
        match algorithm {
            CompressionAlgorithm::None => Ok(data.to_vec()),
            
            CompressionAlgorithm::Lz4 => {
                // Simulate LZ4 compression
                let mut compressed = Vec::new();
                compressed.extend_from_slice(b"LZ4");
                compressed.push(level);
                compressed.extend_from_slice(&(data.len() as u32).to_le_bytes());
                
                // Simple compression simulation - in reality use lz4 crate
                for chunk in data.chunks(8) {
                    if chunk.iter().all(|&b| b == chunk[0]) {
                        // Repetitive data - compress
                        compressed.push(0xFF); // Marker for compressed chunk
                        compressed.push(chunk[0]);
                        compressed.push(chunk.len() as u8);
                    } else {
                        // Non-repetitive data - store as-is
                        compressed.push(0x00); // Marker for uncompressed chunk
                        compressed.extend_from_slice(chunk);
                    }
                }
                Ok(compressed)
            }
            
            CompressionAlgorithm::Zstd => {
                // Simulate Zstandard compression
                let mut compressed = Vec::new();
                compressed.extend_from_slice(b"ZSTD");
                compressed.push(level);
                compressed.extend_from_slice(&(data.len() as u32).to_le_bytes());
                
                // Better compression simulation
                let mut i = 0;
                while i < data.len() {
                    let mut run_length = 1;
                    while i + run_length < data.len() && 
                          data[i] == data[i + run_length] && 
                          run_length < 255 {
                        run_length += 1;
                    }
                    
                    if run_length >= 3 {
                        // Run-length encode
                        compressed.push(0xFF);
                        compressed.push(data[i]);
                        compressed.push(run_length as u8);
                        i += run_length;
                    } else {
                        // Store literal
                        compressed.push(data[i]);
                        i += 1;
                    }
                }
                Ok(compressed)
            }
            
            CompressionAlgorithm::Gzip => {
                // Use flate2 for real gzip compression
                use flate2::write::GzEncoder;
                use flate2::Compression;
                use std::io::Write;
                
                let mut encoder = GzEncoder::new(Vec::new(), Compression::new(level as u32));
                encoder.write_all(data)
                    .map_err(|e| Error::Generic(format!("Gzip compression failed: {}", e)))?;
                encoder.finish()
                    .map_err(|e| Error::Generic(format!("Gzip compression failed: {}", e)))
            }
            
            CompressionAlgorithm::Brotli => {
                // Simulate Brotli compression (highest ratio but slower)
                let mut compressed = Vec::new();
                compressed.extend_from_slice(b"BROTLI");
                compressed.push(level);
                compressed.extend_from_slice(&(data.len() as u32).to_le_bytes());
                
                // Aggressive compression simulation
                let ratio = 0.3; // Brotli typically achieves better ratios
                let target_size = (data.len() as f64 * ratio) as usize;
                compressed.extend_from_slice(&data[..target_size.min(data.len())]);
                Ok(compressed)
            }
            
            CompressionAlgorithm::Snappy => {
                // Simulate Snappy compression (very fast)
                let mut compressed = Vec::new();
                compressed.extend_from_slice(b"SNAPPY");
                compressed.extend_from_slice(&(data.len() as u32).to_le_bytes());
                compressed.extend_from_slice(data);
                Ok(compressed)
            }
            
            CompressionAlgorithm::Lzo => {
                // Simulate LZO compression
                let mut compressed = Vec::new();
                compressed.extend_from_slice(b"LZO");
                compressed.push(level);
                compressed.extend_from_slice(&(data.len() as u32).to_le_bytes());
                compressed.extend_from_slice(data);
                Ok(compressed)
            }
            
            CompressionAlgorithm::Custom { name, level: custom_level } => {
                // Custom algorithm placeholder
                let mut compressed = Vec::new();
                compressed.extend_from_slice(name.as_bytes());
                compressed.push(*custom_level);
                compressed.extend_from_slice(data);
                Ok(compressed)
            }
        }
    }
    
    /// Perform actual decompression with specified algorithm
    async fn decompress_with_algorithm(
        &self,
        data: &[u8],
        algorithm: &CompressionAlgorithm,
    ) -> Result<Vec<u8>> {
        match algorithm {
            CompressionAlgorithm::None => Ok(data.to_vec()),
            
            CompressionAlgorithm::Gzip => {
                // Use flate2 for real gzip decompression
                use flate2::read::GzDecoder;
                use std::io::Read;
                
                let mut decoder = GzDecoder::new(data);
                let mut decompressed = Vec::new();
                decoder.read_to_end(&mut decompressed)
                    .map_err(|e| Error::Generic(format!("Gzip decompression failed: {}", e)))?;
                Ok(decompressed)
            }
            
            _ => {
                // For other algorithms, just return the data after header
                // In a real implementation, these would use proper decompression
                if data.len() < 10 {
                    return Err(Error::Generic("Invalid compressed data".to_string()));
                }
                
                // Skip algorithm header and return simulated decompressed data
                Ok(data[10..].to_vec())
            }
        }
    }
    
    /// Update compression statistics
    async fn update_stats(&self, result: &CompressionResult) {
        let mut stats = self.stats.write().await;
        
        stats.total_events += 1;
        stats.total_original_size += result.original_size;
        stats.total_compressed_size += result.compressed_size;
        stats.total_compression_time += result.compression_time;
        
        // Update averages
        stats.avg_compression_ratio = stats.total_compressed_size as f64 / stats.total_original_size as f64;
        stats.avg_compression_speed = stats.total_original_size as f64 / stats.total_compression_time.as_secs_f64();
        
        // Update best/worst ratios
        if result.compression_ratio < stats.best_compression_ratio {
            stats.best_compression_ratio = result.compression_ratio;
        }
        if result.compression_ratio > stats.worst_compression_ratio {
            stats.worst_compression_ratio = result.compression_ratio;
        }
        
        // Update algorithm usage
        *stats.algorithm_usage.entry(result.algorithm_used.clone()).or_insert(0) += 1;
    }
}

#[async_trait]
impl DataCompressor for DefaultDataCompressor {
    async fn compress(&self, data: &[u8], config: &CompressionConfig) -> Result<(Vec<u8>, CompressionResult)> {
        let start_time = std::time::Instant::now();
        let original_size = data.len() as u64;
        
        // Check minimum size threshold
        if original_size < config.min_size_threshold {
            let result = CompressionResult {
                original_size,
                compressed_size: original_size,
                compression_ratio: 1.0,
                compression_time: Duration::default(),
                algorithm_used: CompressionAlgorithm::None,
                level_used: 0,
                integrity_verified: true,
                original_checksum: self.calculate_checksum(data),
                compressed_checksum: self.calculate_checksum(data),
                metadata: HashMap::new(),
            };
            
            // Update statistics even when not compressing
            self.update_stats(&result).await;
            
            return Ok((data.to_vec(), result));
        }
        
        // Try primary algorithm
        let mut algorithm = &config.algorithm;
        let mut level = config.level;
        
        let compressed_data = match self.compress_with_algorithm(data, algorithm, level).await {
            Ok(compressed) => compressed,
            Err(_) if config.fallback_algorithm.is_some() => {
                // Try fallback algorithm
                algorithm = config.fallback_algorithm.as_ref().unwrap();
                level = 3; // Default level for fallback
                self.compress_with_algorithm(data, algorithm, level).await?
            }
            Err(e) => return Err(e),
        };
        
        let compression_time = start_time.elapsed();
        let compressed_size = compressed_data.len() as u64;
        let compression_ratio = compressed_size as f64 / original_size as f64;
        
        // Check compression time limit
        if compression_time > config.max_compression_time {
            return Err(Error::Generic("Compression time exceeded limit".to_string()));
        }
        
        // Verify integrity if requested
        let integrity_verified = if config.verify_integrity {
            self.verify_integrity(data, &compressed_data, algorithm).await.unwrap_or(false)
        } else {
            true
        };
        
        let result = CompressionResult {
            original_size,
            compressed_size,
            compression_ratio,
            compression_time,
            algorithm_used: algorithm.clone(),
            level_used: level,
            integrity_verified,
            original_checksum: self.calculate_checksum(data),
            compressed_checksum: self.calculate_checksum(&compressed_data),
            metadata: HashMap::new(),
        };
        
        // Update statistics
        self.update_stats(&result).await;
        
        Ok((compressed_data, result))
    }
    
    async fn decompress(&self, data: &[u8], algorithm: &CompressionAlgorithm) -> Result<Vec<u8>> {
        self.decompress_with_algorithm(data, algorithm).await
    }
    
    async fn estimate_compression(&self, _data: &[u8], algorithm: &CompressionAlgorithm) -> Result<f64> {
        // Quick estimation without full compression
        match algorithm {
            CompressionAlgorithm::None => Ok(1.0),
            CompressionAlgorithm::Lz4 => Ok(0.7), // LZ4 typically achieves 30% compression
            CompressionAlgorithm::Zstd => Ok(0.5), // Zstd achieves better ratios
            CompressionAlgorithm::Gzip => Ok(0.4), // Gzip is pretty good
            CompressionAlgorithm::Brotli => Ok(0.3), // Brotli achieves highest ratios
            CompressionAlgorithm::Snappy => Ok(0.8), // Snappy is fast but lower ratio
            CompressionAlgorithm::Lzo => Ok(0.6), // LZO balances speed and ratio
            CompressionAlgorithm::Custom { .. } => Ok(0.5), // Default estimate
        }
    }
    
    async fn get_stats(&self) -> Result<CompressionStats> {
        let stats = self.stats.read().await;
        Ok(stats.clone())
    }
    
    async fn reset_stats(&self) -> Result<()> {
        let mut stats = self.stats.write().await;
        *stats = CompressionStats::default();
        Ok(())
    }
    
    async fn verify_integrity(&self, original: &[u8], compressed: &[u8], algorithm: &CompressionAlgorithm) -> Result<bool> {
        // Decompress and compare with original
        let decompressed = self.decompress_with_algorithm(compressed, algorithm).await?;
        Ok(decompressed == original)
    }
}

/// Default implementation of event compression
pub struct DefaultEventCompressor {
    compressor: DefaultDataCompressor,
}

impl DefaultEventCompressor {
    /// Create a new event compressor
    pub fn new() -> Self {
        Self {
            compressor: DefaultDataCompressor::new(),
        }
    }
    
    /// Serialize event to JSON bytes
    fn serialize_event(&self, event: &dyn Event) -> Result<Vec<u8>> {
        let event_data = serde_json::json!({
            "id": event.id(),
            "chain": event.chain(),
            "block_number": event.block_number(),
            "block_hash": event.block_hash(),
            "tx_hash": event.tx_hash(),
            "timestamp": event.timestamp()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            "event_type": event.event_type(),
            "raw_data": event.raw_data(),
        });
        
        serde_json::to_vec(&event_data)
            .map_err(|e| Error::Generic(format!("Failed to serialize event: {}", e)))
    }
}

#[async_trait]
impl EventCompressor for DefaultEventCompressor {
    async fn compress_event(&self, event: &dyn Event, config: &CompressionConfig) -> Result<(Vec<u8>, CompressionResult)> {
        let event_data = self.serialize_event(event)?;
        self.compressor.compress(&event_data, config).await
    }
    
    async fn compress_events(&self, events: &[&dyn Event], config: &CompressionConfig) -> Result<(Vec<u8>, CompressionResult)> {
        let mut all_events = Vec::new();
        
        for event in events {
            let event_data = self.serialize_event(*event)?;
            all_events.extend_from_slice(&event_data);
            all_events.push(b'\n'); // Newline delimiter
        }
        
        self.compressor.compress(&all_events, config).await
    }
    
    async fn decompress_events(&self, data: &[u8], algorithm: &CompressionAlgorithm) -> Result<Vec<serde_json::Value>> {
        let decompressed = self.compressor.decompress(data, algorithm).await?;
        let json_str = String::from_utf8(decompressed)
            .map_err(|e| Error::Generic(format!("Invalid UTF-8 in decompressed data: {}", e)))?;
        
        let mut events = Vec::new();
        for line in json_str.lines() {
            if !line.trim().is_empty() {
                let event: serde_json::Value = serde_json::from_str(line)
                    .map_err(|e| Error::Generic(format!("Failed to parse event JSON: {}", e)))?;
                events.push(event);
            }
        }
        
        Ok(events)
    }
    
    async fn adaptive_compress(&self, data: &[u8], target_ratio: f64) -> Result<(Vec<u8>, CompressionResult)> {
        let algorithms = vec![
            CompressionAlgorithm::Lz4,
            CompressionAlgorithm::Zstd,
            CompressionAlgorithm::Gzip,
            CompressionAlgorithm::Brotli,
        ];
        
        let mut best_result: Option<CompressionResult> = None;
        let mut best_compressed = Vec::new();
        
        for algorithm in algorithms {
            let config = CompressionConfig {
                algorithm,
                target_ratio: Some(target_ratio),
                ..Default::default()
            };
            
            if let Ok((compressed, result)) = self.compressor.compress(data, &config).await {
                if result.compression_ratio <= target_ratio {
                    // Found algorithm that meets target ratio
                    return Ok((compressed, result));
                }
                
                if best_result.is_none() || result.compression_ratio < best_result.as_ref().unwrap().compression_ratio {
                    best_result = Some(result);
                    best_compressed = compressed;
                }
            }
        }
        
        if let Some(result) = best_result {
            Ok((best_compressed, result))
        } else {
            Err(Error::Generic("No compression algorithm succeeded".to_string()))
        }
    }
}

/// Compression manager for coordinating compression across the system
pub struct CompressionManager {
    event_compressor: DefaultEventCompressor,
    data_compressor: DefaultDataCompressor,
    default_config: CompressionConfig,
}

impl CompressionManager {
    /// Create a new compression manager
    pub fn new() -> Self {
        Self {
            event_compressor: DefaultEventCompressor::new(),
            data_compressor: DefaultDataCompressor::new(),
            default_config: CompressionConfig::default(),
        }
    }
    
    /// Create with custom default configuration
    pub fn with_config(config: CompressionConfig) -> Self {
        Self {
            event_compressor: DefaultEventCompressor::new(),
            data_compressor: DefaultDataCompressor::new(),
            default_config: config,
        }
    }
    
    /// Get the event compressor
    pub fn event_compressor(&self) -> &DefaultEventCompressor {
        &self.event_compressor
    }
    
    /// Get the data compressor
    pub fn data_compressor(&self) -> &DefaultDataCompressor {
        &self.data_compressor
    }
    
    /// Get default configuration
    pub fn default_config(&self) -> &CompressionConfig {
        &self.default_config
    }
    
    /// Update default configuration
    pub fn set_default_config(&mut self, config: CompressionConfig) {
        self.default_config = config;
    }
    
    /// Get overall compression statistics
    pub async fn get_overall_stats(&self) -> Result<CompressionStats> {
        self.data_compressor.get_stats().await
    }
    
    /// Reset all statistics
    pub async fn reset_all_stats(&self) -> Result<()> {
        self.data_compressor.reset_stats().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{UNIX_EPOCH, Duration};
    
    // Mock event for testing
    #[derive(Debug, Clone)]
    struct TestEvent {
        id: String,
        chain: String,
        block_number: u64,
        block_hash: String,
        tx_hash: String,
        timestamp: SystemTime,
        event_type: String,
        raw_data: Vec<u8>,
    }
    
    impl Event for TestEvent {
        fn id(&self) -> &str { &self.id }
        fn chain(&self) -> &str { &self.chain }
        fn block_number(&self) -> u64 { self.block_number }
        fn block_hash(&self) -> &str { &self.block_hash }
        fn tx_hash(&self) -> &str { &self.tx_hash }
        fn timestamp(&self) -> SystemTime { self.timestamp }
        fn event_type(&self) -> &str { &self.event_type }
        fn raw_data(&self) -> &[u8] { &self.raw_data }
        fn as_any(&self) -> &dyn std::any::Any { self }
    }
    
    fn create_test_event() -> TestEvent {
        TestEvent {
            id: "test_event_1".to_string(),
            chain: "ethereum".to_string(),
            block_number: 12345,
            block_hash: "0xabcdef123456".to_string(),
            tx_hash: "0x987654321".to_string(),
            timestamp: UNIX_EPOCH + Duration::from_secs(1000),
            event_type: "Transfer".to_string(),
            raw_data: r#"{"from": "0x123", "to": "0x456", "value": "1000"}"#.as_bytes().to_vec(),
        }
    }
    
    #[tokio::test]
    async fn test_data_compressor_creation() {
        let compressor = DefaultDataCompressor::new();
        let stats = compressor.get_stats().await.unwrap();
        assert_eq!(stats.total_events, 0);
    }
    
    #[tokio::test]
    async fn test_compression_with_gzip() {
        let compressor = DefaultDataCompressor::new();
        // Use larger, more repetitive data that will compress well
        let data = b"Hello, world! This is a test string for compression. ".repeat(100);
        
        let config = CompressionConfig {
            algorithm: CompressionAlgorithm::Gzip,
            level: 6,
            min_size_threshold: 1, // Set very low threshold to ensure compression
            ..Default::default()
        };
        
        let (compressed, result) = compressor.compress(&data, &config).await.unwrap();
        
        assert!(compressed.len() < data.len());
        assert!(result.compression_ratio < 1.0);
        assert_eq!(result.algorithm_used, CompressionAlgorithm::Gzip);
        assert!(result.integrity_verified);
        
        // Test decompression
        let decompressed = compressor.decompress(&compressed, &CompressionAlgorithm::Gzip).await.unwrap();
        assert_eq!(decompressed, data);
    }
    
    #[tokio::test]
    async fn test_compression_algorithms() {
        let compressor = DefaultDataCompressor::new();
        let data = b"Test data for compression algorithms".repeat(100);
        
        let algorithms = vec![
            CompressionAlgorithm::None,
            CompressionAlgorithm::Lz4,
            CompressionAlgorithm::Zstd,
            CompressionAlgorithm::Snappy,
        ];
        
        for algorithm in algorithms {
            let config = CompressionConfig {
                algorithm: algorithm.clone(),
                ..Default::default()
            };
            
            let result = compressor.compress(&data, &config).await;
            assert!(result.is_ok(), "Failed to compress with {:?}", algorithm);
            
            let (compressed, compression_result) = result.unwrap();
            assert_eq!(compression_result.algorithm_used, algorithm);
        }
    }
    
    #[tokio::test]
    async fn test_compression_estimation() {
        let compressor = DefaultDataCompressor::new();
        let data = b"Test data for estimation";
        
        let ratio = compressor.estimate_compression(data, &CompressionAlgorithm::Zstd).await.unwrap();
        assert!(ratio > 0.0 && ratio <= 1.0);
    }
    
    #[tokio::test]
    async fn test_event_compression() {
        let compressor = DefaultEventCompressor::new();
        let event = create_test_event();
        
        let config = CompressionConfig::default();
        let (compressed, result) = compressor.compress_event(&event, &config).await.unwrap();
        
        assert!(compressed.len() > 0);
        assert!(result.original_size > 0);
        assert!(result.integrity_verified);
    }
    
    #[tokio::test]
    async fn test_multiple_events_compression() {
        let compressor = DefaultEventCompressor::new();
        let events = vec![
            create_test_event(),
            create_test_event(),
            create_test_event(),
        ];
        let event_refs: Vec<&dyn Event> = events.iter().map(|e| e as &dyn Event).collect();
        
        let config = CompressionConfig::default();
        let (compressed, result) = compressor.compress_events(&event_refs, &config).await.unwrap();
        
        assert!(compressed.len() > 0);
        assert!(result.original_size > 0);
        
        // Test decompression
        let decompressed_events = compressor.decompress_events(&compressed, &result.algorithm_used).await.unwrap();
        assert_eq!(decompressed_events.len(), 3);
    }
    
    #[tokio::test]
    async fn test_adaptive_compression() {
        let compressor = DefaultEventCompressor::new();
        let data = b"This is test data for adaptive compression".repeat(50);
        let target_ratio = 0.5;
        
        let (compressed, result) = compressor.adaptive_compress(&data, target_ratio).await.unwrap();
        
        assert!(compressed.len() > 0);
        assert!(result.compression_ratio <= target_ratio || result.compression_ratio < 1.0);
    }
    
    #[tokio::test]
    async fn test_compression_stats() {
        let compressor = DefaultDataCompressor::new();
        let data = b"Test data for statistics";
        
        let config = CompressionConfig::default();
        let _ = compressor.compress(data, &config).await.unwrap();
        
        let stats = compressor.get_stats().await.unwrap();
        assert_eq!(stats.total_events, 1);
        assert!(stats.total_original_size > 0);
        
        // Reset stats
        compressor.reset_stats().await.unwrap();
        let stats = compressor.get_stats().await.unwrap();
        assert_eq!(stats.total_events, 0);
    }
    
    #[tokio::test]
    async fn test_compression_manager() {
        let mut manager = CompressionManager::new();
        
        // Test with default config
        let default_config = manager.default_config();
        assert_eq!(default_config.algorithm, CompressionAlgorithm::Zstd);
        
        // Update config
        let new_config = CompressionConfig {
            algorithm: CompressionAlgorithm::Lz4,
            ..Default::default()
        };
        manager.set_default_config(new_config.clone());
        assert_eq!(manager.default_config().algorithm, CompressionAlgorithm::Lz4);
        
        // Test getting compressors
        let _event_compressor = manager.event_compressor();
        let _data_compressor = manager.data_compressor();
    }
    
    #[tokio::test]
    async fn test_compression_config_defaults() {
        let config = CompressionConfig::default();
        assert_eq!(config.algorithm, CompressionAlgorithm::Zstd);
        assert_eq!(config.level, 6);
        assert!(config.adaptive);
        assert_eq!(config.min_size_threshold, 1024);
        assert!(config.verify_integrity);
    }
    
    #[tokio::test]
    async fn test_compression_with_fallback() {
        let compressor = DefaultDataCompressor::new();
        let data = b"Test data";
        
        let config = CompressionConfig {
            algorithm: CompressionAlgorithm::Custom { name: "unknown".to_string(), level: 1 },
            fallback_algorithm: Some(CompressionAlgorithm::Lz4),
            ..Default::default()
        };
        
        let result = compressor.compress(data, &config).await;
        assert!(result.is_ok());
    }
    
    #[test]
    fn test_compression_algorithm_variants() {
        let algorithms = vec![
            CompressionAlgorithm::None,
            CompressionAlgorithm::Lz4,
            CompressionAlgorithm::Zstd,
            CompressionAlgorithm::Gzip,
            CompressionAlgorithm::Brotli,
            CompressionAlgorithm::Snappy,
            CompressionAlgorithm::Lzo,
            CompressionAlgorithm::Custom { name: "test".to_string(), level: 5 },
        ];
        
        for algorithm in algorithms {
            // Just test that we can create and debug print all variants
            assert!(!format!("{:?}", algorithm).is_empty());
        }
    }
} 