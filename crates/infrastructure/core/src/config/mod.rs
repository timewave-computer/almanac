// Modernized configuration system with TOML support and hot-reloading
//
// This module provides a unified TOML-based configuration system with
// environment-specific overrides, hot-reloading capabilities, and enhanced
// validation with detailed error messages.

use std::collections::HashMap;
use std::path::PathBuf;
use std::time::SystemTime;
use serde::{Deserialize, Serialize};

use crate::{Error, Result};

pub mod toml;

/// Environment types for configuration
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Environment {
    #[default]
    Development,
    Staging,
    Production,
    Test,
}

impl std::fmt::Display for Environment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Environment::Development => write!(f, "development"),
            Environment::Staging => write!(f, "staging"),
            Environment::Production => write!(f, "production"),
            Environment::Test => write!(f, "test"),
        }
    }
}

impl std::str::FromStr for Environment {
    type Err = Error;
    
    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "development" | "dev" => Ok(Environment::Development),
            "staging" => Ok(Environment::Staging),
            "production" | "prod" => Ok(Environment::Production),
            "test" => Ok(Environment::Test),
            _ => Err(Error::config(format!("Invalid environment: {}", s))),
        }
    }
}

/// Configuration validation error
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigValidationError {
    /// Field path (e.g., "database.postgres_url")
    pub field: String,
    
    /// Error message
    pub message: String,
    
    /// Suggested fix
    pub suggestion: Option<String>,
    
    /// Error severity
    pub severity: ErrorSeverity,
}

/// Error severity levels
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ErrorSeverity {
    Critical,
    Warning,
    Info,
}

impl std::fmt::Display for ConfigValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}] {}: {}", 
            match self.severity {
                ErrorSeverity::Critical => "ERROR",
                ErrorSeverity::Warning => "WARN",
                ErrorSeverity::Info => "INFO",
            },
            self.field, 
            self.message
        )?;
        
        if let Some(suggestion) = &self.suggestion {
            write!(f, " (Suggestion: {})", suggestion)?;
        }
        
        Ok(())
    }
}

/// Configuration validation result
pub type ConfigValidationResult = std::result::Result<(), Vec<ConfigValidationError>>;

/// Hot-reload configuration change event
#[derive(Debug, Clone)]
pub struct ConfigChangeEvent {
    /// Changed configuration sections
    pub changed_sections: Vec<String>,
    
    /// New configuration
    pub new_config: AlmanacConfig,
    
    /// Previous configuration
    pub previous_config: AlmanacConfig,
    
    /// Change timestamp
    pub timestamp: SystemTime,
}

/// Database configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    /// Database type (postgres, rocks, hybrid)
    pub database_type: DatabaseType,
    
    /// PostgreSQL configuration
    pub postgres: Option<PostgresConfig>,
    
    /// RocksDB configuration
    pub rocks: Option<RocksConfig>,
    
    /// Connection pool settings
    pub pool: ConnectionPoolConfig,
    
    /// Backup and recovery settings
    pub backup: BackupConfig,
}

/// Database type enum
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DatabaseType {
    Postgres,
    Rocks,
    Hybrid,
}

/// PostgreSQL specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostgresConfig {
    /// Connection URL
    pub url: String,
    
    /// SSL mode
    pub ssl_mode: SslMode,
    
    /// Enable prepared statements
    pub prepared_statements: bool,
    
    /// Statement timeout in seconds
    pub statement_timeout: u64,
    
    /// Application name
    pub application_name: String,
}

/// SSL mode for PostgreSQL
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SslMode {
    Disable,
    Prefer,
    Require,
    VerifyCA,
    VerifyFull,
}

/// RocksDB specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RocksConfig {
    /// Database path
    pub path: PathBuf,
    
    /// Cache size in MB
    pub cache_size_mb: usize,
    
    /// Write buffer size in MB
    pub write_buffer_size_mb: usize,
    
    /// Maximum write buffers
    pub max_write_buffers: usize,
    
    /// Enable WAL
    pub enable_wal: bool,
    
    /// Compression type
    pub compression: CompressionType,
    
    /// Block size in bytes
    pub block_size: usize,
}

