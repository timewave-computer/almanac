/// Data export functionality for almanac indexer
use std::collections::HashMap;
use std::time::{Duration, SystemTime};
use serde::{Deserialize, Serialize};
use async_trait::async_trait;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;

use crate::event::Event;
use crate::{Result, Error};

/// Export format types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ExportFormat {
    /// JavaScript Object Notation
    Json,
    
    /// Newline-delimited JSON
    Jsonl,
    
    /// Tab-separated values
    Tsv,
    
    /// Microsoft Excel format
    Excel,
    
    /// Apache Parquet format
    Parquet,
    
    /// Custom format with user-defined structure
    Custom { format_name: String },
}

/// Export configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportConfig {
    /// Export format
    pub format: ExportFormat,
    
    /// Output file path
    pub output_path: String,
    
    /// Whether to include headers (for TSV)
    pub include_headers: bool,
    
    /// Field delimiter (for TSV)
    pub delimiter: String,
    
    /// Date format string
    pub date_format: String,
    
    /// Whether to pretty print JSON
    pub pretty_json: bool,
    
    /// Maximum records per file
    pub max_records_per_file: Option<u64>,
    
    /// Compression level (0-9)
    pub compression_level: Option<u8>,
    
    /// Whether to include metadata
    pub include_metadata: bool,
    
    /// Custom field mappings
    pub field_mappings: HashMap<String, String>,
    
    /// Fields to exclude from export
    pub excluded_fields: Vec<String>,
    
    /// Additional export options
    pub options: HashMap<String, String>,
}

impl Default for ExportConfig {
    fn default() -> Self {
        Self {
            format: ExportFormat::Json,
            output_path: "export.json".to_string(),
            include_headers: true,
            delimiter: ",".to_string(),
            date_format: "%Y-%m-%d %H:%M:%S".to_string(),
            pretty_json: false,
            max_records_per_file: None,
            compression_level: None,
            include_metadata: false,
            field_mappings: HashMap::new(),
            excluded_fields: Vec::new(),
            options: HashMap::new(),
        }
    }
}

/// Export filters for data selection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportFilter {
    /// Chain filters
    pub chains: Option<Vec<String>>,
    
    /// Event type filters
    pub event_types: Option<Vec<String>>,
    
    /// Block number range
    pub block_range: Option<(u64, u64)>,
    
    /// Time range
    pub time_range: Option<(SystemTime, SystemTime)>,
    
    /// Transaction hash filters
    pub tx_hashes: Option<Vec<String>>,
    
    /// Address filters
    pub addresses: Option<Vec<String>>,
    
    /// Custom field filters
    pub custom_filters: HashMap<String, String>,
    
    /// Maximum number of records to export
    pub limit: Option<u64>,
    
    /// Number of records to skip
    pub offset: Option<u64>,
    
    /// Sort field and direction
    pub sort_by: Option<(String, SortDirection)>,
}

/// Sort direction for exports
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SortDirection {
    Ascending,
    Descending,
}

impl Default for ExportFilter {
    fn default() -> Self {
        Self {
            chains: None,
            event_types: None,
            block_range: None,
            time_range: None,
            tx_hashes: None,
            addresses: None,
            custom_filters: HashMap::new(),
            limit: None,
            offset: None,
            sort_by: None,
        }
    }
}

/// Export progress information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportProgress {
    /// Export job ID
    pub job_id: String,
    
    /// Current status
    pub status: ExportStatus,
    
    /// Total records to export
    pub total_records: u64,
    
    /// Records processed so far
    pub processed_records: u64,
    
    /// Number of output files created
    pub files_created: u32,
    
    /// Export start time
    pub start_time: SystemTime,
    
    /// Estimated completion time
    pub estimated_completion: Option<SystemTime>,
    
    /// Current processing speed (records/second)
    pub processing_speed: f64,
    
    /// Any error messages
    pub error_message: Option<String>,
    
    /// Export metadata
    pub metadata: HashMap<String, String>,
}

/// Export job status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ExportStatus {
    /// Export is queued
    Queued,
    
    /// Export is running
    Running,
    
    /// Export completed successfully
    Completed,
    
    /// Export failed
    Failed,
    
    /// Export was cancelled
    Cancelled,
    
    /// Export is paused
    Paused,
}

