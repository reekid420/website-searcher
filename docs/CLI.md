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

| Argument | Description |
|----------|-------------|
| `QUERY` | Search term. If omitted, runs in interactive mode. |

## Options

| Flag | Description | Default |
|------|-------------|---------|
| `--limit <N>` | Maximum results per site | 10 |
| `--sites <a,b,c>` | Restrict to specific sites (comma-separated) | all |
| `--format <json\|table>` | Output format | json |
| `--debug` | Print diagnostics, write HTML samples to `debug/` | off |
| `--no-cf` | Disable Cloudflare solver | CF enabled |
| `--cf_url <URL>` | Override FlareSolverr endpoint | `http://localhost:8191/v1` |
| `--cookie <STR>` | Forward cookies to requests and solver | none |
| `--no-playwright` | Disable Playwright fallback for cs.rin.ru | PW enabled |
| `-h, --help` | Print help | |
| `-V, --version` | Print version | |

## Interactive Mode

Run without arguments to enter interactive mode:

```bash
websearcher
```

1. Enter search phrase when prompted
2. Select sites (type `all` or comma-separated names/numbers)
3. Results display in table format

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

### Keybindings

| Key | Action |
|-----|--------|
| `↑` / `k` | Move selection up |
| `↓` / `j` | Move selection down |
| `Enter` | Open selected URL in browser |
| `c` | Copy selected URL to clipboard |
| `q` / `Esc` | Quit TUI |

### Navigation

- Results are grouped by site
- Use arrow keys to navigate between results
- Terminal resizing updates layout automatically

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

## Site-Specific Notes

| Site | Notes |
|------|-------|
| `gog-games` | CF solver ignored; cookies improve results |
| `elamigos` | Parses homepage (no search), filters locally |
| `csrin` | Uses Playwright by default; `CSRIN_PAGES` controls depth |
| `ankergames` | Uses path-encoded search; falls back to listing page |
| `f95zone` | Browse-only (search requires auth); parses forum listings |

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
```

## Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | Error (network, parse, etc.) |
| 130 | Interrupted (Ctrl+C) |
