// suite.rs - Benchmark suite implementation
//
// Purpose: Defines a structured suite of benchmarks for testing
// different components of the indexer system

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use serde::{Serialize, Deserialize};
use indexer_core::Error;
use super::{Benchmark, Measurement, BenchmarkReport};

/// A benchmark test case
#[derive(Debug)]
pub struct BenchmarkTestCase<T, R> {
    /// Name of the test case
    name: String,
    
    /// Description of what the test case measures
    description: String,
    
    /// The test function to execute
    test_fn: Box<dyn Fn(T) -> Result<(R, u64, u64), Error> + Send + Sync>,
    
    /// Parameters for the test
    params: T,
}

impl<T: 'static, R: 'static> BenchmarkTestCase<T, R> 
where 
    T: Clone + Send + Sync,
    R: Send + Sync,
{
    /// Create a new benchmark test case
    pub fn new<F>(
        name: &str,
        description: &str,
        test_fn: F,
        params: T,
    ) -> Self 
    where
        F: Fn(T) -> Result<(R, u64, u64), Error> + Send + Sync + 'static,
    {
        Self {
            name: name.to_string(),
            description: description.to_string(),
            test_fn: Box::new(test_fn),
            params,
        }
    }
    
    /// Run the test case and measure performance
    pub fn run(&self) -> Result<Measurement, Error> {
        let start = std::time::Instant::now();
        let (_, ops, data_size) = (self.test_fn)(self.params.clone())?;
        let duration = start.elapsed();
        
        Ok(Measurement::new(&self.name, duration, ops, data_size))
    }
    
    /// Get the name of the test case
    pub fn name(&self) -> &str {
        &self.name
    }
    
    /// Get the description of the test case
    pub fn description(&self) -> &str {
        &self.description
    }
}

/// A suite of benchmarks to run
pub struct BenchmarkSuite {
    /// Name of the benchmark suite
    name: String,
    
    /// Description of what the suite measures
    description: String,
    
    /// Test cases in the suite
    test_cases: Arc<Mutex<Vec<Box<dyn BenchmarkTestRunner + Send + Sync>>>>,
    
    /// Setup function to run before benchmarks
    setup: Option<Box<dyn Fn() -> Result<(), Error> + Send + Sync>>,
    
    /// Teardown function to run after benchmarks
    teardown: Option<Box<dyn Fn() -> Result<(), Error> + Send + Sync>>,
}

/// Trait for running benchmark tests
pub trait BenchmarkTestRunner {
    /// Run the benchmark and produce a measurement
    fn run(&self) -> Result<Measurement, Error>;
    
    /// Get the name of the test
    fn name(&self) -> &str;
    
    /// Get the description of the test
    fn description(&self) -> &str;
}

impl<T, R> BenchmarkTestRunner for BenchmarkTestCase<T, R> 
where 
    T: Clone + Send + Sync + 'static,
    R: Send + Sync + 'static,
{
    fn run(&self) -> Result<Measurement, Error> {
        BenchmarkTestCase::run(self)
    }
    
    fn name(&self) -> &str {
        BenchmarkTestCase::name(self)
    }
    
    fn description(&self) -> &str {
        BenchmarkTestCase::description(self)
    }
}

impl BenchmarkSuite {
    /// Create a new benchmark suite
    pub fn new(name: &str, description: &str) -> Self {
        Self {
            name: name.to_string(),
            description: description.to_string(),
            test_cases: Arc::new(Mutex::new(Vec::new())),
            setup: None,
            teardown: None,
        }
    }
    
    /// Add a setup function to run before benchmarks
    pub fn with_setup<F>(mut self, setup_fn: F) -> Self 
    where
        F: Fn() -> Result<(), Error> + Send + Sync + 'static,
    {
        self.setup = Some(Box::new(setup_fn));
        self
    }
    
    /// Add a teardown function to run after benchmarks
    pub fn with_teardown<F>(mut self, teardown_fn: F) -> Self 
    where
        F: Fn() -> Result<(), Error> + Send + Sync + 'static,
    {
        self.teardown = Some(Box::new(teardown_fn));
        self
    }
    
    /// Add a test case to the suite
    pub async fn add_test_case<T, R>(&self, test_case: BenchmarkTestCase<T, R>) 
    where
        T: Clone + Send + Sync + 'static,
        R: Send + Sync + 'static,
    {
        let mut test_cases = self.test_cases.lock().await;
        test_cases.push(Box::new(test_case));
    }
    
    /// Run all benchmarks in the suite
    pub async fn run(&self) -> Result<BenchmarkReport, Error> {
        // Run setup if provided
        if let Some(setup) = &self.setup {
            setup()?;
        }
        
        // Run all test cases
        let mut measurements = Vec::new();
        let test_cases = self.test_cases.lock().await;
        
        for test_case in test_cases.iter() {
            let measurement = test_case.run()?;
            measurements.push(measurement);
        }
        
        // Run teardown if provided
        if let Some(teardown) = &self.teardown {
            teardown()?;
        }
        
        // Create report
        let report = BenchmarkReport::new(&self.name, measurements);
        
        Ok(report)
    }
}