/// Compression types for RocksDB
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CompressionType {
    None,
    Snappy,
    Zlib,
    Bz2,
    Lz4,
    Lz4hc,
    Zstd,
}

/// Connection pool configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionPoolConfig {
    /// Maximum connections
    pub max_connections: u32,
    
    /// Minimum connections
    pub min_connections: u32,
    
    /// Connection timeout in seconds
    pub connection_timeout: u64,
    
    /// Idle timeout in seconds
    pub idle_timeout: u64,
    
    /// Max lifetime of connections in seconds
    pub max_lifetime: u64,
}

/// Backup configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupConfig {
    /// Enable automatic backups
    pub enabled: bool,
    
    /// Backup interval in hours
    pub interval_hours: u64,
    
    /// Backup retention days
    pub retention_days: u64,
    
    /// Backup directory
    pub directory: PathBuf,
    
    /// Compress backups
    pub compress: bool,
}

/// API server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiConfig {
    /// Server host
    pub host: String,
    
    /// Server port
    pub port: u16,
    
    /// Enable TLS
    pub tls_enabled: bool,
    
    /// TLS certificate path
    pub tls_cert_path: Option<PathBuf>,
    
    /// TLS key path
    pub tls_key_path: Option<PathBuf>,
    
    /// CORS configuration
    pub cors: CorsConfig,
    
    /// Rate limiting configuration
    pub rate_limiting: RateLimitConfig,
    
    /// Authentication configuration
    pub auth: AuthConfig,
    
    /// Request configuration
    pub requests: RequestConfig,
}

/// CORS configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorsConfig {
    /// Enable CORS
    pub enabled: bool,
    
    /// Allowed origins
    pub allowed_origins: Vec<String>,
    
    /// Allowed methods
    pub allowed_methods: Vec<String>,
    
    /// Allowed headers
    pub allowed_headers: Vec<String>,
    
    /// Max age in seconds
    pub max_age: u64,
}

/// Rate limiting configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    /// Enable rate limiting
    pub enabled: bool,
    
    /// Requests per minute per IP
    pub requests_per_minute: u32,
    
    /// Burst size
    pub burst_size: u32,
    
    /// Whitelist IPs (no rate limiting)
    pub whitelist: Vec<String>,
}

/// Authentication configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    /// Enable authentication
    pub enabled: bool,
    
    /// JWT secret
    pub jwt_secret: Option<String>,
    
    /// JWT expiration in hours
    pub jwt_expiration_hours: u64,
    
    /// API keys
    pub api_keys: Vec<ApiKeyConfig>,
}

/// API key configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKeyConfig {
    /// Key name
    pub name: String,
    
    /// Key value
    pub key: String,
    
    /// Allowed permissions
    pub permissions: Vec<String>,
    
    /// Expiration date (ISO 8601)
    pub expires_at: Option<String>,
}

/// Request configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestConfig {
    /// Request timeout in seconds
    pub timeout_seconds: u64,
    
    /// Maximum request size in bytes
    pub max_size_bytes: usize,
    
    /// Keep-alive timeout in seconds
    pub keep_alive_timeout: u64,
}

/// Chain configuration for event processing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainConfig {
    /// Chain identifier
    pub chain_id: String,
    
    /// Chain type
    pub chain_type: ChainType,
    
    /// RPC configuration
    pub rpc: RpcConfig,
    
    /// Event processing configuration
    pub events: EventConfig,
    
    /// Finality configuration
    pub finality: FinalityConfig,
    
    /// Rate limiting for this chain
    pub rate_limiting: ChainRateLimitConfig,
}

/// Chain types
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ChainType {
    Ethereum,
    Cosmos,
}

