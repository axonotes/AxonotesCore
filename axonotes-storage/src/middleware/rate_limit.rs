use crate::auth::AuthUser;
use crate::utils::{AppError, Result};
use axum::{
    extract::{Request, State},
    middleware::Next,
    response::Response,
};
use dashmap::DashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tracing::warn;

/// Rate limit window entry
#[derive(Clone)]
struct RateLimitWindow {
    timestamps: Vec<Instant>,
}

/// Sliding window rate limiter
pub struct RateLimiter {
    windows: Arc<DashMap<String, RateLimitWindow>>,
    window_duration: Duration,
    max_requests: usize,
}

impl RateLimiter {
    /// Create new rate limiter
    pub fn new(window_seconds: u64, max_requests: usize) -> Self {
        Self {
            windows: Arc::new(DashMap::new()),
            window_duration: Duration::from_secs(window_seconds),
            max_requests,
        }
    }

    /// Check if request is allowed for the given key (returns Err if rate limited)
    pub fn check(&self, key: &str) -> Result<()> {
        let now = Instant::now();

        // Get or create window for this key
        let mut entry = self
            .windows
            .entry(key.to_string())
            .or_insert(RateLimitWindow {
                timestamps: Vec::new(),
            });

        let window = entry.value_mut();

        // Remove timestamps outside the window
        window
            .timestamps
            .retain(|&t| now.duration_since(t) < self.window_duration);

        // Check if under limit
        if window.timestamps.len() >= self.max_requests {
            warn!("Rate limit exceeded for key: {}", key);
            return Err(AppError::RateLimitExceeded(format!(
                "Rate limit exceeded: {} requests per {} seconds",
                self.max_requests,
                self.window_duration.as_secs()
            )));
        }

        // Add current timestamp
        window.timestamps.push(now);

        Ok(())
    }

    /// Cleanup expired entries (call periodically)
    pub fn cleanup(&self) {
        let now = Instant::now();
        self.windows.retain(|_, window| {
            // Remove all timestamps outside window
            window
                .timestamps
                .retain(|&t| now.duration_since(t) < self.window_duration);
            // Keep entry if it has recent timestamps
            !window.timestamps.is_empty()
        });
    }

    /// Get current count for a key
    #[allow(dead_code)]
    pub fn get_count(&self, key: &str) -> usize {
        if let Some(entry) = self.windows.get(key) {
            let now = Instant::now();
            entry
                .timestamps
                .iter()
                .filter(|&&t| now.duration_since(t) < self.window_duration)
                .count()
        } else {
            0
        }
    }
}

/// Rate limiters for different endpoints
pub struct RateLimiters {
    pub upload: RateLimiter,
    pub download: RateLimiter,
    pub quota: RateLimiter,
}

impl RateLimiters {
    /// Create rate limiters from config
    pub fn from_config(config: &crate::config::RateLimitConfig) -> Self {
        Self {
            upload: RateLimiter::new(60, config.upload_per_minute as usize),
            download: RateLimiter::new(60, config.download_per_minute as usize),
            quota: RateLimiter::new(60, config.quota_per_minute as usize),
        }
    }

    /// Create default rate limiters (for backwards compatibility)
    pub fn new() -> Self {
        Self {
            upload: RateLimiter::new(60, 10),    // 10 uploads/min per JWT
            download: RateLimiter::new(60, 100), // 100 downloads/min per JWT
            quota: RateLimiter::new(60, 60),     // 60 quota checks/min per JWT
        }
    }

    /// Cleanup all rate limiters
    pub fn cleanup_all(&self) {
        self.upload.cleanup();
        self.download.cleanup();
        self.quota.cleanup();
    }
}

impl Default for RateLimiters {
    fn default() -> Self {
        Self::new()
    }
}

/// Middleware for upload rate limiting
pub async fn upload_rate_limit(
    State(limiters): State<Arc<RateLimiters>>,
    request: Request,
    next: Next,
) -> std::result::Result<Response, AppError> {
    // Extract user from request extensions (set by auth middleware)
    let user = request
        .extensions()
        .get::<AuthUser>()
        .ok_or_else(|| AppError::Unauthorized("Not authenticated".to_string()))?;

    // Check rate limit using user_id as key
    limiters.upload.check(&user.user_id)?;

    Ok(next.run(request).await)
}

/// Middleware for download rate limiting
pub async fn download_rate_limit(
    State(limiters): State<Arc<RateLimiters>>,
    request: Request,
    next: Next,
) -> std::result::Result<Response, AppError> {
    // Download is public, but if user is authenticated, apply rate limiting
    if let Some(user) = request.extensions().get::<AuthUser>() {
        limiters.download.check(&user.user_id)?;
    }

    Ok(next.run(request).await)
}

/// Middleware for quota check rate limiting
pub async fn quota_rate_limit(
    State(limiters): State<Arc<RateLimiters>>,
    request: Request,
    next: Next,
) -> std::result::Result<Response, AppError> {
    let user = request
        .extensions()
        .get::<AuthUser>()
        .ok_or_else(|| AppError::Unauthorized("Not authenticated".to_string()))?;

    limiters.quota.check(&user.user_id)?;

    Ok(next.run(request).await)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread::sleep;

    #[test]
    fn test_rate_limiter_allows_under_limit() {
        let limiter = RateLimiter::new(60, 5);

        for i in 0..5 {
            assert!(
                limiter.check("user1").is_ok(),
                "Request {} should be allowed",
                i
            );
        }
    }

    #[test]
    fn test_rate_limiter_blocks_over_limit() {
        let limiter = RateLimiter::new(60, 3);

        // First 3 should pass
        for _ in 0..3 {
            assert!(limiter.check("user1").is_ok());
        }

        // 4th should fail
        assert!(limiter.check("user1").is_err());
    }

    #[test]
    fn test_rate_limiter_sliding_window() {
        let limiter = RateLimiter::new(1, 2); // 2 requests per 1 second

        // First 2 should pass
        assert!(limiter.check("user1").is_ok());
        assert!(limiter.check("user1").is_ok());

        // 3rd should fail
        assert!(limiter.check("user1").is_err());

        // Wait for window to slide
        sleep(Duration::from_millis(1100));

        // Should be allowed again
        assert!(limiter.check("user1").is_ok());
    }

    #[test]
    fn test_rate_limiter_per_key() {
        let limiter = RateLimiter::new(60, 2);

        // User1 can make 2 requests
        assert!(limiter.check("user1").is_ok());
        assert!(limiter.check("user1").is_ok());
        assert!(limiter.check("user1").is_err());

        // User2 can also make 2 requests (independent)
        assert!(limiter.check("user2").is_ok());
        assert!(limiter.check("user2").is_ok());
        assert!(limiter.check("user2").is_err());
    }

    #[test]
    fn test_rate_limiter_cleanup() {
        let limiter = RateLimiter::new(1, 5);

        limiter.check("user1").unwrap();
        assert_eq!(limiter.windows.len(), 1);

        // Wait for expiry
        sleep(Duration::from_millis(1100));

        limiter.cleanup();
        assert_eq!(limiter.windows.len(), 0);
    }

    #[test]
    fn test_get_count() {
        let limiter = RateLimiter::new(60, 10);

        assert_eq!(limiter.get_count("user1"), 0);

        limiter.check("user1").unwrap();
        assert_eq!(limiter.get_count("user1"), 1);

        limiter.check("user1").unwrap();
        assert_eq!(limiter.get_count("user1"), 2);
    }
}
