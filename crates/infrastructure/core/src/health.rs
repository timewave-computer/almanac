// Comprehensive health monitoring system with component health checks
//
// This module provides health monitoring for all Almanac components with
// automatic unhealthy component restart logic, health check aggregation,
// and detailed health reporting.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime, Instant};
use tokio::sync::{RwLock, broadcast};
use tokio::time::{interval, sleep};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::{Error, Result};

/// Health status for a component
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum HealthStatus {
    /// Component is healthy and functioning normally
    Healthy,
    
    /// Component is degraded but still functional
    Degraded,
    
    /// Component is unhealthy and may not be functioning
    Unhealthy,
    
    /// Component status is unknown (e.g., not responding)
    Unknown,
}

impl std::fmt::Display for HealthStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HealthStatus::Healthy => write!(f, "healthy"),
            HealthStatus::Degraded => write!(f, "degraded"),
            HealthStatus::Unhealthy => write!(f, "unhealthy"),
            HealthStatus::Unknown => write!(f, "unknown"),
        }
    }
}

/// Health check result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckResult {
    /// Component name
    pub component: String,
    
    /// Health status
    pub status: HealthStatus,
    
    /// Health check message
    pub message: String,
    
    /// Check timestamp
    pub timestamp: SystemTime,
    
    /// Check duration
    pub duration: Duration,
    
    /// Additional metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Type alias for HealthCheck
pub type HealthCheck = HealthCheckResult;

/// Health check configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckConfig {
    /// Check interval
    pub interval: Duration,
    
    /// Check timeout
    pub timeout: Duration,
    
    /// Number of consecutive failures before marking unhealthy
    pub failure_threshold: u32,
    
    /// Number of consecutive successes before marking healthy
    pub success_threshold: u32,
    
    /// Enable automatic restart for unhealthy components
    pub auto_restart: bool,
    
    /// Maximum restart attempts
    pub max_restart_attempts: u32,
    
    /// Restart cooldown period
    pub restart_cooldown: Duration,
}

impl Default for HealthCheckConfig {
    fn default() -> Self {
        Self {
            interval: Duration::from_secs(30),
            timeout: Duration::from_secs(5),
            failure_threshold: 3,
            success_threshold: 1,
            auto_restart: true,
            max_restart_attempts: 3,
            restart_cooldown: Duration::from_secs(60),
        }
    }
}

/// Component health checker trait
#[async_trait]
pub trait HealthChecker: Send + Sync + std::fmt::Debug {
    /// Component name
    fn component_name(&self) -> &str;
    
    /// Perform health check
    async fn check_health(&self) -> Result<HealthCheckResult>;
    
    /// Restart the component if possible
    async fn restart_component(&self) -> Result<()> {
        Err(Error::generic("Restart not implemented for this component"))
    }
    
    /// Get component configuration
    fn config(&self) -> HealthCheckConfig {
        HealthCheckConfig::default()
    }
}

/// Health check event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthEvent {
    /// Event type
    pub event_type: HealthEventType,
    
    /// Component name
    pub component: String,
    
    /// Current health status
    pub status: HealthStatus,
    
    /// Previous health status
    pub previous_status: Option<HealthStatus>,
    
    /// Event timestamp
    pub timestamp: SystemTime,
    
    /// Additional context
    pub context: HashMap<String, serde_json::Value>,
}

/// Health event types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HealthEventType {
    /// Component became healthy
    Recovered,
    
    /// Component became degraded
    Degraded,
    
    /// Component became unhealthy
    Failed,
    
    /// Component status is unknown
    Unknown,
    
    /// Component restart attempted
    RestartAttempted,
    
    /// Component restart succeeded
    RestartSucceeded,
    
    /// Component restart failed
    RestartFailed,
}

/// Component health state
#[derive(Debug, Clone)]
struct ComponentHealthState {
    /// Current status
    status: HealthStatus,
    
    /// Last check result
    last_result: Option<HealthCheckResult>,
    
    /// Consecutive failure count
    consecutive_failures: u32,
    
    /// Consecutive success count
    consecutive_successes: u32,
    
    /// Restart attempts count
    restart_attempts: u32,
    
    /// Last restart time
    last_restart: Option<SystemTime>,
    
    /// Health checker
    checker: Arc<dyn HealthChecker>,
    
    /// Configuration
    config: HealthCheckConfig,
}

