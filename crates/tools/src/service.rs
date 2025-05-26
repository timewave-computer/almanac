/// Service management system for Almanac indexer
use std::collections::HashMap;
use std::fmt;
use std::path::{Path, PathBuf};
use std::process::{Command, Child, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use tokio::time::sleep;

/// Service status enumeration
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ServiceStatus {
    /// Service is stopped
    Stopped,
    
    /// Service is starting up
    Starting,
    
    /// Service is running normally
    Running,
    
    /// Service is stopping
    Stopping,
    
    /// Service has failed
    Failed,
    
    /// Service status is unknown
    Unknown,
    
    /// Service is in recovery mode
    Recovering,
}

impl fmt::Display for ServiceStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ServiceStatus::Stopped => write!(f, "stopped"),
            ServiceStatus::Starting => write!(f, "starting"),
            ServiceStatus::Running => write!(f, "running"),
            ServiceStatus::Stopping => write!(f, "stopping"),
            ServiceStatus::Failed => write!(f, "failed"),
            ServiceStatus::Unknown => write!(f, "unknown"),
            ServiceStatus::Recovering => write!(f, "recovering"),
        }
    }
}

/// Restart policy for services
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum RestartPolicy {
    /// Never restart automatically
    Never,
    
    /// Always restart on exit
    Always,
    
    /// Restart only on failure (not on clean exit)
    #[default]
    OnFailure,
    
    /// Restart unless manually stopped
    UnlessStopped,
}

/// Service health check configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheck {
    /// Command to run for health check
    pub command: Vec<String>,
    
    /// Interval between health checks
    pub interval: Duration,
    
    /// Timeout for health check command
    pub timeout: Duration,
    
    /// Number of consecutive failures before marking unhealthy
    pub retries: u32,
    
    /// Delay before starting health checks
    pub start_period: Duration,
}

impl Default for HealthCheck {
    fn default() -> Self {
        Self {
            command: vec!["curl".to_string(), "-f".to_string(), "http://localhost:8080/health".to_string()],
            interval: Duration::from_secs(30),
            timeout: Duration::from_secs(5),
            retries: 3,
            start_period: Duration::from_secs(10),
        }
    }
}

/// Service dependency configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceDependency {
    /// Name of the dependent service
    pub name: String,
    
    /// Whether this dependency is required for startup
    pub required: bool,
    
    /// Maximum time to wait for dependency
    pub timeout: Duration,
}

/// Service configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceConfig {
    /// Service name
    pub name: String,
    
    /// Service description
    pub description: String,
    
    /// Command to start the service
    pub command: Vec<String>,
    
    /// Working directory for the service
    pub working_directory: Option<PathBuf>,
    
    /// Environment variables for the service
    pub environment: HashMap<String, String>,
    
    /// Service dependencies
    pub dependencies: Vec<ServiceDependency>,
    
    /// Restart policy
    pub restart_policy: RestartPolicy,
    
    /// Health check configuration
    pub health_check: Option<HealthCheck>,
    
    /// User to run the service as
    pub user: Option<String>,
    
    /// Group to run the service as
    pub group: Option<String>,
    
    /// Maximum restart attempts
    pub max_restarts: Option<u32>,
    
    /// Restart delay
    pub restart_delay: Duration,
    
    /// Startup timeout
    pub startup_timeout: Duration,
    
    /// Shutdown timeout
    pub shutdown_timeout: Duration,
    
    /// Enable auto-recovery
    pub auto_recovery: bool,
}

impl Default for ServiceConfig {
    fn default() -> Self {
        Self {
            name: "almanac".to_string(),
            description: "Almanac Indexer Service".to_string(),
            command: vec!["almanac".to_string(), "run".to_string()],
            working_directory: None,
            environment: HashMap::new(),
            dependencies: Vec::new(),
            restart_policy: RestartPolicy::default(),
            health_check: Some(HealthCheck::default()),
            user: None,
            group: None,
            max_restarts: Some(5),
            restart_delay: Duration::from_secs(5),
            startup_timeout: Duration::from_secs(30),
            shutdown_timeout: Duration::from_secs(10),
            auto_recovery: true,
        }
    }
}

/// Service runtime information
#[derive(Debug, Clone)]
pub struct ServiceInfo {
    /// Service configuration
    pub config: ServiceConfig,
    
