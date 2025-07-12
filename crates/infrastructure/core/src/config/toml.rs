// TOML configuration handling with hot-reloading and enhanced validation
//
// This module provides TOML-specific configuration parsing, validation, and
// hot-reloading capabilities with detailed error reporting.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::SystemTime;
use tokio::sync::{RwLock, broadcast, watch};
use tokio::time::{interval, Duration};
use notify::{Watcher, RecursiveMode, Event, EventKind};
use serde::{Deserialize, Serialize};
use toml;

use crate::{Error, Result};
use super::{
    AlmanacConfig, Environment, ConfigValidationError, ConfigValidationResult,
    ConfigChangeEvent, ErrorSeverity, LogLevel, LogFormat,
};

// Re-export toml::Value for external use
pub use toml::Value;

/// Configuration manager with hot-reloading support
pub struct TomlConfigManager {
    /// Current configuration
    config: Arc<RwLock<AlmanacConfig>>,
    
    /// Configuration file path
    config_path: PathBuf,
    
    /// Environment
    environment: Environment,
    
    /// File watcher for hot-reloading
    _watcher: Option<notify::RecommendedWatcher>,
    
    /// Configuration change broadcaster
    change_broadcaster: broadcast::Sender<ConfigChangeEvent>,
    
    /// Shutdown signal for background tasks
    shutdown_tx: watch::Sender<bool>,
    
    /// Last modification time
    last_modified: Arc<RwLock<SystemTime>>,
    
    /// Hot-reload configuration
    hot_reload_config: HotReloadConfig,
}

/// Hot-reload configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HotReloadConfig {
    /// Enable hot-reloading
    pub enabled: bool,
    
    /// Sections that support hot-reloading
    pub reloadable_sections: Vec<String>,
    
    /// Validation before applying changes
    pub validate_before_reload: bool,
    
    /// Debounce delay in milliseconds
    pub debounce_ms: u64,
    
    /// Maximum reload attempts
    pub max_reload_attempts: u32,
}

impl Default for HotReloadConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            reloadable_sections: vec![
                "logging".to_string(),
                "monitoring".to_string(),
                "api.rate_limiting".to_string(),
                "chains.*.rate_limiting".to_string(),
            ],
            validate_before_reload: true,
            debounce_ms: 500,
            max_reload_attempts: 3,
        }
    }
}

/// Configuration validation trait with detailed error reporting
pub trait ConfigValidator {
    /// Validate the configuration
    fn validate(&self) -> ConfigValidationResult;
    
    /// Validate a specific section
    fn validate_section(&self, section: &str) -> ConfigValidationResult;
}

impl TomlConfigManager {
    /// Create a new configuration manager
    pub fn new(config_path: PathBuf, environment: Environment) -> Result<Self> {
        let (change_broadcaster, _) = broadcast::channel(100);
        let (shutdown_tx, _) = watch::channel(false);
        
        let manager = Self {
            config: Arc::new(RwLock::new(AlmanacConfig::default())),
            config_path,
            environment,
            _watcher: None,
            change_broadcaster,
            shutdown_tx,
            last_modified: Arc::new(RwLock::new(SystemTime::UNIX_EPOCH)),
            hot_reload_config: HotReloadConfig::default(),
        };
        
        Ok(manager)
    }
    
    /// Load configuration from file
    pub async fn load(&mut self) -> Result<()> {
        let content = fs::read_to_string(&self.config_path)
            .map_err(|e| Error::config(format!("Failed to read config file {}: {}", 
                self.config_path.display(), e)))?;
        
        let mut config: AlmanacConfig = toml::from_str(&content)
            .map_err(|e| Error::config(format!("Failed to parse TOML config: {}", e)))?;
        
        // Set environment
        config.environment = self.environment.clone();
        
        // Apply environment overrides
        self.apply_environment_overrides(&mut config).await?;
        
        // Validate configuration
        if let Err(errors) = config.validate() {
            return Err(Error::config(format!("Configuration validation failed: {:?}", errors)));
        }
        
        // Update last modified time
        if let Ok(metadata) = fs::metadata(&self.config_path) {
            if let Ok(modified) = metadata.modified() {
                *self.last_modified.write().await = modified;
            }
        }
        
        // Store configuration
        *self.config.write().await = config;
        
        Ok(())
    }
    
