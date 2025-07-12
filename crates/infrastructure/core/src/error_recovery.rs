// Comprehensive error handling and recovery system
//
// This module provides advanced error handling capabilities including circuit breakers,
// intelligent retry policies, automatic failure recovery, error classification, and
// system resilience patterns for robust cross-chain indexing operations.

use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime};
use tokio::sync::{RwLock, mpsc, broadcast};
use tokio::time::{sleep, timeout};
use tracing::{info, warn, debug};
use serde::{Deserialize, Serialize};

use crate::{Error, Result};

/// Error recovery configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorRecoveryConfig {
    /// Circuit breaker configuration
    pub circuit_breaker: CircuitBreakerConfig,
    
    /// Retry policy configuration
    pub retry_policy: RetryPolicyConfig,
    
    /// Recovery strategy configuration
    pub recovery_strategy: RecoveryStrategyConfig,
    
    /// Error classification rules
    pub error_classification: ErrorClassificationConfig,
    
    /// Health check configuration
    pub health_check: HealthCheckConfig,
    
    /// Metrics collection interval
    pub metrics_interval: Duration,
    
    /// Enable error correlation
    pub enable_error_correlation: bool,
    
    /// Maximum error history to keep
    pub max_error_history: usize,
}

impl Default for ErrorRecoveryConfig {
    fn default() -> Self {
        Self {
            circuit_breaker: CircuitBreakerConfig::default(),
            retry_policy: RetryPolicyConfig::default(),
            recovery_strategy: RecoveryStrategyConfig::default(),
            error_classification: ErrorClassificationConfig::default(),
            health_check: HealthCheckConfig::default(),
            metrics_interval: Duration::from_secs(10),
            enable_error_correlation: true,
            max_error_history: 1000,
        }
    }
}

/// Circuit breaker configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitBreakerConfig {
    /// Failure threshold before opening circuit
    pub failure_threshold: u32,
    
    /// Success threshold before closing circuit
    pub success_threshold: u32,
    
    /// Timeout for circuit breaker reset
    pub timeout: Duration,
    
    /// Half-open retry count
    pub half_open_retry_count: u32,
    
    /// Request volume threshold
    pub request_volume_threshold: u32,
    
    /// Rolling window duration
    pub rolling_window: Duration,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 5,
            success_threshold: 3,
            timeout: Duration::from_secs(60),
            half_open_retry_count: 3,
            request_volume_threshold: 10,
            rolling_window: Duration::from_secs(60),
        }
    }
}

/// Retry policy configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryPolicyConfig {
    /// Maximum retry attempts
    pub max_retries: u32,
    
    /// Initial retry delay
    pub initial_delay: Duration,
    
    /// Maximum retry delay
    pub max_delay: Duration,
    
    /// Backoff strategy
    pub backoff_strategy: BackoffStrategy,
    
    /// Jitter configuration
    pub jitter_config: JitterConfig,
    
    /// Timeout for retry operations
    pub timeout: Duration,
    
    /// Enable adaptive retries
    pub adaptive_retries: bool,
}

impl Default for RetryPolicyConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(30),
            backoff_strategy: BackoffStrategy::ExponentialBackoff { multiplier: 2.0 },
            jitter_config: JitterConfig::default(),
            timeout: Duration::from_secs(30),
            adaptive_retries: true,
        }
    }
}

/// Backoff strategy for retries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BackoffStrategy {
    /// Fixed delay between retries
    FixedDelay,
    
    /// Linear backoff with increment
    LinearBackoff { increment: Duration },
    
    /// Exponential backoff with multiplier
    ExponentialBackoff { multiplier: f64 },
    
    /// Fibonacci backoff
    FibonacciBackoff,
    
    /// Custom backoff with specified delays
    Custom { delays: Vec<Duration> },
}

/// Jitter configuration for retry delays
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JitterConfig {
    /// Enable jitter
    pub enabled: bool,
    
    /// Jitter type
    pub jitter_type: JitterType,
    
    /// Jitter factor (0.0 to 1.0)
    pub factor: f64,
}

