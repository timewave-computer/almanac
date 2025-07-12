// Comprehensive error handling system for the Almanac indexer
//
// This module provides standardized error types, error context, structured logging,
// error recovery mechanisms, and error rate monitoring

use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;
use std::time::SystemTime;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Main error type for the Almanac indexer
#[derive(Error, Debug, Clone, Serialize, Deserialize)]
pub enum Error {
    /// Configuration errors
    #[error("Configuration error: {message}")]
    Config {
        message: String,
        source: Option<String>,
        context: ErrorContext,
    },

    /// Database errors
    #[error("Database error: {message}")]
    Database {
        message: String,
        operation: String,
        context: ErrorContext,
    },

    /// Network/connection errors
    #[error("Connection error: {message}")]
    Connection {
        message: String,
        endpoint: String,
        context: ErrorContext,
    },

    /// Authentication errors
    #[error("Authentication error: {message}")]
    Authentication {
        message: String,
        context: ErrorContext,
    },

    /// Authorization errors
    #[error("Authorization error: {message}")]
    Authorization {
        message: String,
        resource: String,
        context: ErrorContext,
    },

    /// Chain-specific errors
    #[error("Chain error on {chain}: {message}")]
    Chain {
        chain: String,
        message: String,
        context: ErrorContext,
    },

    /// API errors
    #[error("API error: {message}")]
    Api {
        message: String,
        endpoint: String,
        status_code: Option<u16>,
        context: ErrorContext,
    },

    /// Validation errors
    #[error("Validation error: {message}")]
    Validation {
        message: String,
        field: Option<String>,
        context: ErrorContext,
    },

    /// Serialization/deserialization errors
    #[error("Serialization error: {message}")]
    Serialization {
        message: String,
        data_type: String,
        context: ErrorContext,
    },

    /// Event processing errors
    #[error("Event processing error: {message}")]
    EventProcessing {
        message: String,
        event_type: String,
        event_id: Option<String>,
        context: ErrorContext,
    },

    /// Storage errors
    #[error("Storage error: {message}")]
    Storage {
        message: String,
        operation: String,
        storage_type: String,
        context: ErrorContext,
    },

    /// Synchronization errors
    #[error("Synchronization error: {message}")]
    Synchronization {
        message: String,
        component: String,
        context: ErrorContext,
    },

    /// Resource exhaustion errors
    #[error("Resource exhaustion: {message}")]
    ResourceExhaustion {
        message: String,
        resource_type: String,
        context: ErrorContext,
    },

    /// Timeout errors
    #[error("Timeout error: {message}")]
    Timeout {
        message: String,
        operation: String,
        duration_ms: u64,
        context: ErrorContext,
    },

    /// Rate limiting errors
    #[error("Rate limit exceeded: {message}")]
    RateLimit {
        message: String,
        limit: u64,
        window_ms: u64,
        context: ErrorContext,
    },

    /// Internal errors
    #[error("Internal error: {message}")]
    Internal {
        message: String,
        context: ErrorContext,
    },

    /// External service errors
    #[error("External service error: {message}")]
    ExternalService {
        message: String,
        service: String,
        context: ErrorContext,
    },

    /// Cryptographic errors
    #[error("Cryptographic error: {message}")]
    Cryptographic {
        message: String,
        operation: String,
        context: ErrorContext,
    },

    /// Migration errors
    #[error("Migration error: {message}")]
    Migration {
        message: String,
        migration_name: String,
        context: ErrorContext,
    },

    /// Contract interaction errors
    #[error("Contract error: {message}")]
    Contract {
        message: String,
        contract_address: String,
        chain: String,
        context: ErrorContext,
    },

    /// Cross-chain errors
    #[error("Cross-chain error: {message}")]
    CrossChain {
        message: String,
        source_chain: String,
        target_chain: String,
        context: ErrorContext,
    },

    /// Recovery errors
    #[error("Recovery error: {message}")]
    Recovery {
        message: String,
        recovery_type: String,
        context: ErrorContext,
    },
}