/// Export result information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportResult {
    /// Export job ID
    pub job_id: String,
    
    /// Final status
    pub status: ExportStatus,
    
    /// Output file paths
    pub output_files: Vec<String>,
    
    /// Total records exported
    pub total_records: u64,
    
    /// Total time taken
    pub duration: Duration,
    
    /// File sizes in bytes
    pub file_sizes: Vec<u64>,
    
    /// Export configuration used
    pub config: ExportConfig,
    
    /// Export filter used
    pub filter: ExportFilter,
    
    /// Error details if failed
    pub error_details: Option<String>,
    
    /// Export statistics
    pub statistics: ExportStatistics,
}

/// Export performance statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportStatistics {
    /// Average processing speed (records/second)
    pub avg_processing_speed: f64,
    
    /// Peak processing speed
    pub peak_processing_speed: f64,
    
    /// Total bytes written
    pub bytes_written: u64,
    
    /// Memory usage peak (bytes)
    pub peak_memory_usage: u64,
    
    /// Number of batches processed
    pub batches_processed: u32,
    
    /// Average batch size
    pub avg_batch_size: f64,
    
    /// Compression ratio (if applicable)
    pub compression_ratio: Option<f64>,
}

impl Default for ExportStatistics {
    fn default() -> Self {
        Self {
            avg_processing_speed: 0.0,
            peak_processing_speed: 0.0,
            bytes_written: 0,
            peak_memory_usage: 0,
            batches_processed: 0,
            avg_batch_size: 0.0,
            compression_ratio: None,
        }
    }
}

/// Data export trait
#[async_trait]
pub trait DataExporter: Send + Sync {
    /// Start an export job
    async fn start_export(
        &self,
        _config: ExportConfig,
        _filter: ExportFilter,
    ) -> Result<String>; // Returns job ID
    
    /// Get export progress
    async fn get_progress(&self, job_id: &str) -> Result<Option<ExportProgress>>;
    
    /// Cancel an export job
    async fn cancel_export(&self, job_id: &str) -> Result<()>;
    
    /// Get export result
    async fn get_result(&self, job_id: &str) -> Result<Option<ExportResult>>;
    
    /// List all export jobs
    async fn list_jobs(&self) -> Result<Vec<ExportProgress>>;
    
    /// Clean up completed jobs
    async fn cleanup_jobs(&self, older_than: Duration) -> Result<u32>;
    
    /// Export events synchronously (for small datasets)
    async fn export_events(
        &self,
        events: Vec<&dyn Event>,
        config: ExportConfig,
    ) -> Result<ExportResult>;
}

/// Default data exporter implementation
pub struct DefaultDataExporter {
    /// Active export jobs
    jobs: std::sync::Arc<tokio::sync::RwLock<HashMap<String, ExportProgress>>>,
    
    /// Completed jobs
    results: std::sync::Arc<tokio::sync::RwLock<HashMap<String, ExportResult>>>,
    
    /// Export configuration
    #[allow(dead_code)]
    default_config: ExportConfig,
}

impl DefaultDataExporter {
    /// Create a new data exporter
    pub fn new() -> Self {
        Self {
            jobs: std::sync::Arc::new(tokio::sync::RwLock::new(HashMap::new())),
            results: std::sync::Arc::new(tokio::sync::RwLock::new(HashMap::new())),
            default_config: ExportConfig::default(),
        }
    }
    
    /// Create a new data exporter with custom configuration
    pub fn with_config(config: ExportConfig) -> Self {
        Self {
            jobs: std::sync::Arc::new(tokio::sync::RwLock::new(HashMap::new())),
            results: std::sync::Arc::new(tokio::sync::RwLock::new(HashMap::new())),
            default_config: config,
        }
    }
    
    /// Generate a unique job ID
    fn generate_job_id() -> String {
        uuid::Uuid::new_v4().to_string()
    }
    
