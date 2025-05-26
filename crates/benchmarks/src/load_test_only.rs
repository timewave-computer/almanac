// load_test_only.rs - Standalone test for load module functionality
//
// Purpose: Test the load module in isolation to verify it compiles and works correctly

#[cfg(test)]
mod tests {
    use crate::load::*;
    use std::time::Duration;

    #[test]
    fn test_load_test_stats_calculation() {
        let stats = LoadTestStats {
            total_requests: 1000,
            successful_requests: 950,
            failed_requests: 50,
            total_bytes: 1024 * 1024, // 1MB
            duration: Duration::from_secs(10),
            requests_per_second: 100.0,
            avg_response_time: Duration::from_millis(10),
            p95_response_time: Duration::from_millis(25),
            p99_response_time: Duration::from_millis(50),
            max_response_time: Duration::from_millis(100),
            min_response_time: Duration::from_millis(1),
            error_rate: 0.05,
        };

        // Test basic calculations
        assert_eq!(stats.total_requests, 1000);
        assert_eq!(stats.successful_requests, 950);
        assert_eq!(stats.failed_requests, 50);
        assert_eq!(stats.requests_per_second, 100.0);
        assert_eq!(stats.error_rate, 0.05);
        assert_eq!(stats.total_bytes, 1024 * 1024);

        // Test latency values
        assert_eq!(stats.min_response_time, Duration::from_millis(1));
        assert_eq!(stats.max_response_time, Duration::from_millis(100));
        assert_eq!(stats.avg_response_time, Duration::from_millis(10));
        assert_eq!(stats.p95_response_time, Duration::from_millis(25));
        assert_eq!(stats.p99_response_time, Duration::from_millis(50));
    }

    #[test]
    fn test_load_test_config_creation() {
        let config = LoadTestConfig {
            duration: Duration::from_secs(60),
            concurrency: 10,
            ramp_up: Duration::from_secs(5),
            rate_limit: 100,
            wait_time: Some(Duration::from_millis(100)),
        };

        assert_eq!(config.duration, Duration::from_secs(60));
        assert_eq!(config.concurrency, 10);
        assert_eq!(config.ramp_up, Duration::from_secs(5));
        assert_eq!(config.rate_limit, 100);
        assert_eq!(config.wait_time, Some(Duration::from_millis(100)));
    }

    #[test]
    fn test_load_test_config_default() {
        let config = LoadTestConfig::default();
        
        assert_eq!(config.duration, Duration::from_secs(30));
        assert_eq!(config.concurrency, 10);
        assert_eq!(config.ramp_up, Duration::from_secs(5));
        assert_eq!(config.rate_limit, 0); // 0 means no limit
        assert_eq!(config.wait_time, None);
    }

    #[test]
    fn test_response_time() {
        let response = ResponseTime {
            duration: Duration::from_millis(15),
            success: true,
            bytes: 1024,
        };
        
        assert_eq!(response.duration, Duration::from_millis(15));
        assert_eq!(response.success, true);
        assert_eq!(response.bytes, 1024);
    }
} 