    /// Start hot-reloading if enabled
    pub async fn start_hot_reload(&mut self) -> Result<()> {
        if !self.hot_reload_config.enabled {
            return Ok(());
        }
        
        let config_path = self.config_path.clone();
        let change_broadcaster = self.change_broadcaster.clone();
        let config = self.config.clone();
        let last_modified = self.last_modified.clone();
        let hot_reload_config = self.hot_reload_config.clone();
        let mut shutdown_rx = self.shutdown_tx.subscribe();
        
        // Start file watcher
        let (tx, mut rx) = tokio::sync::mpsc::channel(100);
        
        let mut watcher = notify::recommended_watcher(move |res: notify::Result<Event>| {
            if let Ok(event) = res {
                if let EventKind::Modify(_) = event.kind {
                    if let Err(e) = tx.try_send(event) {
                        tracing::debug!("Failed to send file change event: {}", e);
                    }
                }
            }
        }).map_err(|e| Error::config(format!("Failed to create file watcher: {}", e)))?;
        
        watcher.watch(&config_path, RecursiveMode::NonRecursive)
            .map_err(|e| Error::config(format!("Failed to watch config file: {}", e)))?;
        
        self._watcher = Some(watcher);
        
        // Start background reload task
        tokio::spawn(async move {
            let mut debounce_timer = interval(Duration::from_millis(hot_reload_config.debounce_ms));
            let mut pending_reload = false;
            
            loop {
                tokio::select! {
                    // File change detected
                    Some(_event) = rx.recv() => {
                        pending_reload = true;
                    }
                    
                    // Debounce timer tick
                    _ = debounce_timer.tick() => {
                        if pending_reload {
                            pending_reload = false;
                            
                            // Perform hot reload
                            if let Err(e) = Self::perform_hot_reload(
                                &config_path,
                                &config,
                                &last_modified,
                                &hot_reload_config,
                                &change_broadcaster,
                            ).await {
                                tracing::error!("Hot reload failed: {}", e);
                            }
                        }
                    }
                    
                    // Shutdown signal
                    _ = shutdown_rx.changed() => {
                        if *shutdown_rx.borrow() {
                            break;
                        }
                    }
                }
            }
        });
        
        Ok(())
    }
    
