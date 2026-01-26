//! Resilience module with circuit breaker pattern and error categorization.
//!
//! This module provides fault tolerance mechanisms including:
//! - Circuit breaker to prevent cascading failures
//! - Error categorization for better error handling
//! - Fallback strategies for degraded operation

use std::sync::atomic::{AtomicU8, AtomicU32, AtomicU64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use thiserror::Error;

/// Circuit breaker states
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum CircuitState {
    /// Normal operation - requests pass through
    Closed = 0,
    /// Too many failures - requests fail fast
    Open = 1,
    /// Recovery probe - single request allowed
    HalfOpen = 2,
}

impl From<u8> for CircuitState {
    fn from(value: u8) -> Self {
        match value {
            0 => CircuitState::Closed,
            1 => CircuitState::Open,
            2 => CircuitState::HalfOpen,
            _ => CircuitState::Closed,
        }
    }
}

/// Error categories for better error handling and metrics
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCategory {
    /// Connection failures, timeouts, DNS errors
    Network,
    /// HTTP 429 responses
    RateLimit,
    /// HTTP 401/403 responses
    Auth,
    /// HTTP 5xx responses
    ServerError,
    /// HTML parsing failures
    Parse,
    /// Circuit breaker is open
    CircuitOpen,
    /// Unknown or uncategorized errors
    Unknown,
}

impl std::fmt::Display for ErrorCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ErrorCategory::Network => write!(f, "Network"),
            ErrorCategory::RateLimit => write!(f, "RateLimit"),
            ErrorCategory::Auth => write!(f, "Auth"),
            ErrorCategory::ServerError => write!(f, "ServerError"),
            ErrorCategory::Parse => write!(f, "Parse"),
            ErrorCategory::CircuitOpen => write!(f, "CircuitOpen"),
            ErrorCategory::Unknown => write!(f, "Unknown"),
        }
    }
}

/// Circuit breaker errors
#[derive(Debug, Error)]
pub enum CircuitError {
    #[error("Circuit breaker is open - site temporarily unavailable")]
    CircuitOpen,
    #[error("Request failed: {0}")]
    RequestFailed(String),
}

/// Circuit breaker for fault tolerance
///
/// The circuit breaker has three states:
/// - Closed: Normal operation, requests pass through
/// - Open: Too many failures, requests fail immediately
/// - HalfOpen: After recovery timeout, allow one probe request
#[derive(Debug)]
pub struct CircuitBreaker {
    /// Current circuit state
    state: AtomicU8,
    /// Consecutive failure count
    failure_count: AtomicU32,
    /// Failure threshold to trip the circuit
    failure_threshold: u32,
    /// Time to wait before attempting recovery (seconds)
    recovery_timeout_secs: u64,
    /// Timestamp of last failure (seconds since UNIX_EPOCH)
    last_failure_time: AtomicU64,
    /// Site name for logging
    site_name: String,
}

impl CircuitBreaker {
    /// Create a new circuit breaker with default settings
    pub fn new(site_name: &str) -> Self {
        Self::with_settings(site_name, 5, Duration::from_secs(30))
    }

    /// Create a circuit breaker with custom settings
    pub fn with_settings(
        site_name: &str,
        failure_threshold: u32,
        recovery_timeout: Duration,
    ) -> Self {
        Self {
            state: AtomicU8::new(CircuitState::Closed as u8),
            failure_count: AtomicU32::new(0),
            failure_threshold,
            recovery_timeout_secs: recovery_timeout.as_secs(),
            last_failure_time: AtomicU64::new(0),
            site_name: site_name.to_string(),
        }
    }

    /// Get the current circuit state
    pub fn state(&self) -> CircuitState {
        CircuitState::from(self.state.load(Ordering::Acquire))
    }

    /// Check if a request can proceed
    ///
    /// Returns Ok(()) if the request can proceed, Err if the circuit is open
    pub fn check(&self) -> Result<(), CircuitError> {
        match self.state() {
            CircuitState::Closed => Ok(()),
            CircuitState::HalfOpen => Ok(()), // Allow probe request
            CircuitState::Open => {
                // Check if recovery timeout has elapsed
                let now = current_timestamp();
                let last_failure = self.last_failure_time.load(Ordering::Acquire);

                if now.saturating_sub(last_failure) >= self.recovery_timeout_secs {
                    // Transition to half-open state
                    self.state
                        .store(CircuitState::HalfOpen as u8, Ordering::Release);
                    tracing::info!(site = %self.site_name, "Circuit breaker entering half-open state");
                    Ok(())
                } else {
                    Err(CircuitError::CircuitOpen)
                }
            }
        }
    }