/// RPC configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcConfig {
    /// Primary RPC URL
    pub url: String,
    
    /// Fallback RPC URLs
    pub fallback_urls: Vec<String>,
    
    /// Request timeout in seconds
    pub timeout_seconds: u64,
    
    /// Maximum retries
    pub max_retries: u32,
    
    /// Retry delay in milliseconds
    pub retry_delay_ms: u64,
    
    /// Enable WebSocket
    pub websocket_enabled: bool,
    
    /// WebSocket URL
    pub websocket_url: Option<String>,
}

/// Event processing configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventConfig {
    /// Enable event processing for this chain
    pub enabled: bool,
    
    /// Starting block number
    pub start_block: Option<u64>,
    
    /// Ending block number
    pub end_block: Option<u64>,
    
    /// Batch size for processing
    pub batch_size: u32,
    
    /// Poll interval in milliseconds
    pub poll_interval_ms: u64,
    
    /// Maximum concurrent requests
    pub max_concurrent_requests: u32,
    
    /// Event filters
    pub filters: Vec<EventFilterConfig>,
}

/// Event filter configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventFilterConfig {
    /// Filter name
    pub name: String,
    
    /// Contract addresses to filter
    pub addresses: Vec<String>,
    
    /// Event topics to filter
    pub topics: Vec<String>,
    
    /// Enable filter
    pub enabled: bool,
}

/// Finality configuration for chains
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FinalityConfig {
    /// Confirmation blocks required
    pub confirmation_blocks: u64,
    
    /// Enable reorg detection
    pub reorg_detection: bool,
    
    /// Maximum reorg depth to handle
    pub max_reorg_depth: u64,
    
    /// Finality timeout in seconds
    pub finality_timeout_seconds: u64,
}

/// Chain-specific rate limiting
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainRateLimitConfig {
    /// Enable rate limiting
    pub enabled: bool,
    
    /// Requests per second
    pub requests_per_second: u32,
    
    /// Burst capacity
    pub burst_capacity: u32,
}

/// Logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogConfig {
    /// Log level
    pub level: LogLevel,
    
    /// Log format
    pub format: LogFormat,
    
    /// Console logging configuration
    pub console: ConsoleLogConfig,
    
    /// File logging configuration
    pub file: FileLogConfig,
    
    /// Structured logging fields
    pub fields: HashMap<String, String>,
}

/// Log levels
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

/// Log formats
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogFormat {
    Json,
    Pretty,
    Compact,
}

/// Console logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsoleLogConfig {
    /// Enable console logging
    pub enabled: bool,
    
    /// Use colors in output
    pub colored: bool,
    
    /// Show timestamps
    pub show_timestamps: bool,
    
    /// Show thread names
    pub show_thread_names: bool,
}

/// File logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileLogConfig {
    /// Enable file logging
    pub enabled: bool,
    
    /// Log file path
    pub path: PathBuf,
    
    /// Maximum file size in MB
    pub max_size_mb: u64,
    
    /// Maximum number of files
    pub max_files: u32,
    
    /// Compress rotated files
    pub compress: bool,
}

/// Monitoring and observability configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringConfig {
    /// Metrics configuration
    pub metrics: MetricsConfig,
    
    /// Health check configuration
    pub health: HealthConfig,
    
    /// Tracing configuration
    pub tracing: TracingConfig,
    
    /// Alerting configuration
    pub alerting: AlertingConfig,
}

/// Metrics configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsConfig {
    /// Enable metrics collection
    pub enabled: bool,
    
    /// Metrics server host
    pub host: String,
    
    /// Metrics server port
    pub port: u16,
    
    /// Metrics path
    pub path: String,
    
    /// Collection interval in seconds
    pub collection_interval_seconds: u64,
    
    /// Enable Prometheus export
    pub prometheus_enabled: bool,
}