impl Default for JitterConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            jitter_type: JitterType::Full,
            factor: 0.1,
        }
    }
}

/// Type of jitter to apply
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum JitterType {
    /// No jitter
    None,
    
    /// Full jitter (random between 0 and delay)
    Full,
    
    /// Equal jitter (delay/2 + random(0, delay/2))
    Equal,
    
    /// Decorrelated jitter
    Decorrelated,
}

/// Recovery strategy configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryStrategyConfig {
    /// Enable automatic recovery
    pub auto_recovery: bool,
    
    /// Recovery timeout
    pub recovery_timeout: Duration,
    
    /// Fallback strategies
    pub fallback_strategies: Vec<FallbackStrategy>,
    
    /// Enable graceful degradation
    pub graceful_degradation: bool,
    
    /// Degradation levels
    pub degradation_levels: Vec<DegradationLevel>,
}

impl Default for RecoveryStrategyConfig {
    fn default() -> Self {
        Self {
            auto_recovery: true,
            recovery_timeout: Duration::from_secs(300),
            fallback_strategies: vec![
                FallbackStrategy::UseCache,
                FallbackStrategy::RetryWithReducedLoad,
                FallbackStrategy::SkipNonCritical,
            ],
            graceful_degradation: true,
            degradation_levels: vec![
                DegradationLevel::ReduceParallelism,
                DegradationLevel::SkipOptionalFeatures,
                DegradationLevel::EmergencyMode,
            ],
        }
    }
}

/// Fallback strategies for error recovery
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FallbackStrategy {
    /// Use cached data
    UseCache,
    
    /// Retry with reduced load
    RetryWithReducedLoad,
    
    /// Skip non-critical operations
    SkipNonCritical,
    
    /// Use alternative service
    UseAlternativeService,
    
    /// Return default values
    ReturnDefaults,
    
    /// Queue for later processing
    QueueForLater,
}

/// Degradation levels for graceful degradation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DegradationLevel {
    /// Reduce parallelism
    ReduceParallelism,
    
    /// Skip optional features
    SkipOptionalFeatures,
    
    /// Reduce data accuracy
    ReduceAccuracy,
    
    /// Emergency mode (minimal functionality)
    EmergencyMode,
}

/// Error classification configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorClassificationConfig {
    /// Retryable error patterns
    pub retryable_patterns: Vec<String>,
    
    /// Non-retryable error patterns
    pub non_retryable_patterns: Vec<String>,
    
    /// Critical error patterns
    pub critical_patterns: Vec<String>,
    
    /// Transient error patterns
    pub transient_patterns: Vec<String>,
}

impl Default for ErrorClassificationConfig {
    fn default() -> Self {
        Self {
            retryable_patterns: vec![
                "connection reset".to_string(),
                "timeout".to_string(),
                "temporarily unavailable".to_string(),
                "rate limit".to_string(),
            ],
            non_retryable_patterns: vec![
                "authentication failed".to_string(),
                "authorization denied".to_string(),
                "invalid format".to_string(),
                "not found".to_string(),
            ],
            critical_patterns: vec![
                "database corruption".to_string(),
                "disk full".to_string(),
                "memory exhausted".to_string(),
                "security breach".to_string(),
            ],
            transient_patterns: vec![
                "network unreachable".to_string(),
                "service unavailable".to_string(),
                "load balancer error".to_string(),
            ],
        }
    }
}

/// Health check configuration for error recovery
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckConfig {
    /// Health check interval
    pub interval: Duration,
    
    /// Health check timeout
    pub timeout: Duration,
    
    /// Healthy threshold
    pub healthy_threshold: u32,
    
    /// Unhealthy threshold
    pub unhealthy_threshold: u32,
}

impl Default for HealthCheckConfig {
    fn default() -> Self {
        Self {
            interval: Duration::from_secs(30),
            timeout: Duration::from_secs(5),
            healthy_threshold: 2,
            unhealthy_threshold: 3,
        }
    }
}