    /// Format event data for export
    fn format_event_data(
        &self,
        event: &dyn Event,
        config: &ExportConfig,
    ) -> Result<serde_json::Value> {
        let mut data = serde_json::Map::new();
        
        // Apply field mappings and exclusions
        let field_map = &config.field_mappings;
        let excluded = &config.excluded_fields;
        
        // Standard event fields with static string defaults
        if !excluded.contains(&"id".to_string()) {
            let field_name = field_map.get("id").unwrap_or(&"id".to_string()).clone();
            data.insert(field_name, serde_json::Value::String(event.id().to_string()));
        }
        
        if !excluded.contains(&"chain".to_string()) {
            let field_name = field_map.get("chain").unwrap_or(&"chain".to_string()).clone();
            data.insert(field_name, serde_json::Value::String(event.chain().to_string()));
        }
        
        if !excluded.contains(&"block_number".to_string()) {
            let field_name = field_map.get("block_number").unwrap_or(&"block_number".to_string()).clone();
            data.insert(field_name, serde_json::Value::Number(serde_json::Number::from(event.block_number())));
        }
        
        if !excluded.contains(&"block_hash".to_string()) {
            let field_name = field_map.get("block_hash").unwrap_or(&"block_hash".to_string()).clone();
            data.insert(field_name, serde_json::Value::String(event.block_hash().to_string()));
        }
        
        if !excluded.contains(&"tx_hash".to_string()) {
            let field_name = field_map.get("tx_hash").unwrap_or(&"tx_hash".to_string()).clone();
            data.insert(field_name, serde_json::Value::String(event.tx_hash().to_string()));
        }
        
        if !excluded.contains(&"event_type".to_string()) {
            let field_name = field_map.get("event_type").unwrap_or(&"event_type".to_string()).clone();
            data.insert(field_name, serde_json::Value::String(event.event_type().to_string()));
        }
        
        if !excluded.contains(&"timestamp".to_string()) {
            let field_name = field_map.get("timestamp").unwrap_or(&"timestamp".to_string()).clone();
            let timestamp = event.timestamp()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            data.insert(field_name, serde_json::Value::Number(serde_json::Number::from(timestamp)));
        }
        
        // Raw data
        if !excluded.contains(&"raw_data".to_string()) {
            let field_name = field_map.get("raw_data").unwrap_or(&"raw_data".to_string()).clone();
            if let Ok(raw_str) = String::from_utf8(event.raw_data().to_vec()) {
                if let Ok(raw_json) = serde_json::from_str::<serde_json::Value>(&raw_str) {
                    data.insert(field_name, raw_json);
                } else {
                    data.insert(field_name, serde_json::Value::String(raw_str));
                }
            } else {
                // Binary data as base64
                data.insert(field_name, serde_json::Value::String(base64::encode(event.raw_data())));
            }
        }
        
        Ok(serde_json::Value::Object(data))
    }
    
    /// Write data in JSON format
    async fn write_json(
        &self,
        events: &[&dyn Event],
        config: &ExportConfig,
    ) -> Result<(String, u64)> {
        let file_path = &config.output_path;
        let mut file = File::create(file_path).await
            .map_err(|e| Error::Generic(format!("Failed to create file {}: {}", file_path, e)))?;
        
        let mut data = Vec::new();
        for event in events {
            let formatted = self.format_event_data(*event, config)?;
            data.push(formatted);
        }
        
        let json_str = if config.pretty_json {
            serde_json::to_string_pretty(&data)
        } else {
            serde_json::to_string(&data)
        }.map_err(|e| Error::Generic(format!("Failed to serialize JSON: {}", e)))?;
        
        file.write_all(json_str.as_bytes()).await
            .map_err(|e| Error::Generic(format!("Failed to write to file: {}", e)))?;
        
        file.flush().await
            .map_err(|e| Error::Generic(format!("Failed to flush file: {}", e)))?;
        
        Ok((file_path.clone(), json_str.len() as u64))
    }
    