/// Health check configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthConfig {
    /// Enable health checks
    pub enabled: bool,
    
    /// Health check path
    pub path: String,
    
    /// Check interval in seconds
    pub interval_seconds: u64,
    
    /// Timeout for checks in seconds
    pub timeout_seconds: u64,
    
    /// Enable detailed health info
    pub detailed: bool,
}

/// Distributed tracing configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TracingConfig {
    /// Enable distributed tracing
    pub enabled: bool,
    
    /// Tracing service endpoint
    pub endpoint: Option<String>,
    
    /// Service name for tracing
    pub service_name: String,
    
    /// Sampling rate (0.0 to 1.0)
    pub sampling_rate: f64,
}

/// Alerting configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertingConfig {
    /// Enable alerting
    pub enabled: bool,
    
    /// Alert thresholds
    pub thresholds: HashMap<String, AlertThreshold>,
    
    /// Notification channels
    pub channels: Vec<NotificationChannel>,
}

/// Alert threshold configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertThreshold {
    /// Metric name
    pub metric: String,
    
    /// Threshold value
    pub value: f64,
    
    /// Comparison operator
    pub operator: ComparisonOperator,
    
    /// Duration in seconds before triggering
    pub duration_seconds: u64,
}

/// Comparison operators for alerts
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ComparisonOperator {
    GreaterThan,
    LessThan,
    Equal,
    GreaterThanOrEqual,
    LessThanOrEqual,
}

/// Notification channels for alerts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationChannel {
    /// Channel name
    pub name: String,
    
    /// Channel type
    pub channel_type: ChannelType,
    
    /// Channel configuration
    pub config: HashMap<String, String>,
    
    /// Enable channel
    pub enabled: bool,
}

/// Notification channel types
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ChannelType {
    Email,
    Slack,
    Webhook,
    Discord,
}

/// Main Almanac configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlmanacConfig {
    /// Environment
    pub environment: Environment,
    
    /// Database configuration
    pub database: DatabaseConfig,
    
    /// API configuration
    pub api: ApiConfig,
    
    /// Chain configurations
    pub chains: HashMap<String, ChainConfig>,
    
    /// Logging configuration
    pub logging: LogConfig,
    
    /// Monitoring configuration
    pub monitoring: MonitoringConfig,
    
    /// Event processing configuration
    pub event_processing: EventProcessingConfig,
    
    /// Additional custom settings
    pub custom: HashMap<String, toml::Value>,
}

/// Event processing global configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventProcessingConfig {
    /// Event ordering configuration
    pub ordering: super::event_ordering::EventOrderingConfig,
    
    /// Finality processor configuration
    pub finality: super::finality_processor::FinalityProcessorConfig,
    
    /// Ingestion queue configuration
    pub ingestion: super::ingestion_queue::IngestionQueueConfig,
}

