use crate::auth::ApiKeyInfo;
use crate::error::HttpError;
use axum::{
    extract::{Request, State},
    middleware::Next,
    response::Response,
};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

/// Rate limiter for API endpoints
#[derive(Clone)]
pub struct RateLimiter {
    windows: Arc<Mutex<HashMap<String, RateLimitWindow>>>,
    default_limit: u32,
}

struct RateLimitWindow {
    count: u32,
    window_start: Instant,
}

impl RateLimiter {
    pub fn new(default_limit: u32) -> Self {
        Self {
            windows: Arc::new(Mutex::new(HashMap::new())),
            default_limit,
        }
    }

    /// Check if a request should be rate limited
    pub async fn check_rate_limit(&self, key: &str, limit: u32) -> Result<(), HttpError> {
        let mut windows = self.windows.lock().await;
        let now = Instant::now();
        let window_duration = Duration::from_secs(60); // 1 minute window

        let window = windows.entry(key.to_string()).or_insert(RateLimitWindow {
            count: 0,
            window_start: now,
        });

        // Reset window if it's expired
        if now.duration_since(window.window_start) > window_duration {
            window.count = 0;
            window.window_start = now;
        }

        // Check rate limit
        if window.count >= limit {
            return Err(HttpError::Api(crate::types::ApiError::RateLimitExceeded));
        }

        // Increment counter
        window.count += 1;

        Ok(())
    }

    /// Clean up old windows periodically
    pub async fn cleanup_old_windows(&self) {
        let mut windows = self.windows.lock().await;
        let now = Instant::now();
        let window_duration = Duration::from_secs(120); // Keep windows for 2 minutes

        windows.retain(|_, window| now.duration_since(window.window_start) < window_duration);
    }
}

/// Rate limiting middleware
pub async fn rate_limit_middleware(
    State(limiter): State<RateLimiter>,
    request: Request,
    next: Next,
) -> Result<Response, HttpError> {
    // Try to get API key info from request extensions (set by auth middleware)
    let rate_limit: u32 = if let Some(key_info) = request.extensions().get::<ApiKeyInfo>() {
        // Use key-specific rate limit
        let key = &key_info.key;
        let limit = key_info
            .rate_limit
            .unwrap_or(limiter.default_limit as usize) as u32;

        limiter.check_rate_limit(key, limit).await?;
        limit
    } else {
        // Use IP-based rate limiting for unauthenticated requests
        let ip = request
            .headers()
            .get("x-forwarded-for")
            .and_then(|v| v.to_str().ok())
            .or_else(|| {
                request
                    .headers()
                    .get("x-real-ip")
                    .and_then(|v| v.to_str().ok())
            })
            .unwrap_or("unknown");

        let limit = limiter.default_limit / 2; // Lower limit for unauthenticated
        limiter
            .check_rate_limit(&format!("ip:{}", ip), limit)
            .await?;
        limit
    };

    // Add rate limit headers to response
    let mut response = next.run(request).await;
    let headers = response.headers_mut();

    headers.insert("x-ratelimit-limit", rate_limit.to_string().parse().unwrap());
    // In production, you'd also add remaining count and reset time

    Ok(response)
}

/// Create a rate limiter with a cleanup task
pub fn create_rate_limiter(default_limit: u32) -> RateLimiter {
    let limiter = RateLimiter::new(default_limit);

    // Spawn cleanup task
    let limiter_clone = limiter.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(300)); // Clean every 5 minutes
        loop {
            interval.tick().await;
            limiter_clone.cleanup_old_windows().await;
        }
    });

    limiter
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_rate_limiting() {
        let limiter = RateLimiter::new(10);

        // Should allow first 10 requests
        for i in 0..10 {
            assert!(
                limiter.check_rate_limit("test-key", 10).await.is_ok(),
                "Request {} should succeed",
                i
            );
        }

        // 11th request should fail
        assert!(limiter.check_rate_limit("test-key", 10).await.is_err());

        // Different key should work
        assert!(limiter.check_rate_limit("other-key", 10).await.is_ok());
    }

    #[tokio::test]
    async fn test_window_reset() {
        let limiter = RateLimiter::new(1);

        // First request should succeed
        assert!(limiter.check_rate_limit("test-key", 1).await.is_ok());

        // Second request should fail
        assert!(limiter.check_rate_limit("test-key", 1).await.is_err());

        // Manually reset the window by modifying the internal state
        {
            let mut windows = limiter.windows.lock().await;
            if let Some(window) = windows.get_mut("test-key") {
                window.window_start = Instant::now() - Duration::from_secs(61);
            }
        }

        // Now request should succeed again
        assert!(limiter.check_rate_limit("test-key", 1).await.is_ok());
    }
}