    /// Record a successful request
    pub fn record_success(&self) {
        match self.state() {
            CircuitState::HalfOpen => {
                // Recovery successful - close the circuit
                self.state
                    .store(CircuitState::Closed as u8, Ordering::Release);
                self.failure_count.store(0, Ordering::Release);
                tracing::info!(site = %self.site_name, "Circuit breaker closed after successful recovery");
            }
            CircuitState::Closed => {
                // Reset failure count on success
                self.failure_count.store(0, Ordering::Release);
            }
            CircuitState::Open => {
                // Shouldn't happen, but reset anyway
                self.failure_count.store(0, Ordering::Release);
            }
        }
    }

    /// Record a failed request
    pub fn record_failure(&self) {
        let failures = self.failure_count.fetch_add(1, Ordering::AcqRel) + 1;
        self.last_failure_time
            .store(current_timestamp(), Ordering::Release);

        match self.state() {
            CircuitState::HalfOpen => {
                // Probe failed - reopen the circuit
                self.state
                    .store(CircuitState::Open as u8, Ordering::Release);
                tracing::warn!(site = %self.site_name, "Circuit breaker reopened after probe failure");
            }
            CircuitState::Closed => {
                if failures >= self.failure_threshold {
                    // Too many failures - open the circuit
                    self.state
                        .store(CircuitState::Open as u8, Ordering::Release);
                    tracing::warn!(
                        site = %self.site_name,
                        failures = failures,
                        threshold = self.failure_threshold,
                        "Circuit breaker opened"
                    );
                }
            }
            CircuitState::Open => {
                // Already open, just update timestamp
            }
        }
    }

    /// Reset the circuit breaker to closed state
    pub fn reset(&self) {
        self.state
            .store(CircuitState::Closed as u8, Ordering::Release);
        self.failure_count.store(0, Ordering::Release);
        tracing::info!(site = %self.site_name, "Circuit breaker manually reset");
    }

    /// Get the current failure count
    pub fn failure_count(&self) -> u32 {
        self.failure_count.load(Ordering::Acquire)
    }

    /// Check if the circuit is currently open
    pub fn is_open(&self) -> bool {
        matches!(self.state(), CircuitState::Open)
    }

    /// Get the site name
    pub fn site_name(&self) -> &str {
        &self.site_name
    }
}

/// Get current timestamp in seconds since UNIX_EPOCH
fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Categorize an error for better handling and metrics
pub fn categorize_error(err: &anyhow::Error) -> ErrorCategory {
    let err_str = err.to_string().to_lowercase();

    // Check for rate limiting
    if err_str.contains("429") || err_str.contains("rate limit") || err_str.contains("too many") {
        return ErrorCategory::RateLimit;
    }

    // Check for auth errors
    if err_str.contains("401")
        || err_str.contains("403")
        || err_str.contains("unauthorized")
        || err_str.contains("forbidden")
    {
        return ErrorCategory::Auth;
    }

    // Check for server errors
    if err_str.contains("500")
        || err_str.contains("502")
        || err_str.contains("503")
        || err_str.contains("504")
        || err_str.contains("server error")
    {
        return ErrorCategory::ServerError;
    }

    // Check for network errors
    if err_str.contains("connection")
        || err_str.contains("timeout")
        || err_str.contains("dns")
        || err_str.contains("network")
        || err_str.contains("refused")
        || err_str.contains("reset")
    {
        return ErrorCategory::Network;
    }

    // Check for parse errors
    if err_str.contains("parse")
        || err_str.contains("selector")
        || err_str.contains("html")
        || err_str.contains("invalid")
    {
        return ErrorCategory::Parse;
    }

    // Check for circuit breaker
    if err_str.contains("circuit") {
        return ErrorCategory::CircuitOpen;
    }

    ErrorCategory::Unknown
}

/// Determine if an error is retryable based on its category
pub fn is_retryable(category: ErrorCategory) -> bool {
    matches!(
        category,
        ErrorCategory::Network | ErrorCategory::RateLimit | ErrorCategory::ServerError
    )
}

/// Determine if an error should trip the circuit breaker
pub fn should_trip_circuit(category: ErrorCategory) -> bool {
    matches!(
        category,
        ErrorCategory::Network | ErrorCategory::ServerError | ErrorCategory::RateLimit
    )
}