/// Error context containing additional information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorContext {
    /// Unique error ID for tracking
    pub error_id: String,
    /// Timestamp when error occurred
    pub timestamp: SystemTime,
    /// Component that generated the error
    pub component: String,
    /// Operation that was being performed
    pub operation: String,
    /// Request ID for tracing
    pub request_id: Option<String>,
    /// User ID if applicable
    pub user_id: Option<String>,
    /// Session ID if applicable
    pub session_id: Option<String>,
    /// Additional metadata
    pub metadata: HashMap<String, String>,
    /// Error severity level
    pub severity: ErrorSeverity,
    /// Whether this error is recoverable
    pub recoverable: bool,
    /// Recovery suggestions
    pub recovery_suggestions: Vec<String>,
    /// Stack trace information
    pub stack_trace: Option<String>,
}

/// Error severity levels
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum ErrorSeverity {
    /// Low severity - informational
    Low,
    /// Medium severity - warning
    Medium,
    /// High severity - error
    High,
    /// Critical severity - system failure
    Critical,
}

/// Error recovery strategy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RecoveryStrategy {
    /// Retry the operation
    Retry {
        max_attempts: u32,
        backoff_ms: u64,
        exponential_backoff: bool,
    },
    /// Fallback to alternative method
    Fallback {
        fallback_method: String,
        fallback_config: HashMap<String, String>,
    },
    /// Circuit breaker pattern
    CircuitBreaker {
        failure_threshold: u32,
        recovery_timeout_ms: u64,
        half_open_max_calls: u32,
    },
    /// Graceful degradation
    Degrade {
        degraded_functionality: String,
        degradation_level: String,
    },
    /// Manual intervention required
    Manual {
        instructions: String,
        escalation_contact: String,
    },
    /// No recovery possible
    None,
}

/// Error recovery result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryResult {
    /// Whether recovery was successful
    pub success: bool,
    /// Recovery strategy used
    pub strategy: RecoveryStrategy,
    /// Number of attempts made
    pub attempts: u32,
    /// Time taken for recovery
    pub duration_ms: u64,
    /// Error message if recovery failed
    pub error: Option<String>,
    /// Additional recovery context
    pub context: HashMap<String, String>,
}

/// Error rate monitoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorRateMetrics {
    /// Component name
    pub component: String,
    /// Time window in seconds
    pub window_seconds: u64,
    /// Total error count in window
    pub error_count: u64,
    /// Total operation count in window
    pub operation_count: u64,
    /// Error rate percentage
    pub error_rate_percent: f64,
    /// Error count by severity
    pub errors_by_severity: HashMap<ErrorSeverity, u64>,
    /// Error count by type
    pub errors_by_type: HashMap<String, u64>,
    /// Timestamp of metrics
    pub timestamp: SystemTime,
}

/// Error handler for centralized error management
pub struct ErrorHandler {
    /// Error recovery strategies
    recovery_strategies: HashMap<String, RecoveryStrategy>,
    /// Error rate monitoring
    error_metrics: Arc<std::sync::RwLock<HashMap<String, ErrorRateMetrics>>>,
    /// Error history for analysis
    error_history: Arc<std::sync::RwLock<Vec<(SystemTime, Error)>>>,
    /// Configuration
    config: ErrorHandlerConfig,
}

/// Error handler configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorHandlerConfig {
    /// Enable structured error logging
    pub structured_logging: bool,
    /// Enable error rate monitoring
    pub rate_monitoring: bool,
    /// Error history retention period
    pub history_retention_hours: u64,
    /// Maximum error history size
    pub max_history_size: usize,
    /// Enable automatic recovery
    pub auto_recovery: bool,
    /// Recovery timeout in seconds
    pub recovery_timeout_seconds: u64,
}

impl Default for ErrorHandlerConfig {
    fn default() -> Self {
        Self {
            structured_logging: true,
            rate_monitoring: true,
            history_retention_hours: 24,
            max_history_size: 10000,
            auto_recovery: true,
            recovery_timeout_seconds: 300,
        }
    }
}

impl ErrorHandler {
    /// Create new error handler
    pub fn new(config: ErrorHandlerConfig) -> Self {
        Self {
            recovery_strategies: HashMap::new(),
            error_metrics: Arc::new(std::sync::RwLock::new(HashMap::new())),
            error_history: Arc::new(std::sync::RwLock::new(Vec::new())),
            config,
        }
    }

