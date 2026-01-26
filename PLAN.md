# Website Searcher Improvement Plan

## Overview
This document outlines the implementation plan for enhancing the Website Searcher project with improved performance, reliability, and user experience features.

## Selected Improvements
1. Rate Limiting & Adaptive Backoff
2. Cache TTL & Expiration (12 hours)
3. Anti-Detection Enhancements
4. External Configuration
5. Error Handling & Resilience
7. GUI Improvements (Real-time Results)
9. Search Features (Advanced Operators)
11. Monitoring & Observability
12. Docker & Deployment Enhancements
13. Content Analysis

---

## 1. Rate Limiting & Adaptive Backoff (completed)

### Implementation Details
- Create `RateLimiter` struct in `crates/core/src/rate_limiter.rs`
- Implement per-site rate limiting with configurable delays
- Add exponential backoff with jitter for failed requests
- Track site-specific response times to adapt scraping speed

### Code Changes
```rust
// New file: crates/core/src/rate_limiter.rs
pub struct RateLimiter {
    // Per-site last request timestamps
    // Adaptive delay calculations
    // Backoff state tracking
}
```

### Integration Points
- Modify `fetch_with_retry()` in `fetcher.rs`
- Update CLI main loop (`crates/cli/main.rs`)
- Update GUI backend (`src-tauri/src/lib.rs`)

---

## 2. Cache TTL & Expiration (12 hours)

### Implementation Details
- Add `timestamp` and `ttl` fields to `CacheEntry`
- Implement cache expiration check in `get()` method
- Add automatic cleanup of expired entries
- Cache warming for frequently accessed queries

### Code Changes
```rust
// Modify: crates/core/src/cache.rs
pub struct CacheEntry {
    pub query: String,
    pub results: Vec<SearchResult>,
    pub timestamp: SystemTime,
    pub ttl: Duration, // Default 12 hours
}
```

### Files to Modify
- `crates/core/src/cache.rs`
- `crates/cli/main.rs` (cache loading logic)
- `src-tauri/src/lib.rs` (GUI cache commands)

---

## 3. Anti-Detection Enhancements

### Implementation Details
- Create user agent rotation system
- Add proxy support configuration
- Implement request header randomization
- Browser fingerprinting evasion techniques

### Code Changes
```rust
// New file: crates/core/src/anti_detection.rs
pub struct AntiDetectionConfig {
    user_agents: Vec<String>,
    proxy_config: Option<ProxyConfig>,
    header_randomization: bool,
}
```

### Integration
- Update `build_http_client()` in `fetcher.rs`
- Add configuration options to CLI args
- Add GUI settings for anti-detection

---

## 4. External Configuration

### Implementation Details
- Move site configs to `config/sites.toml`
- Add hot-reloading capability
- Site-specific timeout and retry configs
- Validation schema for configuration files

### Configuration Structure
```toml
# config/sites.toml
[sites.fitgirl]
name = "fitgirl"
base_url = "https://fitgirl-repacks.site/"
search_kind = "QueryParam"
query_param = "s"
result_selector = "h2.entry-title a"
requires_cloudflare = true
timeout_seconds = 30
retry_attempts = 3
rate_limit_delay_ms = 1000
```

### Code Changes
- Create `config/mod.rs` for config loading
- Modify `crates/core/src/config.rs` to use external config
- Add config validation functions

---

## 5. Error Handling & Resilience

### Implementation Details
- Implement circuit breaker pattern
- Error categorization system
- Fallback mechanisms
- Comprehensive error reporting

### Code Changes
```rust
// New file: crates/core/src/resilience.rs
pub struct CircuitBreaker {
    failure_count: AtomicU32,
    state: CircuitState,
    threshold: u32,
    timeout: Duration,
}
```

### Integration
- Wrap site requests in circuit breaker
- Add error categories to error types
- Implement fallback strategies

---

## 7. GUI Improvements - Real-time Results

### Implementation Details
- Implement WebSocket or SSE for real-time updates
- Add per-site progress indicators
- Stream results as they become available
- Update React components for live updates

### Frontend Changes
```typescript
// gui/src/hooks/useRealtimeSearch.ts
export const useRealtimeSearch = () => {
  // WebSocket connection for live results
  // Progress tracking per site
  // Result streaming state management
};
```

### Backend Changes
- Add Tauri events for progress updates
- Modify search to emit results as ready
- Implement result streaming in CLI and GUI

