# Core Library API Reference

The `website_searcher_core` crate provides shared functionality for searching game download sites.

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
website_searcher_core = { path = "crates/core" }
```

## Overview

```rust
use website_searcher_core::{
    config::site_configs,
    models::{SearchResult, SiteConfig, SearchKind},
    query::{build_search_url, normalize_query},
    parser::parse_results,
    fetcher::fetch_page,
    cf::solve_cloudflare,
    output::{format_table, format_json},
};
```

## Modules

### models

Data types used throughout the library.

#### SearchResult

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub site: String,   // Site identifier (e.g., "fitgirl")
    pub title: String,  // Game title
    pub url: String,    // Full URL to result
}
```

#### SearchKind

```rust
#[derive(Debug, Clone, Copy)]
pub enum SearchKind {
    QueryParam,    // URL query parameter (?s=query)
    FrontPage,     // Parse homepage, filter locally
    PathEncoded,   // Query in URL path (/search/query)
    ListingPage,   // Predefined listing URL
    PhpBBSearch,   // phpBB forum search
}
```

#### SiteConfig

```rust
pub struct SiteConfig {
    pub name: &'static str,
    pub base_url: &'static str,
    pub search_kind: SearchKind,
    pub query_param: Option<&'static str>,
    pub listing_path: Option<&'static str>,
    pub result_selector: &'static str,
    pub title_attr: &'static str,
    pub url_attr: &'static str,
    pub requires_js: bool,
    pub requires_cloudflare: bool,
}
```

### config

Site configuration definitions.

#### site_configs()

Returns all supported site configurations:

```rust
pub fn site_configs() -> Vec<SiteConfig>
```

**Example:**
```rust
let sites = site_configs();
let fitgirl = sites.iter().find(|s| s.name == "fitgirl").unwrap();
println!("FitGirl URL: {}", fitgirl.base_url);
```

### query

URL building and query processing.

#### build_search_url()

Constructs a search URL for a site:

```rust
pub fn build_search_url(site: &SiteConfig, query: &str) -> String
```

**Example:**
```rust
let site = site_configs().iter().find(|s| s.name == "steamrip").unwrap();
let url = build_search_url(site, "elden ring");
// "https://steamrip.com/?s=elden+ring"
```

#### normalize_query()

Normalizes a search query:

```rust
pub fn normalize_query(query: &str) -> String
```

**Example:**
```rust
let normalized = normalize_query("  Elden   Ring  ");
// "elden ring"
```

### parser

HTML parsing and result extraction.

#### parse_results()

Parses HTML and extracts search results:

```rust
pub fn parse_results(
    site: &SiteConfig, 
    html: &str, 
    query: &str
) -> Vec<SearchResult>
```

**Example:**
```rust
let html = r#"<html><body>
    <h2 class="entry-title">
        <a href="/game/1">Elden Ring</a>
    </h2>
</body></html>"#;

let site = site_configs().iter().find(|s| s.name == "steamrip").unwrap();
let results = parse_results(site, html, "elden ring");
```

### fetcher

HTTP client with retry logic.

#### fetch_page()

Fetches a page with automatic retries:

```rust
pub async fn fetch_page(
    client: &reqwest::Client,
    url: &str,
    headers: Option<HeaderMap>,
) -> Result<String>
```

**Example:**
```rust
let client = reqwest::Client::new();
let html = fetch_page(&client, "https://example.com", None).await?;
```

### cf

FlareSolverr integration for Cloudflare bypass.

#### solve_cloudflare()

Uses FlareSolverr to bypass Cloudflare:

```rust
pub async fn solve_cloudflare(
    client: &reqwest::Client,
    cf_url: &str,
    target_url: &str,
    cookies: Option<String>,
) -> Result<String>
```

**Example:**
```rust
let html = solve_cloudflare(
    &client,
    "http://localhost:8191/v1",
    "https://fitgirl-repacks.site/?s=elden+ring",
    None,
).await?;
```

### output

Formatting utilities.

#### format_table()

Formats results as a table string:

```rust
pub fn format_table(results: &[SearchResult]) -> String
```

#### format_json()

Formats results as JSON:

```rust
pub fn format_json(results: &[SearchResult]) -> String
```

## Error Handling

The library uses `anyhow::Result` for error handling with `thiserror` for error types:

```rust
use website_searcher_core::Error;

match fetch_page(&client, url, None).await {
    Ok(html) => println!("Got {} bytes", html.len()),
    Err(e) => eprintln!("Fetch failed: {}", e),
}
```

## Usage Example

```rust
use website_searcher_core::{
    config::site_configs,
    query::build_search_url,
    parser::parse_results,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let client = reqwest::Client::new();
    
    // Get site config
    let site = site_configs()
        .into_iter()
        .find(|s| s.name == "steamrip")
        .unwrap();
    
    // Build search URL
    let url = build_search_url(&site, "elden ring");
    
    // Fetch HTML
    let resp = client.get(&url).send().await?;
    let html = resp.text().await?;
    
    // Parse results
    let results = parse_results(&site, &html, "elden ring");
    
    for result in results {
        println!("{}: {}", result.title, result.url);
    }
    
    Ok(())
}
```

## Thread Safety

- All types are `Send + Sync`
- Site configs are static references
- Safe for concurrent use across threads
