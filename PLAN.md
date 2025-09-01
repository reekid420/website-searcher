## Website-Searcher – Consolidated Plan (Status: Implemented)

This document consolidates and supersedes prior plans (PLAN.md, CLI_CLI_APP_PLAN.md, CLOUDFLARE_BYPASS_PLAN.md). The main plan is complete and shipped. Optional follow‑ups are listed at the end.

### 1) Project Overview
Command-line app that takes a search phrase, queries multiple game-download sites in parallel (max 3 concurrent), scrapes the first N results per site, and prints either pretty JSON or a grouped table.

Supported sites:
1. steamgg – `?s=`
2. gog-games – root `?search=` (special handling; see Cloudflare section)
3. atopgames – `?s=`
4. elamigos – FrontPage filter
5. fitgirl – `?s=` (Cloudflare heavy)
6. dodi – `?s=` (Cloudflare heavy)
7. skidrowrepacks – `?s=`
8. steamrip – `?s=`
9. reloadedsteam – `?s=`
10. ankergames – path-encoded search/listing (spaces as `%20`)

### 2) Tech & Modules
- Rust + tokio + reqwest + scraper + clap + serde/colored_json
- Core modules: `main.rs` (CLI), `config.rs` (sites), `query.rs`, `fetcher.rs`, `parser.rs`, `output.rs`, `cf.rs`

### 3) CLI & UX (Delivered)
- Flags: `--sites a,b,c`, `--limit N`, `--debug`, `--format [json|table]`, `--no-cf`, `--cf_url`, `--cookie "k=v; …"`
- Interactive mode (no query): prompts for phrase and site selection; defaults to table output
- Results sorted and grouped by site; deduped by `(site, url)`

### 4) Fetching & Cloudflare (Delivered)
- CF solver (FlareSolverr) is ON by default; disable with `--no-cf`; endpoint via `--cf_url`
- Sites can mark `requires_cloudflare = true`
- `gog-games` is solver‑blacklisted; we use normal fetch and an AJAX JSON/HTML fragment fallback. Cookies may be passed with `--cookie` and are forwarded to requests and solver headers

### 5) Parsing & Fallbacks (Delivered)
- Site-specific selectors in `config.rs`
- Parser fallback scans all anchors and derives titles from slugs
- Normalization steps improve titles (whitespace trimming, site tweaks)

### 6) Concurrency & Resilience (Delivered)
- `Semaphore(3)` to cap concurrency; retries with backoff; non-fatal errors logged in debug

### 7) Testing & CI (Delivered)
- Unit/integration tests (incl. FlareSolverr mock)
- GitHub Actions CI: fmt + clippy (deny warnings) + tests

### 8) Docker & Devcontainer (Delivered)
- Dockerfile builds a runtime image (interactive default)
- `docker-compose.dev.yml` launches FlareSolverr + app
- `.devcontainer` for VS Code Containers

### 9) Verification
- All plan items delivered; tests green; README documents usage (including CF default-on, `--no-cf`, cookies, format, interactive mode)

### 10) Optional Follow-ups
- `--stats` flag to summarize per-site fetch/fallback counts
- Optional fixtures for AJAX fallbacks in tests
- Expand site list / add TUI later
