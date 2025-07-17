// Comprehensive testing framework for the Almanac indexer
//
// This crate provides a complete testing suite including integration tests,
// property-based tests, chaos engineering, and performance benchmarks

pub mod benchmarks;
pub mod chaos;
pub mod fixtures;
pub mod integration;
pub mod performance;
pub mod property;

// Distributed testing module disabled due to missing dependencies
// #[cfg(feature = "distributed")]
// pub mod distributed;

use std::time::{Duration, Instant};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use tracing::{info, warn, error};

/// Test configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestConfig {
    /// Test environment name
    pub environment: String,
    /// Database configuration for testing
    pub database: TestDatabaseConfig,
    /// API configuration for testing
    pub api: TestApiConfig,
    /// Blockchain configurations
    pub chains: TestChainConfig,
    /// Performance testing configuration
    pub performance: PerformanceTestConfig,
    /// Chaos testing configuration
    pub chaos: ChaosTestConfig,
}

/// Database testing configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestDatabaseConfig {
    /// PostgreSQL test database URL
    pub postgres_url: Option<String>,
    /// RocksDB test directory
    pub rocks_path: Option<String>,
    /// Use in-memory database for tests
    pub use_in_memory: bool,
    /// Clean database between tests
    pub clean_between_tests: bool,
}

/// API testing configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestApiConfig {
    /// Test server port
    pub port: u16,
    /// Enable GraphQL testing
    pub test_graphql: bool,
    /// Enable REST API testing
    pub test_rest: bool,
    /// Enable WebSocket testing
    pub test_websockets: bool,
    /// Request timeout for tests
    pub request_timeout_ms: u64,
}

/// Chain testing configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestChainConfig {
    /// Ethereum test configuration
    pub ethereum: Option<EthereumTestConfig>,
    /// Cosmos test configuration
    pub cosmos: Option<CosmosTestConfig>,
}

/// Ethereum testing configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EthereumTestConfig {
    /// Test RPC URL (e.g., Anvil)
    pub rpc_url: String,
    /// Test chain ID
    pub chain_id: u64,
    /// Test contract addresses
    pub test_contracts: Vec<String>,
}

/// Cosmos testing configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CosmosTestConfig {
    /// Test RPC URL
    pub rpc_url: String,
    /// Test chain ID
    pub chain_id: String,
    /// Test contract addresses
    pub test_contracts: Vec<String>,
}

/// Performance testing configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceTestConfig {
    /// Number of concurrent connections for load testing
    pub concurrent_connections: usize,
    /// Duration for load tests
    pub test_duration_seconds: u64,
    /// Target requests per second
    pub target_rps: f64,
    /// Memory limit for tests (MB)
    pub memory_limit_mb: u64,
    /// CPU limit for tests (percentage)
    pub cpu_limit_percent: f64,
}

/// Chaos testing configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChaosTestConfig {
    /// Enable chaos testing
    pub enabled: bool,
    /// Network failure injection rate (0.0 to 1.0)
    pub network_failure_rate: f64,
    /// Database failure injection rate (0.0 to 1.0)
    pub database_failure_rate: f64,
    /// Memory pressure injection
    pub memory_pressure: bool,
    /// CPU pressure injection
    pub cpu_pressure: bool,
}

/// Test result summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestResult {
    /// Test name
    pub name: String,
    /// Test success status
    pub success: bool,
    /// Test duration
    pub duration: Duration,
    /// Test output/logs
    pub output: String,
    /// Error message if failed
    pub error: Option<String>,
    /// Test metrics
    pub metrics: TestMetrics,
}

/// Test metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestMetrics {
    /// Memory usage during test (MB)
    pub memory_usage_mb: f64,
    /// CPU usage during test (percentage)
    pub cpu_usage_percent: f64,
    /// Database operations count
    pub db_operations: u64,
    /// API requests count
    pub api_requests: u64,
    /// Events processed
    pub events_processed: u64,
}

/// Test suite runner
pub struct TestSuite {
    config: TestConfig,
    results: Vec<TestResult>,
}

impl TestSuite {
    /// Create a new test suite
    pub fn new(config: TestConfig) -> Self {
        Self {
            config,
            results: Vec::new(),
        }
    }

