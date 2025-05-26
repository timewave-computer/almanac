/// Configuration management system for Almanac indexer
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::fmt;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

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

impl fmt::Display for Environment {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Environment::Development => write!(f, "development"),
            Environment::Staging => write!(f, "staging"),
            Environment::Production => write!(f, "production"),
            Environment::Test => write!(f, "test"),
        }
    }
}

/// Database configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    /// Database type (postgres, rocks)
    pub db_type: String,
    
    /// Connection URL for PostgreSQL
    pub postgres_url: Option<String>,
    
    /// RocksDB path
    pub rocks_path: Option<String>,
    
    /// Maximum connections in pool
    pub max_connections: Option<u32>,
    
    /// Connection timeout in seconds
    pub connection_timeout: Option<u64>,
    
    /// Cache size in MB
    pub cache_size_mb: Option<usize>,
    
    /// Enable WAL (Write-Ahead Logging)
    pub enable_wal: Option<bool>,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            db_type: "rocks".to_string(),
            postgres_url: None,
            rocks_path: Some("./data/rocks".to_string()),
            max_connections: Some(10),
            connection_timeout: Some(30),
            cache_size_mb: Some(128),
            enable_wal: Some(true),
        }
    }
}

/// API server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiConfig {
    /// Server host
    pub host: String,
    
    /// Server port
    pub port: u16,
    
    /// Enable CORS
    pub cors_enabled: bool,
    
    /// Allowed CORS origins
    pub cors_origins: Vec<String>,
    
    /// Request timeout in seconds
    pub request_timeout: u64,
    
    /// Maximum request size in bytes
    pub max_request_size: usize,
    
    /// Rate limiting: requests per minute
    pub rate_limit: Option<u32>,
    
    /// Enable API key authentication
    pub auth_enabled: bool,
    
    /// JWT secret for authentication
    pub jwt_secret: Option<String>,
    
    /// API key for simple authentication
    pub api_key: Option<String>,
}

impl Default for ApiConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 8080,
            cors_enabled: true,
            cors_origins: vec!["*".to_string()],
            request_timeout: 30,
            max_request_size: 1024 * 1024, // 1MB
            rate_limit: Some(100),
            auth_enabled: false,
            jwt_secret: None,
            api_key: None,
        }
    }
}

/// Chain configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainConfig {
    /// Chain identifier
    pub chain_id: String,
    
    /// Human-readable chain name
    pub name: String,
    
    /// RPC endpoint URL
    pub rpc_url: String,
    
    /// WebSocket endpoint URL
    pub ws_url: Option<String>,
    
    /// Starting block number for indexing
    pub start_block: Option<u64>,
    
    /// Block confirmation count for finality
    pub confirmations: u64,
    
    /// Polling interval in milliseconds
    pub poll_interval_ms: u64,
    
    /// Batch size for bulk operations
    pub batch_size: u32,
    
    /// Enable event indexing
    pub index_events: bool,
    
    /// Enable transaction indexing
    pub index_transactions: bool,
    
    /// Contract addresses to monitor
    pub contract_addresses: HashMap<String, String>,
    
    /// Event signatures to filter
    pub event_signatures: Vec<String>,
}

impl Default for ChainConfig {
    fn default() -> Self {
        Self {
            chain_id: "1".to_string(),
            name: "ethereum".to_string(),
            rpc_url: "https://mainnet.infura.io/v3/YOUR_PROJECT_ID".to_string(),
            ws_url: None,
            start_block: Some(0),
            confirmations: 12,
            poll_interval_ms: 12000, // 12 seconds for Ethereum
            batch_size: 100,
            index_events: true,
            index_transactions: false,
            contract_addresses: HashMap::new(),
            event_signatures: Vec::new(),
        }
    }
}

/// Logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogConfig {
    /// Log level (trace, debug, info, warn, error)
    pub level: String,
    
    /// Log format (json, pretty, compact)
    pub format: String,
    
    /// Log to file
    pub file_enabled: bool,
    
    /// Log file path
    pub file_path: Option<String>,
    
    /// Maximum log file size in MB
    pub max_file_size_mb: Option<u64>,
    
    /// Number of log files to keep
    pub max_files: Option<u32>,
    
    /// Enable structured logging
    pub structured: bool,
}

impl Default for LogConfig {
    fn default() -> Self {
        Self {
            level: "info".to_string(),
            format: "pretty".to_string(),
            file_enabled: false,
            file_path: Some("./logs/almanac.log".to_string()),
            max_file_size_mb: Some(100),
            max_files: Some(10),
            structured: false,
        }
    }
}

/// Monitoring configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringConfig {
    /// Enable metrics collection
    pub metrics_enabled: bool,
    
    /// Metrics endpoint host
    pub metrics_host: String,
    
    /// Metrics endpoint port
    pub metrics_port: u16,
    
    /// Enable health check endpoint
    pub health_check_enabled: bool,
    
    /// Health check endpoint path
    pub health_check_path: String,
    
    /// Performance monitoring enabled
    pub performance_monitoring: bool,
    
    /// Alert thresholds
    pub alert_thresholds: HashMap<String, f64>,
}

impl Default for MonitoringConfig {
    fn default() -> Self {
        let mut alert_thresholds = HashMap::new();
        alert_thresholds.insert("memory_usage_percent".to_string(), 90.0);
        alert_thresholds.insert("cpu_usage_percent".to_string(), 80.0);
        alert_thresholds.insert("disk_usage_percent".to_string(), 85.0);
        alert_thresholds.insert("error_rate_percent".to_string(), 5.0);
        
        Self {
            metrics_enabled: true,
            metrics_host: "127.0.0.1".to_string(),
            metrics_port: 9090,
            health_check_enabled: true,
            health_check_path: "/health".to_string(),
            performance_monitoring: true,
            alert_thresholds,
        }
    }
}

/// Main configuration structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlmanacConfig {
    /// Environment (development, staging, production, test)
    pub environment: Environment,
    
    /// Database configuration
    pub database: DatabaseConfig,
    
    /// API server configuration
    pub api: ApiConfig,
    
    /// Chain configurations
    pub chains: HashMap<String, ChainConfig>,
    
    /// Logging configuration
    pub logging: LogConfig,
    
    /// Monitoring configuration
    pub monitoring: MonitoringConfig,
    
    /// Additional custom settings
    pub custom: HashMap<String, serde_json::Value>,
}

impl Default for AlmanacConfig {
    fn default() -> Self {
        let mut chains = HashMap::new();
        chains.insert("ethereum".to_string(), ChainConfig::default());
        
        Self {
            environment: Environment::default(),
            database: DatabaseConfig::default(),
            api: ApiConfig::default(),
            chains,
            logging: LogConfig::default(),
            monitoring: MonitoringConfig::default(),
            custom: HashMap::new(),
        }
    }
}

/// Configuration validation error
#[derive(Debug)]
pub struct ValidationError {
    pub field: String,
    pub message: String,
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Validation error in field '{}': {}", self.field, self.message)
    }
}

/// Configuration validation result
pub type ValidationResult = std::result::Result<(), Vec<ValidationError>>;

/// Configuration validator trait
pub trait ConfigValidator {
    /// Validate the configuration
    fn validate(&self) -> ValidationResult;
}