/// Configuration for running benchmark suites
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkConfig {
    /// Number of iterations to run each benchmark
    pub iterations: usize,
    
    /// Warm-up iterations (not included in measurements)
    pub warmup_iterations: usize,
    
    /// Whether to include detailed measurements in reports
    pub detailed_report: bool,
    
    /// Path to save reports
    pub report_path: Option<String>,
    
    /// Additional configuration options
    pub options: HashMap<String, String>,
}

impl Default for BenchmarkConfig {
    fn default() -> Self {
        Self {
            iterations: 3,
            warmup_iterations: 1,
            detailed_report: true,
            report_path: Some("benchmark_reports".to_string()),
            options: HashMap::new(),
        }
    }
}

/// Runner for executing benchmark suites
pub struct BenchmarkRunner {
    /// Configuration for the benchmark runner
    config: BenchmarkConfig,
    
    /// Suites to run
    suites: Vec<Arc<BenchmarkSuite>>,
}

impl BenchmarkRunner {
    /// Create a new benchmark runner with default configuration
    pub fn new() -> Self {
        Self {
            config: BenchmarkConfig::default(),
            suites: Vec::new(),
        }
    }
    
    /// Create a new benchmark runner with custom configuration
    pub fn with_config(config: BenchmarkConfig) -> Self {
        Self {
            config,
            suites: Vec::new(),
        }
    }
    
    /// Add a benchmark suite to run
    pub fn add_suite(&mut self, suite: Arc<BenchmarkSuite>) {
        self.suites.push(suite);
    }
    
    /// Run all benchmark suites
    pub async fn run_all(&self) -> Result<Vec<BenchmarkReport>, Error> {
        let mut reports = Vec::new();
        
        for suite in &self.suites {
            // Run warmup iterations
            for _ in 0..self.config.warmup_iterations {
                let _ = suite.run().await?;
            }
            
            // Run measured iterations
            let mut suite_measurements = Vec::new();
            for i in 0..self.config.iterations {
                let report = suite.run().await?;
                suite_measurements.extend(report.measurements.clone());
                
                // Save individual iteration report if configured
                if let Some(path) = &self.config.report_path {
                    let iteration_path = format!("{}/{}_{}.json", path, suite.name, i);
                    let iteration_report = BenchmarkReport::new(&format!("{}_{}", suite.name, i), report.measurements);
                    iteration_report.save_to_file(&iteration_path)?;
                }
            }
            
            // Create aggregated report
            let report = BenchmarkReport::new(&suite.name, suite_measurements);
            
            // Save report if configured
            if let Some(path) = &self.config.report_path {
                std::fs::create_dir_all(path).map_err(|e| Error::Io(e))?;
                let report_path = format!("{}/{}.json", path, suite.name);
                report.save_to_file(&report_path)?;
            }
            
            reports.push(report);
        }
        
        Ok(reports)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_benchmark_suite() -> Result<(), Error> {
        // Create a test case
        let test_case = BenchmarkTestCase::new(
            "test_case",
            "A test case that simulates work",
            |_: ()| {
                // Simulate some work
                std::thread::sleep(Duration::from_millis(10));
                Ok(((), 100, 1024))
            },
            (),
        );
        
        // Create a suite
        let suite = BenchmarkSuite::new(
            "test_suite",
            "A test benchmark suite",
        );
        
        // Add the test case
        suite.add_test_case(test_case).await;
        
        // Run the suite
        let report = suite.run().await?;
        
        // Verify the report
        assert_eq!(report.name, "test_suite");
        assert_eq!(report.measurements.len(), 1);
        assert_eq!(report.measurements[0].name, "test_case");
        assert_eq!(report.measurements[0].operations, 100);
        assert_eq!(report.measurements[0].data_size, 1024);
        
        Ok(())
    }
    
    #[tokio::test]
    async fn test_benchmark_runner() -> Result<(), Error> {
        // Create a config
        let config = BenchmarkConfig {
            iterations: 2,
            warmup_iterations: 1,
            detailed_report: true,
            report_path: None,
            options: HashMap::new(),
        };
        
        // Create a test case
        let test_case = BenchmarkTestCase::new(
            "test_case",
            "A test case that simulates work",
            |_: ()| {
                // Simulate some work
                std::thread::sleep(Duration::from_millis(5));
                Ok(((), 50, 512))
            },
            (),
        );
        
        // Create a suite
        let suite = Arc::new(BenchmarkSuite::new(
            "test_suite",
            "A test benchmark suite",
        ));
        
        // Add the test case
        suite.add_test_case(test_case).await;
        
        // Create a runner
        let mut runner = BenchmarkRunner::with_config(config);
        
        // Add the suite
        runner.add_suite(suite);
        
        // Run all suites
        let reports = runner.run_all().await?;
        
        // Verify the results
        assert_eq!(reports.len(), 1);
        assert_eq!(reports[0].name, "test_suite");
        assert_eq!(reports[0].measurements.len(), 2); // 2 iterations
        
        Ok(())
    }
} 