    /// Run all tests
    pub async fn run_all(&mut self) -> Result<TestSummary> {
        info!("Starting comprehensive test suite");
        let start_time = Instant::now();

        // Run integration tests
        if let Err(e) = self.run_integration_tests().await {
            error!("Integration tests failed: {}", e);
        }

        // Run property-based tests
        if let Err(e) = self.run_property_tests().await {
            error!("Property-based tests failed: {}", e);
        }

        // Run performance tests
        if let Err(e) = self.run_performance_tests().await {
            error!("Performance tests failed: {}", e);
        }

        // Run chaos tests if enabled
        if self.config.chaos.enabled {
            if let Err(e) = self.run_chaos_tests().await {
                error!("Chaos tests failed: {}", e);
            }
        }

        let total_duration = start_time.elapsed();
        let summary = self.generate_summary(total_duration);

        info!("Test suite completed in {:?}", total_duration);
        info!("Results: {} passed, {} failed", summary.passed, summary.failed);

        Ok(summary)
    }

    /// Run integration tests
    pub async fn run_integration_tests(&mut self) -> Result<()> {
        info!("Starting integration tests");
        
        // Clone configs to avoid borrowing issues
        let db_config = self.config.database.clone();
        let api_config = self.config.api.clone();
        let chains_config = self.config.chains.clone();
        
        self.run_test("database_integration", move || {
            let db_cfg = db_config.clone();
            async move {
                integration::test_database_integration(&db_cfg).await
            }
        }).await?;
        
        self.run_test("api_integration", move || {
            let api_cfg = api_config.clone();
            async move {
                integration::test_api_integration(&api_cfg).await
            }
        }).await?;
        
        self.run_test("cross_chain_integration", move || {
            let chains_cfg = chains_config.clone();
            async move {
                integration::test_cross_chain_integration(&chains_cfg).await
            }
        }).await?;
        
        Ok(())
    }

    /// Run property-based tests
    async fn run_property_tests(&mut self) -> Result<()> {
        info!("Running property-based tests");

        // Event property tests
        self.run_test("event_properties", || {
            property::test_event_properties()
        }).await?;

        // Storage property tests
        self.run_test("storage_properties", || {
            property::test_storage_properties()
        }).await?;

        // API property tests
        self.run_test("api_properties", || {
            property::test_api_properties()
        }).await?;

        Ok(())
    }

    /// Run performance tests
    pub async fn run_performance_tests(&mut self) -> Result<()> {
        info!("Starting performance tests");
        
        let perf_config = self.config.performance.clone();
        self.run_test("load_test", move || {
            let cfg = perf_config.clone();
            async move {
                let results = performance::run_load_test(&cfg).await?;
                Ok(format!("Load test completed: {} requests", results.total_requests))
            }
        }).await?;
        
        let perf_config = self.config.performance.clone();
        self.run_test("stress_test", move || {
            let cfg = perf_config.clone();
            async move {
                let results = performance::run_stress_test(&cfg).await?;
                Ok(format!("Stress test completed: {} requests", results.total_requests))
            }
        }).await?;
        
        let perf_config = self.config.performance.clone();
        self.run_test("memory_leak_test", move || {
            let cfg = perf_config.clone();
            async move {
                let results = performance::run_memory_leak_test(&cfg).await?;
                Ok(format!("Memory leak test completed: {} requests", results.total_requests))
            }
        }).await?;
        
        Ok(())
    }
    
    /// Run chaos tests
    async fn run_chaos_tests(&mut self) -> Result<()> {
        info!("Starting chaos tests");
        
        let chaos_config = self.config.chaos.clone();
        self.run_test("network_chaos", move || {
            let config = chaos_config.clone();
            async move {
                chaos::test_network_failures(config).await
            }
        }).await?;
        
        let chaos_config = self.config.chaos.clone();
        self.run_test("database_chaos", move || {
            let config = chaos_config.clone();
            async move {
                chaos::test_database_failures(config).await
            }
        }).await?;
        
        let chaos_config = self.config.chaos.clone();
        self.run_test("resource_chaos", move || {
            let config = chaos_config.clone();
            async move {
                chaos::test_resource_exhaustion(config).await
            }
        }).await?;
        
        Ok(())
    }

