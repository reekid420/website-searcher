use rand::Rng;
use std::collections::HashMap;
use std::time::{Duration, Instant};

/// Per-site rate limiting state
#[derive(Debug, Clone)]
struct SiteRateState {
    /// Last request timestamp
    last_request: Instant,
    /// Current delay between requests
    current_delay: Duration,
    /// Number of consecutive failures
    failure_count: u32,
    /// Average response time for adaptive delays
    avg_response_time: Duration,
    /// Response time samples for averaging
    response_samples: Vec<Duration>,
}

impl Default for SiteRateState {
    fn default() -> Self {
        Self {
            last_request: Instant::now() - Duration::from_secs(1), // Allow first request immediately
            current_delay: Duration::from_millis(1000), // Default 1 second between requests
            failure_count: 0,
            avg_response_time: Duration::from_millis(500),
            response_samples: Vec::with_capacity(5),
        }
    }
}

/// Rate limiter with adaptive backoff and per-site tracking
#[derive(Debug)]
pub struct RateLimiter {
    /// Per-site rate limiting state
    sites: HashMap<String, SiteRateState>,
    /// Base delay for new sites
    base_delay: Duration,
    /// Maximum delay between requests
    max_delay: Duration,
    /// Backoff multiplier for failures
    backoff_multiplier: f64,
    /// Jitter factor to add randomness (0.0 to 1.0)
    jitter_factor: f64,
    /// Maximum number of consecutive failures before giving up
    max_failures: u32,
}

impl Default for RateLimiter {
    fn default() -> Self {
        Self::new()
    }
}

impl RateLimiter {
    /// Create a new rate limiter with default settings
    pub fn new() -> Self {
        Self::with_settings(
            Duration::from_millis(1000), // base_delay
            Duration::from_secs(30),     // max_delay
            2.0,                         // backoff_multiplier
            0.1,                         // jitter_factor
            5,                           // max_failures
        )
    }

    /// Create a rate limiter with custom settings
    pub fn with_settings(
        base_delay: Duration,
        max_delay: Duration,
        backoff_multiplier: f64,
        jitter_factor: f64,
        max_failures: u32,
    ) -> Self {
        Self {
            sites: HashMap::new(),
            base_delay,
            max_delay,
            backoff_multiplier,
            jitter_factor,
            max_failures,
        }
    }

    /// Wait if necessary before making a request to the given site
    pub async fn wait_for_site(&mut self, site: &str) -> Result<(), RateLimitError> {
        let state = self.sites.entry(site.to_string()).or_default();

        // Check if we've exceeded max failures
        if state.failure_count >= self.max_failures {
            return Err(RateLimitError::TooManyFailures);
        }

        let now = Instant::now();
        let time_since_last = now.duration_since(state.last_request);

        // Calculate how long we need to wait
        let wait_time = if time_since_last < state.current_delay {
            state.current_delay - time_since_last
        } else {
            Duration::ZERO
        };

        // Add jitter to prevent thundering herd
        let jittered_wait = {
            if self.jitter_factor == 0.0 {
                wait_time
            } else {
                let mut rng = rand::thread_rng();
                let jitter_ms = (wait_time.as_millis() as f64 * self.jitter_factor) as u64;
                let jitter = Duration::from_millis(rng.gen_range(0..=jitter_ms));
                wait_time + jitter
            }
        };

        if !jittered_wait.is_zero() {
            tokio::time::sleep(jittered_wait).await;
        }

        state.last_request = Instant::now();
        Ok(())
    }

    /// Record a successful request for adaptive timing
    pub fn record_success(&mut self, site: &str, response_time: Duration) {
        if let Some(state) = self.sites.get_mut(site) {
            // Reset failure count on success
            state.failure_count = 0;

            // Update response time tracking
            state.response_samples.push(response_time);
            if state.response_samples.len() > 5 {
                state.response_samples.remove(0);
            }

            // Calculate new average
            if !state.response_samples.is_empty() {
                let sum: Duration = state.response_samples.iter().sum();
                state.avg_response_time = sum / state.response_samples.len() as u32;

                // Adapt delay based on response time (aim for 2x average response time)
                let target_delay = state.avg_response_time * 2;
                state.current_delay = target_delay.clamp(self.base_delay, self.max_delay);
            }
        }
    }