    /// Perform hot reload
    async fn perform_hot_reload(
        config_path: &Path,
        config: &Arc<RwLock<AlmanacConfig>>,
        last_modified: &Arc<RwLock<SystemTime>>,
        hot_reload_config: &HotReloadConfig,
        change_broadcaster: &broadcast::Sender<ConfigChangeEvent>,
    ) -> Result<()> {
        // Check if file was actually modified
        let metadata = fs::metadata(config_path)
            .map_err(|e| Error::config(format!("Failed to get file metadata: {}", e)))?;
        
        let current_modified = metadata.modified()
            .map_err(|e| Error::config(format!("Failed to get modification time: {}", e)))?;
        
        let last_mod = *last_modified.read().await;
        if current_modified <= last_mod {
            return Ok(()); // File not actually modified
        }
        
        tracing::info!("Detected configuration file change, performing hot reload...");
        
        // Read new configuration
        let content = fs::read_to_string(config_path)
            .map_err(|e| Error::config(format!("Failed to read config file: {}", e)))?;
        
        let new_config: AlmanacConfig = toml::from_str(&content)
            .map_err(|e| Error::config(format!("Failed to parse TOML config: {}", e)))?;
        
        // Validate new configuration
        if hot_reload_config.validate_before_reload {
            if let Err(errors) = new_config.validate() {
                tracing::error!("Hot reload validation failed: {:?}", errors);
                return Err(Error::config("Configuration validation failed during hot reload"));
            }
        }
        
        // Get current configuration for comparison
        let previous_config = config.read().await.clone();
        
        // Determine changed sections
        let changed_sections = Self::detect_changed_sections(&previous_config, &new_config);
        
        // Check if changes are hot-reloadable
        let reloadable_changes: Vec<String> = changed_sections.iter()
            .filter(|section| Self::is_section_reloadable(section, &hot_reload_config.reloadable_sections))
            .cloned()
            .collect();
        
        if reloadable_changes.is_empty() {
            tracing::info!("No hot-reloadable changes detected");
            return Ok(());
        }
        
        // Apply hot-reloadable changes
        let mut current_config = config.write().await;
        Self::apply_hot_reload_changes(&mut current_config, &new_config, &reloadable_changes)?;
        
        // Update last modified time
        *last_modified.write().await = current_modified;
        
        // Broadcast change event
        let change_event = ConfigChangeEvent {
            changed_sections: reloadable_changes,
            new_config: current_config.clone(),
            previous_config,
            timestamp: SystemTime::now(),
        };
        
        if let Err(e) = change_broadcaster.send(change_event) {
            tracing::debug!("No receivers for config change event: {}", e);
        }
        
        tracing::info!("Hot reload completed successfully");
        Ok(())
    }
    
    /// Detect changed sections between configurations
    fn detect_changed_sections(old: &AlmanacConfig, new: &AlmanacConfig) -> Vec<String> {
        let mut changed = Vec::new();
        
        // Compare logging configuration
        if old.logging.level != new.logging.level || 
           old.logging.format != new.logging.format ||
           old.logging.console.enabled != new.logging.console.enabled ||
           old.logging.file.enabled != new.logging.file.enabled {
            changed.push("logging".to_string());
        }
        
        // Compare monitoring configuration
        if old.monitoring.metrics.enabled != new.monitoring.metrics.enabled ||
           old.monitoring.health.enabled != new.monitoring.health.enabled {
            changed.push("monitoring".to_string());
        }
        
        // Compare API rate limiting
        if old.api.rate_limiting.enabled != new.api.rate_limiting.enabled ||
           old.api.rate_limiting.requests_per_minute != new.api.rate_limiting.requests_per_minute {
            changed.push("api.rate_limiting".to_string());
        }
        
        // Compare chain configurations
        for (chain_id, old_chain) in &old.chains {
            if let Some(new_chain) = new.chains.get(chain_id) {
                if old_chain.rate_limiting.enabled != new_chain.rate_limiting.enabled ||
                   old_chain.rate_limiting.requests_per_second != new_chain.rate_limiting.requests_per_second {
                    changed.push(format!("chains.{}.rate_limiting", chain_id));
                }
            }
        }
        
        changed
    }
    
    /// Check if a section is hot-reloadable
    fn is_section_reloadable(section: &str, reloadable_sections: &[String]) -> bool {
        reloadable_sections.iter().any(|pattern| {
            if pattern.contains('*') {
                // Simple wildcard matching
                let pattern_parts: Vec<&str> = pattern.split('*').collect();
                if pattern_parts.len() == 2 {
                    section.starts_with(pattern_parts[0]) && section.ends_with(pattern_parts[1])
                } else {
                    false
                }
            } else {
                section == pattern
            }
        })
    }
    
