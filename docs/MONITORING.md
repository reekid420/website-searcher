# Monitoring Guide

Website-searcher includes comprehensive monitoring capabilities with Prometheus metrics and structured logging.

## Overview

The monitoring system provides:
- Real-time metrics collection via Prometheus
- Structured logging with tracing
- Per-site performance tracking
- Cache hit/miss statistics
- Request success/failure rates

## Metrics

### Available Metrics

All metrics are exported to Prometheus on port 9898 (or next available port):

```bash
curl http://localhost:9898/metrics
```

#### Counters
- `website_searcher_searches_total` - Total search attempts per site
- `website_searcher_searches_success_total` - Successful searches per site
- `website_searcher_searches_failure_total` - Failed searches per site
- `website_searcher_cache_hits_total` - Cache hits
- `website_searcher_cache_misses_total` - Cache misses
- `website_searcher_starts` - Application starts

#### Gauges
- `website_searcher_active_requests` - Currently active requests per site

#### Histograms
- `website_searcher_search_duration_seconds` - Request duration distribution per site

### Metrics Labels

All per-site metrics include a `site` label:
```
website_searcher_searches_total{site="fitgirl"} 42
website_searcher_searches_success_total{site="dodi"} 38
```

## Logging

### Structured Logging

The application uses `tracing` for structured logging with the following levels:

#### Error Level
- HTTP request failures
- Rate limit errors
- Cache I/O errors

#### Warn Level
- Rate limiting (429 responses)
- Server errors (5xx)
- Access denied (403)

#### Info Level
- Successful requests with response time
- Cache operations
- Monitoring system initialization

#### Debug Level
- Detailed request/response info
- Cache entry details
- Rate limiter state

### Log Output

#### JSON Mode
When using `--format json`, logging is minimized to avoid interfering with output:
```bash
websearcher "query" --format json
# Only errors are logged
```

#### Table Mode
Full logging is enabled:
```bash
websearcher "query" --format table
# All info+ level logs shown
```

### Log Format

Default format includes timestamp, level, target, and structured fields:
```
2024-01-20T10:30:45.123456Z  INFO website_searcher_core::fetcher: site=fitgirl url="https://..." response_time_ms=250 "Received response"
```

## Configuration

### Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `WEBSITE_SEARCHER_NO_METRICS` | unset | Disable metrics exporter |
| `RUST_LOG` | info | Log level filter |

### Port Configuration

Metrics exporter tries ports 9898-9907:
1. Attempt port 9898
2. If occupied, try 9899
3. Continue up to 9907
4. Exit if all ports occupied

### Disabling Metrics

For tests or embedded use:
```bash
export WEBSITE_SEARCHER_NO_METRICS=1
websearcher "query"
```

## Integration

### Prometheus Setup

Add to `prometheus.yml`:
```yaml
scrape_configs:
  - job_name: 'website-searcher'
    static_configs:
      - targets: ['localhost:9898']
    scrape_interval: 5s
```

### Grafana Dashboard

Example panel queries:
- Requests per second: `rate(website_searcher_searches_total[5m])`
- Success rate: `rate(website_searcher_searches_success_total[5m]) / rate(website_searcher_searches_total[5m])`
- Cache hit ratio: `website_searcher_cache_hits_total / (website_searcher_cache_hits_total + website_searcher_cache_misses_total)`
- Average response time: `histogram_quantile(0.95, rate(website_searcher_search_duration_seconds_bucket[5m]))`

## Programmatic Access

### Recording Metrics

```rust
use website_searcher_core::monitoring::{get_metrics, record_search_metrics};

// Get metrics instance
let metrics = get_metrics();

// Record custom metrics
metrics.record_request("fitgirl", Duration::from_millis(250), true).await;
metrics.record_cache_hit();
```

### Logging

```rust
use tracing::{info, warn, error};

#[tracing::instrument]
async fn fetch_site(url: &str) {
    info!(url = url, "Starting fetch");
    
    match result {
        Ok(data) => info!(size = data.len(), "Fetch successful"),
        Err(e) => error!(error = %e, "Fetch failed"),
    }
}
```

## Troubleshooting

### Metrics Not Available

1. Check if port is in use:
   ```bash
   netstat -an | grep 9898
   ```

2. Disable other instances or use different port

3. Check environment variable:
   ```bash
   echo $WEBSITE_SEARCHER_NO_METRICS
   ```

### Excessive Logging

1. Set log level:
   ```bash
   RUST_LOG=warn websearcher "query"
   ```

2. Use JSON output to suppress logs:
   ```bash
   websearcher "query" --format json
   ```

### Performance Impact

- Metrics collection adds ~1ms per request
- Logging overhead depends on level and output
- Cache metrics are updated asynchronously
- Prometheus exporter runs in a separate task

## Best Practices

1. **Production**: Use JSON output with error-level logging
2. **Development**: Use table output with info-level logging
3. **Monitoring**: Keep metrics enabled for observability
4. **Testing**: Disable metrics to avoid port conflicts
5. **High Throughput**: Consider increasing log level to warn