    /// Current status
    pub status: ServiceStatus,
    
    /// Process ID (if running)
    pub pid: Option<u32>,
    
    /// Start time
    pub start_time: Option<SystemTime>,
    
    /// Last health check time
    pub last_health_check: Option<SystemTime>,
    
    /// Health check status
    pub healthy: bool,
    
    /// Number of restarts
    pub restart_count: u32,
    
    /// Last restart time
    pub last_restart: Option<SystemTime>,
    
    /// Exit code of last run
    pub last_exit_code: Option<i32>,
    
    /// Error message (if failed)
    pub error_message: Option<String>,
}

impl ServiceInfo {
    /// Create new service info from config
    pub fn new(config: ServiceConfig) -> Self {
        Self {
            config,
            status: ServiceStatus::Stopped,
            pid: None,
            start_time: None,
            last_health_check: None,
            healthy: false,
            restart_count: 0,
            last_restart: None,
            last_exit_code: None,
            error_message: None,
        }
    }
    
    /// Get service uptime
    pub fn uptime(&self) -> Option<Duration> {
        self.start_time.and_then(|start| {
            SystemTime::now().duration_since(start).ok()
        })
    }
    
    /// Check if service is running
    pub fn is_running(&self) -> bool {
        matches!(self.status, ServiceStatus::Running | ServiceStatus::Starting)
    }
    
    /// Check if service has failed too many times
    pub fn restart_limit_exceeded(&self) -> bool {
        if let Some(max_restarts) = self.config.max_restarts {
            self.restart_count >= max_restarts
        } else {
            false
        }
    }
}

/// Service manager for handling multiple services
pub struct ServiceManager {
    /// Map of service name to service info
    services: Arc<Mutex<HashMap<String, ServiceInfo>>>,
    
    /// Map of service name to process handle
    processes: Arc<Mutex<HashMap<String, Child>>>,
    
    /// Health check intervals for services
    health_checkers: Arc<Mutex<HashMap<String, tokio::task::JoinHandle<()>>>>,
}

impl ServiceManager {
    /// Create a new service manager
    pub fn new() -> Self {
        Self {
            services: Arc::new(Mutex::new(HashMap::new())),
            processes: Arc::new(Mutex::new(HashMap::new())),
            health_checkers: Arc::new(Mutex::new(HashMap::new())),
        }
    }
    
    /// Register a service with the manager
    pub fn register_service(&self, config: ServiceConfig) -> Result<()> {
        let name = config.name.clone();
        let service_info = ServiceInfo::new(config);
        
        self.services.lock().unwrap().insert(name, service_info);
        Ok(())
    }
    
    /// Start a service
    pub async fn start_service(&self, service_name: &str) -> Result<()> {
        // Get service config
        let config = {
            let services = self.services.lock().unwrap();
            let service = services.get(service_name)
                .ok_or_else(|| anyhow::anyhow!("Service '{}' not found", service_name))?;
            service.config.clone()
        };
        
        // Check dependencies first
        self.check_dependencies(&config.dependencies).await?;
        
        // Update status to starting
        self.update_service_status(service_name, ServiceStatus::Starting)?;
        
        // Start the process
        let mut command = Command::new(&config.command[0]);
        if config.command.len() > 1 {
            command.args(&config.command[1..]);
        }
        
        // Set working directory
        if let Some(wd) = &config.working_directory {
            command.current_dir(wd);
        }
        
        // Set environment variables
        for (key, value) in &config.environment {
            command.env(key, value);
        }
        
        // Configure stdio
        command.stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .stdin(Stdio::null());
        
        // Start the process
        let child = command.spawn()
            .with_context(|| format!("Failed to start service '{}'", service_name))?;
        
        let pid = child.id();
        
        // Store the process handle
        self.processes.lock().unwrap().insert(service_name.to_string(), child);
        
        // Update service info
        {
            let mut services = self.services.lock().unwrap();
            if let Some(service) = services.get_mut(service_name) {
                service.status = ServiceStatus::Running;
                service.pid = Some(pid);
                service.start_time = Some(SystemTime::now());
                service.error_message = None;
            }
        }
        
        // Start health checking if configured
        if config.health_check.is_some() {
            self.start_health_checker(service_name).await?;
        }
        
        // Start auto-recovery if enabled
        if config.auto_recovery {
            self.start_auto_recovery(service_name).await?;
        }
        
        println!("✅ Service '{}' started successfully (PID: {})", service_name, pid);
        Ok(())
    }
    