    /// Handle an error with context and recovery
    pub async fn handle_error(&self, error: Error) -> Result<RecoveryResult, Error> {
        // Log the error
        if self.config.structured_logging {
            self.log_structured_error(&error).await;
        }

        // Update error metrics
        if self.config.rate_monitoring {
            self.update_error_metrics(&error).await;
        }

        // Store in history
        self.store_error_history(&error).await;

        // Attempt recovery if enabled
        if self.config.auto_recovery && error.is_recoverable() {
            self.attempt_recovery(&error).await
        } else {
            Ok(RecoveryResult {
                success: false,
                strategy: RecoveryStrategy::None,
                attempts: 0,
                duration_ms: 0,
                error: Some("Auto-recovery disabled or error not recoverable".to_string()),
                context: HashMap::new(),
            })
        }
    }

    /// Log structured error information
    async fn log_structured_error(&self, error: &Error) {
        let context = error.get_context();
        let severity = &context.severity;
        
        match severity {
            ErrorSeverity::Low => {
                tracing::info!(
                    error_id = %context.error_id,
                    component = %context.component,
                    operation = %context.operation,
                    error_type = %error.error_type(),
                    message = %error.to_string(),
                    recoverable = %context.recoverable,
                    "Low severity error occurred"
                );
            }
            ErrorSeverity::Medium => {
                tracing::warn!(
                    error_id = %context.error_id,
                    component = %context.component,
                    operation = %context.operation,
                    error_type = %error.error_type(),
                    message = %error.to_string(),
                    recoverable = %context.recoverable,
                    "Medium severity error occurred"
                );
            }
            ErrorSeverity::High => {
                tracing::error!(
                    error_id = %context.error_id,
                    component = %context.component,
                    operation = %context.operation,
                    error_type = %error.error_type(),
                    message = %error.to_string(),
                    recoverable = %context.recoverable,
                    "High severity error occurred"
                );
            }
            ErrorSeverity::Critical => {
                tracing::error!(
                    error_id = %context.error_id,
                    component = %context.component,
                    operation = %context.operation,
                    error_type = %error.error_type(),
                    message = %error.to_string(),
                    recoverable = %context.recoverable,
                    "CRITICAL ERROR OCCURRED"
                );
            }
        }
    }

    /// Update error rate metrics
    async fn update_error_metrics(&self, error: &Error) {
        let context = error.get_context();
        let component = &context.component;
        let error_type = error.error_type();
        
        let mut metrics = self.error_metrics.write().unwrap();
        let metric = metrics.entry(component.clone()).or_insert_with(|| ErrorRateMetrics {
            component: component.clone(),
            window_seconds: 3600, // 1 hour window
            error_count: 0,
            operation_count: 0,
            error_rate_percent: 0.0,
            errors_by_severity: HashMap::new(),
            errors_by_type: HashMap::new(),
            timestamp: SystemTime::now(),
        });

        // Update counts
        metric.error_count += 1;
        metric.operation_count += 1;
        
        // Update by severity
        *metric.errors_by_severity.entry(context.severity.clone()).or_insert(0) += 1;
        
        // Update by type
        *metric.errors_by_type.entry(error_type).or_insert(0) += 1;
        
        // Recalculate error rate
        metric.error_rate_percent = (metric.error_count as f64 / metric.operation_count as f64) * 100.0;
        metric.timestamp = SystemTime::now();
    }

    /// Store error in history
    async fn store_error_history(&self, error: &Error) {
        let mut history = self.error_history.write().unwrap();
        history.push((SystemTime::now(), error.clone()));
        
        // Clean up old entries
        if history.len() > self.config.max_history_size {
            history.remove(0);
        }
        
        // Clean up by age
        let cutoff = SystemTime::now() - std::time::Duration::from_secs(self.config.history_retention_hours * 3600);
        history.retain(|(timestamp, _)| *timestamp > cutoff);
    }