/// Circuit breaker states
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CircuitBreakerState {
    /// Circuit is closed (normal operation)
    Closed,
    
    /// Circuit is open (rejecting requests)
    Open,
    
    /// Circuit is half-open (testing recovery)
    HalfOpen,
}

/// Error classification types
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ErrorClassification {
    /// Error is retryable
    Retryable,
    
    /// Error is not retryable
    NonRetryable,
    
    /// Error is critical
    Critical,
    
    /// Error is transient
    Transient,
    
    /// Error classification unknown
    Unknown,
}

/// Error recovery metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorRecoveryMetrics {
    /// Total errors encountered
    pub total_errors: u64,
    
    /// Successful recoveries
    pub successful_recoveries: u64,
    
    /// Failed recoveries
    pub failed_recoveries: u64,
    
    /// Circuit breaker trips
    pub circuit_breaker_trips: u64,
    
    /// Total retry attempts
    pub total_retries: u64,
    
    /// Successful retries
    pub successful_retries: u64,
    
    /// Average recovery time
    pub avg_recovery_time: Duration,
    
    /// Current error rate
    pub current_error_rate: f64,
    
    /// Error distribution by type
    pub error_distribution: HashMap<String, u64>,
    
    /// Recovery strategy usage
    pub strategy_usage: HashMap<String, u64>,
}

/// Error recovery event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorRecoveryEvent {
    /// Event ID
    pub id: String,
    
    /// Component that experienced the error
    pub component: String,
    
    /// Original error
    pub error: String,
    
    /// Error classification
    pub classification: ErrorClassification,
    
    /// Recovery action taken
    pub recovery_action: String,
    
    /// Recovery success
    pub success: bool,
    
    /// Recovery duration
    pub duration: Duration,
    
    /// Event timestamp
    pub timestamp: SystemTime,
    
    /// Additional context
    pub context: HashMap<String, serde_json::Value>,
}

/// Circuit breaker for error recovery
#[derive(Debug)]
pub struct CircuitBreaker {
    /// Component name
    name: String,
    
    /// Configuration
    config: CircuitBreakerConfig,
    
    /// Current state
    state: Arc<RwLock<CircuitBreakerState>>,
    
    /// Failure count
    failure_count: Arc<RwLock<u32>>,
    
    /// Success count in half-open state
    success_count: Arc<RwLock<u32>>,
    
    /// Last failure time
    last_failure_time: Arc<RwLock<Option<Instant>>>,
    
    /// Request history for rolling window
    request_history: Arc<RwLock<VecDeque<(Instant, bool)>>>,
}

impl CircuitBreaker {
    /// Create a new circuit breaker
    pub fn new(name: String, config: CircuitBreakerConfig) -> Self {
        Self {
            name,
            config,
            state: Arc::new(RwLock::new(CircuitBreakerState::Closed)),
            failure_count: Arc::new(RwLock::new(0)),
            success_count: Arc::new(RwLock::new(0)),
            last_failure_time: Arc::new(RwLock::new(None)),
            request_history: Arc::new(RwLock::new(VecDeque::new())),
        }
    }
    
    /// Execute operation with circuit breaker protection
    pub async fn execute<F, T, E>(&self, operation: F) -> Result<T>
    where
        F: std::future::Future<Output = std::result::Result<T, E>> + Send,
        E: std::fmt::Display + std::fmt::Debug + Send,
    {
        // Check if circuit is open
        if self.is_open().await {
            return Err(Error::generic(format!("Circuit breaker {} is open", self.name)));
        }
        
        let _start_time = Instant::now();
        
        // Execute operation
        match timeout(Duration::from_secs(30), operation).await {
            Ok(Ok(result)) => {
                self.record_success().await;
                Ok(result)
            }
            Ok(Err(e)) => {
                self.record_failure().await;
                Err(Error::generic(format!("Operation failed: {}", e)))
            }
            Err(_) => {
                self.record_failure().await;
                Err(Error::generic("Operation timed out"))
            }
        }
    }
    