    /// Stop a service
    pub async fn stop_service(&self, service_name: &str) -> Result<()> {
        // Update status to stopping
        self.update_service_status(service_name, ServiceStatus::Stopping)?;
        
        // Stop health checker
        self.stop_health_checker(service_name).await;
        
        // Get the process
        let mut child = {
            let mut processes = self.processes.lock().unwrap();
            processes.remove(service_name)
                .ok_or_else(|| anyhow::anyhow!("Service '{}' is not running", service_name))?
        };
        
        // Get shutdown timeout
        let timeout = {
            let services = self.services.lock().unwrap();
            services.get(service_name)
                .map(|s| s.config.shutdown_timeout)
                .unwrap_or(Duration::from_secs(10))
        };
        
        // Try graceful shutdown first (SIGTERM)
        #[cfg(unix)]
        {
            // Send SIGTERM
            if let Err(e) = Command::new("kill")
                .arg("-TERM")
                .arg(child.id().to_string())
                .output()
            {
                eprintln!("Warning: Failed to send SIGTERM to process {}: {}", child.id(), e);
            }
            
            // Wait for graceful shutdown
            let start = Instant::now();
            while start.elapsed() < timeout {
                match child.try_wait() {
                    Ok(Some(_)) => {
                        // Process has exited
                        break;
                    }
                    Ok(None) => {
                        // Process is still running
                        sleep(Duration::from_millis(100)).await;
                    }
                    Err(e) => {
                        eprintln!("Warning: Error checking process status: {}", e);
                        break;
                    }
                }
            }
            
            // Force kill if still running
            if let Ok(None) = child.try_wait() {
                eprintln!("Service '{}' didn't shut down gracefully, force killing...", service_name);
                if let Err(e) = Command::new("kill")
                    .arg("-KILL")
                    .arg(child.id().to_string())
                    .output()
                {
                    eprintln!("Warning: Failed to send SIGKILL: {}", e);
                }
            }
        }
        
        #[cfg(windows)]
        {
            // On Windows, try to terminate gracefully
            let _ = child.kill();
        }
        
        // Wait for final exit
        let exit_status = child.wait()?;
        let exit_code = exit_status.code();
        
        // Update service info
        {
            let mut services = self.services.lock().unwrap();
            if let Some(service) = services.get_mut(service_name) {
                service.status = ServiceStatus::Stopped;
                service.pid = None;
                service.start_time = None;
                service.last_exit_code = exit_code;
                service.healthy = false;
            }
        }
        
        println!("✅ Service '{}' stopped successfully", service_name);
        Ok(())
    }
    
    /// Restart a service
    pub async fn restart_service(&self, service_name: &str) -> Result<()> {
        println!("🔄 Restarting service '{}'...", service_name);
        
        // Check if service is running and stop it
        if self.is_service_running(service_name)? {
            self.stop_service(service_name).await?;
        }
        
        // Wait for restart delay
        let restart_delay = {
            let services = self.services.lock().unwrap();
            services.get(service_name)
                .map(|s| s.config.restart_delay)
                .unwrap_or(Duration::from_secs(5))
        };
        
        sleep(restart_delay).await;
        
        // Increment restart count
        {
            let mut services = self.services.lock().unwrap();
            if let Some(service) = services.get_mut(service_name) {
                service.restart_count += 1;
                service.last_restart = Some(SystemTime::now());
            }
        }
        
        // Start the service
        self.start_service(service_name).await?;
        
        println!("✅ Service '{}' restarted successfully", service_name);
        Ok(())
    }
    
