use std::sync::Arc;
use std::time::{Duration, Instant};
use metrics::{counter, gauge};
use std::sync::OnceLock;
use tokio::sync::RwLock;
use tracing::{Level, debug, info, span};

/// Global metrics collector
pub static METRICS: OnceLock<Arc<SearchMetrics>> = OnceLock::new();

/// Get the global metrics instance
pub fn get_metrics() -> &'static Arc<SearchMetrics> {
    METRICS.get_or_init(|| Arc::new(SearchMetrics::new()))
}

/// Initialize tracing subscriber and metrics exporter
pub fn init_monitoring() -> anyhow::Result<()> {
    init_monitoring_with_json(false)
}

/// Initialize monitoring with option to suppress logging for JSON output
pub fn init_monitoring_with_json(json_output: bool) -> anyhow::Result<()> {
    // Initialize tracing subscriber
    if json_output {
        // For JSON output, use a minimal logger that only writes errors
        init_tracing_json();
    } else {
        init_tracing();
    }

    // Skip metrics exporter in tests or when disabled
    if std::env::var("WEBSITE_SEARCHER_NO_METRICS").is_ok() {
        return Ok(());
    }

    // Try to initialize metrics exporter on port 9898, fall back to random port if occupied
    let port = find_available_port(9898).unwrap_or(9899);
    
    metrics_exporter_prometheus::PrometheusBuilder::new()
        .with_http_listener(([0, 0, 0, 0], port))
        .install()?;

    if !json_output {
        info!("Monitoring system initialized");
        info!("Metrics endpoint available at http://localhost:{}/metrics", port);
    }

    Ok(())
}

/// Initialize tracing for JSON output (errors only)
fn init_tracing_json() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from("error"))
        .with_target(false)
        .with_ansi(false)
        .init();
}

/// Find an available port starting from the given port
fn find_available_port(start_port: u16) -> Option<u16> {
    (start_port..(start_port + 10)).find(|&port| port_is_available(port))
}

/// Check if a port is available
fn port_is_available(port: u16) -> bool {
    std::net::TcpListener::bind(("0.0.0.0", port)).is_ok()
}

/// Initialize tracing subscriber with default configuration
pub fn init_tracing() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "website_searcher=debug,tower_http=debug".into()),
        )
        .with_target(false)
        .with_thread_ids(true)
        .with_file(true)
        .with_line_number(true)
        .init();

    info!("Tracing initialized");
}

/// A metrics collector for tracking search operations with Prometheus integration
#[derive(Debug)]
pub struct SearchMetrics {
    pub total_requests: u64,
    pub successful_requests: u64,
    pub failed_requests: u64,
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub total_duration: Duration,
    pub site_metrics: RwLock<std::collections::HashMap<String, SiteMetrics>>,
    pub start_time: Instant,
}

#[derive(Debug, Default, Clone)]
pub struct SiteMetrics {
    pub requests: u64,
    pub successes: u64,
    pub failures: u64,
    pub avg_response_time: Duration,
}

impl Default for SearchMetrics {
    fn default() -> Self {
        Self::new()
    }
}

impl SearchMetrics {
    pub fn new() -> Self {
        // Initialize global metrics
        counter!("website_searcher_starts");
        gauge!("website_searcher_active_requests");
        
        Self {
            total_requests: 0,
            successful_requests: 0,
            failed_requests: 0,
            cache_hits: 0,
            cache_misses: 0,
            total_duration: Duration::ZERO,
            site_metrics: RwLock::new(std::collections::HashMap::new()),
            start_time: Instant::now(),
        }
    }

    pub async fn record_request(&self, site_name: &str, duration: Duration, success: bool) {
        // Update Prometheus metrics
        counter!("website_searcher_searches_total", "site" => site_name.to_string());
        counter!("website_searcher_active_requests", "site" => site_name.to_string());
        
        if success {
            counter!("website_searcher_searches_success_total", "site" => site_name.to_string());
        } else {
            counter!("website_searcher_searches_failure_total", "site" => site_name.to_string());
        }
        
        counter!("website_searcher_search_duration", "site" => site_name.to_string());
        counter!("website_searcher_active_requests_complete", "site" => site_name.to_string());
        
        // Update internal metrics
        let mut site_metrics = self.site_metrics.write().await;
        let site_metric = site_metrics.entry(site_name.to_string()).or_default();

        site_metric.requests += 1;

        if success {
            site_metric.successes += 1;
        } else {
            site_metric.failures += 1;
        }

        // Update average response time for site
        let total_time_ms = site_metric.avg_response_time.as_millis() as u64
            * (site_metric.requests - 1)
            + duration.as_millis() as u64;
        site_metric.avg_response_time =
            Duration::from_millis(total_time_ms / site_metric.requests);
    }