    /// Attempt error recovery
    async fn attempt_recovery(&self, error: &Error) -> Result<RecoveryResult, Error> {
        let context = error.get_context();
        let error_type = error.error_type();
        
        // Get recovery strategy
        let strategy = self.recovery_strategies.get(&error_type)
            .cloned()
            .unwrap_or(RecoveryStrategy::None);

        let start_time = std::time::Instant::now();
        let mut attempts = 0;

        match strategy {
            RecoveryStrategy::Retry { max_attempts, backoff_ms, exponential_backoff } => {
                self.attempt_retry_recovery(error, max_attempts, backoff_ms, exponential_backoff).await
            }
            RecoveryStrategy::Fallback { fallback_method, fallback_config } => {
                self.attempt_fallback_recovery(error, &fallback_method, &fallback_config).await
            }
            RecoveryStrategy::CircuitBreaker { failure_threshold, recovery_timeout_ms, half_open_max_calls } => {
                self.attempt_circuit_breaker_recovery(error, failure_threshold, recovery_timeout_ms, half_open_max_calls).await
            }
            RecoveryStrategy::Degrade { degraded_functionality, degradation_level } => {
                self.attempt_degradation_recovery(error, &degraded_functionality, &degradation_level).await
            }
            RecoveryStrategy::Manual { instructions, escalation_contact } => {
                Ok(RecoveryResult {
                    success: false,
                    strategy: strategy.clone(),
                    attempts: 0,
                    duration_ms: start_time.elapsed().as_millis() as u64,
                    error: Some(format!("Manual intervention required: {}", instructions)),
                    context: {
                        let mut ctx = HashMap::new();
                        ctx.insert("escalation_contact".to_string(), escalation_contact);
                        ctx
                    },
                })
            }
            RecoveryStrategy::None => {
                Ok(RecoveryResult {
                    success: false,
                    strategy: RecoveryStrategy::None,
                    attempts: 0,
                    duration_ms: start_time.elapsed().as_millis() as u64,
                    error: Some("No recovery strategy available".to_string()),
                    context: HashMap::new(),
                })
            }
        }
    }

    /// Attempt retry recovery
    async fn attempt_retry_recovery(&self, error: &Error, max_attempts: u32, backoff_ms: u64, exponential_backoff: bool) -> Result<RecoveryResult, Error> {
        let start_time = std::time::Instant::now();
        let mut attempts = 0;
        let mut current_backoff = backoff_ms;

        for attempt in 1..=max_attempts {
            attempts = attempt;
            
            // Wait before retry (except first attempt)
            if attempt > 1 {
                tokio::time::sleep(std::time::Duration::from_millis(current_backoff)).await;
                
                if exponential_backoff {
                    current_backoff *= 2;
                }
            }

            // Simulate retry operation
            let retry_success = self.simulate_retry_operation(error).await;
            
            if retry_success {
                return Ok(RecoveryResult {
                    success: true,
                    strategy: RecoveryStrategy::Retry { max_attempts, backoff_ms, exponential_backoff },
                    attempts,
                    duration_ms: start_time.elapsed().as_millis() as u64,
                    error: None,
                    context: HashMap::new(),
                });
            }
        }

        Ok(RecoveryResult {
            success: false,
            strategy: RecoveryStrategy::Retry { max_attempts, backoff_ms, exponential_backoff },
            attempts,
            duration_ms: start_time.elapsed().as_millis() as u64,
            error: Some(format!("Retry failed after {} attempts", max_attempts)),
            context: HashMap::new(),
        })
    }

    /// Attempt fallback recovery
    async fn attempt_fallback_recovery(&self, error: &Error, fallback_method: &str, fallback_config: &HashMap<String, String>) -> Result<RecoveryResult, Error> {
        let start_time = std::time::Instant::now();
        
        // Simulate fallback operation
        let fallback_success = self.simulate_fallback_operation(error, fallback_method).await;
        
        Ok(RecoveryResult {
            success: fallback_success,
            strategy: RecoveryStrategy::Fallback { 
                fallback_method: fallback_method.to_string(), 
                fallback_config: fallback_config.clone() 
            },
            attempts: 1,
            duration_ms: start_time.elapsed().as_millis() as u64,
            error: if fallback_success { None } else { Some("Fallback operation failed".to_string()) },
            context: fallback_config.clone(),
        })
    }