    /// Run a single test with metrics collection
    async fn run_test<F, Fut>(&mut self, name: &str, test_fn: F) -> Result<()>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<String>>,
    {
        let start_time = Instant::now();
        let start_memory = self.get_memory_usage();

        info!("Running test: {}", name);

        let result = match test_fn().await {
            Ok(output) => TestResult {
                name: name.to_string(),
                success: true,
                duration: start_time.elapsed(),
                output,
                error: None,
                metrics: self.collect_metrics(start_memory, start_time),
            },
            Err(e) => TestResult {
                name: name.to_string(),
                success: false,
                duration: start_time.elapsed(),
                output: String::new(),
                error: Some(e.to_string()),
                metrics: self.collect_metrics(start_memory, start_time),
            },
        };

        if result.success {
            info!("✅ Test {} passed in {:?}", name, result.duration);
        } else {
            warn!("❌ Test {} failed in {:?}: {}", name, result.duration, 
                  result.error.as_ref().unwrap_or(&"Unknown error".to_string()));
        }

        self.results.push(result);
        Ok(())
    }

    /// Collect test metrics
    fn collect_metrics(&self, start_memory: f64, _start_time: Instant) -> TestMetrics {
        let current_memory = self.get_memory_usage();
        let memory_delta = current_memory - start_memory;

        TestMetrics {
            memory_usage_mb: memory_delta.max(0.0),
            cpu_usage_percent: self.get_cpu_usage(),
            db_operations: 0, // Would be collected from actual monitoring
            api_requests: 0,  // Would be collected from actual monitoring
            events_processed: 0, // Would be collected from actual monitoring
        }
    }

    /// Get current memory usage in MB
    fn get_memory_usage(&self) -> f64 {
        #[cfg(feature = "performance")]
        {
            if let Some(usage) = memory_stats::memory_stats() {
                return usage.physical_mem as f64 / 1024.0 / 1024.0;
            }
        }
        0.0
    }

    /// Get current CPU usage percentage
    fn get_cpu_usage(&self) -> f64 {
        // Simplified CPU usage - would use proper system monitoring in real implementation
        0.0
    }

    /// Generate test summary
    fn generate_summary(&self, total_duration: Duration) -> TestSummary {
        let total = self.results.len();
        let passed = self.results.iter().filter(|r| r.success).count();
        let failed = total - passed;

        let avg_duration = if total > 0 {
            self.results.iter().map(|r| r.duration).sum::<Duration>() / total as u32
        } else {
            Duration::ZERO
        };

        TestSummary {
            total,
            passed,
            failed,
            total_duration,
            average_test_duration: avg_duration,
            test_results: self.results.clone(),
        }
    }
}

/// Test summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestSummary {
    pub total: usize,
    pub passed: usize,
    pub failed: usize,
    pub total_duration: Duration,
    pub average_test_duration: Duration,
    pub test_results: Vec<TestResult>,
}

impl TestSummary {
    /// Generate a detailed report
    pub fn generate_report(&self) -> String {
        let mut report = String::new();
        
        report.push_str("# Almanac Test Suite Report\n\n");
        report.push_str(&format!("**Total Tests:** {}\n", self.total));
        report.push_str(&format!("**Passed:** {}\n", self.passed));
        report.push_str(&format!("**Failed:** {}\n", self.failed));
        report.push_str(&format!("**Success Rate:** {:.1}%\n", 
            (self.passed as f64 / self.total as f64) * 100.0));
        report.push_str(&format!("**Total Duration:** {:?}\n", self.total_duration));
        report.push_str(&format!("**Average Test Duration:** {:?}\n\n", self.average_test_duration));

        if self.failed > 0 {
            report.push_str("## Failed Tests\n\n");
            for result in &self.test_results {
                if !result.success {
                    report.push_str(&format!("### {}\n", result.name));
                    report.push_str(&format!("**Duration:** {:?}\n", result.duration));
                    if let Some(error) = &result.error {
                        report.push_str(&format!("**Error:** {}\n", error));
                    }
                    report.push_str("\n");
                }
            }
        }

        report.push_str("## All Test Results\n\n");
        for result in &self.test_results {
            let status = if result.success { "✅ PASS" } else { "❌ FAIL" };
            report.push_str(&format!("- {} {} ({:?})\n", status, result.name, result.duration));
        }

        report
    }