    /// Apply hot-reload changes to configuration
    fn apply_hot_reload_changes(
        current: &mut AlmanacConfig,
        new: &AlmanacConfig,
        changed_sections: &[String],
    ) -> Result<()> {
        for section in changed_sections {
            match section.as_str() {
                "logging" => {
                    current.logging = new.logging.clone();
                }
                "monitoring" => {
                    current.monitoring = new.monitoring.clone();
                }
                "api.rate_limiting" => {
                    current.api.rate_limiting = new.api.rate_limiting.clone();
                }
                section if section.starts_with("chains.") && section.ends_with(".rate_limiting") => {
                    // Extract chain ID from section path
                    let parts: Vec<&str> = section.split('.').collect();
                    if parts.len() == 3 {
                        let chain_id = parts[1];
                        if let (Some(current_chain), Some(new_chain)) = (
                            current.chains.get_mut(chain_id),
                            new.chains.get(chain_id)
                        ) {
                            current_chain.rate_limiting = new_chain.rate_limiting.clone();
                        }
                    }
                }
                _ => {
                    tracing::debug!("Unknown section for hot reload: {}", section);
                }
            }
        }
        
        Ok(())
    }
    
    /// Apply environment variable overrides
    async fn apply_environment_overrides(&self, config: &mut AlmanacConfig) -> Result<()> {
        // Database overrides
        if let Ok(postgres_url) = env::var("ALMANAC_POSTGRES_URL") {
            if let Some(postgres) = &mut config.database.postgres {
                postgres.url = postgres_url;
            }
        }
        
        if let Ok(rocks_path) = env::var("ALMANAC_ROCKS_PATH") {
            if let Some(rocks) = &mut config.database.rocks {
                rocks.path = PathBuf::from(rocks_path);
            }
        }
        
        // API overrides
        if let Ok(api_host) = env::var("ALMANAC_API_HOST") {
            config.api.host = api_host;
        }
        
        if let Ok(api_port) = env::var("ALMANAC_API_PORT") {
            config.api.port = api_port.parse()
                .map_err(|e| Error::config(format!("Invalid ALMANAC_API_PORT: {}", e)))?;
        }
        
        if let Ok(jwt_secret) = env::var("ALMANAC_JWT_SECRET") {
            config.api.auth.jwt_secret = Some(jwt_secret);
        }
        
        // Logging overrides
        if let Ok(log_level) = env::var("ALMANAC_LOG_LEVEL") {
            config.logging.level = log_level.parse()
                .unwrap_or(LogLevel::Info);
        }
        
        // Environment override
        if let Ok(env_str) = env::var("ALMANAC_ENVIRONMENT") {
            config.environment = env_str.parse()
                .map_err(|e| Error::config(format!("Invalid ALMANAC_ENVIRONMENT: {}", e)))?;
        }
        
        Ok(())
    }
    
    /// Get current configuration
    pub async fn get_config(&self) -> AlmanacConfig {
        self.config.read().await.clone()
    }
    
    /// Subscribe to configuration changes
    pub fn subscribe_to_changes(&self) -> broadcast::Receiver<ConfigChangeEvent> {
        self.change_broadcaster.subscribe()
    }
    
    /// Validate current configuration
    pub async fn validate(&self) -> ConfigValidationResult {
        let config = self.config.read().await;
        config.validate()
    }
    
    /// Save current configuration to file
    pub async fn save(&self) -> Result<()> {
        let config = self.config.read().await;
        let content = toml::to_string_pretty(&*config)
            .map_err(|e| Error::config(format!("Failed to serialize config to TOML: {}", e)))?;
        
        fs::write(&self.config_path, content)
            .map_err(|e| Error::config(format!("Failed to write config file: {}", e)))?;
        
        Ok(())
    }
    
    /// Generate default configuration file
    pub fn generate_default_config(path: &Path, environment: Environment) -> Result<()> {
        let mut config = AlmanacConfig {
            environment: environment.clone(),
            ..Default::default()
        };
        
        // Adjust defaults based on environment
        match environment {
            Environment::Development => {
                config.api.auth.enabled = false;
                config.logging.level = LogLevel::Debug;
                config.monitoring.metrics.enabled = false;
            }
            Environment::Test => {
                config.api.port = 8081;
                config.logging.level = LogLevel::Warn;
                config.monitoring.metrics.enabled = false;
            }
            Environment::Staging => {
                config.api.auth.enabled = true;
                config.logging.level = LogLevel::Info;
                config.logging.file.enabled = true;
            }
            Environment::Production => {
                config.api.auth.enabled = true;
                config.api.cors.allowed_origins = vec!["https://yourdomain.com".to_string()];
                config.logging.level = LogLevel::Warn;
                config.logging.file.enabled = true;
                config.logging.format = LogFormat::Json;
                config.database.pool.max_connections = 50;
            }
        }
        
        let content = toml::to_string_pretty(&config)
            .map_err(|e| Error::config(format!("Failed to serialize default config: {}", e)))?;
        
        fs::write(path, content)
            .map_err(|e| Error::config(format!("Failed to write default config file: {}", e)))?;
        
        Ok(())
    }
    
