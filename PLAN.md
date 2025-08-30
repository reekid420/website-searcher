### 1. Project Overview
Build a cross-platform command-line application that takes a search phrase, queries several game-download sites in parallel (max 3 concurrent requests), scrapes the first N results from each site, and prints a nicely formatted JSON table containing the title and link for every hit.

Supported sites (initial):
1. https://steamgg.net/?s={query}
2. https://gog-games.to/?search={query}
3. https://atopgames.com/?s={query}
4. https://elamigos.site  *(no onsite search; scrape front page and filter titles locally)*
5. https://fitgirl-repacks.site/?s={query}
6. https://dodi-repacks.download/?s={query}
7. https://skidrowrepacks.com/?s={query}
8. https://steamrip.com/?s={query}
9. https://reloadedsteam.com/?s={query}
10.https://ankergames.net/search/{query}   *(use %20 for this website  not + )* 


### 2. Tech Stack & Dependencies
* **Language**: Rust 1.78+  – fast, single binary, cross-platform, no external runtime.
* **Async Runtime**: `tokio` – lightweight tasks & concurrency limits.
* **HTTP Client**: `reqwest` – async TLS-enabled requests.
* **HTML Parsing**: `scraper` – CSS selectors for scraping titles/links.
* **CLI Parsing**: `clap` – ergonomic argument handling & help output.
* **Pretty JSON/TTY Output**: `serde_json` + `colored_json`.
* **Caching (optional v2)**: `sled` (embedded KV) or local JSON file + timestamps.
* **Testing**: `cargo test`, `mockito` for HTTP stubbing.

### 3. High-Level Flow
1. Parse CLI args → search phrase (+ optional flags: `--sites`, `--limit`, `--no-cache`).
2. Normalize query: trim, replace spaces with `+`.
3. Build site-specific URLs **or landing pages** according to `SiteConfig`. If a site lacks a search endpoint (e.g. *elamigos.site*), fetch the landing page and perform client-side filtering on the parsed titles.
4. Dispatch async fetch tasks via `tokio::spawn`, governed by a semaphore (max 3).
5. For each response:
   a. Verify status 200, fallback retry ×3 with back-off.
   b. Parse HTML, extract `<a>`/heading nodes matching site-specific selectors.
   c. Produce `SearchResult { site, title, url }`.
6. Collect results → sort by site → serialize to pretty-printed JSON.
7. Print to stdout with syntax highlighting.
8. Exit with code 0; non-fatal errors captured in stderr log.

### 4. Detailed Module Breakdown
```
src/
  main.rs              → CLI entry & orchestration
  config.rs            → Static site descriptors & CSS selectors (incl. `SearchKind::{QueryParam, FrontPage}`)
  query.rs             → Query normalisation + URL builder
  fetcher.rs           → HTTP logic, retry, concurrency control
  parser.rs            → HTML parse helpers per site (four implementations to start)
  models.rs            → SearchResult, SiteConfig structs
  output.rs            → Pretty JSON & table view
  cache.rs (opt)       → Read/write local cache
```

### 5. Concurrency Strategy
* Use `tokio::sync::Semaphore(3)`; tasks acquire permit before HTTP call.
* Futures joined via `futures::stream::FuturesUnordered` for efficient gathering.

### 6. Error Handling & Resilience
* Timeouts (15s) & retries with exponential back-off (300ms→1.2s).
* Graceful degradation: log site failures, continue others.
* Custom `ResultExt` trait for context‐rich errors.

### 7. Caching (Nice-to-have)
* Key: `site|query` → `Vec<SearchResult>` + `fetched_at`.
* TTL configurable (e.g., 7 days); bypass with `--no-cache`.
* Implement after MVP; gated by feature flag `cache`.

### 8. Testing Strategy
* Unit tests for query normalisation, URL generation, HTML parser per site (using saved fixture pages).
* Integration test using `mockito` to simulate site responses.
* CI workflow (GitHub Actions) → `cargo fmt` + `clippy` + tests.

### 9. Security & Compliance
* Respect `robots.txt`; expose `--respect-robots` flag (default on).
* User-Agent header identifies the tool.
* No storage of secrets; all open websites.

### 10. Performance Considerations
* Streaming body parsing to lower memory.
* Concurrency capped to 3 → avoids overloading sites or local network.

### 11. Installation & Usage (Windows)
1. Install Rust (https://rustup.rs).
2. Clone repo → `cargo build --release`.
3. Add `target\release` to PATH or `cargo install --path .`.
4. Example:
   ```powershell
   websearcher "rust game" --limit 20 --output json
   ```

### 12. Future Enhancements
* Add more sites via config file.
* Optional UI (TUI with `ratatui`).
* Result filtering & keyword highlighting.
* Export CSV.
* Background scheduler to refresh cache weekly.

---

**Milestones**
1. Scaffold project & CLI (clap) – 0.5 d
2. Implement site configs & query builder – 0.5 d
3. HTTP fetcher with concurrency & retries – 1 d
4. HTML parsers for 4 sites – 1 d
5. Pretty JSON output – 0.5 d
6. Tests & docs – 1 d
7. Packaging & Windows instructions – 0.5 d
8. Optional cache layer – 1 d

_Total: ~6 developer-days._