    /// Record a failed request and apply backoff
    pub fn record_failure(&mut self, site: &str) -> Result<(), RateLimitError> {
        // Ensure site state exists
        self.sites
            .entry(site.to_string())
            .or_insert_with(|| SiteRateState {
                last_request: Instant::now(),
                current_delay: self.base_delay,
                failure_count: 0,
                avg_response_time: Duration::from_millis(500),
                response_samples: Vec::new(),
            });

        if let Some(state) = self.sites.get_mut(site) {
            state.failure_count += 1;

            if state.failure_count > self.max_failures {
                return Err(RateLimitError::TooManyFailures);
            }

            // Apply exponential backoff
            let backoff_delay = Duration::from_millis(
                (state.current_delay.as_millis() as f64 * self.backoff_multiplier) as u64,
            )
            .clamp(self.base_delay, self.max_delay);

            state.current_delay = backoff_delay;
        }

        Ok(())
    }

    /// Get the current delay for a site
    pub fn get_delay(&self, site: &str) -> Duration {
        self.sites
            .get(site)
            .map(|s| s.current_delay)
            .unwrap_or(self.base_delay)
    }

    /// Reset failure count for a site (useful for manual retry)
    pub fn reset_failures(&mut self, site: &str) {
        if let Some(state) = self.sites.get_mut(site) {
            state.failure_count = 0;
            state.current_delay = self.base_delay;
        }
    }

    /// Get statistics for all sites
    pub fn get_stats(&self) -> HashMap<String, RateStats> {
        self.sites
            .iter()
            .map(|(site, state)| {
                (
                    site.clone(),
                    RateStats {
                        current_delay: state.current_delay,
                        failure_count: state.failure_count,
                        avg_response_time: state.avg_response_time,
                    },
                )
            })
            .collect()
    }
}

/// Errors that can occur during rate limiting
#[derive(Debug, thiserror::Error)]
pub enum RateLimitError {
    #[error("Too many consecutive failures for site")]
    TooManyFailures,
}

/// Statistics for a site's rate limiting
#[derive(Debug, Clone)]
pub struct RateStats {
    pub current_delay: Duration,
    pub failure_count: u32,
    pub avg_response_time: Duration,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::Duration;

    #[tokio::test]
    async fn test_basic_rate_limiting() {
        let mut limiter = RateLimiter::with_settings(
            Duration::from_millis(100),
            Duration::from_secs(1),
            2.0,
            0.0,
            3,
        );

        let site = "test-site";

        // First request should pass immediately
        let start = Instant::now();
        limiter.wait_for_site(site).await.unwrap();
        assert!(start.elapsed() < Duration::from_millis(50));

        // Second request should wait
        let start = Instant::now();
        limiter.wait_for_site(site).await.unwrap();
        assert!(start.elapsed() >= Duration::from_millis(90)); // Account for small variations
    }

    #[tokio::test]
    async fn test_failure_backoff() {
        let mut limiter = RateLimiter::with_settings(
            Duration::from_millis(100),
            Duration::from_secs(10),
            2.0,
            0.0,
            3,
        );

        let site = "test-site-failure-backoff";

        // Record a failure
        limiter.record_failure(site).unwrap();

        // Next request should have increased delay
        let delay = limiter.get_delay(site);
        assert!(delay >= Duration::from_millis(190)); // ~2x base delay
    }

    #[tokio::test]
    async fn test_max_failures() {
        let mut limiter = RateLimiter::with_settings(
            Duration::from_millis(100),
            Duration::from_secs(1),
            2.0,
            0.0,
            2, // Max 2 failures
        );

        let site = "test-site-max-failures";

        // Record failures up to max
        limiter.record_failure(site).unwrap();
        limiter.record_failure(site).unwrap();

        // Next failure should return error
        assert!(matches!(
            limiter.record_failure(site),
            Err(RateLimitError::TooManyFailures)
        ));

        // Waiting should also fail
        assert!(matches!(
            limiter.wait_for_site(site).await,
            Err(RateLimitError::TooManyFailures)
        ));
    }
}