    /// Shutdown the configuration manager
    pub async fn shutdown(&self) -> Result<()> {
        let _ = self.shutdown_tx.send(true);
        Ok(())
    }
}

impl Drop for TomlConfigManager {
    fn drop(&mut self) {
        let _ = self.shutdown_tx.send(true);
    }
}

/// Implement validation for AlmanacConfig
impl ConfigValidator for AlmanacConfig {
    fn validate(&self) -> ConfigValidationResult {
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
            errors.push(ConfigValidationError {
                field: "chains".to_string(),
                message: "At least one chain configuration is required".to_string(),
                suggestion: Some("Add at least one chain configuration".to_string()),
                severity: ErrorSeverity::Critical,
            });
        }
        
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
    
    fn validate_section(&self, section: &str) -> ConfigValidationResult {
        match section {
            "database" => self.database.validate(),
            "api" => self.api.validate(),
            "logging" => self.logging.validate(),
            "monitoring" => self.monitoring.validate(),
            _ => Ok(()),
        }
    }
}

// Implement validation for individual configuration sections
impl ConfigValidator for super::DatabaseConfig {
    fn validate(&self) -> ConfigValidationResult {
        let mut errors = Vec::new();
        
        match self.database_type {
            super::DatabaseType::Postgres => {
                if self.postgres.is_none() {
                    errors.push(ConfigValidationError {
                        field: "postgres".to_string(),
                        message: "PostgreSQL configuration is required when using postgres database".to_string(),
                        suggestion: Some("Add postgres configuration section".to_string()),
                        severity: ErrorSeverity::Critical,
                    });
                }
            }
            super::DatabaseType::Rocks => {
                if self.rocks.is_none() {
                    errors.push(ConfigValidationError {
                        field: "rocks".to_string(),
                        message: "RocksDB configuration is required when using rocks database".to_string(),
                        suggestion: Some("Add rocks configuration section".to_string()),
                        severity: ErrorSeverity::Critical,
                    });
                }
            }
            super::DatabaseType::Hybrid => {
                if self.postgres.is_none() || self.rocks.is_none() {
                    errors.push(ConfigValidationError {
                        field: "hybrid".to_string(),
                        message: "Both PostgreSQL and RocksDB configurations are required for hybrid mode".to_string(),
                        suggestion: Some("Add both postgres and rocks configuration sections".to_string()),
                        severity: ErrorSeverity::Critical,
                    });
                }
            }
        }
        
        // Validate connection pool
        if self.pool.max_connections == 0 {
            errors.push(ConfigValidationError {
                field: "pool.max_connections".to_string(),
                message: "Maximum connections must be greater than 0".to_string(),
                suggestion: Some("Set max_connections to a positive value".to_string()),
                severity: ErrorSeverity::Critical,
            });
        }
        
        if self.pool.min_connections > self.pool.max_connections {
            errors.push(ConfigValidationError {
                field: "pool.min_connections".to_string(),
                message: "Minimum connections cannot exceed maximum connections".to_string(),
                suggestion: Some("Reduce min_connections or increase max_connections".to_string()),
                severity: ErrorSeverity::Critical,
            });
        }
        
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
    
    fn validate_section(&self, _section: &str) -> ConfigValidationResult {
        self.validate()
    }
}

impl ConfigValidator for super::ApiConfig {
    fn validate(&self) -> ConfigValidationResult {
        let mut errors = Vec::new();
        
        // Validate port
        if self.port == 0 {
            errors.push(ConfigValidationError {
                field: "port".to_string(),
                message: "API port must be greater than 0".to_string(),
                suggestion: Some("Set port to a value between 1 and 65535".to_string()),
                severity: ErrorSeverity::Critical,
            });
        }
        
        // Validate TLS configuration
        if self.tls_enabled {
            if self.tls_cert_path.is_none() {
                errors.push(ConfigValidationError {
                    field: "tls_cert_path".to_string(),
                    message: "TLS certificate path is required when TLS is enabled".to_string(),
                    suggestion: Some("Provide path to TLS certificate file".to_string()),
                    severity: ErrorSeverity::Critical,
                });
            }
            
            if self.tls_key_path.is_none() {
                errors.push(ConfigValidationError {
                    field: "tls_key_path".to_string(),
                    message: "TLS key path is required when TLS is enabled".to_string(),
                    suggestion: Some("Provide path to TLS private key file".to_string()),
                    severity: ErrorSeverity::Critical,
                });
            }
        }
        
        // Validate authentication
        if self.auth.enabled && self.auth.jwt_secret.is_none() && self.auth.api_keys.is_empty() {
            errors.push(ConfigValidationError {
                field: "auth".to_string(),
                message: "JWT secret or API keys are required when authentication is enabled".to_string(),
                suggestion: Some("Set jwt_secret or add api_keys".to_string()),
                severity: ErrorSeverity::Critical,
            });
        }
        
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
    
    fn validate_section(&self, _section: &str) -> ConfigValidationResult {
        self.validate()
    }
}

impl ConfigValidator for super::ChainConfig {
    fn validate(&self) -> ConfigValidationResult {
        let mut errors = Vec::new();
        
        // Validate RPC URL
        if self.rpc.url.is_empty() {
            errors.push(ConfigValidationError {
                field: "rpc.url".to_string(),
                message: "RPC URL cannot be empty".to_string(),
                suggestion: Some("Provide a valid RPC endpoint URL".to_string()),
                severity: ErrorSeverity::Critical,
            });
        }
        
        // Validate event configuration
        if self.events.enabled && self.events.batch_size == 0 {
            errors.push(ConfigValidationError {
                field: "events.batch_size".to_string(),
                message: "Event batch size must be greater than 0".to_string(),
                suggestion: Some("Set batch_size to a positive value".to_string()),
                severity: ErrorSeverity::Critical,
            });
        }
        
        if self.events.poll_interval_ms == 0 {
            errors.push(ConfigValidationError {
                field: "events.poll_interval_ms".to_string(),
                message: "Poll interval must be greater than 0".to_string(),
                suggestion: Some("Set poll_interval_ms to a positive value".to_string()),
                severity: ErrorSeverity::Warning,
            });
        }
        
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
    
    fn validate_section(&self, _section: &str) -> ConfigValidationResult {
        self.validate()
    }
}

impl ConfigValidator for super::LogConfig {
    fn validate(&self) -> ConfigValidationResult {
        let mut errors = Vec::new();
        
        // Validate file configuration
        if self.file.enabled {
            if self.file.max_size_mb == 0 {
                errors.push(ConfigValidationError {
                    field: "file.max_size_mb".to_string(),
                    message: "Maximum file size must be greater than 0".to_string(),
                    suggestion: Some("Set max_size_mb to a positive value".to_string()),
                    severity: ErrorSeverity::Warning,
                });
            }
            
            if self.file.max_files == 0 {
                errors.push(ConfigValidationError {
                    field: "file.max_files".to_string(),
                    message: "Maximum files must be greater than 0".to_string(),
                    suggestion: Some("Set max_files to a positive value".to_string()),
                    severity: ErrorSeverity::Warning,
                });
            }
        }
        
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
    
    fn validate_section(&self, _section: &str) -> ConfigValidationResult {
        self.validate()
    }
}

impl ConfigValidator for super::MonitoringConfig {
    fn validate(&self) -> ConfigValidationResult {
        let mut errors = Vec::new();
        
        // Validate metrics configuration
        if self.metrics.enabled && self.metrics.port == 0 {
            errors.push(ConfigValidationError {
                field: "metrics.port".to_string(),
                message: "Metrics port must be greater than 0".to_string(),
                suggestion: Some("Set metrics port to a value between 1 and 65535".to_string()),
                severity: ErrorSeverity::Critical,
            });
        }
        
        // Validate health check configuration
        if self.health.enabled && self.health.interval_seconds == 0 {
            errors.push(ConfigValidationError {
                field: "health.interval_seconds".to_string(),
                message: "Health check interval must be greater than 0".to_string(),
                suggestion: Some("Set interval_seconds to a positive value".to_string()),
                severity: ErrorSeverity::Warning,
            });
        }
        
        // Validate tracing configuration
        if self.tracing.enabled && self.tracing.sampling_rate < 0.0 || self.tracing.sampling_rate > 1.0 {
            errors.push(ConfigValidationError {
                field: "tracing.sampling_rate".to_string(),
                message: "Sampling rate must be between 0.0 and 1.0".to_string(),
                suggestion: Some("Set sampling_rate to a value between 0.0 and 1.0".to_string()),
                severity: ErrorSeverity::Warning,
            });
        }
        
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
    
    fn validate_section(&self, _section: &str) -> ConfigValidationResult {
        self.validate()
    }
}

// Parse trait for log level from string
impl std::str::FromStr for LogLevel {
    type Err = Error;
    
    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "trace" => Ok(LogLevel::Trace),
            "debug" => Ok(LogLevel::Debug),
            "info" => Ok(LogLevel::Info),
            "warn" => Ok(LogLevel::Warn),
            "error" => Ok(LogLevel::Error),
            _ => Err(Error::config(format!("Invalid log level: {}", s))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;
    
    #[tokio::test]
    async fn test_config_loading() {
        let _config = AlmanacConfig::default();
        let temp_file = NamedTempFile::new().unwrap();
        let config_path = temp_file.path().to_path_buf();
        
        // Generate default config
        TomlConfigManager::generate_default_config(&config_path, Environment::Development).unwrap();
        
        // Load config
        let mut manager = TomlConfigManager::new(config_path, Environment::Development).unwrap();
        manager.load().await.unwrap();
        
        let loaded_config = manager.get_config().await;
        assert_eq!(loaded_config.environment, Environment::Development);
    }
    
    #[tokio::test]
    async fn test_config_validation() {
        let mut config = AlmanacConfig::default();
        
        // Valid config should pass
        assert!(config.validate().is_ok());
        
        // Invalid config should fail
        config.database.pool.max_connections = 0;
        assert!(config.validate().is_err());
    }
    
    #[tokio::test]
    async fn test_hot_reload() {
        let temp_file = NamedTempFile::new().unwrap();
        let config_path = temp_file.path().to_path_buf();
        
        // Generate initial config
        TomlConfigManager::generate_default_config(&config_path, Environment::Development).unwrap();
        
        // Create manager
        let mut manager = TomlConfigManager::new(config_path.clone(), Environment::Development).unwrap();
        manager.load().await.unwrap();
        
        // Test basic functionality without file watching
        let initial_config = manager.get_config().await;
        assert_eq!(initial_config.environment, Environment::Development);
        
        // Test manual reload after file change
        let mut new_config = initial_config.clone();
        new_config.api.port = 9999;
        
        let content = toml::to_string_pretty(&new_config).unwrap();
        fs::write(&config_path, content).unwrap();
        
        // Manually reload
        manager.load().await.unwrap();
        
        let reloaded_config = manager.get_config().await;
        assert_eq!(reloaded_config.api.port, 9999);
        
        manager.shutdown().await.unwrap();
    }
} 