/// Health monitoring statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthStats {
    /// Total health checks performed
    pub total_checks: u64,
    
    /// Total successful checks
    pub successful_checks: u64,
    
    /// Total failed checks
    pub failed_checks: u64,
    
    /// Total restart attempts
    pub restart_attempts: u64,
    
    /// Total successful restarts
    pub successful_restarts: u64,
    
    /// Component counts by status
    pub component_counts: HashMap<HealthStatus, u32>,
    
    /// Average check duration
    pub avg_check_duration: Duration,
    
    /// Last update time
    pub last_update: SystemTime,
}

/// Health monitoring system
pub struct HealthMonitor {
    /// Component health states
    components: Arc<RwLock<HashMap<String, ComponentHealthState>>>,
    
    /// Health event broadcaster
    event_broadcaster: broadcast::Sender<HealthEvent>,
    
    /// Health statistics
    stats: Arc<RwLock<HealthStats>>,
    
    /// Shutdown signal
    shutdown_tx: broadcast::Sender<()>,
}

impl Default for HealthMonitor {
    fn default() -> Self {
        Self::new()
    }
}

impl HealthMonitor {
    /// Create a new health monitor
    pub fn new() -> Self {
        let (event_broadcaster, _) = broadcast::channel(1000);
        let (shutdown_tx, _) = broadcast::channel(1);
        
        Self {
            components: Arc::new(RwLock::new(HashMap::new())),
            event_broadcaster,
            stats: Arc::new(RwLock::new(HealthStats {
                total_checks: 0,
                successful_checks: 0,
                failed_checks: 0,
                restart_attempts: 0,
                successful_restarts: 0,
                component_counts: HashMap::new(),
                avg_check_duration: Duration::from_millis(0),
                last_update: SystemTime::now(),
            })),
            shutdown_tx,
        }
    }
    
    /// Register a component for health monitoring
    pub async fn register_component(&self, checker: Arc<dyn HealthChecker>) {
        let component_name = checker.component_name().to_string();
        let config = checker.config();
        
        let state = ComponentHealthState {
            status: HealthStatus::Unknown,
            last_result: None,
            consecutive_failures: 0,
            consecutive_successes: 0,
            restart_attempts: 0,
            last_restart: None,
            checker,
            config,
        };
        
        self.components.write().await.insert(component_name.clone(), state);
        
        // Start monitoring task for this component
        let components = self.components.clone();
        let event_broadcaster = self.event_broadcaster.clone();
        let stats = self.stats.clone();
        let shutdown_rx = self.shutdown_tx.subscribe();
        
        tokio::spawn(async move {
            Self::monitor_component(
                component_name,
                components,
                event_broadcaster,
                stats,
                shutdown_rx,
            ).await;
        });
    }
    
    /// Monitor a specific component
    async fn monitor_component(
        component_name: String,
        components: Arc<RwLock<HashMap<String, ComponentHealthState>>>,
        event_broadcaster: broadcast::Sender<HealthEvent>,
        stats: Arc<RwLock<HealthStats>>,
        mut shutdown_rx: broadcast::Receiver<()>,
    ) {
        // Get component configuration
        let config = {
            let components_read = components.read().await;
            if let Some(state) = components_read.get(&component_name) {
                state.config.clone()
            } else {
                return;
            }
        };
        
        let mut interval = interval(config.interval);
        
        loop {
            tokio::select! {
                _ = interval.tick() => {
                    Self::perform_health_check(
                        &component_name,
                        &components,
                        &event_broadcaster,
                        &stats,
                    ).await;
                }
                _ = shutdown_rx.recv() => {
                    break;
                }
            }
        }
    }
    