impl Default for AlmanacConfig {
    fn default() -> Self {
        // Default chains
        let mut chains = HashMap::new();
        
        // Ethereum configuration
        chains.insert("ethereum".to_string(), ChainConfig {
            chain_id: "ethereum".to_string(),
            chain_type: ChainType::Ethereum,
            rpc: RpcConfig {
                url: "https://eth-mainnet.alchemyapi.io/v2/your-api-key".to_string(),
                fallback_urls: vec![
                    "https://mainnet.infura.io/v3/your-project-id".to_string(),
                ],
                timeout_seconds: 30,
                max_retries: 3,
                retry_delay_ms: 1000,
                websocket_enabled: true,
                websocket_url: Some("wss://eth-mainnet.alchemyapi.io/v2/your-api-key".to_string()),
            },
            events: EventConfig {
                enabled: true,
                start_block: None,
                end_block: None,
                batch_size: 100,
                poll_interval_ms: 12000,
                max_concurrent_requests: 10,
                filters: vec![],
            },
            finality: FinalityConfig {
                confirmation_blocks: 12,
                reorg_detection: true,
                max_reorg_depth: 100,
                finality_timeout_seconds: 300,
            },
            rate_limiting: ChainRateLimitConfig {
                enabled: true,
                requests_per_second: 10,
                burst_capacity: 20,
            },
        });
        
        Self {
            environment: Environment::Development,
            database: DatabaseConfig {
                database_type: DatabaseType::Hybrid,
                postgres: Some(PostgresConfig {
                    url: "postgresql://localhost:5432/almanac".to_string(),
                    ssl_mode: SslMode::Prefer,
                    prepared_statements: true,
                    statement_timeout: 30,
                    application_name: "almanac".to_string(),
                }),
                rocks: Some(RocksConfig {
                    path: PathBuf::from("./data/rocks"),
                    cache_size_mb: 256,
                    write_buffer_size_mb: 64,
                    max_write_buffers: 3,
                    enable_wal: true,
                    compression: CompressionType::Lz4,
                    block_size: 16384,
                }),
                pool: ConnectionPoolConfig {
                    max_connections: 10,
                    min_connections: 1,
                    connection_timeout: 30,
                    idle_timeout: 600,
                    max_lifetime: 3600,
                },
                backup: BackupConfig {
                    enabled: false,
                    interval_hours: 24,
                    retention_days: 7,
                    directory: PathBuf::from("./backups"),
                    compress: true,
                },
            },
            api: ApiConfig {
                host: "127.0.0.1".to_string(),
                port: 8080,
                tls_enabled: false,
                tls_cert_path: None,
                tls_key_path: None,
                cors: CorsConfig {
                    enabled: true,
                    allowed_origins: vec!["*".to_string()],
                    allowed_methods: vec!["GET".to_string(), "POST".to_string()],
                    allowed_headers: vec!["content-type".to_string()],
                    max_age: 3600,
                },
                rate_limiting: RateLimitConfig {
                    enabled: true,
                    requests_per_minute: 100,
                    burst_size: 20,
                    whitelist: vec![],
                },
                auth: AuthConfig {
                    enabled: false,
                    jwt_secret: None,
                    jwt_expiration_hours: 24,
                    api_keys: vec![],
                },
                requests: RequestConfig {
                    timeout_seconds: 30,
                    max_size_bytes: 1024 * 1024, // 1MB
                    keep_alive_timeout: 75,
                },
            },
            chains,
            logging: LogConfig {
                level: LogLevel::Info,
                format: LogFormat::Pretty,
                console: ConsoleLogConfig {
                    enabled: true,
                    colored: true,
                    show_timestamps: true,
                    show_thread_names: false,
                },
                file: FileLogConfig {
                    enabled: false,
                    path: PathBuf::from("./logs/almanac.log"),
                    max_size_mb: 100,
                    max_files: 10,
                    compress: true,
                },
                fields: HashMap::new(),
            },
            monitoring: MonitoringConfig {
                metrics: MetricsConfig {
                    enabled: true,
                    host: "127.0.0.1".to_string(),
                    port: 9090,
                    path: "/metrics".to_string(),
                    collection_interval_seconds: 15,
                    prometheus_enabled: true,
                },
                health: HealthConfig {
                    enabled: true,
                    path: "/health".to_string(),
                    interval_seconds: 30,
                    timeout_seconds: 5,
                    detailed: false,
                },
                tracing: TracingConfig {
                    enabled: false,
                    endpoint: None,
                    service_name: "almanac".to_string(),
                    sampling_rate: 0.1,
                },
                alerting: AlertingConfig {
                    enabled: false,
                    thresholds: HashMap::new(),
                    channels: vec![],
                },
            },
            event_processing: EventProcessingConfig {
                ordering: super::event_ordering::EventOrderingConfig::default(),
                finality: super::finality_processor::FinalityProcessorConfig::default(),
                ingestion: super::ingestion_queue::IngestionQueueConfig::default(),
            },
            custom: HashMap::new(),
        }
    }
} 