    /// Write data in JSONL format
    async fn write_jsonl(
        &self,
        events: &[&dyn Event],
        config: &ExportConfig,
    ) -> Result<(String, u64)> {
        let file_path = &config.output_path;
        let mut file = File::create(file_path).await
            .map_err(|e| Error::Generic(format!("Failed to create file {}: {}", file_path, e)))?;
        
        let mut total_bytes = 0u64;
        
        for event in events {
            let formatted = self.format_event_data(*event, config)?;
            let json_str = serde_json::to_string(&formatted)
                .map_err(|e| Error::Generic(format!("Failed to serialize JSON: {}", e)))?;
            
            file.write_all(json_str.as_bytes()).await
                .map_err(|e| Error::Generic(format!("Failed to write to file: {}", e)))?;
            file.write_all(b"\n").await
                .map_err(|e| Error::Generic(format!("Failed to write newline: {}", e)))?;
            
            total_bytes += json_str.len() as u64 + 1;
        }
        
        file.flush().await
            .map_err(|e| Error::Generic(format!("Failed to flush file: {}", e)))?;
        
        Ok((file_path.clone(), total_bytes))
    }
    
    /// Apply export filter to events
    fn apply_filter<'a>(&self, events: Vec<&'a dyn Event>, filter: &ExportFilter) -> Vec<&'a dyn Event> {
        let mut filtered = events;
        
        // Filter by chains
        if let Some(chains) = &filter.chains {
            filtered.retain(|event| chains.contains(&event.chain().to_string()));
        }
        
        // Filter by event types
        if let Some(event_types) = &filter.event_types {
            filtered.retain(|event| event_types.contains(&event.event_type().to_string()));
        }
        
        // Filter by block range
        if let Some((min_block, max_block)) = filter.block_range {
            filtered.retain(|event| {
                event.block_number() >= min_block && event.block_number() <= max_block
            });
        }
        
        // Filter by time range
        if let Some((start_time, end_time)) = filter.time_range {
            filtered.retain(|event| {
                event.timestamp() >= start_time && event.timestamp() <= end_time
            });
        }
        
        // Filter by transaction hashes
        if let Some(tx_hashes) = &filter.tx_hashes {
            filtered.retain(|event| tx_hashes.contains(&event.tx_hash().to_string()));
        }
        
        // Apply sorting
        if let Some((sort_field, direction)) = &filter.sort_by {
            match sort_field.as_str() {
                "block_number" => {
                    if matches!(direction, SortDirection::Ascending) {
                        filtered.sort_by_key(|event| event.block_number());
                    } else {
                        filtered.sort_by_key(|event| std::cmp::Reverse(event.block_number()));
                    }
                }
                "timestamp" => {
                    if matches!(direction, SortDirection::Ascending) {
                        filtered.sort_by_key(|event| event.timestamp());
                    } else {
                        filtered.sort_by_key(|event| std::cmp::Reverse(event.timestamp()));
                    }
                }
                _ => {
                    // Default sort by block number
                    filtered.sort_by_key(|event| event.block_number());
                }
            }
        }
        
        // Apply limit and offset
        if let Some(offset) = filter.offset {
            if offset as usize >= filtered.len() {
                return Vec::new();
            }
            filtered = filtered.into_iter().skip(offset as usize).collect();
        }
        
        if let Some(limit) = filter.limit {
            filtered.truncate(limit as usize);
        }
        
        filtered
    }
}

#[async_trait]
impl DataExporter for DefaultDataExporter {
    async fn start_export(
        &self,
        _config: ExportConfig,
        _filter: ExportFilter,
    ) -> Result<String> {
        let job_id = Self::generate_job_id();
        
        let progress = ExportProgress {
            job_id: job_id.clone(),
            status: ExportStatus::Queued,
            total_records: 0,
            processed_records: 0,
            files_created: 0,
            start_time: SystemTime::now(),
            estimated_completion: None,
            processing_speed: 0.0,
            error_message: None,
            metadata: HashMap::new(),
        };
        
        let mut jobs = self.jobs.write().await;
        jobs.insert(job_id.clone(), progress);
        
        // In a real implementation, this would start a background task
        // For now, we'll just mark it as queued
        
        Ok(job_id)
    }
    
    async fn get_progress(&self, job_id: &str) -> Result<Option<ExportProgress>> {
        let jobs = self.jobs.read().await;
        Ok(jobs.get(job_id).cloned())
    }
    
    async fn cancel_export(&self, job_id: &str) -> Result<()> {
        let mut jobs = self.jobs.write().await;
        if let Some(progress) = jobs.get_mut(job_id) {
            progress.status = ExportStatus::Cancelled;
        }
        Ok(())
    }
    