/// Get recommended backoff duration based on error category
pub fn get_backoff_duration(category: ErrorCategory, attempt: u32) -> Duration {
    let base_ms = match category {
        ErrorCategory::RateLimit => 2000,   // Start with 2s for rate limits
        ErrorCategory::ServerError => 1000, // 1s for server errors
        ErrorCategory::Network => 500,      // 500ms for network errors
        _ => 300,                           // 300ms for others
    };

    // Exponential backoff with cap
    let backoff_ms = base_ms * 2u64.pow(attempt.min(5));
    Duration::from_millis(backoff_ms.min(30000)) // Cap at 30 seconds
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_circuit_breaker_starts_closed() {
        let cb = CircuitBreaker::new("test");
        assert_eq!(cb.state(), CircuitState::Closed);
        assert!(cb.check().is_ok());
    }

    #[test]
    fn test_circuit_breaker_opens_after_failures() {
        let cb = CircuitBreaker::with_settings("test", 3, Duration::from_secs(30));

        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Closed);

        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Closed);

        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Open);

        // Should fail fast now
        assert!(cb.check().is_err());
    }

    #[test]
    fn test_circuit_breaker_resets_on_success() {
        let cb = CircuitBreaker::with_settings("test", 3, Duration::from_secs(30));

        cb.record_failure();
        cb.record_failure();
        assert_eq!(cb.failure_count(), 2);

        cb.record_success();
        assert_eq!(cb.failure_count(), 0);
    }

    #[test]
    fn test_circuit_breaker_half_open_success() {
        let cb = CircuitBreaker::with_settings("test", 1, Duration::from_secs(0));

        // Open the circuit
        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Open);

        // Wait for recovery (0 seconds)
        std::thread::sleep(Duration::from_millis(10));

        // Should transition to half-open
        assert!(cb.check().is_ok());
        assert_eq!(cb.state(), CircuitState::HalfOpen);

        // Success should close it
        cb.record_success();
        assert_eq!(cb.state(), CircuitState::Closed);
    }

    #[test]
    fn test_circuit_breaker_half_open_failure() {
        let cb = CircuitBreaker::with_settings("test", 1, Duration::from_secs(0));

        // Open the circuit
        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Open);

        // Wait for recovery
        std::thread::sleep(Duration::from_millis(10));

        // Check transitions to half-open
        assert!(cb.check().is_ok());
        assert_eq!(cb.state(), CircuitState::HalfOpen);

        // Failure should reopen
        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Open);
    }

    #[test]
    fn test_circuit_breaker_manual_reset() {
        let cb = CircuitBreaker::with_settings("test", 1, Duration::from_secs(60));

        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Open);

        cb.reset();
        assert_eq!(cb.state(), CircuitState::Closed);
        assert_eq!(cb.failure_count(), 0);
    }

    #[test]
    fn test_error_categorization_rate_limit() {
        let err = anyhow::anyhow!("HTTP 429 Too Many Requests");
        assert_eq!(categorize_error(&err), ErrorCategory::RateLimit);
    }

    #[test]
    fn test_error_categorization_auth() {
        let err = anyhow::anyhow!("HTTP 403 Forbidden");
        assert_eq!(categorize_error(&err), ErrorCategory::Auth);
    }

    #[test]
    fn test_error_categorization_server_error() {
        let err = anyhow::anyhow!("HTTP 500 Internal Server Error");
        assert_eq!(categorize_error(&err), ErrorCategory::ServerError);
    }

    #[test]
    fn test_error_categorization_network() {
        let err = anyhow::anyhow!("Connection timeout");
        assert_eq!(categorize_error(&err), ErrorCategory::Network);
    }

    #[test]
    fn test_error_categorization_parse() {
        let err = anyhow::anyhow!("Failed to parse HTML");
        assert_eq!(categorize_error(&err), ErrorCategory::Parse);
    }

    #[test]
    fn test_is_retryable() {
        assert!(is_retryable(ErrorCategory::Network));
        assert!(is_retryable(ErrorCategory::RateLimit));
        assert!(is_retryable(ErrorCategory::ServerError));
        assert!(!is_retryable(ErrorCategory::Auth));
        assert!(!is_retryable(ErrorCategory::Parse));
    }

    #[test]
    fn test_backoff_duration() {
        let rate_limit_0 = get_backoff_duration(ErrorCategory::RateLimit, 0);
        let rate_limit_1 = get_backoff_duration(ErrorCategory::RateLimit, 1);

        assert!(rate_limit_1 > rate_limit_0);
        assert!(rate_limit_0 >= Duration::from_millis(2000));

        // Test cap at 30 seconds
        let max_backoff = get_backoff_duration(ErrorCategory::RateLimit, 10);
        assert!(max_backoff <= Duration::from_secs(30));
    }

    #[test]
    fn test_circuit_state_from_u8() {
        assert_eq!(CircuitState::from(0), CircuitState::Closed);
        assert_eq!(CircuitState::from(1), CircuitState::Open);
        assert_eq!(CircuitState::from(2), CircuitState::HalfOpen);
        assert_eq!(CircuitState::from(255), CircuitState::Closed); // Invalid defaults to Closed
    }
}