### TUI Real-time Updates
- Modify `run_live_tui()` to accept streamed results
- Add progress bars per site
- Implement incremental result display

---

## 9. Search Features - Advanced Operators

### Implementation Details
- Add parser for search operators
- Implement site: , -exclude , "exact phrase" operators
- Add regular expression support
- Update query normalization to preserve operators

### Code Changes
```rust
// New file: crates/core/src/query_parser.rs
pub struct AdvancedQuery {
    terms: Vec<String>,
    exclude_terms: Vec<String>,
    site_restrictions: Vec<String>,
    exact_phrases: Vec<String>,
    regex_patterns: Vec<Regex>,
}
```

### Integration
- Update `normalize_query()` to parse operators
- Modify result filtering to apply advanced criteria
- Add help text for operators in CLI and GUI

---

## 11. Monitoring & Observability

### Implementation Details
- Structured logging with `tracing` crate
- Metrics collection using `metrics` crate
- Performance profiling endpoints
- Health check system

### Code Changes
```rust
// New file: crates/core/src/monitoring.rs
pub struct Metrics {
    search_duration: Histogram,
    success_rate: Counter,
    active_requests: Gauge,
}
```

### Implementation
- Add logging throughout the application
- Create `/metrics` endpoint for GUI
- Add health check for Docker

---

## 12. Docker & Deployment Enhancements

### Implementation Details
- Health checks for FlareSolverr
- Graceful shutdown handling
- Docker Compose profiles
- Kubernetes manifests

### Docker Changes
```dockerfile
# Add health check to Dockerfile
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
  CMD curl -f http://localhost:8191/v1 || exit 1
```

### Docker Compose Profiles
```yaml
# docker-compose.yml
profiles:
  development:
    # Debug tools, hot reload
  production:
    # Optimized builds, monitoring
  testing:
    # Test environment, mocks
```

---

## 13. Content Analysis

### Implementation Details
- Extract additional metadata (file sizes, dates)
- Duplicate detection across sites
- Content categorization
- Similarity scoring

### Code Changes
```rust
// New file: crates/core/src/analyzer.rs
pub struct ContentAnalyzer {
    duplicate_threshold: f32,
    similarity_algorithm: SimilarityType,
}
```

### Integration
- Extend `SearchResult` with metadata fields
- Add analysis pipeline after parsing
- Implement duplicate detection in result aggregation

---

## Implementation Timeline

### Phase 1 (Week 1-2): Core Infrastructure
1. Rate limiting implementation
2. Cache TTL system
3. External configuration system
4. Basic monitoring setup

### Phase 2 (Week 3-4): Reliability & Features
1. Anti-detection enhancements
2. Error handling & resilience
3. Advanced search operators
4. Content analysis basics

### Phase 3 (Week 5-6): User Experience
1. Real-time results in GUI
2. TUI real-time updates
3. Docker enhancements
4. Full observability suite

### Phase 4 (Week 7-8): Polish & Testing
1. Comprehensive testing
2. Documentation updates
3. Performance optimization
4. Release preparation

---

## Dependencies to Add

### Rust Dependencies
```toml
# Rate limiting
governor = "0.6"

# Configuration
serde = { version = "1.0", features = ["derive"] }
toml = "0.8"

# Monitoring
tracing = "0.1"
tracing-subscriber = "0.3"
metrics = "0.22"
metrics-exporter-prometheus = "0.13"

# Anti-detection
reqwest = { version = "0.11", features = ["socks"] }

# Content analysis
similarity = "2.4"
regex = "1.10"
```

### TypeScript Dependencies
```json
{
  "ws": "^8.14.0",
  "@types/ws": "^8.5.0"
}
```

---

## Testing Strategy

1. Unit tests for each new module
2. Integration tests with mock servers
3. Performance benchmarks
4. End-to-end tests for full flows
5. Property-based testing for edge cases

---

## Backward Compatibility

- Maintain existing CLI interface
- Provide migration path for configuration
- Support legacy cache format
- Feature flags for new functionality

---

## Success Metrics

1. Reduced IP bans through rate limiting
2. Improved cache hit rates with TTL
3. Better detection evasion success rates
4. Faster time-to-first-result in GUI
5. Enhanced search relevance with operators
6. Improved system observability
7. Better deployment reliability

This plan provides a roadmap for implementing substantial improvements to the Website Searcher project while maintaining its core functionality and reliability.