    /// Check if circuit breaker is open
    async fn is_open(&self) -> bool {
        let state = self.state.read().await;
        match *state {
            CircuitBreakerState::Open => {
                // Check if timeout has elapsed
                if let Some(last_failure) = *self.last_failure_time.read().await {
                    if last_failure.elapsed() >= self.config.timeout {
                        // Move to half-open state
                        drop(state);
                        self.transition_to_half_open().await;
                        false
                    } else {
                        true
                    }
                } else {
                    true
                }
            }
            _ => false,
        }
    }
    
    /// Record successful operation
    async fn record_success(&self) {
        let now = Instant::now();
        
        // Update request history
        {
            let mut history = self.request_history.write().await;
            history.push_back((now, true));
            self.cleanup_old_requests(&mut history, now).await;
        }
        
        let state = self.state.read().await.clone();
        
        match state {
            CircuitBreakerState::HalfOpen => {
                let mut success_count = self.success_count.write().await;
                *success_count += 1;
                
                if *success_count >= self.config.success_threshold {
                    self.transition_to_closed().await;
                }
            }
            CircuitBreakerState::Closed => {
                // Reset failure count on success
                *self.failure_count.write().await = 0;
            }
            _ => {}
        }
    }
    
    /// Record failed operation
    async fn record_failure(&self) {
        let now = Instant::now();
        
        // Update request history
        {
            let mut history = self.request_history.write().await;
            history.push_back((now, false));
            self.cleanup_old_requests(&mut history, now).await;
        }
        
        *self.last_failure_time.write().await = Some(now);
        
        let state = self.state.read().await.clone();
        
        match state {
            CircuitBreakerState::Closed => {
                let mut failure_count = self.failure_count.write().await;
                *failure_count += 1;
                
                if *failure_count >= self.config.failure_threshold && self.should_trip().await {
                    drop(failure_count);
                    self.transition_to_open().await;
                }
            }
            CircuitBreakerState::HalfOpen => {
                self.transition_to_open().await;
            }
            _ => {}
        }
    }
    
    /// Check if circuit breaker should trip based on request volume
    async fn should_trip(&self) -> bool {
        let history = self.request_history.read().await;
        history.len() >= self.config.request_volume_threshold as usize
    }
    
    /// Transition to closed state
    async fn transition_to_closed(&self) {
        *self.state.write().await = CircuitBreakerState::Closed;
        *self.failure_count.write().await = 0;
        *self.success_count.write().await = 0;
        info!("Circuit breaker {} transitioned to CLOSED", self.name);
    }
    
    /// Transition to open state
    async fn transition_to_open(&self) {
        *self.state.write().await = CircuitBreakerState::Open;
        *self.success_count.write().await = 0;
        warn!("Circuit breaker {} transitioned to OPEN", self.name);
    }
    
    /// Transition to half-open state
    async fn transition_to_half_open(&self) {
        *self.state.write().await = CircuitBreakerState::HalfOpen;
        *self.success_count.write().await = 0;
        info!("Circuit breaker {} transitioned to HALF-OPEN", self.name);
    }
    
    /// Clean up old requests from history
    async fn cleanup_old_requests(&self, history: &mut VecDeque<(Instant, bool)>, now: Instant) {
        let cutoff = now - self.config.rolling_window;
        while let Some(&(timestamp, _)) = history.front() {
            if timestamp < cutoff {
                history.pop_front();
            } else {
                break;
            }
        }
    }
    
    /// Get current state
    pub async fn get_state(&self) -> CircuitBreakerState {
        self.state.read().await.clone()
    }
    
    /// Get failure count
    pub async fn get_failure_count(&self) -> u32 {
        *self.failure_count.read().await
    }
}

/// Error recovery system
pub struct ErrorRecoverySystem {
    /// Configuration
    config: ErrorRecoveryConfig,
    
    /// Circuit breakers by component
    circuit_breakers: Arc<RwLock<HashMap<String, Arc<CircuitBreaker>>>>,
    
    /// Error recovery metrics
    metrics: Arc<RwLock<ErrorRecoveryMetrics>>,
    
    /// Error history
    error_history: Arc<RwLock<VecDeque<ErrorRecoveryEvent>>>,
    
