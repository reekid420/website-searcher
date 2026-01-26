# Architecture

Website-Searcher is a cross-platform application for searching multiple game download sites in parallel. It consists of a Rust workspace with three main crates plus a React frontend.

## Workspace Structure

```
website-searcher/
├── crates/
│   ├── core/          # Shared library - scraping, parsing, fetching, caching, monitoring
│   └── cli/           # CLI application with TUI
├── src-tauri/         # Tauri backend (Rust)
├── gui/               # React + TypeScript frontend
├── scripts/           # Helper scripts (Playwright, aliases)
├── config/            # External configuration (sites.toml)
├── quickstart.py      # Prerequisites installer
└── compile.py         # Build and packaging script
```

## Component Diagram

```mermaid
graph TB
    subgraph "User Interfaces"
        CLI[CLI Binary<br/>crates/cli]
        GUI[GUI App<br/>src-tauri + gui/]
    end
    
    subgraph "Core Library"
        Core[website_searcher_core<br/>crates/core]
        Cache[Cache Module<br/>TTL-based]
        Monitor[Monitoring Module<br/>Metrics/Logging]
    end
    
    subgraph "External Services"
        Sites[Game Sites<br/>13 supported]
        CF[FlareSolverr<br/>Cloudflare bypass]
        PW[Playwright<br/>cs.rin.ru]
        Prometheus[Prometheus<br/>Metrics endpoint]
    end
    
    CLI --> Core
    GUI --> Core
    Core --> Cache
    Core --> Monitor
    Monitor --> Prometheus
    Core --> Sites
    Core --> CF
    CLI --> PW
```

## Core Library Modules

The `website_searcher_core` crate (`crates/core/src/`) contains:

| Module | Purpose |
|--------|---------|
| `lib.rs` | Public exports |
| `models.rs` | Data types: `SearchResult`, `SiteConfig`, `SearchKind` |
| `config.rs` | Site configurations for all 13 supported sites |
| `query.rs` | URL building and query normalization |
| `fetcher.rs` | HTTP fetching with retry/backoff logic |
| `parser.rs` | HTML parsing and result extraction |
| `cf.rs` | FlareSolverr integration for Cloudflare bypass |
| `cache.rs` | TTL-based result caching with persistence |
| `rate_limiter.rs` | Per-site rate limiting with exponential backoff |
| `monitoring.rs` | Prometheus metrics and structured logging |
| `output.rs` | Table/JSON formatting utilities |

## Data Flow

```mermaid
sequenceDiagram
    participant User
    participant CLI/GUI
    participant Core
    participant Cache
    participant Monitor
    participant Fetcher
    participant Parser
    participant Site

    User->>CLI/GUI: Search query
    CLI/GUI->>Core: search(query, sites)
    Core->>Cache: check_cache(query)
    Cache-->>Core: cached results or None
    
    alt Cache miss
        loop For each site (parallel)
            Core->>Monitor: start_timer(site)
            Core->>Fetcher: fetch_page(url)
            
            alt Cloudflare protected
                Fetcher->>FlareSolverr: solve challenge
                FlareSolverr-->>Fetcher: HTML
            else Direct fetch
                Fetcher->>Site: HTTP GET
                Site-->>Fetcher: HTML
            end
            
            Fetcher-->>Core: HTML response
            Core->>Monitor: record_request(site, duration, success)
            Core->>Parser: parse_results(html)
            Parser-->>Core: Vec<SearchResult>
        end
        
        Core->>Cache: store_results(query, results)
    end
    
    Core-->>CLI/GUI: Combined results
    CLI/GUI-->>User: Display (table/JSON)
```

## Search Strategies (SearchKind)

The `SearchKind` enum defines how each site is searched:

| Variant | Description | Example |
|---------|-------------|---------|
| `QueryParam` | URL query parameter (`?s=query`) | fitgirl, steamrip |
| `FrontPage` | Parse homepage, filter locally | elamigos |
| `PathEncoded` | Query in URL path (`/search/query`) | ankergames |
| `ListingPage` | Use predefined listing URL | f95zone |
| `PhpBBSearch` | phpBB forum search with keywords param | csrin |

## SiteConfig Structure

Each site is defined with a `SiteConfig`:

```rust
pub struct SiteConfig {
    pub name: &'static str,              // Identifier (e.g., "fitgirl")
    pub base_url: &'static str,          // Base URL
    pub search_kind: SearchKind,         // How to search
    pub query_param: Option<&'static str>, // Query parameter name
    pub listing_path: Option<&'static str>, // Fallback listing URL
    pub result_selector: &'static str,   // CSS selector for results
    pub title_attr: &'static str,        // How to get title (text/attr)
    pub url_attr: &'static str,          // How to get URL (href)
    pub requires_js: bool,               // Needs JavaScript
    pub requires_cloudflare: bool,       // Needs FlareSolverr
    pub timeout_seconds: u64,            // Request timeout
    pub retry_attempts: u32,             // Number of retries
    pub rate_limit_delay_ms: u64,        // Base delay between requests
}
```

## GUI Architecture

The GUI uses Tauri 2.x with a React frontend:

- **Backend** (`src-tauri/`): Rust binary that wraps `website_searcher_core`
- **Frontend** (`gui/`): React 19 + TypeScript + Vite 7
- **Communication**: Tauri IPC commands defined in `src-tauri/src/lib.rs`
- **Sidecar**: CLI binary bundled as external executable for advanced features

## Concurrency Model

- HTTP requests use `tokio` async runtime
- Parallel site fetching with `FuturesUnordered`
- Semaphore limits concurrent requests to 3
- Each site fetch is independent; failures don't block others
- Rate limiter enforces per-site delays with exponential backoff
- Cache operations use async RwLock for concurrent access

## Caching System

The cache module provides:
- TTL-based entries with configurable expiration
- Persistent storage to platform cache directory
- LRU eviction when size limit exceeded
- Automatic cleanup of expired entries
- Thread-safe operations with async locks

## Monitoring System

The monitoring module tracks:
- Request counts per site (success/failure)
- Response time histograms
- Cache hit/miss ratios
- Active request gauges
- Structured logging with tracing
- Prometheus metrics on configurable port (9898-9907)

## Error Handling

- `anyhow` for CLI error context
- `thiserror` for library error types
- Graceful degradation: if one site fails, others continue
- Debug mode (`--debug`) writes HTML samples for troubleshooting