    /// Perform health check for a component
    async fn perform_health_check(
        component_name: &str,
        components: &Arc<RwLock<HashMap<String, ComponentHealthState>>>,
        event_broadcaster: &broadcast::Sender<HealthEvent>,
        stats: &Arc<RwLock<HealthStats>>,
    ) {
        let start_time = Instant::now();
        
        // Get component state
        let (checker, config, _previous_status) = {
            let components_read = components.read().await;
            if let Some(state) = components_read.get(component_name) {
                (state.checker.clone(), state.config.clone(), state.status.clone())
            } else {
                return;
            }
        };
        
        // Perform health check with timeout
        let check_result = tokio::time::timeout(config.timeout, checker.check_health()).await;
        
        let result = match check_result {
            Ok(Ok(result)) => result,
            Ok(Err(e)) => HealthCheckResult {
                component: component_name.to_string(),
                status: HealthStatus::Unhealthy,
                message: format!("Health check failed: {}", e),
                timestamp: SystemTime::now(),
                duration: start_time.elapsed(),
                metadata: HashMap::new(),
            },
            Err(_) => HealthCheckResult {
                component: component_name.to_string(),
                status: HealthStatus::Unhealthy,
                message: "Health check timed out".to_string(),
                timestamp: SystemTime::now(),
                duration: start_time.elapsed(),
                metadata: HashMap::new(),
            },
        };
        
        // Update component state
        let should_restart = {
            let mut components_write = components.write().await;
            if let Some(state) = components_write.get_mut(component_name) {
                let old_status = state.status.clone();
                state.last_result = Some(result.clone());
                
                // Update counters based on result
                match result.status {
                    HealthStatus::Healthy => {
                        state.consecutive_successes += 1;
                        state.consecutive_failures = 0;
                        
                        // Mark as healthy if threshold met
                        if state.consecutive_successes >= config.success_threshold {
                            state.status = HealthStatus::Healthy;
                        }
                    }
                    HealthStatus::Degraded => {
                        state.status = HealthStatus::Degraded;
                        state.consecutive_successes = 0;
                        state.consecutive_failures = 0;
                    }
                    HealthStatus::Unhealthy | HealthStatus::Unknown => {
                        state.consecutive_failures += 1;
                        state.consecutive_successes = 0;
                        
                        // Mark as unhealthy if threshold met
                        if state.consecutive_failures >= config.failure_threshold {
                            state.status = HealthStatus::Unhealthy;
                        }
                    }
                }
                
                // Check if restart is needed
                let should_restart = config.auto_restart &&
                    state.status == HealthStatus::Unhealthy &&
                    state.restart_attempts < config.max_restart_attempts &&
                    state.last_restart.map(|last| {
                        SystemTime::now().duration_since(last).unwrap_or_default() > config.restart_cooldown
                    }).unwrap_or(true);
                
                // Emit event if status changed
                if old_status != state.status {
                    let event_type = match state.status {
                        HealthStatus::Healthy => HealthEventType::Recovered,
                        HealthStatus::Degraded => HealthEventType::Degraded,
                        HealthStatus::Unhealthy => HealthEventType::Failed,
                        HealthStatus::Unknown => HealthEventType::Unknown,
                    };
                    
                    let event = HealthEvent {
                        event_type,
                        component: component_name.to_string(),
                        status: state.status.clone(),
                        previous_status: Some(old_status),
                        timestamp: SystemTime::now(),
                        context: HashMap::new(),
                    };
                    
                    let _ = event_broadcaster.send(event);
                }
                
                should_restart
            } else {
                false
            }
        };
        
        // Attempt restart if needed
        if should_restart {
            Self::attempt_restart(component_name, components, event_broadcaster).await;
        }
        
        // Update statistics
        Self::update_stats(stats, &result).await;
    }
    
    /// Attempt to restart a component
    async fn attempt_restart(
        component_name: &str,
        components: &Arc<RwLock<HashMap<String, ComponentHealthState>>>,
        event_broadcaster: &broadcast::Sender<HealthEvent>,
    ) {
        let checker = {
            let mut components_write = components.write().await;
            if let Some(state) = components_write.get_mut(component_name) {
                state.restart_attempts += 1;
                state.last_restart = Some(SystemTime::now());
                state.checker.clone()
            } else {
                return;
            }
        };
        
        // Emit restart attempt event
        let event = HealthEvent {
            event_type: HealthEventType::RestartAttempted,
            component: component_name.to_string(),
            status: HealthStatus::Unhealthy,
            previous_status: None,
            timestamp: SystemTime::now(),
            context: HashMap::new(),
        };
        let _ = event_broadcaster.send(event);
        
        // Attempt restart
        match checker.restart_component().await {
            Ok(()) => {
                // Reset failure counters on successful restart
                {
                    let mut components_write = components.write().await;
                    if let Some(state) = components_write.get_mut(component_name) {
                        state.consecutive_failures = 0;
                        state.status = HealthStatus::Unknown; // Will be determined by next health check
                    }
                }
                
                let event = HealthEvent {
                    event_type: HealthEventType::RestartSucceeded,
                    component: component_name.to_string(),
                    status: HealthStatus::Unknown,
                    previous_status: Some(HealthStatus::Unhealthy),
                    timestamp: SystemTime::now(),
                    context: HashMap::new(),
                };
                let _ = event_broadcaster.send(event);
                
                tracing::info!("Successfully restarted component: {}", component_name);
            }
            Err(e) => {
                let event = HealthEvent {
                    event_type: HealthEventType::RestartFailed,
                    component: component_name.to_string(),
                    status: HealthStatus::Unhealthy,
                    previous_status: None,
                    timestamp: SystemTime::now(),
                    context: {
                        let mut context = HashMap::new();
                        context.insert("error".to_string(), serde_json::Value::String(e.to_string()));
                        context
                    },
                };
                let _ = event_broadcaster.send(event);
                
                tracing::error!("Failed to restart component {}: {}", component_name, e);
            }
        }
    }
    