impl ConfigValidator for AlmanacConfig {
    fn validate(&self) -> ValidationResult {
        let mut errors = Vec::new();
        
        // Validate database configuration
        if let Err(mut db_errors) = self.database.validate() {
            errors.append(&mut db_errors);
        }
        
        // Validate API configuration
        if let Err(mut api_errors) = self.api.validate() {
            errors.append(&mut api_errors);
        }
        
        // Validate chain configurations
        for (chain_id, chain_config) in &self.chains {
            if let Err(mut chain_errors) = chain_config.validate() {
                // Prefix errors with chain ID
                for error in &mut chain_errors {
                    error.field = format!("chains.{}.{}", chain_id, error.field);
                }
                errors.append(&mut chain_errors);
            }
        }
        
        // Validate logging configuration
        if let Err(mut log_errors) = self.logging.validate() {
            errors.append(&mut log_errors);
        }
        
        // Validate monitoring configuration
        if let Err(mut monitor_errors) = self.monitoring.validate() {
            errors.append(&mut monitor_errors);
        }
        
        // Check for required chains
        if self.chains.is_empty() {
            errors.push(ValidationError {
                field: "chains".to_string(),
                message: "At least one chain configuration is required".to_string(),
            });
        }
        
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

impl ConfigValidator for DatabaseConfig {
    fn validate(&self) -> ValidationResult {
        let mut errors = Vec::new();
        
        match self.db_type.as_str() {
            "postgres" => {
                if self.postgres_url.is_none() {
                    errors.push(ValidationError {
                        field: "database.postgres_url".to_string(),
                        message: "PostgreSQL URL is required when using postgres database".to_string(),
                    });
                }
            }
            "rocks" => {
                if self.rocks_path.is_none() {
                    errors.push(ValidationError {
                        field: "database.rocks_path".to_string(),
                        message: "RocksDB path is required when using rocks database".to_string(),
                    });
                }
            }
            _ => {
                errors.push(ValidationError {
                    field: "database.db_type".to_string(),
                    message: format!("Invalid database type '{}'. Supported types: postgres, rocks", self.db_type),
                });
            }
        }
        
        if let Some(max_conn) = self.max_connections {
            if max_conn == 0 {
                errors.push(ValidationError {
                    field: "database.max_connections".to_string(),
                    message: "Maximum connections must be greater than 0".to_string(),
                });
            }
        }
        
        if let Some(timeout) = self.connection_timeout {
            if timeout == 0 {
                errors.push(ValidationError {
                    field: "database.connection_timeout".to_string(),
                    message: "Connection timeout must be greater than 0".to_string(),
                });
            }
        }
        
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

impl ConfigValidator for ApiConfig {
    fn validate(&self) -> ValidationResult {
        let mut errors = Vec::new();
        
        // Validate port range
        if self.port == 0 {
            errors.push(ValidationError {
                field: "api.port".to_string(),
                message: "Port must be greater than 0".to_string(),
            });
        }
        
        // Validate host
        if self.host.is_empty() {
            errors.push(ValidationError {
                field: "api.host".to_string(),
                message: "Host cannot be empty".to_string(),
            });
        }
        
        // Validate request timeout
        if self.request_timeout == 0 {
            errors.push(ValidationError {
                field: "api.request_timeout".to_string(),
                message: "Request timeout must be greater than 0".to_string(),
            });
        }
        
        // Validate authentication settings
        if self.auth_enabled && self.jwt_secret.is_none() && self.api_key.is_none() {
            errors.push(ValidationError {
                field: "api".to_string(),
                message: "Either jwt_secret or api_key must be provided when auth_enabled is true".to_string(),
            });
        }
        
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

impl ConfigValidator for ChainConfig {
    fn validate(&self) -> ValidationResult {
        let mut errors = Vec::new();
        
        // Validate chain ID
        if self.chain_id.is_empty() {
            errors.push(ValidationError {
                field: "chain_id".to_string(),
                message: "Chain ID cannot be empty".to_string(),
            });
        }
        
        // Validate name
        if self.name.is_empty() {
            errors.push(ValidationError {
                field: "name".to_string(),
                message: "Chain name cannot be empty".to_string(),
            });
        }
        
        // Validate RPC URL
        if self.rpc_url.is_empty() {
            errors.push(ValidationError {
                field: "rpc_url".to_string(),
                message: "RPC URL cannot be empty".to_string(),
            });
        } else if !self.rpc_url.starts_with("http://") && !self.rpc_url.starts_with("https://") {
            errors.push(ValidationError {
                field: "rpc_url".to_string(),
                message: "RPC URL must start with http:// or https://".to_string(),
            });
        }
        
        // Validate WebSocket URL if provided
        if let Some(ws_url) = &self.ws_url {
            if !ws_url.starts_with("ws://") && !ws_url.starts_with("wss://") {
                errors.push(ValidationError {
                    field: "ws_url".to_string(),
                    message: "WebSocket URL must start with ws:// or wss://".to_string(),
                });
            }
        }
        
        // Validate confirmations
        if self.confirmations == 0 {
            errors.push(ValidationError {
                field: "confirmations".to_string(),
                message: "Confirmations must be greater than 0".to_string(),
            });
        }
        
        // Validate poll interval
        if self.poll_interval_ms == 0 {
            errors.push(ValidationError {
                field: "poll_interval_ms".to_string(),
                message: "Poll interval must be greater than 0".to_string(),
            });
        }
        
        // Validate batch size
        if self.batch_size == 0 {
            errors.push(ValidationError {
                field: "batch_size".to_string(),
                message: "Batch size must be greater than 0".to_string(),
            });
        }
        
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

impl ConfigValidator for LogConfig {
    fn validate(&self) -> ValidationResult {
        let mut errors = Vec::new();
        
        // Validate log level
        let valid_levels = ["trace", "debug", "info", "warn", "error"];
        if !valid_levels.contains(&self.level.as_str()) {
            errors.push(ValidationError {
                field: "logging.level".to_string(),
                message: format!("Invalid log level '{}'. Valid levels: {}", self.level, valid_levels.join(", ")),
            });
        }
        
        // Validate log format
        let valid_formats = ["json", "pretty", "compact"];
        if !valid_formats.contains(&self.format.as_str()) {
            errors.push(ValidationError {
                field: "logging.format".to_string(),
                message: format!("Invalid log format '{}'. Valid formats: {}", self.format, valid_formats.join(", ")),
            });
        }
        
        // Validate file configuration
        if self.file_enabled {
            if self.file_path.is_none() {
                errors.push(ValidationError {
                    field: "logging.file_path".to_string(),
                    message: "File path is required when file logging is enabled".to_string(),
                });
            }
            
            if let Some(max_size) = self.max_file_size_mb {
                if max_size == 0 {
                    errors.push(ValidationError {
                        field: "logging.max_file_size_mb".to_string(),
                        message: "Maximum file size must be greater than 0".to_string(),
                    });
                }
            }
            
            if let Some(max_files) = self.max_files {
                if max_files == 0 {
                    errors.push(ValidationError {
                        field: "logging.max_files".to_string(),
                        message: "Maximum number of files must be greater than 0".to_string(),
                    });
                }
            }
        }
        
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

impl ConfigValidator for MonitoringConfig {
    fn validate(&self) -> ValidationResult {
        let mut errors = Vec::new();
        
        // Validate metrics port
        if self.metrics_port == 0 {
            errors.push(ValidationError {
                field: "monitoring.metrics_port".to_string(),
                message: "Metrics port must be greater than 0".to_string(),
            });
        }
        
        // Validate metrics host
        if self.metrics_host.is_empty() {
            errors.push(ValidationError {
                field: "monitoring.metrics_host".to_string(),
                message: "Metrics host cannot be empty".to_string(),
            });
        }
        
        // Validate health check path
        if self.health_check_enabled && self.health_check_path.is_empty() {
            errors.push(ValidationError {
                field: "monitoring.health_check_path".to_string(),
                message: "Health check path cannot be empty when health check is enabled".to_string(),
            });
        }
        
        // Validate alert thresholds
        for (metric, threshold) in &self.alert_thresholds {
            if *threshold < 0.0 || *threshold > 100.0 {
                errors.push(ValidationError {
                    field: format!("monitoring.alert_thresholds.{}", metric),
                    message: "Alert threshold must be between 0.0 and 100.0".to_string(),
                });
            }
        }
        
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

/// Configuration manager for loading, validating, and managing configurations
pub struct ConfigManager {
    config: AlmanacConfig,
    #[allow(dead_code)]
    config_path: PathBuf,
}

impl ConfigManager {
    /// Create a new configuration manager with the default configuration
    pub fn new() -> Self {
        Self {
            config: AlmanacConfig::default(),
            config_path: PathBuf::from("almanac.toml"),
        }
    }
    
    /// Load configuration from a file
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();
        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read configuration file: {}", path.display()))?;
        
        let config = match path.extension().and_then(|s| s.to_str()) {
            Some("toml") => toml::from_str(&content)
                .with_context(|| format!("Failed to parse TOML configuration file: {}", path.display()))?,
            Some("json") => serde_json::from_str(&content)
                .with_context(|| format!("Failed to parse JSON configuration file: {}", path.display()))?,
            Some("yaml") | Some("yml") => serde_yaml::from_str(&content)
                .with_context(|| format!("Failed to parse YAML configuration file: {}", path.display()))?,
            _ => return Err(anyhow::anyhow!("Unsupported configuration file format. Supported formats: .toml, .json, .yaml, .yml")),
        };
        
        let mut manager = Self {
            config,
            config_path: path.to_path_buf(),
        };
        
        // Apply environment overrides
        manager.apply_environment_overrides()?;
        
        Ok(manager)
    }
    
    /// Load configuration for a specific environment
    pub fn load_for_environment<P: AsRef<Path>>(base_path: P, environment: Environment) -> Result<Self> {
        let base_path = base_path.as_ref();
        
        // Try environment-specific file first
        let env_file = base_path.with_file_name(
            format!("almanac.{}.toml", environment)
        );
        
        if env_file.exists() {
            return Self::load_from_file(env_file);
        }
        
        // Fall back to base configuration and set environment
        let config = AlmanacConfig { 
            environment: environment.clone(), 
            ..Default::default() 
        };
        
        let mut manager = Self {
            config,
            config_path: base_path.to_path_buf(),
        };
        
        manager.apply_environment_overrides()?;
        
        Ok(manager)
    }
    
    /// Apply environment variable overrides
    pub fn apply_environment_overrides(&mut self) -> Result<()> {
        // Database overrides
        if let Ok(db_type) = env::var("ALMANAC_DB_TYPE") {
            self.config.database.db_type = db_type;
        }
        if let Ok(postgres_url) = env::var("ALMANAC_POSTGRES_URL") {
            self.config.database.postgres_url = Some(postgres_url);
        }
        if let Ok(rocks_path) = env::var("ALMANAC_ROCKS_PATH") {
            self.config.database.rocks_path = Some(rocks_path);
        }
        
        // API overrides
        if let Ok(api_host) = env::var("ALMANAC_API_HOST") {
            self.config.api.host = api_host;
        }
        if let Ok(api_port) = env::var("ALMANAC_API_PORT") {
            self.config.api.port = api_port.parse()
                .with_context(|| "Invalid ALMANAC_API_PORT value")?;
        }
        if let Ok(jwt_secret) = env::var("ALMANAC_JWT_SECRET") {
            self.config.api.jwt_secret = Some(jwt_secret);
        }
        if let Ok(api_key) = env::var("ALMANAC_API_KEY") {
            self.config.api.api_key = Some(api_key);
        }
        
        // Log level override
        if let Ok(log_level) = env::var("ALMANAC_LOG_LEVEL") {
            self.config.logging.level = log_level;
        }
        
        // Environment override
        if let Ok(env_str) = env::var("ALMANAC_ENVIRONMENT") {
            self.config.environment = match env_str.to_lowercase().as_str() {
                "development" | "dev" => Environment::Development,
                "staging" => Environment::Staging,
                "production" | "prod" => Environment::Production,
                "test" => Environment::Test,
                _ => return Err(anyhow::anyhow!("Invalid ALMANAC_ENVIRONMENT value: {}", env_str)),
            };
        }
        
        Ok(())
    }
    
    /// Validate the current configuration
    pub fn validate(&self) -> ValidationResult {
        self.config.validate()
    }
    
    /// Get the configuration
    pub fn config(&self) -> &AlmanacConfig {
        &self.config
    }
    
    /// Get a mutable reference to the configuration
    pub fn config_mut(&mut self) -> &mut AlmanacConfig {
        &mut self.config
    }
    
    /// Save the current configuration to a file
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let path = path.as_ref();
        
        let content = match path.extension().and_then(|s| s.to_str()) {
            Some("toml") => toml::to_string_pretty(&self.config)
                .with_context(|| "Failed to serialize configuration to TOML")?,
            Some("json") => serde_json::to_string_pretty(&self.config)
                .with_context(|| "Failed to serialize configuration to JSON")?,
            Some("yaml") | Some("yml") => serde_yaml::to_string(&self.config)
                .with_context(|| "Failed to serialize configuration to YAML")?,
            _ => return Err(anyhow::anyhow!("Unsupported configuration file format. Supported formats: .toml, .json, .yaml, .yml")),
        };
        
        fs::write(path, content)
            .with_context(|| format!("Failed to write configuration file: {}", path.display()))?;
        
        Ok(())
    }
    
    /// Generate a default configuration file
    pub fn generate_default_config<P: AsRef<Path>>(path: P, environment: Environment) -> Result<()> {
        let mut config = AlmanacConfig { 
            environment: environment.clone(), 
            ..Default::default() 
        };
        
        // Adjust defaults based on environment
        match environment {
            Environment::Development => {
                config.api.auth_enabled = false;
                config.logging.level = "debug".to_string();
                config.monitoring.metrics_enabled = false;
            }
            Environment::Test => {
                config.api.port = 8081;
                config.database.rocks_path = Some("./data/test_rocks".to_string());
                config.logging.level = "warn".to_string();
                config.monitoring.metrics_enabled = false;
            }
            Environment::Staging => {
                config.api.auth_enabled = true;
                config.logging.level = "info".to_string();
                config.logging.file_enabled = true;
            }
            Environment::Production => {
                config.api.auth_enabled = true;
                config.api.cors_origins = vec!["https://yourdomain.com".to_string()];
                config.logging.level = "warn".to_string();
                config.logging.file_enabled = true;
                config.logging.structured = true;
                config.database.max_connections = Some(50);
            }
        }
        
        let manager = ConfigManager {
            config,
            config_path: path.as_ref().to_path_buf(),
        };
        
        manager.save_to_file(path)?;
        Ok(())
    }
}

impl Default for ConfigManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;
    
    #[test]
    fn test_default_config_validation() {
        let config = AlmanacConfig::default();
        let result = config.validate();
        assert!(result.is_ok(), "Default configuration should be valid: {:?}", result);
    }
    
    #[test]
    fn test_database_config_validation() {
        // Test invalid database type
        let config = DatabaseConfig { 
            db_type: "invalid".to_string(), 
            ..Default::default() 
        };
        let result = config.validate();
        assert!(result.is_err());
        
        // Test missing postgres URL
        let config = DatabaseConfig { 
            db_type: "postgres".to_string(), 
            postgres_url: None, 
            ..Default::default() 
        };
        let result = config.validate();
        assert!(result.is_err());
        
        // Test valid postgres config
        let config = DatabaseConfig { 
            db_type: "postgres".to_string(), 
            postgres_url: Some("postgresql://localhost/test".to_string()), 
            ..Default::default() 
        };
        let result = config.validate();
        assert!(result.is_ok());
    }
    
    #[test]
    fn test_api_config_validation() {
        // Test invalid port
        let config = ApiConfig { 
            port: 0, 
            ..Default::default() 
        };
        let result = config.validate();
        assert!(result.is_err());
        
        // Test empty host
        let config = ApiConfig { 
            port: 8080, 
            host: "".to_string(), 
            ..Default::default() 
        };
        let result = config.validate();
        assert!(result.is_err());
        
        // Test auth enabled without credentials
        let config = ApiConfig { 
            host: "localhost".to_string(), 
            auth_enabled: true, 
            jwt_secret: None, 
            api_key: None, 
            ..Default::default() 
        };
        let result = config.validate();
        assert!(result.is_err());
    }
    
    #[test]
    fn test_chain_config_validation() {
        // Test empty chain ID
        let config = ChainConfig { 
            chain_id: "".to_string(), 
            ..Default::default() 
        };
        let result = config.validate();
        assert!(result.is_err());
        
        // Test invalid RPC URL
        let config = ChainConfig { 
            chain_id: "1".to_string(), 
            rpc_url: "invalid-url".to_string(), 
            ..Default::default() 
        };
        let result = config.validate();
        assert!(result.is_err());
        
        // Test valid config
        let config = ChainConfig { 
            chain_id: "1".to_string(), 
            rpc_url: "https://mainnet.infura.io/v3/test".to_string(), 
            ..Default::default() 
        };
        let result = config.validate();
        assert!(result.is_ok());
    }
    
    #[test]
    fn test_environment_overrides() {
        std::env::set_var("ALMANAC_API_PORT", "9090");
        std::env::set_var("ALMANAC_LOG_LEVEL", "debug");
        std::env::set_var("ALMANAC_ENVIRONMENT", "production");
        
        let mut manager = ConfigManager::new();
        manager.apply_environment_overrides().unwrap();
        
        assert_eq!(manager.config.api.port, 9090);
        assert_eq!(manager.config.logging.level, "debug");
        assert_eq!(manager.config.environment, Environment::Production);
        
        // Clean up
        std::env::remove_var("ALMANAC_API_PORT");
        std::env::remove_var("ALMANAC_LOG_LEVEL");
        std::env::remove_var("ALMANAC_ENVIRONMENT");
    }
    
    #[test]
    fn test_config_file_operations() {
        let config = AlmanacConfig::default();
        
        // Test TOML serialization
        let temp_file = NamedTempFile::new().unwrap();
        let toml_path = temp_file.path().with_extension("toml");
        
        let manager = ConfigManager {
            config: config.clone(),
            config_path: toml_path.clone(),
        };
        
        manager.save_to_file(&toml_path).unwrap();
        let loaded_manager = ConfigManager::load_from_file(&toml_path).unwrap();
        
        assert_eq!(loaded_manager.config.environment, config.environment);
        assert_eq!(loaded_manager.config.api.port, config.api.port);
    }
    
    #[test]
    fn test_environment_specific_configs() {
        let temp_dir = tempfile::tempdir().unwrap();
        let base_path = temp_dir.path().join("almanac.toml");
        
        // Generate configs for different environments
        ConfigManager::generate_default_config(&base_path, Environment::Development).unwrap();
        
        let dev_manager = ConfigManager::load_from_file(&base_path).unwrap();
        assert_eq!(dev_manager.config.environment, Environment::Development);
        assert!(!dev_manager.config.api.auth_enabled);
        assert_eq!(dev_manager.config.logging.level, "debug");
        
        // Test production config generation
        let prod_path = temp_dir.path().join("almanac.production.toml");
        ConfigManager::generate_default_config(&prod_path, Environment::Production).unwrap();
        
        let prod_manager = ConfigManager::load_from_file(&prod_path).unwrap();
        assert_eq!(prod_manager.config.environment, Environment::Production);
        assert!(prod_manager.config.api.auth_enabled);
        assert_eq!(prod_manager.config.logging.level, "warn");
    }
} 