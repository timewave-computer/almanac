// Almanac benchmarking suite for storage and indexing performance
//
// Purpose: Provides comprehensive benchmarking tools for measuring and optimizing
// performance across all components of the indexer

use std::collections::HashMap;
use std::fmt;
use std::path::Path;
use std::time::{Duration, Instant};
use chrono::Utc;
use serde::{Serialize, Deserialize};
use indexer_core::Error;
use std::fs::File;

// Benchmark modules
pub mod concurrency;
pub mod load;
pub mod load_test_only;
pub mod postgres_opt;
pub mod postgres_pool;
pub mod profile;
pub mod report;
pub mod rocksdb_opt;
pub mod suite;

/// A measurement of a single benchmark operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Measurement {
    /// Name of the measurement
    pub name: String,
    
    /// Duration of the measurement
    pub duration: Duration,
    
    /// Number of operations performed
    pub operations: u64,
    
    /// Total size of data processed in bytes
    pub data_size: u64,
    
    /// Additional metrics (response time, CPU usage, etc.)
    pub metrics: HashMap<String, f64>,
    
    /// Timestamp when the measurement was taken
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl Measurement {
    /// Create a new measurement
    pub fn new(name: &str, duration: Duration, operations: u64, data_size: u64) -> Self {
        Self {
            name: name.to_string(),
            duration,
            operations,
            data_size,
            metrics: HashMap::new(),
            timestamp: Utc::now(),
        }
    }
    
    /// Calculate operations per second
    pub fn ops_per_second(&self) -> f64 {
        if self.duration.as_secs_f64() > 0.0 {
            self.operations as f64 / self.duration.as_secs_f64()
        } else {
            0.0
        }
    }
    
    /// Calculate throughput in bytes per second
    pub fn throughput(&self) -> f64 {
        if self.duration.as_secs_f64() > 0.0 {
            self.data_size as f64 / self.duration.as_secs_f64()
        } else {
            0.0
        }
    }
    
    /// Add a metric to the measurement
    pub fn with_metric(mut self, key: &str, value: f64) -> Self {
        self.metrics.insert(key.to_string(), value);
        self
    }
}

/// A benchmark that can be executed to collect measurements
#[derive(Debug)]
pub struct Benchmark {
    /// Name of the benchmark
    pub name: String,
    
    /// Current measurement in progress
    current_measurement: Option<(Instant, u64, u64)>,
    
    /// Completed measurements
    pub measurements: Vec<Measurement>,
}

impl Benchmark {
    /// Create a new benchmark
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            current_measurement: None,
            measurements: Vec::new(),
        }
    }
    
    /// Start a measurement
    pub fn start_measurement(&mut self) {
        self.current_measurement = Some((Instant::now(), 0, 0));
    }
    
    /// Record operations and data size for the current measurement
    pub fn record(&mut self, operations: u64, data_size: u64) {
        if let Some((_, ops, size)) = &mut self.current_measurement {
            *ops += operations;
            *size += data_size;
        }
    }
    
    /// Stop the current measurement and add it to the list
    pub fn stop_measurement(&mut self, name: &str) -> Option<Measurement> {
        if let Some((start_time, operations, data_size)) = self.current_measurement.take() {
            let duration = start_time.elapsed();
            let measurement = Measurement::new(name, duration, operations, data_size);
            self.measurements.push(measurement.clone());
            Some(measurement)
        } else {
            None
        }
    }
    
    /// Measure a specific operation
    pub fn measure<F>(&mut self, name: &str, f: F) -> Result<Measurement, Error>
    where
        F: FnOnce() -> Result<(u64, u64), Error>,
    {
        let start_time = Instant::now();
        let (operations, data_size) = f()?;
        let duration = start_time.elapsed();
        
        let measurement = Measurement::new(name, duration, operations, data_size);
        self.measurements.push(measurement.clone());
        
        Ok(measurement)
    }
    
    /// Create a summary of all measurements
    pub fn summary(&self) -> HashMap<String, f64> {
        let mut summary = HashMap::new();
        
        if self.measurements.is_empty() {
            return summary;
        }
        
        // Calculate total and average values
        let total_duration_secs: f64 = self.measurements.iter()
            .map(|m| m.duration.as_secs_f64())
            .sum();
        
        let total_operations: u64 = self.measurements.iter()
            .map(|m| m.operations)
            .sum();
        
        let total_data_size: u64 = self.measurements.iter()
            .map(|m| m.data_size)
            .sum();
        
        let avg_duration_secs = total_duration_secs / self.measurements.len() as f64;
        
        let avg_operations = total_operations as f64 / self.measurements.len() as f64;
        
        let avg_ops_per_second = if avg_duration_secs > 0.0 {
            avg_operations / avg_duration_secs
        } else {
            0.0
        };
        
        let avg_throughput = if avg_duration_secs > 0.0 {
            total_data_size as f64 / total_duration_secs
        } else {
            0.0
        };
        
        // Add summary values
        summary.insert("total_duration_secs".to_string(), total_duration_secs);
        summary.insert("total_operations".to_string(), total_operations as f64);
        summary.insert("total_data_size".to_string(), total_data_size as f64);
        summary.insert("avg_duration_secs".to_string(), avg_duration_secs);
        summary.insert("avg_operations".to_string(), avg_operations);
        summary.insert("avg_ops_per_second".to_string(), avg_ops_per_second);
        summary.insert("avg_throughput".to_string(), avg_throughput);
        
        // Aggregate metrics from all measurements
        let mut metric_counts: HashMap<String, usize> = HashMap::new();
        
        for measurement in &self.measurements {
            for (key, value) in &measurement.metrics {
                let sum_key = format!("sum_{}", key);
                let entry = summary.entry(sum_key).or_insert(0.0);
                *entry += value;
                
                let count = metric_counts.entry(key.clone()).or_insert(0);
                *count += 1;
            }
        }
        
        // Calculate averages for metrics
        for (key, count) in metric_counts {
            let sum_key = format!("sum_{}", key);
            let avg_key = format!("avg_{}", key);
            
            if let Some(sum_value) = summary.get(&sum_key) {
                summary.insert(avg_key, sum_value / count as f64);
            }
        }
        
        summary
    }
    
    /// Convert to a benchmark report
    pub fn to_report(&self) -> BenchmarkReport {
        BenchmarkReport::new(&self.name, self.measurements.clone())
    }
}