    /// Update health monitoring statistics
    async fn update_stats(stats: &Arc<RwLock<HealthStats>>, result: &HealthCheckResult) {
        let mut stats_write = stats.write().await;
        
        stats_write.total_checks += 1;
        
        match result.status {
            HealthStatus::Healthy => stats_write.successful_checks += 1,
            _ => stats_write.failed_checks += 1,
        }
        
        // Update average check duration
        let new_avg = if stats_write.total_checks == 1 {
            result.duration
        } else {
            Duration::from_nanos(
                (stats_write.avg_check_duration.as_nanos() as u64 + result.duration.as_nanos() as u64) / 2
            )
        };
        stats_write.avg_check_duration = new_avg;
        stats_write.last_update = SystemTime::now();
    }
    
    /// Get health status for a specific component
    pub async fn get_component_health(&self, component_name: &str) -> Option<HealthCheckResult> {
        let components = self.components.read().await;
        components.get(component_name)?.last_result.clone()
    }
    
    /// Get health status for all components
    pub async fn get_all_health(&self) -> HashMap<String, HealthCheckResult> {
        let components = self.components.read().await;
        let mut results = HashMap::new();
        
        for (name, state) in components.iter() {
            if let Some(result) = &state.last_result {
                results.insert(name.clone(), result.clone());
            }
        }
        
        results
    }
    
    /// Get overall system health
    pub async fn get_system_health(&self) -> HealthStatus {
        let components = self.components.read().await;
        
        if components.is_empty() {
            return HealthStatus::Unknown;
        }
        
        let mut healthy_count = 0;
        let mut degraded_count = 0;
        let mut unhealthy_count = 0;
        
        for state in components.values() {
            match state.status {
                HealthStatus::Healthy => healthy_count += 1,
                HealthStatus::Degraded => degraded_count += 1,
                HealthStatus::Unhealthy => unhealthy_count += 1,
                HealthStatus::Unknown => {} // Don't count unknown
            }
        }
        
        let total_known = healthy_count + degraded_count + unhealthy_count;
        
        if total_known == 0 {
            HealthStatus::Unknown
        } else if unhealthy_count > 0 {
            HealthStatus::Unhealthy
        } else if degraded_count > 0 {
            HealthStatus::Degraded
        } else {
            HealthStatus::Healthy
        }
    }
    
    /// Subscribe to health events
    pub fn subscribe_to_events(&self) -> broadcast::Receiver<HealthEvent> {
        self.event_broadcaster.subscribe()
    }
    
    /// Get health monitoring statistics
    pub async fn get_stats(&self) -> HealthStats {
        let mut stats = self.stats.read().await.clone();
        
        // Update component counts
        let components = self.components.read().await;
        let mut component_counts = HashMap::new();
        
        for state in components.values() {
            *component_counts.entry(state.status.clone()).or_insert(0) += 1;
        }
        
        stats.component_counts = component_counts;
        stats
    }
    
    /// Shutdown the health monitor
    pub async fn shutdown(&self) {
        let _ = self.shutdown_tx.send(());
    }
}

/// Example health checker for database connections
#[derive(Debug)]
pub struct DatabaseHealthChecker {
    component_name: String,
    check_query: String,
}

impl DatabaseHealthChecker {
    pub fn new(component_name: String, check_query: String) -> Self {
        Self {
            component_name,
            check_query,
        }
    }
}