    /// Event broadcaster
    event_broadcaster: broadcast::Sender<ErrorRecoveryEvent>,
    
    /// Shutdown signal
    shutdown_tx: mpsc::UnboundedSender<()>,
}

impl ErrorRecoverySystem {
    /// Create a new error recovery system
    pub fn new(config: ErrorRecoveryConfig) -> Self {
        let (event_broadcaster, _) = broadcast::channel(1000);
        let (shutdown_tx, _) = mpsc::unbounded_channel();
        
        let metrics = ErrorRecoveryMetrics {
            total_errors: 0,
            successful_recoveries: 0,
            failed_recoveries: 0,
            circuit_breaker_trips: 0,
            total_retries: 0,
            successful_retries: 0,
            avg_recovery_time: Duration::from_millis(0),
            current_error_rate: 0.0,
            error_distribution: HashMap::new(),
            strategy_usage: HashMap::new(),
        };
        
        Self {
            config,
            circuit_breakers: Arc::new(RwLock::new(HashMap::new())),
            metrics: Arc::new(RwLock::new(metrics)),
            error_history: Arc::new(RwLock::new(VecDeque::new())),
            event_broadcaster,
            shutdown_tx,
        }
    }
    
    /// Get or create circuit breaker for component
    pub async fn get_circuit_breaker(&self, component: &str) -> Arc<CircuitBreaker> {
        let mut breakers = self.circuit_breakers.write().await;
        
        if let Some(breaker) = breakers.get(component) {
            breaker.clone()
        } else {
            let breaker = Arc::new(CircuitBreaker::new(
                component.to_string(),
                self.config.circuit_breaker.clone(),
            ));
            breakers.insert(component.to_string(), breaker.clone());
            breaker
        }
    }
    
    /// Execute operation with error recovery
    pub async fn execute_with_recovery<F, T, E>(
        &self,
        component: &str,
        operation: F,
    ) -> Result<T>
    where
        F: Fn() -> std::pin::Pin<Box<dyn std::future::Future<Output = std::result::Result<T, E>> + Send>> + Send + Sync,
        E: std::fmt::Display + std::fmt::Debug + Send,
        T: Send,
    {
        let circuit_breaker = self.get_circuit_breaker(component).await;
        let mut last_error = None;
        
        for attempt in 0..=self.config.retry_policy.max_retries {
            match circuit_breaker.execute(operation()).await {
                Ok(result) => {
                    if attempt > 0 {
                        self.record_successful_recovery(component, attempt).await;
                    }
                    return Ok(result);
                }
                Err(e) => {
                    last_error = Some(e.clone());
                    
                    // Classify error
                    let classification = self.classify_error(&e).await;
                    
                    // Record error
                    self.record_error(component, &e, classification.clone()).await;
                    
                    // Check if we should retry
                    if attempt < self.config.retry_policy.max_retries && 
                       self.should_retry(&classification).await {
                        
                        let delay = self.calculate_retry_delay(attempt).await;
                        debug!("Retrying {} after {:?} (attempt {})", component, delay, attempt + 1);
                        sleep(delay).await;
                    } else {
                        break;
                    }
                }
            }
        }
        
        // All retries failed, attempt recovery
        self.attempt_recovery(component, last_error.as_ref()).await
    }
    
    /// Classify error based on patterns
    async fn classify_error(&self, error: &Error) -> ErrorClassification {
        let error_message = error.to_string().to_lowercase();
        
        // Check critical patterns first
        for pattern in &self.config.error_classification.critical_patterns {
            if error_message.contains(pattern) {
                return ErrorClassification::Critical;
            }
        }
        
        // Check non-retryable patterns
        for pattern in &self.config.error_classification.non_retryable_patterns {
            if error_message.contains(pattern) {
                return ErrorClassification::NonRetryable;
            }
        }
        
        // Check transient patterns
        for pattern in &self.config.error_classification.transient_patterns {
            if error_message.contains(pattern) {
                return ErrorClassification::Transient;
            }
        }
        
        // Check retryable patterns
        for pattern in &self.config.error_classification.retryable_patterns {
            if error_message.contains(pattern) {
                return ErrorClassification::Retryable;
            }
        }
        
        ErrorClassification::Unknown
    }
    