    /// Save report to file
    pub fn save_report(&self, path: &str) -> Result<()> {
        let report = self.generate_report();
        std::fs::write(path, report)?;
        Ok(())
    }

    /// Export results as JSON
    pub fn export_json(&self) -> Result<String> {
        Ok(serde_json::to_string_pretty(self)?)
    }
}

/// Configuration for benchmark tests
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkConfig {
    pub enabled: bool,
    pub iterations: u32,
    pub warmup_iterations: u32,
    pub target_ops_per_second: u32,
    pub max_duration_seconds: u32,
    pub operations_count: u32,
    pub concurrent_users: u32,
}

impl Default for BenchmarkConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            iterations: 1000,
            warmup_iterations: 100,
            target_ops_per_second: 1000,
            max_duration_seconds: 300,
            operations_count: 0,
            concurrent_users: 0,
        }
    }
}

/// Default test configuration
impl Default for TestConfig {
    fn default() -> Self {
        Self {
            environment: "test".to_string(),
            database: TestDatabaseConfig {
                postgres_url: Some("postgres://test:test@localhost:5433/indexer_test".to_string()),
                rocks_path: Some("./test_data/rocks".to_string()),
                use_in_memory: true,
                clean_between_tests: true,
            },
            api: TestApiConfig {
                port: 8081,
                test_graphql: true,
                test_rest: true,
                test_websockets: true,
                request_timeout_ms: 5000,
            },
            chains: TestChainConfig {
                ethereum: Some(EthereumTestConfig {
                    rpc_url: "http://localhost:8545".to_string(),
                    chain_id: 1337,
                    test_contracts: vec![
                        "0x1234567890123456789012345678901234567890".to_string(),
                    ],
                }),
                cosmos: Some(CosmosTestConfig {
                    rpc_url: "http://localhost:26657".to_string(),
                    chain_id: "test-chain-1".to_string(),
                    test_contracts: vec![
                        "cosmos1234567890123456789012345678901234567890".to_string(),
                    ],
                }),
            },
            performance: PerformanceTestConfig {
                concurrent_connections: 100,
                test_duration_seconds: 60,
                target_rps: 1000.0,
                memory_limit_mb: 1024,
                cpu_limit_percent: 80.0,
            },
            chaos: ChaosTestConfig {
                enabled: false, // Disabled by default for safety
                network_failure_rate: 0.1,
                database_failure_rate: 0.05,
                memory_pressure: false,
                cpu_pressure: false,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = TestConfig::default();
        assert_eq!(config.environment, "test");
        assert!(config.database.use_in_memory);
        assert!(config.api.test_graphql);
        assert!(!config.chaos.enabled);
    }

    #[tokio::test]
    async fn test_suite_creation() {
        let config = TestConfig::default();
        let suite = TestSuite::new(config);
        assert_eq!(suite.results.len(), 0);
    }

    #[test]
    fn test_summary_generation() {
        let results = vec![
            TestResult {
                name: "test1".to_string(),
                success: true,
                duration: Duration::from_millis(100),
                output: "Success".to_string(),
                error: None,
                metrics: TestMetrics {
                    memory_usage_mb: 10.0,
                    cpu_usage_percent: 5.0,
                    db_operations: 5,
                    api_requests: 10,
                    events_processed: 100,
                },
            },
            TestResult {
                name: "test2".to_string(),
                success: false,
                duration: Duration::from_millis(200),
                output: String::new(),
                error: Some("Test failed".to_string()),
                metrics: TestMetrics {
                    memory_usage_mb: 15.0,
                    cpu_usage_percent: 8.0,
                    db_operations: 3,
                    api_requests: 5,
                    events_processed: 50,
                },
            },
        ];

        let mut suite = TestSuite::new(TestConfig::default());
        suite.results = results;

        let summary = suite.generate_summary(Duration::from_secs(1));
        assert_eq!(summary.total, 2);
        assert_eq!(summary.passed, 1);
        assert_eq!(summary.failed, 1);
    }
} 