#[async_trait]
impl HealthChecker for DatabaseHealthChecker {
    fn component_name(&self) -> &str {
        &self.component_name
    }
    
    async fn check_health(&self) -> Result<HealthCheckResult> {
        let start_time = Instant::now();
        
        // Simulate database health check
        // In real implementation, this would execute the check query
        sleep(Duration::from_millis(10)).await;
        
        let status = HealthStatus::Healthy;
        let message = "Database connection is healthy".to_string();
        
        Ok(HealthCheckResult {
            component: self.component_name.clone(),
            status,
            message,
            timestamp: SystemTime::now(),
            duration: start_time.elapsed(),
            metadata: {
                let mut metadata = HashMap::new();
                metadata.insert("query".to_string(), serde_json::Value::String(self.check_query.clone()));
                metadata
            },
        })
    }
    
    fn config(&self) -> HealthCheckConfig {
        HealthCheckConfig {
            interval: Duration::from_secs(10),
            timeout: Duration::from_secs(5),
            failure_threshold: 2,
            success_threshold: 1,
            auto_restart: false, // Can't restart database from here
            max_restart_attempts: 0,
            restart_cooldown: Duration::from_secs(0),
        }
    }
}

/// Example health checker for API endpoints
#[derive(Debug)]
pub struct ApiHealthChecker {
    component_name: String,
    endpoint_url: String,
}

impl ApiHealthChecker {
    pub fn new(component_name: String, endpoint_url: String) -> Self {
        Self {
            component_name,
            endpoint_url,
        }
    }
}

#[async_trait]
impl HealthChecker for ApiHealthChecker {
    fn component_name(&self) -> &str {
        &self.component_name
    }
    
    async fn check_health(&self) -> Result<HealthCheckResult> {
        let start_time = Instant::now();
        
        // Simulate API health check
        // In real implementation, this would make an HTTP request
        sleep(Duration::from_millis(50)).await;
        
        let status = HealthStatus::Healthy;
        let message = "API endpoint is responding".to_string();
        
        Ok(HealthCheckResult {
            component: self.component_name.clone(),
            status,
            message,
            timestamp: SystemTime::now(),
            duration: start_time.elapsed(),
            metadata: {
                let mut metadata = HashMap::new();
                metadata.insert("endpoint".to_string(), serde_json::Value::String(self.endpoint_url.clone()));
                metadata
            },
        })
    }
    
    async fn restart_component(&self) -> Result<()> {
        // Simulate API restart
        tracing::info!("Restarting API component: {}", self.component_name);
        sleep(Duration::from_millis(100)).await;
        Ok(())
    }
    
    fn config(&self) -> HealthCheckConfig {
        HealthCheckConfig {
            interval: Duration::from_secs(15),
            timeout: Duration::from_secs(10),
            failure_threshold: 3,
            success_threshold: 1,
            auto_restart: true,
            max_restart_attempts: 2,
            restart_cooldown: Duration::from_secs(30),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::timeout;
    
    #[tokio::test]
    async fn test_health_monitor_creation() {
        let monitor = HealthMonitor::new();
        
        // Test initial system health
        let system_health = monitor.get_system_health().await;
        assert_eq!(system_health, HealthStatus::Unknown);
        
        let stats = monitor.get_stats().await;
        assert_eq!(stats.total_checks, 0);
    }
    
    #[tokio::test]
    async fn test_component_registration() {
        let monitor = HealthMonitor::new();
        
        let checker = Arc::new(DatabaseHealthChecker::new(
            "test_db".to_string(),
            "SELECT 1".to_string(),
        ));
        
        monitor.register_component(checker).await;
        
        // Wait for initial health check
        sleep(Duration::from_millis(100)).await;
        
        let health = monitor.get_component_health("test_db").await;
        assert!(health.is_some());
        
        monitor.shutdown().await;
    }
    
    #[tokio::test]
    async fn test_health_events() {
        let monitor = HealthMonitor::new();
        
        let mut event_receiver = monitor.subscribe_to_events();
        
        let checker = Arc::new(ApiHealthChecker::new(
            "test_api".to_string(),
            "http://localhost:8080/health".to_string(),
        ));
        
        monitor.register_component(checker).await;
        
        // Wait for health events
        let event = timeout(Duration::from_secs(1), event_receiver.recv()).await;
        assert!(event.is_ok());
        
        monitor.shutdown().await;
    }
} 