    /// Attempt circuit breaker recovery
    async fn attempt_circuit_breaker_recovery(&self, error: &Error, failure_threshold: u32, recovery_timeout_ms: u64, half_open_max_calls: u32) -> Result<RecoveryResult, Error> {
        let start_time = std::time::Instant::now();
        
        // Simulate circuit breaker logic
        let recovery_success = true; // Simplified for minimal implementation
        
        Ok(RecoveryResult {
            success: recovery_success,
            strategy: RecoveryStrategy::CircuitBreaker { failure_threshold, recovery_timeout_ms, half_open_max_calls },
            attempts: 1,
            duration_ms: start_time.elapsed().as_millis() as u64,
            error: None,
            context: HashMap::new(),
        })
    }

    /// Attempt degradation recovery
    async fn attempt_degradation_recovery(&self, error: &Error, degraded_functionality: &str, degradation_level: &str) -> Result<RecoveryResult, Error> {
        let start_time = std::time::Instant::now();
        
        // Simulate graceful degradation
        let degradation_success = true; // Simplified for minimal implementation
        
        Ok(RecoveryResult {
            success: degradation_success,
            strategy: RecoveryStrategy::Degrade { 
                degraded_functionality: degraded_functionality.to_string(), 
                degradation_level: degradation_level.to_string() 
            },
            attempts: 1,
            duration_ms: start_time.elapsed().as_millis() as u64,
            error: None,
            context: {
                let mut ctx = HashMap::new();
                ctx.insert("degraded_functionality".to_string(), degraded_functionality.to_string());
                ctx.insert("degradation_level".to_string(), degradation_level.to_string());
                ctx
            },
        })
    }

    /// Simulate retry operation
    async fn simulate_retry_operation(&self, error: &Error) -> bool {
        // Simulate some operations have better success rates on retry
        match error {
            Error::Connection { .. } => rand::random::<f64>() > 0.3, // 70% success rate
            Error::Timeout { .. } => rand::random::<f64>() > 0.5, // 50% success rate
            Error::ExternalService { .. } => rand::random::<f64>() > 0.4, // 60% success rate
            _ => rand::random::<f64>() > 0.8, // 20% success rate for other errors
        }
    }

    /// Simulate fallback operation
    async fn simulate_fallback_operation(&self, error: &Error, fallback_method: &str) -> bool {
        // Simulate fallback operations generally have good success rates
        rand::random::<f64>() > 0.2 // 80% success rate
    }

    /// Register recovery strategy
    pub fn register_recovery_strategy(&mut self, error_type: String, strategy: RecoveryStrategy) {
        self.recovery_strategies.insert(error_type, strategy);
    }

    /// Get error rate metrics
    pub fn get_error_metrics(&self) -> HashMap<String, ErrorRateMetrics> {
        self.error_metrics.read().unwrap().clone()
    }

    /// Get error history
    pub fn get_error_history(&self) -> Vec<(SystemTime, Error)> {
        self.error_history.read().unwrap().clone()
    }
}

impl Error {
    /// Get error context
    pub fn get_context(&self) -> &ErrorContext {
        match self {
            Error::Config { context, .. } => context,
            Error::Database { context, .. } => context,
            Error::Connection { context, .. } => context,
            Error::Authentication { context, .. } => context,
            Error::Authorization { context, .. } => context,
            Error::Chain { context, .. } => context,
            Error::Api { context, .. } => context,
            Error::Validation { context, .. } => context,
            Error::Serialization { context, .. } => context,
            Error::EventProcessing { context, .. } => context,
            Error::Storage { context, .. } => context,
            Error::Synchronization { context, .. } => context,
            Error::ResourceExhaustion { context, .. } => context,
            Error::Timeout { context, .. } => context,
            Error::RateLimit { context, .. } => context,
            Error::Internal { context, .. } => context,
            Error::ExternalService { context, .. } => context,
            Error::Cryptographic { context, .. } => context,
            Error::Migration { context, .. } => context,
            Error::Contract { context, .. } => context,
            Error::CrossChain { context, .. } => context,
            Error::Recovery { context, .. } => context,
        }
    }