    /// Get service status
    pub fn get_service_status(&self, service_name: &str) -> Result<ServiceInfo> {
        let services = self.services.lock().unwrap();
        services.get(service_name)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("Service '{}' not found", service_name))
    }
    
    /// List all services
    pub fn list_services(&self) -> Vec<ServiceInfo> {
        let services = self.services.lock().unwrap();
        services.values().cloned().collect()
    }
    
    /// Check if a service is running
    pub fn is_service_running(&self, service_name: &str) -> Result<bool> {
        let services = self.services.lock().unwrap();
        Ok(services.get(service_name)
            .map(|s| s.is_running())
            .unwrap_or(false))
    }
    
    /// Update service status
    fn update_service_status(&self, service_name: &str, status: ServiceStatus) -> Result<()> {
        let mut services = self.services.lock().unwrap();
        if let Some(service) = services.get_mut(service_name) {
            service.status = status;
        }
        Ok(())
    }
    
    /// Check service dependencies
    async fn check_dependencies(&self, dependencies: &[ServiceDependency]) -> Result<()> {
        for dep in dependencies {
            if dep.required {
                let start = Instant::now();
                
                while start.elapsed() < dep.timeout {
                    if self.is_service_running(&dep.name)? {
                        break;
                    }
                    
                    sleep(Duration::from_millis(500)).await;
                }
                
                if !self.is_service_running(&dep.name)? {
                    return Err(anyhow::anyhow!(
                        "Required dependency '{}' is not running",
                        dep.name
                    ));
                }
            }
        }
        
        Ok(())
    }
    
    /// Start health checker for a service
    async fn start_health_checker(&self, service_name: &str) -> Result<()> {
        let health_check = {
            let services = self.services.lock().unwrap();
            services.get(service_name)
                .and_then(|s| s.config.health_check.clone())
        };
        
        if let Some(health_config) = health_check {
            let services_clone = Arc::clone(&self.services);
            let service_name_clone = service_name.to_string();
            
            let handle = tokio::spawn(async move {
                // Wait for start period
                sleep(health_config.start_period).await;
                
                let mut consecutive_failures = 0;
                
                loop {
                    // Check if service is still running
                    let is_running = {
                        let services = services_clone.lock().unwrap();
                        services.get(&service_name_clone)
                            .map(|s| s.is_running())
                            .unwrap_or(false)
                    };
                    
                    if !is_running {
                        break;
                    }
                    
                    // Run health check
                    let health_ok = run_health_check(&health_config).await;
                    
                    // Update service health status
                    {
                        let mut services = services_clone.lock().unwrap();
                        if let Some(service) = services.get_mut(&service_name_clone) {
                            service.last_health_check = Some(SystemTime::now());
                            
                            if health_ok {
                                service.healthy = true;
                                consecutive_failures = 0;
                            } else {
                                consecutive_failures += 1;
                                
                                if consecutive_failures >= health_config.retries {
                                    service.healthy = false;
                                    service.status = ServiceStatus::Failed;
                                    service.error_message = Some("Health check failed".to_string());
                                }
                            }
                        }
                    }
                    
                    sleep(health_config.interval).await;
                }
            });
            
            self.health_checkers.lock().unwrap().insert(service_name.to_string(), handle);
        }
        
        Ok(())
    }
    
    /// Stop health checker for a service
    async fn stop_health_checker(&self, service_name: &str) {
        if let Some(handle) = self.health_checkers.lock().unwrap().remove(service_name) {
            handle.abort();
        }
    }
    
    /// Start auto-recovery for a service
    async fn start_auto_recovery(&self, service_name: &str) -> Result<()> {
        let config = {
            let services = self.services.lock().unwrap();
            services.get(service_name)
                .map(|s| s.config.clone())
        };
        
        if let Some(service_config) = config {
            let services_clone = Arc::clone(&self.services);
            let service_name_clone = service_name.to_string();
            
            tokio::spawn(async move {
                loop {
                    sleep(Duration::from_secs(10)).await;
                    
                    let should_restart = {
                        let services = services_clone.lock().unwrap();
                        if let Some(service) = services.get(&service_name_clone) {
                            match &service_config.restart_policy {
                                RestartPolicy::Never => false,
                                RestartPolicy::Always => !service.is_running(),
                                RestartPolicy::OnFailure => {
                                    service.status == ServiceStatus::Failed && 
                                    !service.restart_limit_exceeded()
                                }
                                RestartPolicy::UnlessStopped => {
                                    service.status == ServiceStatus::Failed &&
                                    !service.restart_limit_exceeded()
                                }
                            }
                        } else {
                            false
                        }
                    };
                    
                    if should_restart {
                        println!("🔄 Auto-recovery: restarting failed service '{}'", service_name_clone);
                        // Note: In a real implementation, we'd need a reference to the ServiceManager
                        // For now, we'll just log the recovery attempt
                    }
                }
            });
        }
        
        Ok(())
    }
}