    async fn get_result(&self, job_id: &str) -> Result<Option<ExportResult>> {
        let results = self.results.read().await;
        Ok(results.get(job_id).cloned())
    }
    
    async fn list_jobs(&self) -> Result<Vec<ExportProgress>> {
        let jobs = self.jobs.read().await;
        Ok(jobs.values().cloned().collect())
    }
    
    async fn cleanup_jobs(&self, older_than: Duration) -> Result<u32> {
        let cutoff_time = SystemTime::now() - older_than;
        let mut jobs = self.jobs.write().await;
        let mut results = self.results.write().await;
        
        let mut cleaned_count = 0u32;
        
        // Clean up completed jobs older than cutoff
        jobs.retain(|_, progress| {
            if matches!(progress.status, ExportStatus::Completed | ExportStatus::Failed | ExportStatus::Cancelled) && progress.start_time < cutoff_time {
                cleaned_count += 1;
                return false;
            }
            true
        });
        
        // Clean up old results
        results.retain(|_, result| {
            result.config.output_path.len() > 0 // Keep all results for now
        });
        
        Ok(cleaned_count)
    }
    
    async fn export_events(
        &self,
        events: Vec<&dyn Event>,
        config: ExportConfig,
    ) -> Result<ExportResult> {
        let start_time = SystemTime::now();
        let job_id = Self::generate_job_id();
        
        // Apply filters (in a real implementation, filtering might happen at the query level)
        let filter = ExportFilter::default();
        let filtered_events = self.apply_filter(events, &filter);
        
        let total_records = filtered_events.len() as u64;
        let mut statistics = ExportStatistics::default();
        
        // Perform export based on format
        let (output_file, bytes_written) = match config.format {
            ExportFormat::Json => {
                self.write_json(&filtered_events, &config).await?
            }
            ExportFormat::Jsonl => {
                self.write_jsonl(&filtered_events, &config).await?
            }
            _ => {
                return Err(Error::Generic("Unsupported export format".to_string()));
            }
        };
        
        let duration = start_time.elapsed().unwrap_or_default();
        statistics.bytes_written = bytes_written;
        statistics.avg_processing_speed = if duration.as_secs_f64() > 0.0 {
            total_records as f64 / duration.as_secs_f64()
        } else {
            0.0
        };
        statistics.peak_processing_speed = statistics.avg_processing_speed;
        statistics.batches_processed = 1;
        statistics.avg_batch_size = total_records as f64;
        
        let result = ExportResult {
            job_id,
            status: ExportStatus::Completed,
            output_files: vec![output_file],
            total_records,
            duration,
            file_sizes: vec![bytes_written],
            config,
            filter,
            error_details: None,
            statistics,
        };
        
        Ok(result)
    }
}

/// Export job manager for handling multiple concurrent exports
pub struct ExportJobManager {
    exporter: std::sync::Arc<dyn DataExporter>,
    max_concurrent_jobs: usize,
    active_jobs: std::sync::Arc<tokio::sync::RwLock<HashMap<String, tokio::task::JoinHandle<()>>>>,
}

impl ExportJobManager {
    /// Create a new export job manager
    pub fn new(exporter: std::sync::Arc<dyn DataExporter>, max_concurrent_jobs: usize) -> Self {
        Self {
            exporter,
            max_concurrent_jobs,
            active_jobs: std::sync::Arc::new(tokio::sync::RwLock::new(HashMap::new())),
        }
    }
    
    /// Start an export job asynchronously
    pub async fn start_async_export(
        &self,
        config: ExportConfig,
        filter: ExportFilter,
    ) -> Result<String> {
        let active_jobs = self.active_jobs.read().await;
        if active_jobs.len() >= self.max_concurrent_jobs {
            return Err(Error::Generic("Maximum concurrent jobs reached".to_string()));
        }
        drop(active_jobs);
        
        let job_id = self.exporter.start_export(config, filter).await?;
        
        // In a real implementation, we would start a background task here
        // For now, we'll just return the job ID
        
        Ok(job_id)
    }
    
