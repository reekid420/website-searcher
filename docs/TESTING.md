# Testing Guide

Website-searcher has comprehensive unit and integration tests covering all major components.

## Test Structure

```
website-searcher/
├── crates/
│   ├── core/src/
│   │   ├── config.rs     # Unit tests for site configs
│   │   ├── parser.rs     # Unit tests for parsing
│   │   ├── fetcher.rs    # Unit tests for HTTP client
│   │   ├── cf.rs         # Unit tests for FlareSolverr
│   │   ├── output.rs     # Unit tests for output formatting
│   │   └── query.rs      # Unit tests for URL building
│   └── cli/
│       ├── main.rs       # Unit tests for CLI logic
│       └── tests/        # Integration tests
│           ├── cli_cf_mock.rs
│           ├── cli_cookie_forwarding.rs
│           ├── cli_dedup_and_limit.rs
│           ├── cli_no_results_table.rs
│           ├── cli_playwright_smoke.rs
│           └── integration_smoke.rs
├── src-tauri/src/
│   └── lib.rs            # Unit tests for Tauri commands
└── gui/
    ├── src/
    │   ├── api.test.ts   # Unit tests for API module
    │   └── App.test.tsx  # Unit tests for App component
    └── e2e/
        └── app.spec.ts   # Playwright E2E tests
```

## Running Tests

### Using test.py (Recommended)

The `test.py` script provides a unified interface for all tests:

```bash
# Run all tests (Rust + GUI + E2E)
python test.py

# Run specific test types
python test.py --rust          # Rust tests only
python test.py --gui           # GUI unit tests only
python test.py --e2e           # Playwright E2E tests only

# With coverage
python test.py --rust --coverage
python test.py --gui --coverage

# With verbose output
python test.py --verbose

# Enable logging to timestamped file
python test.py --log

# Run linting and audit
python test.py --clippy --audit
```

### Direct Rust Tests

```bash
# Standard cargo test
cargo test --workspace

# With nextest (recommended, parallel execution)
cargo nextest run --workspace
```

### GUI Unit Tests

```bash
cd gui
npm install
npm test                    # Run once
npm run test:watch          # Watch mode
npm run test:coverage       # With coverage
```

### E2E Tests

```bash
cd gui
npx playwright install      # First time only
npm run test:e2e           # Run E2E tests
npm run test:e2e:ui        # Interactive UI mode
```

### Single Test File

```bash
cargo test --test integration_smoke
cargo test --test cli_dedup_and_limit
```

### Specific Test

```bash
cargo test normalize_title_collapses_whitespace
```

### With Output

```bash
cargo test --test integration_smoke -- --nocapture
```

## Unit Tests

### config.rs

Tests site configuration invariants:

| Test                                               | Purpose             |
| -------------------------------------------------- | ------------------- |
| `fitgirl_and_dodi_require_cloudflare`              | Verify CF flags     |
| `gog_games_not_cloudflare_and_has_queryparam`      | Verify GOG config   |
| `elamigos_is_frontpage_and_ankergames_pathencoded` | Verify search kinds |

### parser.rs

Tests HTML parsing:

| Test                                                     | Purpose               |
| -------------------------------------------------------- | --------------------- |
| `primary_selector_is_filtered_by_query`                  | Query filtering       |
| `primary_relative_href_becomes_absolute`                 | URL resolution        |
| `fallback_finds_query_text`                              | Fallback parsing      |
| `derives_title_from_empty_anchor_text`                   | Title extraction      |
| `fitgirl_filters_and_normalizes`                         | Site-specific parsing |
| `parse_elamigos_headings_extract_title_and_absolute_url` | ElAmigos parsing      |
| `parse_f95zone_extracts_thread_links`                    | F95zone parsing       |
| `parse_f95zone_deduplicates_urls`                        | F95zone deduplication |
| `parse_nswpedia_extracts_game_links`                     | NSWpedia parsing      |
| `parse_nswpedia_skips_nav_elements`                      | NSWpedia filtering    |
| `csrin_topictitle_parses_relative_url_with_query`        | csrin search page     |
| `derive_title_handles_query_strings`                     | Title from URL        |
| `steamrip_filter_drops_nav_links`                        | SteamRIP filtering    |
| `looks_like_date_detects_date_format`                    | Date detection        |

### fetcher.rs

Tests HTTP fetching:

| Test                   | Purpose              |
| ---------------------- | -------------------- |
| Retry/backoff behavior | Automatic retries    |
| 200/302/403 handling   | Status code handling |
| Header forwarding      | Custom headers       |

### cf.rs

Tests FlareSolverr integration:

| Test                         | Purpose           |
| ---------------------------- | ----------------- |
| Success JSON handling        | Valid responses   |
| Error JSON handling          | Error responses   |
| Header forwarding in payload | Cookie forwarding |

### query.rs

Tests URL building:

| Test                                     | Purpose        |
| ---------------------------------------- | -------------- |
| Normalization                            | Query cleanup  |
| URL building for all SearchKind variants | URL generation |

## Integration Tests

### cli_cf_mock.rs

Mocks FlareSolverr to test:

- CF solver enabled by default
- JSON output format
- Table output grouping

### cli_cookie_forwarding.rs

Tests `--cookie` flag:

- Cookies forwarded to solver payload
- Cookie header format

### cli_dedup_and_limit.rs

Tests result processing:

- Deduplication of identical results
- Per-site `--limit` enforcement

### cli_no_results_table.rs

Tests empty result handling:

- Table output shows "No results."
- Clean exit on empty results

### cli_playwright_smoke.rs

Smoke test for Playwright:

- Executes with stubbed HTML
- Verifies Playwright code path

### integration_smoke.rs

Comprehensive CLI tests:

- Interactive empty input error
- Multi-site grouping
- Site filtering
- Unknown sites → empty JSON
- `--debug` writes HTML samples
- Per-site limit across sites

## Mocking

### FlareSolverr Mock

Tests use `mockito` to mock FlareSolverr:

```rust
use mockito::Server;

let mut server = Server::new_with_port(8191);
let mock = server.mock("POST", "/v1")
    .with_status(200)
    .with_body(r#"{"status":"ok","solution":{"response":"..."}}"#)
    .create();
```

Override CF URL in tests:

```bash
websearcher "test" --cf_url http://localhost:8191/v1
```

### Environment Variables

| Variable                      | Purpose                                      |
| ------------------------------ | -------------------------------------------- |
| `NO_COLOR=1`                  | Disable ANSI colors (for assertion matching) |
| `RUST_BACKTRACE=1`             | Show backtraces on failure                   |
| `WEBSITE_SEARCHER_NO_METRICS=1`| Disable metrics exporter (auto-set in tests) |
| `CSRIN_PAGES`                  | Number of csrin pages to fetch (default: 1)  |

### Test Configuration

Tests automatically disable the metrics exporter to avoid port conflicts. This is configured via:

1. `.cargo/config.toml` - Sets `WEBSITE_SEARCHER_NO_METRICS=1` for all cargo commands
2. Test files use `--no-cache` flag to avoid cache interference
3. Unique site names in rate limiter tests prevent state leakage

## Playwright Tests

Playwright tests require Node.js and Playwright installed:

```bash
# Setup
npm i -D playwright
npx playwright install --with-deps

# Run smoke test
cargo test --test cli_playwright_smoke
```

### CI Configuration

CI runs Playwright tests with:

```yaml
- run: npx playwright install --with-deps
- run: cargo test --test cli_playwright_smoke
  env:
    CSRIN_PAGES: 1
```

## Debug Mode in Tests

Some tests enable `--debug` and write HTML samples:

```bash
# Clean up debug files
rm -f debug/*.html
# Or on Windows:
Remove-Item -Force -ErrorAction SilentlyContinue debug\*.html
```

## Coverage

Generate coverage reports with `cargo-llvm-cov`:

```bash
# Install
cargo install cargo-llvm-cov

# HTML report
cargo llvm-cov --workspace --html
# Opens: target/llvm-cov/html/index.html

# Summary only
cargo llvm-cov --workspace --summary-only

# Via test.py with logging
python test.py --rust --coverage --log
```

### Linux Alternative

```bash
cargo install cargo-tarpaulin
cargo tarpaulin --out Html
```

## CI Test Matrix

Tests run on 5 platforms:

| Platform      | Runner           |
| ------------- | ---------------- |
| Linux x64     | ubuntu-latest    |
| Linux ARM64   | ubuntu-24.04-arm |
| macOS ARM64   | macos-latest     |
| Windows x64   | windows-latest   |
| Windows ARM64 | windows-11-arm   |

See `.github/workflows/ci.yml` for full CI configuration.

## Writing New Tests

### Unit Test

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn my_function_does_thing() {
        let result = my_function("input");
        assert_eq!(result, "expected");
    }
}
```

### Integration Test

Create `crates/cli/tests/my_test.rs`:

```rust
use assert_cmd::Command;

#[test]
fn cli_does_thing() {
    let mut cmd = Command::cargo_bin("website-searcher").unwrap();
    cmd.arg("test")
       .assert()
       .success();
}
```

### Async Test

```rust
#[tokio::test]
async fn async_function_works() {
    let result = async_function().await;
    assert!(result.is_ok());
}
```
