# CLI Reference

The `website-searcher` CLI searches multiple game download sites in parallel and displays results in JSON or table format.

## Basic Usage

```bash
# Search all sites
websearcher "elden ring"

# Limit results per site
websearcher "elden ring" --limit 5

# Search specific sites
websearcher "baldur's gate 3" --sites fitgirl,dodi,steamrip

# Table output
websearcher "starfield" --format table
```

## Command Synopsis

```
websearcher [OPTIONS] [QUERY]
```

## Arguments

| Argument | Description                                        |
| -------- | -------------------------------------------------- |
| `QUERY`  | Search term. If omitted, runs in interactive mode. |

## Options

| Flag                     | Description                                       | Default                    |
| ------------------------ | ------------------------------------------------- | -------------------------- |
| `--limit <N>`            | Maximum results per site                          | 10                         |
| `--sites <a,b,c>`        | Restrict to specific sites (comma-separated)      | all                        |
| `--invert-sites`         | Invert site selection (search all EXCEPT listed)  | off                        |
| `--format <json\|table>` | Output format                                     | json                       |
| `--json`                 | Alias for `--format json`                         | json                       |
| `-v, --verbose`          | Enable info-level logging                         | off                        |
| `--debug`                | Print diagnostics, write HTML samples to `debug/` | off                        |
| `--no-cf`                | Disable Cloudflare solver                         | CF enabled                 |
| `--cf_url <URL>`         | Override FlareSolverr endpoint                    | `http://localhost:8191/v1` |
| `--cookie <STR>`         | Forward cookies to requests and solver            | none                       |
| `--no-playwright`        | Disable Playwright fallback for cs.rin.ru         | PW enabled                 |
| `--cache-size <N>`       | Number of searches to cache (3-20)                | 3                          |
| `--no-cache`             | Disable search result caching                     | cache enabled              |
| `--clear-cache`          | Clear the search cache and exit                   |                            |
| `-h, --help`             | Print help                                        |                            |
| `-V, --version`          | Print version                                     |                            |

## Interactive Mode

Run without arguments to enter interactive mode:

```bash
websearcher
```

1. Enter search phrase when prompted
2. Select sites (type `all` or comma-separated names/numbers)
3. **Live progress TUI** displays per-site status in real-time
4. Results display in navigable TUI when search completes

### Live Search Progress

In interactive mode, a real-time progress display shows:

- Per-site status with emoji indicators:
  - `⏳` Pending - Site queued for search
  - `🔄` Fetching - HTTP request in progress
  - `📄` Parsing - Extracting results from HTML
  - `✅` Completed - Site finished with result count
  - `❌` Failed - Site encountered an error
- Overall progress bar showing sites completed
- Total results found so far

The progress TUI automatically transitions to the results browser when all sites complete.

## Output Formats

### JSON (default)

```bash
websearcher "cyberpunk" --limit 2
```

```json
[
  {
    "site": "fitgirl",
    "title": "Cyberpunk 2077 (v2.0)",
    "url": "https://fitgirl-repacks.site/cyberpunk-2077/"
  },
  {
    "site": "steamrip",
    "title": "Cyberpunk 2077",
    "url": "https://steamrip.com/cyberpunk-2077/"
  }
]
```

### Table

```bash
websearcher "cyberpunk" --limit 2 --format table
```

```
┌──────────┬─────────────────────────────────────────────────┐
│ Site     │ Title / URL                                     │
├──────────┼─────────────────────────────────────────────────┤
│ fitgirl  │ Cyberpunk 2077 (v2.0)                           │
│          │ https://fitgirl-repacks.site/cyberpunk-2077/    │
├──────────┼─────────────────────────────────────────────────┤
│ steamrip │ Cyberpunk 2077                                  │
│          │ https://steamrip.com/cyberpunk-2077/            │
└──────────┴─────────────────────────────────────────────────┘
```

## TUI Mode

Table output automatically launches a TUI when running interactively:

### Search Progress TUI

During the search phase:

| Key         | Action                      |
| ----------- | --------------------------- |
| `q` / `Esc` | Cancel search and quit      |
| `Enter`     | View results (when done)    |

The progress display shows:
- Gauge showing overall completion percentage
- List of all sites with their current status
- Running count of total results found

### Results Browser TUI

After search completes:

| Key           | Action                         |
| ------------- | ------------------------------ |
| `↑` / `k`     | Move selection up              |
| `↓` / `j`     | Move selection down            |
| `PgUp`        | Scroll up 10 rows              |
| `PgDn`        | Scroll down 10 rows            |
| `Home`        | Jump to top                    |
| `End`         | Jump to bottom                 |
| `Enter` / `o` | Open selected URL in browser   |
| `q` / `Esc`   | Quit TUI                       |

### Navigation

- Results are grouped by site in bordered boxes
- Use arrow keys to navigate between results
- Terminal resizing updates layout automatically
- Selected URL is shown in footer

## Cloudflare Handling

FlareSolverr is used to bypass Cloudflare protection on supported sites.