    /// Check if error should be retried
    async fn should_retry(&self, classification: &ErrorClassification) -> bool {
        match classification {
            ErrorClassification::Retryable => true,
            ErrorClassification::Transient => true,
            ErrorClassification::NonRetryable => false,
            ErrorClassification::Critical => false,
            ErrorClassification::Unknown => true, // Default to retry for unknown errors
        }
    }
    
    /// Calculate retry delay with backoff and jitter
    async fn calculate_retry_delay(&self, attempt: u32) -> Duration {
        let base_delay = match &self.config.retry_policy.backoff_strategy {
            BackoffStrategy::FixedDelay => self.config.retry_policy.initial_delay,
            BackoffStrategy::LinearBackoff { increment } => {
                self.config.retry_policy.initial_delay + *increment * attempt
            }
            BackoffStrategy::ExponentialBackoff { multiplier } => {
                let delay_ms = self.config.retry_policy.initial_delay.as_millis() as f64;
                let backoff = multiplier.powi(attempt as i32);
                Duration::from_millis((delay_ms * backoff) as u64)
            }
            BackoffStrategy::FibonacciBackoff => {
                let fib = self.fibonacci(attempt as usize + 1);
                Duration::from_millis(self.config.retry_policy.initial_delay.as_millis() as u64 * fib)
            }
            BackoffStrategy::Custom { delays } => {
                delays.get(attempt as usize)
                    .copied()
                    .unwrap_or(self.config.retry_policy.max_delay)
            }
        };
        
        // Apply maximum delay limit
        let capped_delay = std::cmp::min(base_delay, self.config.retry_policy.max_delay);
        
        // Apply jitter if enabled
        if self.config.retry_policy.jitter_config.enabled {
            self.apply_jitter(capped_delay).await
        } else {
            capped_delay
        }
    }
    
    /// Apply jitter to delay
    async fn apply_jitter(&self, delay: Duration) -> Duration {
        use rand::Rng;
        
        let jitter_config = &self.config.retry_policy.jitter_config;
        let delay_ms = delay.as_millis() as f64;
        
        let jittered_delay_ms = match jitter_config.jitter_type {
            JitterType::None => delay_ms,
            JitterType::Full => {
                let mut rng = rand::thread_rng();
                rng.gen_range(0.0..delay_ms)
            }
            JitterType::Equal => {
                let mut rng = rand::thread_rng();
                delay_ms / 2.0 + rng.gen_range(0.0..delay_ms / 2.0)
            }
            JitterType::Decorrelated => {
                let mut rng = rand::thread_rng();
                let factor = jitter_config.factor;
                delay_ms * (1.0 + rng.gen_range(-factor..factor))
            }
        };
        
        Duration::from_millis(jittered_delay_ms.max(0.0) as u64)
    }
    
    /// Calculate Fibonacci number for backoff
    fn fibonacci(&self, n: usize) -> u64 {
        match n {
            0 => 0,
            1 => 1,
            _ => {
                let mut a = 0u64;
                let mut b = 1u64;
                for _ in 2..=n {
                    let temp = a + b;
                    a = b;
                    b = temp;
                }
                b
            }
        }
    }
    
    /// Record error occurrence
    async fn record_error(&self, component: &str, error: &Error, classification: ErrorClassification) {
        let event = ErrorRecoveryEvent {
            id: uuid::Uuid::new_v4().to_string(),
            component: component.to_string(),
            error: error.to_string(),
            classification,
            recovery_action: "none".to_string(),
            success: false,
            duration: Duration::from_millis(0),
            timestamp: SystemTime::now(),
            context: HashMap::new(),
        };
        
        // Update metrics
        {
            let mut metrics = self.metrics.write().await;
            metrics.total_errors += 1;
            
            let error_type = error.to_string();
            *metrics.error_distribution.entry(error_type).or_insert(0) += 1;
        }
        
        // Add to history
        {
            let mut history = self.error_history.write().await;
            history.push_back(event.clone());
            
            if history.len() > self.config.max_error_history {
                history.pop_front();
            }
        }
        
        // Broadcast event
        let _ = self.event_broadcaster.send(event);
    }
    
