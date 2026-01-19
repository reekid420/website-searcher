# Development Guide

This guide covers setting up a development environment and contributing to website-searcher.

## Prerequisites

- Rust 1.89+ (via rustup)
- Node.js 20+, pnpm 10+
- Python 3.8+ (for build scripts)
- Git

Run the quickstart script to install everything:
```bash
python quickstart.py
```

## Development Environment

### VS Code

Recommended extensions (auto-installed in devcontainer):

| Extension | Purpose |
|-----------|---------|
| rust-analyzer | Rust language support |
| Even Better TOML | TOML file support |
| Dependi | Cargo.toml dependency updates |
| Todo2 | Task management |

### Devcontainer

Open the project in VS Code and choose "Reopen in Container" for a preconfigured environment.

The devcontainer:
- Uses `mcr.microsoft.com/devcontainers/base:ubuntu`
- Auto-runs `quickstart.sh` and `compile.sh`
- Includes all recommended extensions

## Project Structure

```
website-searcher/
├── crates/
│   ├── core/           # Shared library
│   │   └── src/
│   │       ├── lib.rs      # Public exports
│   │       ├── models.rs   # Data types
│   │       ├── config.rs   # Site configurations
│   │       ├── query.rs    # URL building
│   │       ├── fetcher.rs  # HTTP client
│   │       ├── parser.rs   # HTML parsing
│   │       ├── cf.rs       # FlareSolverr
│   │       └── output.rs   # Formatting
│   └── cli/            # CLI binary
│       ├── main.rs         # Entry point
│       └── tests/          # Integration tests
├── src-tauri/          # Tauri backend
│   └── src/
│       └── lib.rs          # IPC commands
├── gui/                # React frontend
│   └── src/
│       ├── App.tsx
│       └── main.tsx
└── scripts/            # Helper scripts
    └── csrin_search.cjs    # Playwright helper
```

## Building

### Quick Build

```bash
python compile.py
```

### Development Build (with caching)

```bash
# CLI only
cargo build -p website-searcher

# Full workspace
cargo build --workspace

# GUI dev mode (hot reload)
cargo tauri dev
```

## Code Style

### Formatting

```bash
# Check formatting
cargo fmt --all -- --check

# Auto-fix formatting
cargo fmt --all
```

### Linting

```bash
cargo clippy --all-targets -- -D warnings
```

## Running Tests

```bash
# All tests
cargo test --workspace

# With nextest (recommended)
cargo nextest run --workspace

# Single test file
cargo test --test integration_smoke

# Show output
cargo test --test cli_dedup_and_limit -- --nocapture
```

See [TESTING.md](TESTING.md) for detailed testing documentation.

## Adding a New Site

### 1. Create Site Configuration

Edit `crates/core/src/config.rs`:

```rust
SiteConfig {
    name: "newsite",                           // Unique identifier
    base_url: "https://newsite.com/",          // Base URL
    search_kind: SearchKind::QueryParam,       // How to search
    query_param: Some("s"),                    // Query parameter
    listing_path: None,                        // Fallback URL
    result_selector: "h2.entry-title a",       // CSS selector
    title_attr: "text",                        // Title source
    url_attr: "href",                          // URL source
    requires_js: false,                        // Needs JavaScript?
    requires_cloudflare: false,                // Needs FlareSolverr?
},
```

### 2. Add Parser (if needed)

For sites needing custom parsing, add to `crates/core/src/parser.rs`:

```rust
fn parse_newsite(site: &SiteConfig, html: &str, query: &str) -> Vec<SearchResult> {
    // Custom parsing logic
}
```

Then call it from `parse_results()`.

### 3. Add Tests

Create tests in the appropriate module:

```rust
#[test]
fn newsite_parses_results() {
    let html = r#"<html>...</html>"#;
    let cfg = site_configs().iter().find(|c| c.name == "newsite").unwrap();
    let results = parse_results(cfg, html, "test");
    assert!(!results.is_empty());
}
```

### 4. Update Documentation

Add the new site to `docs/SITES.md`.

## Debug Mode

Enable debug output and HTML samples:

```bash
websearcher "test" --debug
```

HTML samples are written to `debug/{site}_sample.html`.

## Making a Pull Request

1. Fork the repository
2. Create a feature branch: `git checkout -b feature/my-feature`
3. Make changes and commit: `git commit -am 'Add feature'`
4. Run checks:
   ```bash
   cargo fmt --all
   cargo clippy --all-targets
   cargo test --workspace
   ```
5. Push and create PR: `git push origin feature/my-feature`

## Environment Variables

| Variable | Purpose | Default |
|----------|---------|---------|
| `CF_URL` | FlareSolverr endpoint | `http://localhost:8191/v1` |
| `CSRIN_PAGES` | cs.rin.ru result pages | `1` |
| `RUST_LOG` | Log level | `info` |
| `NO_COLOR` | Disable colors | unset |

## Useful Commands

```bash
# Check for outdated dependencies
cargo outdated

# Security audit
cargo audit

# Coverage report
cargo llvm-cov --workspace --html

# Release build
cargo build --release --workspace
```