```bash
# Use default solver (localhost:8191)
websearcher "elden ring"

# Disable solver
websearcher "elden ring" --no-cf

# Custom solver endpoint
websearcher "elden ring" --cf_url http://my-solver:8191/v1
```

**Start FlareSolverr locally:**

```bash
docker run -d --name flaresolverr -p 8191:8191 \
  ghcr.io/flaresolverr/flaresolverr:latest
```

## Cookie Forwarding

Some sites require cookies for full results:

```bash
# gog-games benefits from session cookies
websearcher "cyberpunk" --sites gog-games \
  --cookie "cf_clearance=...; session=..."
```

Cookies are forwarded to both direct requests and FlareSolverr payload.

## Playwright Integration

cs.rin.ru uses Playwright for JavaScript-rendered search:

```bash
# Auto-uses Playwright (default)
websearcher "elden ring" --sites csrin

# Disable Playwright (uses HTML fallback)
websearcher "elden ring" --sites csrin --no-playwright
```

**Environment variables:**

- `CSRIN_PAGES` - Number of result pages to fetch (default: 1)

**Setup Playwright locally:**

```bash
npm i -D playwright
npx playwright install --with-deps
```

## Debug Mode

Debug mode helps troubleshoot parsing issues:

```bash
websearcher "elden ring" --debug
```

- Prints verbose diagnostics to stderr
- Writes HTML samples to `debug/` directory
- Filenames: `debug/{site}_sample.html`

## Monitoring and Logging

The application includes structured logging and Prometheus metrics:

### Metrics

When running, Prometheus metrics are available on port 9898 (or next available port):

```bash
curl http://localhost:9898/metrics
```

Metrics include:
- Request counts per site
- Success/failure rates
- Response times
- Cache hit/miss ratios

### Logging

Structured logging with configurable verbosity:

```bash
# Default: error-level logging only
websearcher "query"

# Verbose: info-level logging
websearcher "query" --verbose

# Debug: debug-level logging (most detailed)
websearcher "query" --debug

# JSON output always uses error-level only (clean output)
websearcher "query" --format json
```

**Log levels:**

| Flag        | Level | Description                            |
| ----------- | ----- | -------------------------------------- |
| (default)   | Error | Only errors shown                      |
| `--verbose` | Info  | General operation info + errors        |
| `--debug`   | Debug | Detailed diagnostics + info + errors   |

### Environment Variables

- `WEBSITE_SEARCHER_NO_METRICS=1` - Disable metrics exporter

## Site-Specific Notes

| Site         | Notes                                                     |
| ------------ | --------------------------------------------------------- |
| `gog-games`  | CF solver ignored; cookies improve results                |
| `elamigos`   | Parses homepage (no search), filters locally              |
| `csrin`      | Uses Playwright by default; `CSRIN_PAGES` controls depth  |
| `ankergames` | Uses path-encoded search; falls back to listing page      |
| `f95zone`    | Browse-only (search requires auth); parses forum listings |

## Multi-Query Syntax

Use the pipe (`|`) separator to search different games on different sites simultaneously:

```bash
# Search fitgirl for "elden ring" AND csrin for "minecraft"
websearcher "elden ring site:fitgirl | minecraft site:csrin"

# Complex multi-query with exclusions
websearcher "elden ring -nightreign site:fitgirl,dodi | minecraft site:elamigos,csrin"
```

**Multi-query behavior:**

- Each segment (separated by `|`) is parsed independently
- Segments with `site:` restrictions apply only to those sites
- Segments without `site:` restrictions apply to ALL sites
- Sites not explicitly mentioned in any segment search for ALL segments without site restrictions

**Example:**
```bash
websearcher "elden ring site:fitgirl | minecraft site:csrin | cyberpunk"
```
- `fitgirl` searches for "elden ring" AND "cyberpunk"
- `csrin` searches for "minecraft" AND "cyberpunk"
- Other sites search only for "cyberpunk"

## Invert Site Selection

The `--invert-sites` flag inverts the site selection:

```bash
# Search all sites EXCEPT fitgirl and dodi
websearcher "elden ring" --sites fitgirl,dodi --invert-sites

# Equivalent to: search all sites minus the listed ones
```

## Examples

```bash
# Quick search, JSON output
websearcher "baldur's gate 3"

# Table output with specific sites
websearcher "starfield" --sites fitgirl,dodi --format table

# High limit, debug mode
websearcher "elden ring" --limit 50 --debug

# No Cloudflare solver
websearcher "cyberpunk" --no-cf

# cs.rin.ru with 3 pages of results
CSRIN_PAGES=3 websearcher "elden ring" --sites csrin --format table

# Interactive mode
websearcher

# Verbose logging
websearcher "elden ring" --verbose

# Multi-query: different games on different sites
websearcher "elden ring site:fitgirl | minecraft site:csrin"

# Invert site selection: search all EXCEPT fitgirl
websearcher "elden ring" --sites fitgirl --invert-sites
```

## Exit Codes

| Code | Meaning                      |
| ---- | ---------------------------- |
| 0    | Success                      |
| 1    | Error (network, parse, etc.) |
| 130  | Interrupted (Ctrl+C)         |