/// A timer for measuring durations
#[derive(Debug)]
pub struct Timer {
    /// Start time of the timer
    start_time: Instant,
    
    /// Current elapsed duration
    current_elapsed: Duration,
    
    /// Whether the timer is running
    running: bool,
}

impl Timer {
    /// Start a new timer
    pub fn start() -> Self {
        Self {
            start_time: Instant::now(),
            current_elapsed: Duration::from_secs(0),
            running: true,
        }
    }
    
    /// Create a new timer (not started)
    pub fn new() -> Self {
        Self {
            start_time: Instant::now(),
            current_elapsed: Duration::from_secs(0),
            running: false,
        }
    }
    
    /// Start the timer
    pub fn start_timer(&mut self) {
        self.start_time = Instant::now();
        self.running = true;
    }
    
    /// Stop the timer and return elapsed duration
    pub fn stop(&mut self) -> Duration {
        if self.running {
            self.current_elapsed = self.start_time.elapsed();
            self.running = false;
        }
        self.current_elapsed
    }
    
    /// Reset the timer
    pub fn reset(&mut self) {
        self.start_time = Instant::now();
        self.current_elapsed = Duration::from_secs(0);
        self.running = false;
    }
    
    /// Get the current elapsed time
    pub fn elapsed(&self) -> Duration {
        if self.running {
            self.start_time.elapsed()
        } else {
            self.current_elapsed
        }
    }
}

impl Default for Timer {
    fn default() -> Self {
        Self::new()
    }
}

/// A report of benchmark results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkReport {
    /// Name of the benchmark
    pub name: String,
    
    /// Measurements collected during the benchmark
    pub measurements: Vec<Measurement>,
    
    /// Summary of the benchmark results
    pub summary: HashMap<String, f64>,
    
    /// Timestamp when the report was created
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl BenchmarkReport {
    /// Create a new benchmark report
    pub fn new(name: &str, measurements: Vec<Measurement>) -> Self {
        let mut report = Self {
            name: name.to_string(),
            measurements,
            summary: HashMap::new(),
            timestamp: Utc::now(),
        };
        
        // Calculate summary
        if !report.measurements.is_empty() {
            let benchmark = Benchmark {
                name: name.to_string(),
                current_measurement: None,
                measurements: report.measurements.clone(),
            };
            report.summary = benchmark.summary();
        }
        
        report
    }
    
    /// Save the report to a file
    pub fn save(&self, path: &Path) -> Result<(), Error> {
        let mut file = File::create(path)
            .map_err(Error::IO)?;

        serde_json::to_writer_pretty(&mut file, self)
            .map_err(|e| Error::generic(format!("Failed to serialize report: {}", e)))?;
        
        Ok(())
    }
    
    /// Load a report from a file
    pub fn load(path: &Path) -> Result<Self, Error> {
        let json = std::fs::read_to_string(path)
            .map_err(|e| Error::IO(e))?;
        
        let report: Self = serde_json::from_str(&json)
            .map_err(|e| Error::generic(format!("Failed to deserialize report: {}", e)))?;
        
        Ok(report)
    }
}

impl fmt::Display for BenchmarkReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Benchmark Report: {}", self.name)?;
        writeln!(f, "Generated: {}", self.timestamp.to_rfc3339())?;
        writeln!(f)?;
        
        // Print summary
        writeln!(f, "Summary:")?;
        for (key, value) in &self.summary {
            writeln!(f, "  {}: {:.2}", key, value)?;
        }
        writeln!(f)?;
        
        // Print measurements
        writeln!(f, "Measurements:")?;
        for measurement in &self.measurements {
            writeln!(
                f,
                "  {}: {:.2} ms, {} ops, {} bytes, {:.2} ops/s, {:.2} bytes/s",
                measurement.name,
                measurement.duration.as_secs_f64() * 1000.0,
                measurement.operations,
                measurement.data_size,
                measurement.ops_per_second(),
                measurement.throughput(),
            )?;
            
            // Print additional metrics
            if !measurement.metrics.is_empty() {
                writeln!(f, "    Metrics:")?;
                for (key, value) in &measurement.metrics {
                    writeln!(f, "      {}: {:.2}", key, value)?;
                }
            }
        }
        
        Ok(())
    }
} 