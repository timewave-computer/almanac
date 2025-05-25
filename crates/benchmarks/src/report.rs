// report.rs - Benchmark reporting module
//
// Purpose: Provides tools for generating detailed reports of benchmark results

use std::collections::HashMap;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use chrono::{DateTime, Utc};
use serde::{Serialize, Deserialize};
use indexer_core::Error;
use super::BenchmarkReport;

/// A comparison between benchmark runs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkComparison {
    /// Name of the baseline benchmark
    pub baseline_name: String,
    
    /// Timestamp of the baseline benchmark
    pub baseline_timestamp: DateTime<Utc>,
    
    /// Name of the current benchmark
    pub current_name: String,
    
    /// Timestamp of the current benchmark
    pub current_timestamp: DateTime<Utc>,
    
    /// Percent change in operations per second
    pub ops_per_second_change: f64,
    
    /// Percent change in throughput
    pub throughput_change: f64,
    
    /// Detailed changes by metric
    pub metric_changes: HashMap<String, f64>,
}

/// Compare two benchmark reports
pub fn compare_reports(baseline: &BenchmarkReport, current: &BenchmarkReport) -> BenchmarkComparison {
    let mut metric_changes = HashMap::new();
    
    // Calculate changes for each metric in the summary
    for (key, current_value) in &current.summary {
        if let Some(baseline_value) = baseline.summary.get(key) {
            if *baseline_value != 0.0 {
                let percent_change = (current_value - baseline_value) / baseline_value * 100.0;
                metric_changes.insert(key.clone(), percent_change);
            }
        }
    }
    
    // Extract specific metrics for the comparison
    let ops_per_second_change = metric_changes.get("avg_ops_per_second").copied().unwrap_or(0.0);
    let throughput_change = metric_changes.get("avg_throughput").copied().unwrap_or(0.0);
    
    BenchmarkComparison {
        baseline_name: baseline.name.clone(),
        baseline_timestamp: baseline.timestamp,
        current_name: current.name.clone(),
        current_timestamp: current.timestamp,
        ops_per_second_change,
        throughput_change,
        metric_changes,
    }
}

/// Format for benchmark reports
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReportFormat {
    /// JSON format
    Json,
    
    /// Markdown format
    Markdown,
}

/// Generate a report in the specified format
pub fn generate_report(
    report: &BenchmarkReport, 
    format: ReportFormat, 
    path: &Path,
) -> Result<(), Error> {
    match format {
        ReportFormat::Json => {
            let json = serde_json::to_string_pretty(report)
                .map_err(|e| Error::generic(format!("Failed to serialize report: {}", e)))?;
            
            let mut file = File::create(path)
                .map_err(Error::IO)?;
            
            file.write_all(json.as_bytes())
                .map_err(Error::IO)?;
        }
        ReportFormat::Markdown => {
            let markdown = generate_markdown_report(report)?;
            
            let mut file = File::create(path)
                .map_err(Error::IO)?;
            
            file.write_all(markdown.as_bytes())
                .map_err(Error::IO)?;
        }
    }
    
    Ok(())
}