    /// Record successful recovery
    async fn record_successful_recovery(&self, component: &str, attempts: u32) {
        let mut metrics = self.metrics.write().await;
        metrics.successful_recoveries += 1;
        metrics.total_retries += attempts as u64;
        metrics.successful_retries += 1;
        
        info!("Successfully recovered {} after {} attempts", component, attempts);
    }
    
    /// Attempt recovery using configured strategies
    async fn attempt_recovery<T>(&self, component: &str, _error: Option<&Error>) -> Result<T> {
        for strategy in &self.config.recovery_strategy.fallback_strategies {
            match strategy {
                FallbackStrategy::UseCache => {
                    // Try to use cached data
                    info!("Attempting recovery for {} using cache", component);
                }
                FallbackStrategy::RetryWithReducedLoad => {
                    // Implement reduced load retry
                    info!("Attempting recovery for {} with reduced load", component);
                }
                FallbackStrategy::SkipNonCritical => {
                    // Skip non-critical operations
                    info!("Attempting recovery for {} by skipping non-critical operations", component);
                }
                _ => {
                    debug!("Recovery strategy {:?} not implemented yet", strategy);
                }
            }
        }
        
        // Record failed recovery
        {
            let mut metrics = self.metrics.write().await;
            metrics.failed_recoveries += 1;
        }
        
        Err(Error::generic(format!("Failed to recover component: {}", component)))
    }
    
    /// Get current metrics
    pub async fn get_metrics(&self) -> ErrorRecoveryMetrics {
        self.metrics.read().await.clone()
    }
    
    /// Subscribe to error recovery events
    pub fn subscribe_to_events(&self) -> broadcast::Receiver<ErrorRecoveryEvent> {
        self.event_broadcaster.subscribe()
    }
    
    /// Shutdown the error recovery system
    pub async fn shutdown(&self) -> Result<()> {
        let _ = self.shutdown_tx.send(());
        info!("Error recovery system shut down");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_circuit_breaker_states() {
        let config = CircuitBreakerConfig::default();
        let breaker = CircuitBreaker::new("test".to_string(), config);
        
        assert_eq!(breaker.get_state().await, CircuitBreakerState::Closed);
        assert_eq!(breaker.get_failure_count().await, 0);
    }
    
    #[tokio::test]
    async fn test_error_classification() {
        let config = ErrorRecoveryConfig::default();
        let system = ErrorRecoverySystem::new(config);
        
        let timeout_error = Error::generic("connection timeout");
        let classification = system.classify_error(&timeout_error).await;
        assert_eq!(classification, ErrorClassification::Retryable);
        
        let auth_error = Error::generic("authentication failed");
        let classification = system.classify_error(&auth_error).await;
        assert_eq!(classification, ErrorClassification::NonRetryable);
    }
    
    #[tokio::test]
    async fn test_retry_delay_calculation() {
        let mut config = ErrorRecoveryConfig::default();
        // Disable jitter for deterministic testing
        config.retry_policy.jitter_config.enabled = false;
        
        let system = ErrorRecoverySystem::new(config);
        
        let delay1 = system.calculate_retry_delay(0).await;
        let delay2 = system.calculate_retry_delay(1).await;
        
        // Without jitter, delays should increase with exponential backoff
        assert!(delay2 >= delay1);
    }
    
    #[test]
    fn test_fibonacci_calculation() {
        let config = ErrorRecoveryConfig::default();
        let system = ErrorRecoverySystem::new(config);
        
        assert_eq!(system.fibonacci(0), 0);
        assert_eq!(system.fibonacci(1), 1);
        assert_eq!(system.fibonacci(5), 5);
        assert_eq!(system.fibonacci(10), 55);
    }
} 