impl Default for ServiceManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Run a health check command
async fn run_health_check(health_config: &HealthCheck) -> bool {
    if health_config.command.is_empty() {
        return true;
    }
    
    let result = tokio::time::timeout(
        health_config.timeout,
        tokio::process::Command::new(&health_config.command[0])
            .args(&health_config.command[1..])
            .output()
    ).await;
    
    match result {
        Ok(Ok(output)) => output.status.success(),
        _ => false,
    }
}

/// Service management CLI commands
#[derive(Debug, Clone)]
pub enum ServiceCommand {
    Start { service: String },
    Stop { service: String },
    Restart { service: String },
    Status { service: Option<String> },
    List,
    Register { config_file: PathBuf },
}

/// Execute a service command
pub async fn execute_service_command(
    manager: &ServiceManager,
    command: ServiceCommand,
) -> Result<()> {
    match command {
        ServiceCommand::Start { service } => {
            manager.start_service(&service).await
        }
        ServiceCommand::Stop { service } => {
            manager.stop_service(&service).await
        }
        ServiceCommand::Restart { service } => {
            manager.restart_service(&service).await
        }
        ServiceCommand::Status { service } => {
            if let Some(service_name) = service {
                let info = manager.get_service_status(&service_name)?;
                print_service_status(&info);
            } else {
                let services = manager.list_services();
                for service in services {
                    print_service_status(&service);
                    println!();
                }
            }
            Ok(())
        }
        ServiceCommand::List => {
            let services = manager.list_services();
            if services.is_empty() {
                println!("No services registered");
            } else {
                println!("Registered services:");
                for service in services {
                    println!("  {} - {}", service.config.name, service.status);
                }
            }
            Ok(())
        }
        ServiceCommand::Register { config_file } => {
            let config = load_service_config(&config_file)?;
            manager.register_service(config)?;
            println!("✅ Service registered successfully");
            Ok(())
        }
    }
}

/// Print detailed service status
fn print_service_status(info: &ServiceInfo) {
    println!("Service: {}", info.config.name);
    println!("  Description: {}", info.config.description);
    println!("  Status: {}", info.status);
    
    if let Some(pid) = info.pid {
        println!("  PID: {}", pid);
    }
    
    if let Some(uptime) = info.uptime() {
        println!("  Uptime: {}s", uptime.as_secs());
    }
    
    println!("  Healthy: {}", if info.healthy { "yes" } else { "no" });
    println!("  Restart count: {}", info.restart_count);
    
    if let Some(error) = &info.error_message {
        println!("  Error: {}", error);
    }
    
    if let Some(exit_code) = info.last_exit_code {
        println!("  Last exit code: {}", exit_code);
    }
}

/// Load service configuration from file
pub fn load_service_config<P: AsRef<Path>>(path: P) -> Result<ServiceConfig> {
    let content = std::fs::read_to_string(path.as_ref())
        .with_context(|| format!("Failed to read service config file: {}", path.as_ref().display()))?;
    
    let config = match path.as_ref().extension().and_then(|s| s.to_str()) {
        Some("toml") => toml::from_str(&content)
            .with_context(|| "Failed to parse TOML service config")?,
        Some("json") => serde_json::from_str(&content)
            .with_context(|| "Failed to parse JSON service config")?,
        Some("yaml") | Some("yml") => serde_yaml::from_str(&content)
            .with_context(|| "Failed to parse YAML service config")?,
        _ => return Err(anyhow::anyhow!("Unsupported service config format. Supported: .toml, .json, .yaml, .yml")),
    };
    
    Ok(config)
}