    pub fn record_cache_hit(&self) {
        counter!("website_searcher_cache_hits_total");
        debug!("Cache hit recorded");
    }

    pub fn record_cache_miss(&self) {
        counter!("website_searcher_cache_misses_total");
        debug!("Cache miss recorded");
    }

    pub async fn get_site_metrics(&self, site: &str) -> Option<SiteMetrics> {
        self.site_metrics.read().await.get(site).cloned()
    }
    
    pub async fn get_all_site_metrics(&self) -> std::collections::HashMap<String, SiteMetrics> {
        self.site_metrics.read().await.clone()
    }
    
    pub fn uptime(&self) -> Duration {
        self.start_time.elapsed()
    }

    pub async fn log_summary(&self) {
        let site_metrics = self.site_metrics.read().await;
        let total_requests: u64 = site_metrics.values().map(|m| m.requests).sum();
        let total_successes: u64 = site_metrics.values().map(|m| m.successes).sum();
        let total_failures: u64 = site_metrics.values().map(|m| m.failures).sum();
        
        let success_rate = if total_requests > 0 {
            total_successes as f64 / total_requests as f64 * 100.0
        } else {
            0.0
        };
        
        info!(
            uptime_seconds = self.uptime().as_secs(),
            requests = total_requests,
            successes = total_successes,
            failures = total_failures,
            success_rate = format!("{:.1}%", success_rate),
            "Search metrics summary"
        );

        for (site, metrics) in site_metrics.iter() {
            let site_success_rate = if metrics.requests > 0 {
                metrics.successes as f64 / metrics.requests as f64 * 100.0
            } else {
                0.0
            };
            
            info!(
                site = site,
                requests = metrics.requests,
                successes = metrics.successes,
                failures = metrics.failures,
                success_rate = format!("{:.1}%", site_success_rate),
                avg_response_time_ms = metrics.avg_response_time.as_millis(),
                "Site metrics"
            );
        }
    }
}

/// A timer for measuring operation duration
pub struct Timer {
    start: Instant,
}

impl Timer {
    pub fn start(name: &'static str) -> Self {
        let _span = span!(Level::DEBUG, "timer", name);
        let _enter = _span.enter();
        debug!("Starting timer: {}", name);

        Self {
            start: Instant::now(),
        }
    }

    pub fn finish(self) -> Duration {
        let duration = self.start.elapsed();
        debug!(duration_ms = duration.as_millis(), "Timer finished");
        duration
    }
}

/// Macro for timing operations with metrics
#[macro_export]
macro_rules! time_it {
    ($name:expr, $block:block) => {{
        let _timer = $crate::monitoring::Timer::start($name);
        let result = $block;
        let _duration = _timer.finish();
        result
    }};
}

/// Macro for convenient metric recording
#[macro_export]
macro_rules! record_search_metrics {
    ($site:expr, $duration:expr, $result:expr) => {
        match $result {
            Ok(results) => {
                $crate::monitoring::get_metrics().record_request($site, $duration, true).await;
                counter!("website_searcher_results_count", "site" => $site.to_string());
            }
            Err(e) => {
                $crate::monitoring::get_metrics().record_request($site, $duration, false).await;
                error!("Search failed for {}: {}", $site, e);
            }
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_metrics_recording() {
        let metrics = SearchMetrics::new();
        
        metrics.record_request("test-site", Duration::from_millis(100), true).await;
        metrics.record_request("test-site", Duration::from_millis(200), false).await;
        
        let site_metrics = metrics.get_site_metrics("test-site").await.unwrap();
        assert_eq!(site_metrics.requests, 2);
        assert_eq!(site_metrics.successes, 1);
        assert_eq!(site_metrics.failures, 1);
    }

    #[tokio::test]
    async fn test_timer() {
        let timer = Timer::start("test");
        tokio::time::sleep(Duration::from_millis(10)).await;
        let duration = timer.finish();
        assert!(duration >= Duration::from_millis(10));
    }
}
