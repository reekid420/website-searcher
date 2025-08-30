# Cloudflare & Dynamic Site Handling Plan

## Executive Summary
This plan describes how to extend **website-searcher** to retrieve results from sites that are currently blocked or dynamically rendered:

1. `fitgirl-repacks.site` and `dodi-repacks.download` – both protected by Cloudflare antibot (returning HTTP 403 / JS challenge).
2. `ankergames.net` – delivers fully-rendered HTML but the current CSS selector misses the result cards.

The solution set **must be free** (no paid APIs like ScrapeStack or Bright Data) and integrate cleanly with the existing Rust async architecture.

---

## Research Findings
- Cloudflare blocks plain HTTP clients with a JavaScript challenge that sets a `cf_clearance` cookie after executing JS.
- Open-source headless-browser projects (**FlareSolverr**, **cloudscraper-node**, **puppeteer-stealth**) automatically solve this challenge and expose a simple HTTP API; FlareSolverr is the most mature and is free/self-hosted.
- Rust crates that *directly* bypass Cloudflare (e.g. `cloudflare-scraper`, `anticaptcha`) are either unmaintained or require paid CAPTCHA keys.
- `ankergames.net` search pages are rendered server-side (Laravel + Livewire). Result links live under `div.game-card a` and include absolute URLs – so no JS execution is required; only CSS selector update.
- Sample HTML in `debug/ankergames_sample.html` confirms presence of search results despite large inline JS/CSS.

Sources:
1. FlareSolverr GitHub – https://github.com/FlareSolverr/FlareSolverr
2. Community discussion on bypassing Cloudflare with FlareSolverr – https://github.com/FlareSolverr/FlareSolverr/issues/73
3. Livewire HTML pattern in AnkerGames – analysis of sample file in `debug/`.

---

## Objectives & Goals
- ✅ Successfully fetch HTML from Cloudflare-protected sites within 3 attempts.
- ✅ Parse and return at least the first *N* search results for FitGirl & DODI.
- ✅ Fix Ankergames selector so it yields results >0.
- ⚙️ Keep binary completely offline/stand-alone except for optional FlareSolverr helper.
- 🆓 Rely exclusively on open-source software.

Success Metrics:
- Unit test `cargo test` passes new fixtures (`fitgirl_ok.html`, `dodi_ok.html`, `ankergames_ok.html`).
- Integration smoke test returns ≥1 result for each of the three sites.

---

## Methodology / Approach
### 1. Introduce Optional Cloudflare Solver Layer
```
src/
  cf.rs         // new – interface to FlareSolverr
```
- Provide `async fn get_html(url) -> Result<String>` inside `cf.rs`.
- Behind the scenes, it makes a POST to `http://localhost:8191/v1` (`FlareSolverr` default).
- On success, it extracts `solution.response` field containing the solved HTML.
- On failure (FlareSolverr down, timeout, CAPTCHA), fall back to existing `fetcher::fetch_with_retry`.
- Gate behind CLI flag `--use-cf` and auto-enable when site config sets `requires_cloudflare = true`.

### 2. Extend `SiteConfig`
Add boolean `requires_cloudflare` (default *false*). Set it *true* for FitGirl & DODI.

### 3. Adjust Orchestration
In `main.rs` choose fetcher:
```
html = if site.requires_cloudflare { cf::fetch_via_solver(&client, &url).await } else { fetch_with_retry(&client, &url).await }
```
Both share the same error handling / debug path.

### 4. Deploy FlareSolverr (developer instructions only)
- Provide Docker compose snippet in README:
```
docker run -d --name flaresolverr -p 8191:8191 ghcr.io/flaresolverr/flaresolverr:latest
```
- Add note: if Docker unavailable, run native Node/PM2 build (link wiki).

### 5. Update Ankergames Selector
In `config.rs` change `result_selector` to:
```
"div.game-card a[href^='/game/'], a.game-title, h2 a"
```
Add unit test fixture to guarantee non-empty parse.

### 6. Tests & Fixtures
- Save solved HTML samples (once) to `tests/fixtures/`. Use them for unit tests without hitting real site.
- Integration tests use FlareSolverr; skip automatically when `FLARESOLVERR_URL` env var missing (CI friendly).

---

## Timeline & Milestones
| Day | Milestone | Deliverables |
|-----|-----------|--------------|
|0.5|Research & spike FlareSolverr calls in Rust|PoC script `examples/cf_poc.rs` runs successfully|
|0.5|Add `cf.rs` + feature flag + update `SiteConfig`|Unit tests compile|
|0.5|Update config selector for Ankergames + parser tests|`cargo test` green|
|0.5|Integration smoke test with live FlareSolverr|`tests/integration_cloudflare.rs`|
|0.25|Docs update (`README`, `PLAN.md`) | New sections & Docker snippet|
|**Total: 2.25 dev-days**|

---

## Resource Requirements
- Human: 1 Rust dev familiar with async & HTTP.
- Tools:
  - Docker (or Podman) to run FlareSolverr locally.
  - Existing project dependencies (`reqwest`, `tokio`). No new paid services.
- No extra budget required.

---

## Risk Assessment
| Risk | Impact | Likelihood | Mitigation |
|---|---|---|---|
|Cloudflare introduces CAPTCHA that FlareSolverr cannot auto-solve|High|Medium|Detect failure and surface clear error; allow manual `cf_clearance` cookie injection as fallback|
|FlareSolverr container size (~600 MB) may deter some users|Medium|Medium|Document lightweight alternatives (playwright-cloudflare-browser) or provide pre-solved cache|
|Site HTML changes regularly|Medium|High|Keep CSS selectors configurable in `config.rs`; add parser fallback already present|
|CI environment cannot run Docker|Low|Medium|Skip Cloudflare tests when `FLARESOLVERR_URL` not set|

---

## Implementation Strategy
1. **Phase 1 – PoC**: hard-code call to FlareSolverr for one URL; commit example.
2. **Phase 2 – Library integration**: create `cf.rs`, update `SiteConfig`, feature-flag.
3. **Phase 3 – Parser tweaks**: fix Ankergames selector & add tests.
4. **Phase 4 – Docs & CI**: update docs; mark Cloudflare tests as optional.

Responsibility Matrix:
- **Developer**: code + tests
- **Reviewer**: code review & security audit
- **Maintainer**: merge & release

Communication Plan:
- Daily Slack stand-up.
- PR descriptions must include screenshots of CLI output.

Quality Assurance:
- `cargo test --all-features`
- `cargo clippy -- -D warnings`
- Manual run with FlareSolverr off (fallback path).

---

## Monitoring & Evaluation
- Track success rate (HTTP 200 vs 403) per site via debug logs.
- Add `--stats` flag to output fetch success counts.
- Re-evaluate every 30 days or on user bug reports.

---

## Next Steps
1. Merge this plan into `master`.
2. Create feature branch `feat/cloudflare-bypass`.
3. Implement Phase 1 within the next commit.
4. Schedule review meeting after PoC results.