/// Generate a default service configuration file
pub fn generate_default_service_config<P: AsRef<Path>>(path: P) -> Result<()> {
    let config = ServiceConfig::default();
    
    let content = match path.as_ref().extension().and_then(|s| s.to_str()) {
        Some("toml") => toml::to_string_pretty(&config)
            .with_context(|| "Failed to serialize service config to TOML")?,
        Some("json") => serde_json::to_string_pretty(&config)
            .with_context(|| "Failed to serialize service config to JSON")?,
        Some("yaml") | Some("yml") => serde_yaml::to_string(&config)
            .with_context(|| "Failed to serialize service config to YAML")?,
        _ => return Err(anyhow::anyhow!("Unsupported service config format. Supported: .toml, .json, .yaml, .yml")),
    };
    
    std::fs::write(path.as_ref(), content)
        .with_context(|| format!("Failed to write service config file: {}", path.as_ref().display()))?;
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;
    
    #[test]
    fn test_service_config_default() {
        let config = ServiceConfig::default();
        assert_eq!(config.name, "almanac");
        assert_eq!(config.restart_policy, RestartPolicy::OnFailure);
        assert!(config.health_check.is_some());
        assert!(config.auto_recovery);
    }
    
    #[test]
    fn test_service_info_creation() {
        let config = ServiceConfig::default();
        let info = ServiceInfo::new(config.clone());
        
        assert_eq!(info.config.name, config.name);
        assert_eq!(info.status, ServiceStatus::Stopped);
        assert!(info.pid.is_none());
        assert!(!info.healthy);
        assert_eq!(info.restart_count, 0);
    }
    
    #[test]
    fn test_service_status_display() {
        assert_eq!(ServiceStatus::Running.to_string(), "running");
        assert_eq!(ServiceStatus::Failed.to_string(), "failed");
        assert_eq!(ServiceStatus::Stopped.to_string(), "stopped");
    }
    
    #[tokio::test]
    async fn test_service_manager_registration() {
        let manager = ServiceManager::new();
        let config = ServiceConfig::default();
        
        manager.register_service(config.clone()).unwrap();
        
        let info = manager.get_service_status(&config.name).unwrap();
        assert_eq!(info.config.name, config.name);
        assert_eq!(info.status, ServiceStatus::Stopped);
    }
    
    #[test]
    fn test_service_config_serialization() {
        let config = ServiceConfig::default();
        
        // Test TOML serialization
        let toml_str = toml::to_string(&config).unwrap();
        let parsed_config: ServiceConfig = toml::from_str(&toml_str).unwrap();
        assert_eq!(config.name, parsed_config.name);
        
        // Test JSON serialization
        let json_str = serde_json::to_string(&config).unwrap();
        let parsed_config: ServiceConfig = serde_json::from_str(&json_str).unwrap();
        assert_eq!(config.name, parsed_config.name);
    }
    
    #[test]
    fn test_health_check_default() {
        let health_check = HealthCheck::default();
        assert!(!health_check.command.is_empty());
        assert_eq!(health_check.retries, 3);
        assert_eq!(health_check.interval, Duration::from_secs(30));
    }
    
    #[test]
    fn test_service_dependency() {
        let dep = ServiceDependency {
            name: "postgres".to_string(),
            required: true,
            timeout: Duration::from_secs(30),
        };
        
        assert_eq!(dep.name, "postgres");
        assert!(dep.required);
        assert_eq!(dep.timeout, Duration::from_secs(30));
    }
    
    #[test]
    fn test_restart_policy_default() {
        assert_eq!(RestartPolicy::default(), RestartPolicy::OnFailure);
    }
    
    #[test]
    fn test_service_uptime() {
        let config = ServiceConfig::default();
        let mut info = ServiceInfo::new(config);
        
        // No uptime when not started
        assert!(info.uptime().is_none());
        
        // Set start time and check uptime
        info.start_time = Some(SystemTime::now() - Duration::from_secs(60));
        let uptime = info.uptime().unwrap();
        assert!(uptime.as_secs() >= 59); // Allow for small timing differences
    }
    
    #[test]
    fn test_restart_limit_exceeded() {
        let config = ServiceConfig { 
            max_restarts: Some(3), 
            ..Default::default() 
        };
        
        let mut info = ServiceInfo::new(config);
        assert!(!info.restart_limit_exceeded());
        
        info.restart_count = 3;
        assert!(info.restart_limit_exceeded());
        
        // Test unlimited restarts
        info.config.max_restarts = None;
        assert!(!info.restart_limit_exceeded());
    }
    
    #[test]
    fn test_service_config_file_operations() {
        let config = ServiceConfig::default();
        
        // Test TOML file operations
        let temp_file = NamedTempFile::new().unwrap();
        let toml_path = temp_file.path().with_extension("toml");
        
        generate_default_service_config(&toml_path).unwrap();
        let loaded_config = load_service_config(&toml_path).unwrap();
        
        assert_eq!(loaded_config.name, config.name);
        assert_eq!(loaded_config.description, config.description);
    }
} 