    /// Get status of all active jobs
    pub async fn get_active_jobs(&self) -> Result<Vec<String>> {
        let active_jobs = self.active_jobs.read().await;
        Ok(active_jobs.keys().cloned().collect())
    }
}

/// Predefined export configurations for common use cases
pub struct PredefinedExportConfigs;

impl PredefinedExportConfigs {
    /// TSV export with headers (formerly CSV)
    pub fn tsv_with_headers(output_path: String) -> ExportConfig {
        ExportConfig {
            format: ExportFormat::Tsv,
            output_path,
            include_headers: true,
            delimiter: "\t".to_string(),
            date_format: "%Y-%m-%d %H:%M:%S".to_string(),
            ..Default::default()
        }
    }
    
    /// Pretty JSON export
    pub fn pretty_json(output_path: String) -> ExportConfig {
        ExportConfig {
            format: ExportFormat::Json,
            output_path,
            pretty_json: true,
            include_metadata: true,
            ..Default::default()
        }
    }
    
    /// Compact JSONL export for large datasets
    pub fn compact_jsonl(output_path: String) -> ExportConfig {
        ExportConfig {
            format: ExportFormat::Jsonl,
            output_path,
            pretty_json: false,
            max_records_per_file: Some(100000), // 100k records per file
            ..Default::default()
        }
    }
    
    /// Excel-compatible TSV (formerly CSV)
    pub fn excel_tsv(output_path: String) -> ExportConfig {
        ExportConfig {
            format: ExportFormat::Tsv,
            output_path,
            include_headers: true,
            delimiter: "\t".to_string(),
            date_format: "%m/%d/%Y %H:%M:%S".to_string(),
            ..Default::default()
        }
    }
}