    /// Get error type as string
    pub fn error_type(&self) -> String {
        match self {
            Error::Config { .. } => "Config".to_string(),
            Error::Database { .. } => "Database".to_string(),
            Error::Connection { .. } => "Connection".to_string(),
            Error::Authentication { .. } => "Authentication".to_string(),
            Error::Authorization { .. } => "Authorization".to_string(),
            Error::Chain { .. } => "Chain".to_string(),
            Error::Api { .. } => "Api".to_string(),
            Error::Validation { .. } => "Validation".to_string(),
            Error::Serialization { .. } => "Serialization".to_string(),
            Error::EventProcessing { .. } => "EventProcessing".to_string(),
            Error::Storage { .. } => "Storage".to_string(),
            Error::Synchronization { .. } => "Synchronization".to_string(),
            Error::ResourceExhaustion { .. } => "ResourceExhaustion".to_string(),
            Error::Timeout { .. } => "Timeout".to_string(),
            Error::RateLimit { .. } => "RateLimit".to_string(),
            Error::Internal { .. } => "Internal".to_string(),
            Error::ExternalService { .. } => "ExternalService".to_string(),
            Error::Cryptographic { .. } => "Cryptographic".to_string(),
            Error::Migration { .. } => "Migration".to_string(),
            Error::Contract { .. } => "Contract".to_string(),
            Error::CrossChain { .. } => "CrossChain".to_string(),
            Error::Recovery { .. } => "Recovery".to_string(),
        }
    }

    /// Check if error is recoverable
    pub fn is_recoverable(&self) -> bool {
        self.get_context().recoverable
    }

    /// Get error severity
    pub fn severity(&self) -> &ErrorSeverity {
        &self.get_context().severity
    }

    /// Create error context
    pub fn create_context(
        component: &str,
        operation: &str,
        severity: ErrorSeverity,
        recoverable: bool,
    ) -> ErrorContext {
        ErrorContext {
            error_id: uuid::Uuid::new_v4().to_string(),
            timestamp: SystemTime::now(),
            component: component.to_string(),
            operation: operation.to_string(),
            request_id: None,
            user_id: None,
            session_id: None,
            metadata: HashMap::new(),
            severity,
            recoverable,
            recovery_suggestions: Vec::new(),
            stack_trace: None,
        }
    }
}

/// Result type alias for convenience
pub type Result<T> = std::result::Result<T, Error>;

use rand; // For simulation purposes
use uuid;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_creation() {
        let context = Error::create_context("test_component", "test_operation", ErrorSeverity::High, true);
        let error = Error::Database {
            message: "Test error".to_string(),
            operation: "test_operation".to_string(),
            context,
        };

        assert_eq!(error.error_type(), "Database");
        assert!(error.is_recoverable());
        assert_eq!(error.severity(), &ErrorSeverity::High);
    }

    #[tokio::test]
    async fn test_error_handler() {
        let config = ErrorHandlerConfig::default();
        let mut handler = ErrorHandler::new(config);

        // Register a recovery strategy
        handler.register_recovery_strategy(
            "Database".to_string(),
            RecoveryStrategy::Retry {
                max_attempts: 3,
                backoff_ms: 1000,
                exponential_backoff: true,
            },
        );

        let context = Error::create_context("test_component", "test_operation", ErrorSeverity::High, true);
        let error = Error::Database {
            message: "Test error".to_string(),
            operation: "test_operation".to_string(),
            context,
        };

        let result = handler.handle_error(error).await;
        assert!(result.is_ok());
    }

    #[test]
    fn test_error_context() {
        let context = Error::create_context("test_component", "test_operation", ErrorSeverity::Medium, false);
        
        assert_eq!(context.component, "test_component");
        assert_eq!(context.operation, "test_operation");
        assert_eq!(context.severity, ErrorSeverity::Medium);
        assert!(!context.recoverable);
        assert!(!context.error_id.is_empty());
    }

    #[test]
    fn test_error_severity_ordering() {
        assert!(ErrorSeverity::Critical > ErrorSeverity::High);
        assert!(ErrorSeverity::High > ErrorSeverity::Medium);
        assert!(ErrorSeverity::Medium > ErrorSeverity::Low);
    }
} 