/// Generate a markdown report
fn generate_markdown_report(report: &BenchmarkReport) -> Result<String, Error> {
    let mut markdown = String::new();
    
    // Report header
    markdown.push_str(&format!("# Benchmark Report: {}\n\n", report.name));
    markdown.push_str(&format!("Generated: {}\n\n", report.timestamp.to_rfc3339()));
    
    // Summary section
    markdown.push_str("## Summary\n\n");
    markdown.push_str("| Metric | Value |\n");
    markdown.push_str("|--------|-------|\n");
    
    for (key, value) in &report.summary {
        let formatted_key = key.replace('_', " ");
        let formatted_value = format_metric_value(key, *value);
        markdown.push_str(&format!("| {} | {} |\n", formatted_key, formatted_value));
    }
    
    markdown.push('\n');
    
    // Measurements section
    markdown.push_str("## Measurements\n\n");
    markdown.push_str("| Name | Duration | Operations | Data Size | Ops/s | Throughput |\n");
    markdown.push_str("|------|----------|------------|-----------|-------|------------|\n");
    
    for measurement in &report.measurements {
        let ops_per_second = measurement.ops_per_second();
        let throughput = measurement.throughput();
        
        markdown.push_str(&format!(
            "| {} | {:.2} ms | {} | {} bytes | {:.2} ops/s | {:.2} bytes/s |\n",
            measurement.name,
            measurement.duration.as_secs_f64() * 1000.0,
            measurement.operations,
            measurement.data_size,
            ops_per_second,
            throughput,
        ));
    }
    
    markdown.push('\n');
    
    // Additional metrics section if any measurement has metrics
    if report.measurements.iter().any(|m| !m.metrics.is_empty()) {
        markdown.push_str("## Additional Metrics\n\n");
        
        // Get all metric keys
        let mut all_metrics = std::collections::HashSet::new();
        for measurement in &report.measurements {
            for key in measurement.metrics.keys() {
                all_metrics.insert(key.clone());
            }
        }
        
        // Create table header
        markdown.push_str("| Name |");
        for key in &all_metrics {
            markdown.push_str(&format!(" {} |", key.replace('_', " ")));
        }
        markdown.push('\n');
        
        // Create table separator
        markdown.push_str("|------|");
        for _ in &all_metrics {
            markdown.push_str("--------|");
        }
        markdown.push('\n');
        
        // Add rows for each measurement
        for measurement in &report.measurements {
            markdown.push_str(&format!("| {} |", measurement.name));
            
            for key in &all_metrics {
                if let Some(value) = measurement.metrics.get(key) {
                    markdown.push_str(&format!(" {:.2} |", value));
                } else {
                    markdown.push_str(" - |");
                }
            }
            
            markdown.push('\n');
        }
    }
    
    Ok(markdown)
}

/// Format a metric value based on its key
fn format_metric_value(key: &str, value: f64) -> String {
    if key.contains("time") && !key.contains("total") {
        format!("{:.2} ms", value)
    } else if key.contains("throughput") {
        format!("{:.2} bytes/s", value)
    } else if key.contains("ops_per_second") {
        format!("{:.2} ops/s", value)
    } else if key.contains("percentage") {
        format!("{:.2}%", value)
    } else {
        format!("{:.2}", value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Measurement;
    use std::time::Duration;
    
    #[test]
    fn test_compare_reports() {
        // Create baseline measurements
        let baseline_measurement = Measurement::new(
            "test",
            Duration::from_millis(100),
            1000,
            10000,
        );
        
        // Create current measurements (25% faster)
        let current_measurement = Measurement::new(
            "test",
            Duration::from_millis(75),
            1000,
            10000,
        );
        
        // Create reports
        let baseline_report = BenchmarkReport::new("baseline", vec![baseline_measurement]);
        let current_report = BenchmarkReport::new("current", vec![current_measurement]);
        
        // Compare reports
        let comparison = compare_reports(&baseline_report, &current_report);
        
        // Verify operations per second change (expect 33% improvement)
        assert!(comparison.ops_per_second_change > 30.0 && comparison.ops_per_second_change < 35.0);
    }
    
    #[test]
    fn test_generate_markdown_report() {
        // Create measurements
        let measurement = Measurement::new(
            "test",
            Duration::from_millis(100),
            1000,
            10000,
        ).with_metric("avg_response_time_ms", 10.0);
        
        // Create report
        let report = BenchmarkReport::new("Test Report", vec![measurement]);
        
        // Generate markdown
        let markdown = generate_markdown_report(&report).unwrap();
        
        // Verify markdown contains expected content
        assert!(markdown.contains("# Benchmark Report: Test Report"));
        assert!(markdown.contains("| test | 100.00 ms | 1000 | 10000 bytes | 10000.00 ops/s | 100000.00 bytes/s |"));
    }
} 