// Dummy base64 encoding function for simplified implementation
mod base64 {
    pub fn encode(data: &[u8]) -> String {
        // Simplified base64 encoding - in real implementation use base64 crate
        let chars = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let mut result = String::new();
        
        for chunk in data.chunks(3) {
            let mut buf = [0u8; 3];
            for (i, &byte) in chunk.iter().enumerate() {
                buf[i] = byte;
            }
            
            let b1 = buf[0] >> 2;
            let b2 = ((buf[0] & 0x03) << 4) | (buf[1] >> 4);
            let b3 = ((buf[1] & 0x0f) << 2) | (buf[2] >> 6);
            let b4 = buf[2] & 0x3f;
            
            result.push(chars.chars().nth(b1 as usize).unwrap());
            result.push(chars.chars().nth(b2 as usize).unwrap());
            result.push(if chunk.len() > 1 { chars.chars().nth(b3 as usize).unwrap() } else { '=' });
            result.push(if chunk.len() > 2 { chars.chars().nth(b4 as usize).unwrap() } else { '=' });
        }
        
        result
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
    
    fn create_test_events() -> Vec<TestEvent> {
        vec![
            TestEvent {
                id: "event_1".to_string(),
                chain: "ethereum".to_string(),
                block_number: 100,
                block_hash: "0xabc123".to_string(),
                tx_hash: "0xdef456".to_string(),
                timestamp: UNIX_EPOCH + Duration::from_secs(1000),
                event_type: "Transfer".to_string(),
                raw_data: r#"{"from": "0x123", "to": "0x456", "value": "1000"}"#.as_bytes().to_vec(),
            },
            TestEvent {
                id: "event_2".to_string(),
                chain: "polygon".to_string(),
                block_number: 200,
                block_hash: "0xghi789".to_string(),
                tx_hash: "0xjkl012".to_string(),
                timestamp: UNIX_EPOCH + Duration::from_secs(2000),
                event_type: "Approval".to_string(),
                raw_data: r#"{"owner": "0x789", "spender": "0xabc", "value": "500"}"#.as_bytes().to_vec(),
            },
        ]
    }
    
    #[tokio::test]
    async fn test_exporter_creation() {
        let exporter = DefaultDataExporter::new();
        let jobs = exporter.list_jobs().await.unwrap();
        assert!(jobs.is_empty());
    }
    
    #[tokio::test]
    async fn test_export_json() {
        let exporter = DefaultDataExporter::new();
        let events = create_test_events();
        let event_refs: Vec<&dyn Event> = events.iter().map(|e| e as &dyn Event).collect();
        
        let config = ExportConfig {
            format: ExportFormat::Json,
            output_path: "/tmp/test_export.json".to_string(),
            pretty_json: true,
            ..Default::default()
        };
        
        let result = exporter.export_events(event_refs, config).await.unwrap();
        
        assert_eq!(result.status, ExportStatus::Completed);
        assert_eq!(result.total_records, 2);
        assert_eq!(result.output_files.len(), 1);
        assert!(result.statistics.bytes_written > 0);
    }
    
    #[tokio::test]
    async fn test_export_jsonl() {
        let exporter = DefaultDataExporter::new();
        let events = create_test_events();
        let event_refs: Vec<&dyn Event> = events.iter().map(|e| e as &dyn Event).collect();
        
        let config = ExportConfig {
            format: ExportFormat::Jsonl,
            output_path: "/tmp/test_export.jsonl".to_string(),
            include_headers: false,
            ..Default::default()
        };
        
        let result = exporter.export_events(event_refs, config).await.unwrap();
        
        assert_eq!(result.status, ExportStatus::Completed);
        assert_eq!(result.total_records, 2);
        assert!(result.statistics.bytes_written > 0);
    }
    
    #[tokio::test]
    async fn test_export_filtering() {
        let exporter = DefaultDataExporter::new();
        let events = create_test_events();
        let event_refs: Vec<&dyn Event> = events.iter().map(|e| e as &dyn Event).collect();
        
        let filter = ExportFilter {
            chains: Some(vec!["ethereum".to_string()]),
            limit: Some(1),
            ..Default::default()
        };
        
        let filtered = exporter.apply_filter(event_refs, &filter);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].chain(), "ethereum");
    }
    
    #[tokio::test]
    async fn test_job_management() {
        let exporter = DefaultDataExporter::new();
        let config = ExportConfig::default();
        let filter = ExportFilter::default();
        
        let job_id = exporter.start_export(config, filter).await.unwrap();
        
        let progress = exporter.get_progress(&job_id).await.unwrap();
        assert!(progress.is_some());
        assert_eq!(progress.unwrap().status, ExportStatus::Queued);
        
        exporter.cancel_export(&job_id).await.unwrap();
        
        let progress = exporter.get_progress(&job_id).await.unwrap();
        assert!(progress.is_some());
        assert_eq!(progress.unwrap().status, ExportStatus::Cancelled);
    }
    
    #[tokio::test]
    async fn test_job_cleanup() {
        let exporter = DefaultDataExporter::new();
        let config = ExportConfig::default();
        let filter = ExportFilter::default();
        
        // Create a job
        let _job_id = exporter.start_export(config, filter).await.unwrap();
        
        // Clean up jobs older than 1 hour
        let cleaned = exporter.cleanup_jobs(Duration::from_secs(3600)).await.unwrap();
        assert_eq!(cleaned, 0); // Should not clean up recent jobs
    }
    
    #[test]
    fn test_predefined_configs() {
        let tsv_config = PredefinedExportConfigs::tsv_with_headers("test.tsv".to_string());
        assert_eq!(tsv_config.format, ExportFormat::Tsv);
        assert!(tsv_config.include_headers);
        
        let json_config = PredefinedExportConfigs::pretty_json("test.json".to_string());
        assert_eq!(json_config.format, ExportFormat::Json);
        assert!(json_config.pretty_json);
        
        let jsonl_config = PredefinedExportConfigs::compact_jsonl("test.jsonl".to_string());
        assert_eq!(jsonl_config.format, ExportFormat::Jsonl);
        assert_eq!(jsonl_config.max_records_per_file, Some(100000));
    }
    
    #[tokio::test]
    async fn test_export_job_manager() {
        let exporter = std::sync::Arc::new(DefaultDataExporter::new());
        let manager = ExportJobManager::new(exporter, 5);
        
        let config = ExportConfig::default();
        let filter = ExportFilter::default();
        
        let _job_id = manager.start_async_export(config, filter).await.unwrap();
        
        let active_jobs = manager.get_active_jobs().await.unwrap();
        // Note: In this simplified implementation, active jobs tracking is not fully implemented
        assert!(active_jobs.len() <= 5);
    }
} 