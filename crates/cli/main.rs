use anyhow::Result;
use clap::{Parser, ValueEnum};
use futures::stream::{FuturesUnordered, StreamExt};
use scraper::{Html, Selector};
use std::sync::Arc;
use tokio::sync::Semaphore;

use website_searcher_core::cache::{MIN_CACHE_SIZE, SearchCache};
use website_searcher_core::{cf, fetcher, output};

use crossterm::event::KeyEventKind;
use crossterm::{event, execute, terminal};
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap},
};
use reqwest::header::{
    ACCEPT, COOKIE, HeaderMap as ReqHeaderMap, HeaderName, HeaderValue, REFERER,
};
use serde_json::Value;
use std::io::IsTerminal;
use std::io::stdout;
use std::process::Stdio;
use tokio::io::AsyncReadExt;
use tokio::process::Command;
use website_searcher_core::cf::fetch_via_solver;
use website_searcher_core::config::site_configs;
use website_searcher_core::fetcher::{build_http_client, fetch_with_retry};
use website_searcher_core::models::{SearchKind, SearchResult};
use website_searcher_core::parser::parse_results;
use website_searcher_core::query::{build_search_url, normalize_query};

#[derive(Copy, Clone, Debug, Eq, PartialEq, ValueEnum)]
enum OutputFormat {
    Json,
    Table,
}

fn normalize_title(site: &str, title: &str) -> String {
    // Collapse whitespace
    let mut cleaned = title
        .lines()
        .find(|l| !l.trim().is_empty())
        .unwrap_or(title)
        .to_string();
    cleaned = cleaned.replace(['\n', '\r'], " ");
    cleaned = cleaned.split_whitespace().collect::<Vec<_>>().join(" ");
    cleaned = cleaned.trim().to_string();
    if site.eq_ignore_ascii_case("ankergames") {
        // Remove trailing size like "64.91 GB" or similar tokens
        if let Some(idx) = cleaned.rfind(" GB") {
            // keep only if the tail looks like a size pattern
            if idx > 0 && idx + 3 == cleaned.len() {
                // trim back to before the size block (try to cut last token group)
                if let Some(space_idx) = cleaned[..idx].rfind(' ') {
                    cleaned = cleaned[..space_idx].trim().to_string();
                }
            }
        }
    } else if site.eq_ignore_ascii_case("csrin") {
        // Drop forum boilerplate like "Main Forum •" and leading "Re:"
        let mut c = cleaned.replace("Main Forum •", "");
        let c_trim = c.trim_start();
        if let Some(stripped) = c_trim.strip_prefix("Re: ") {
            c = stripped.to_string();
        } else if let Some(stripped) = c_trim.strip_prefix("Re:") {
            c = stripped.to_string();
        }
        cleaned = c.trim().to_string();
    }
    cleaned
}

#[derive(Debug, Parser)]
#[command(name = "websearcher", version, about = "Parallel game site searcher")]
struct Cli {
    /// Search phrase
    query: Option<String>,

    /// Limit results per site
    #[arg(long, default_value_t = 10)]
    limit: usize,

    /// Comma-separated site list to include (default: all)
    #[arg(long)]
    sites: Option<String>,

    /// Print per-site debug info
    #[arg(long, default_value_t = false)]
    debug: bool,

    /// Output format: json or table
    #[arg(long, value_enum, default_value_t = OutputFormat::Json)]
    format: OutputFormat,

    /// Disable FlareSolverr Cloudflare solver (enabled by default). Use this to opt out.
    #[arg(long, default_value_t = false)]
    no_cf: bool,
    /// FlareSolverr endpoint
    #[arg(long, default_value = "http://localhost:8191/v1")]
    cf_url: String,

    /// Cookie header to forward (e.g., from your browser) for protected sites
    #[arg(long)]
    cookie: Option<String>,

    /// Disable Playwright fallback for cs.rin.ru (forces non-PW backups only)
    #[arg(long, default_value_t = false)]
    no_playwright: bool,

    /// Maximum number of searches to cache (default: 3, max: 20)
    #[arg(long, default_value_t = MIN_CACHE_SIZE)]
    cache_size: usize,

    /// Disable search result caching
    #[arg(long, default_value_t = false)]
    no_cache: bool,

    /// Clear the search cache and exit
    #[arg(long, default_value_t = false)]
    clear_cache: bool,
}

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Cache file path - use platform-appropriate cache directory
    let cache_path = dirs::cache_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("website-searcher")
        .join("search_cache.json");

    // Handle --clear-cache flag
    if cli.clear_cache {
        if cache_path.exists() {
            std::fs::remove_file(&cache_path)?;
            println!("Cache cleared successfully.");
        } else {
            println!("No cache to clear.");
        }
        return Ok(());
    }

    // Load or create cache
    let mut search_cache = if !cli.no_cache && cache_path.exists() {
        SearchCache::load_from_file_sync(&cache_path)
            .unwrap_or_else(|_| SearchCache::new(cli.cache_size))
    } else {
        SearchCache::new(cli.cache_size)
    };
    // Update cache size if specified
    search_cache.set_max_size(cli.cache_size);

    // Interactive prompt when query omitted
    let query_value: String = match &cli.query {
        Some(q) => q.clone(),
        None => {
            println!("Website Searcher (interactive)\n");

            // Show recent searches if any
            if !search_cache.is_empty() {
                println!("Recent searches:");
                for (i, entry) in search_cache.entries_newest_first().enumerate().take(5) {
                    println!(
                        "  {}. {} ({} results)",
                        i + 1,
                        entry.query,
                        entry.results.len()
                    );
                }
                println!();
            }

            // Prefer TUI prompt when attached to a TTY; fall back to stdin prompt otherwise
            if std::io::stdin().is_terminal() && std::io::stdout().is_terminal() {
                let ans = inquire::Text::new("Search phrase:")
                    .with_placeholder("e.g., elden ring")
                    .prompt();
                match ans {
                    Ok(s) if !s.trim().is_empty() => s.trim().to_string(),
                    _ => anyhow::bail!("empty search phrase"),
                }
            } else {
                use std::io::{self, Write};
                print!("Search phrase: ");
                let _ = io::stdout().flush();
                let mut line = String::new();
                io::stdin().read_line(&mut line)?;
                let s = line.trim().to_string();
                if s.is_empty() {
                    anyhow::bail!("empty search phrase");
                }
                s
            }
        }
    };
    let normalized = normalize_query(&query_value);

    // Check cache first (unless disabled)
    if !cli.no_cache
        && let Some(cached) = search_cache.get(&normalized)
    {
        if cli.debug {
            eprintln!(
                "[debug] Cache hit for \"{}\" ({} results)",
                normalized,
                cached.results.len()
            );
        }
        // Use cached results
        let combined = cached.results.clone();
        let out_format = if cli.query.is_none() {
            OutputFormat::Table
        } else {
            cli.format
        };
        let interactive_tui = cli.query.is_none()
            && std::io::stdin().is_terminal()
            && std::io::stdout().is_terminal();
        if interactive_tui && matches!(out_format, OutputFormat::Table) {
            run_live_tui(&combined)?;
        } else {
            match out_format {
                OutputFormat::Json => output::print_pretty_json(&combined),
                OutputFormat::Table => output::print_table_grouped(&combined),
            }
        }
        return Ok(());
    }

    // Resolve CF URL: prefer CLI if non-default; otherwise allow CF_URL env override (for Docker)
    let mut resolved_cf_url = cli.cf_url.clone();
    if let (true, Some(env_cf)) = (
        resolved_cf_url == "http://localhost:8191/v1",
        std::env::var("CF_URL")
            .ok()
            .filter(|s| !s.trim().is_empty()),
    ) {
        resolved_cf_url = env_cf;
    }

    // All site configs loaded once
    let all_sites = site_configs();

    // Interactive site selection only when no --sites provided and interactive mode
    let interactive_selection: Option<Vec<String>> = if cli.sites.is_none() && cli.query.is_none() {
        if std::io::stdin().is_terminal() && std::io::stdout().is_terminal() {
            // First ask if the user wants to search ALL sites (faster flow)
            match inquire::Confirm::new("Search all sites?")
                .with_default(true)
                .with_help_message("Choose 'No' to pick specific sites")
                .prompt()
            {
                Ok(true) => None,
                Ok(false) => {
                    let site_names: Vec<&str> = all_sites.iter().map(|s| s.name).collect();
                    // Multi-select with all preselected so you can quickly uncheck a few
                    match inquire::MultiSelect::new(
                        "Select sites (Space toggles, Enter confirms):",
                        site_names.clone(),
                    )
                    .with_default(&[])
                    .with_help_message("Use ↑/↓ to navigate, Space to toggle, Enter to confirm")
                    .with_page_size(12)
                    .prompt()
                    {
                        Ok(selected) => {
                            if selected.is_empty() {
                                None
                            } else {
                                Some(selected.into_iter().map(|s| s.to_string()).collect())
                            }
                        }
                        Err(_) => None,
                    }
                }
                Err(_) => None,
            }
        } else {
            use std::io::{self, Write};
            println!("\nAvailable sites:");
            for (i, s) in all_sites.iter().enumerate() {
                println!("  {}. {}", i + 1, s.name);
            }
            print!("\nSelect sites (names or numbers, space-separated). Press Enter for ALL: ");
            let _ = io::stdout().flush();
            let mut line = String::new();
            io::stdin().read_line(&mut line)?;
            let raw = line.trim();
            if raw.is_empty() || raw.eq_ignore_ascii_case("all") {
                None
            } else {
                let tokens: Vec<String> = raw
                    .split_whitespace()
                    .map(|t| t.trim().to_string())
                    .filter(|t| !t.is_empty())
                    .collect();
                Some(tokens)
            }
        }
    } else {
        None
    };

    let selected_sites = if let Some(sites_csv) = cli.sites.as_deref() {
        let wanted: Vec<String> = sites_csv
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        all_sites
            .into_iter()
            .filter(|s| wanted.iter().any(|w| w.eq_ignore_ascii_case(s.name)))
            .collect()
    } else if let Some(tokens) = interactive_selection {
        // Map tokens to unique site names by name or 1-based index
        let mut chosen: Vec<&str> = Vec::new();
        for t in tokens {
            match t.parse::<usize>() {
                Ok(idx1) if (1..=all_sites.len()).contains(&idx1) => {
                    let name = all_sites[idx1 - 1].name;
                    if !chosen.iter().any(|c| c.eq_ignore_ascii_case(name)) {
                        chosen.push(name);
                    }
                    continue;
                }
                _ => {}
            }
            // match by name
            if let Some(s) = all_sites.iter().find(|s| s.name.eq_ignore_ascii_case(&t)) {
                if !chosen.iter().any(|c| c.eq_ignore_ascii_case(s.name)) {
                    chosen.push(s.name);
                }
            } else {
                eprintln!("[info] ignoring unknown site token: {}", t);
            }
        }
        if chosen.is_empty() {
            eprintln!("[info] no valid sites selected; using ALL");
            all_sites
        } else {
            all_sites
                .into_iter()
                .filter(|s| chosen.iter().any(|c| c.eq_ignore_ascii_case(s.name)))
                .collect()
        }
    } else {
        all_sites
    };

    let client = build_http_client();
    let semaphore = Arc::new(Semaphore::new(3));
    let mut tasks = FuturesUnordered::new();

    // Build optional headers (Cookie) for forwarding
    let cookie_headers: Option<ReqHeaderMap> = if let Some(ref c) = cli.cookie {
        match HeaderValue::from_str(c) {
            Ok(v) => {
                let mut h = ReqHeaderMap::new();
                h.insert(COOKIE, v);
                Some(h)
            }
            Err(_) => None,
        }
    } else {
        None
    };

    for site in selected_sites {
        let permit = semaphore.clone().acquire_owned().await.unwrap();
        let client = client.clone();
        let query = normalized.clone();
        let debug = cli.debug;
        let use_cf = !cli.no_cf;
        let cf_url = resolved_cf_url.clone();
        let cookie_headers = cookie_headers.clone();

        let no_playwright = cli.no_playwright;
        tasks.push(tokio::spawn(async move {
            let _permit = permit; // hold until task end
            let base_url = match site.search_kind {
                SearchKind::ListingPage => site.listing_path.unwrap_or(site.base_url).to_string(),
                SearchKind::PhpBBSearch => build_search_url(&site, &query), // Uses search.php URL
                _ => build_search_url(&site, &query),
            };
            // Build page URLs: for most sites, just one URL. csrin uses PhpBBSearch URL directly.
            let page_urls: Vec<String> = vec![base_url.clone()];

            let mut results: Vec<SearchResult> = Vec::new();
            // If requested, try Playwright to load dynamic results (skip when solver is explicitly configured/local)
            let cf_local = cf_url.contains("127.0.0.1") || cf_url.contains("localhost");
            let non_default_cf = cf_url != "http://localhost:8191/v1";
            let prefer_solver = use_cf && (cf_local || non_default_cf);
            if site.name.eq_ignore_ascii_case("csrin") && !no_playwright && !prefer_solver {
                let cookie_val = cookie_headers
                    .as_ref()
                    .and_then(|h| h.get(COOKIE))
                    .and_then(|v| v.to_str().ok())
                    .map(|s| s.to_string());
                if let Some(html) = fetch_csrin_playwright_html(&query, cookie_val).await {
                    if debug {
                        eprintln!(
                            "[debug] site={} via Playwright html_len={}",
                            site.name,
                            html.len()
                        );
                        let _ = tokio::fs::create_dir_all("debug").await;
                        let _ = tokio::fs::write("debug/csrin_playwright.html", &html).await;
                    }
                    results = parse_results(&site, &html, &query);
                }
                // If Playwright mode is used, do not fall back to solver-based listing fetches
                // when we already have results. If empty, try feed fallback to get recent topics.
                if results.is_empty()
                    && let Some(feed_results) = fetch_csrin_feed(
                        &client,
                        &site,
                        &query,
                        false,
                        &cf_url,
                        cookie_headers.clone(),
                        debug,
                    )
                    .await
                {
                    results = feed_results;
                }
                // Do not return early; allow common filtering/normalization/truncation below.
            }
            if results.is_empty() {
                for url in page_urls {
                    // Solver gating:
                    // - Default: use solver when the site requires Cloudflare
                    // - csrin: allow solver when explicitly enabled via env, or when a non-default/local CF URL is provided (for tests)
                    let allow_env = std::env::var("ALLOW_CSRIN_SOLVER")
                        .ok()
                        .map(|v| v == "1")
                        .unwrap_or(false);
                    let cf_local = cf_url.contains("127.0.0.1") || cf_url.contains("localhost");
                    let non_default_cf = cf_url != "http://localhost:8191/v1";
                    let csrin_solver_allowed = site.name.eq_ignore_ascii_case("csrin")
                        && (allow_env || cf_local || non_default_cf);
                    let use_solver_for_this =
                        use_cf && (site.requires_cloudflare || csrin_solver_allowed);
                    let html = if use_solver_for_this {
                        if debug {
                            eprintln!("[debug] site={} using FlareSolverr {}", site.name, cf_url);
                        }
                        (if cookie_headers.is_some() {
                            cf::fetch_via_solver_with_headers(
                                &client,
                                &url,
                                &cf_url,
                                cookie_headers.clone(),
                            )
                            .await
                        } else {
                            fetch_via_solver(&client, &url, &cf_url).await
                        })
                        .unwrap_or_default()
                    } else {
                        (if cookie_headers.is_some() {
                            fetcher::fetch_with_retry_headers(&client, &url, cookie_headers.clone())
                                .await
                        } else {
                            fetch_with_retry(&client, &url).await
                        })
                        .unwrap_or_default()
                    };
                    if debug {
                        eprintln!(
                            "[debug] site={} url={} html_len={}",
                            site.name,
                            url,
                            html.len()
                        );
                    }
                    let mut page_results = parse_results(&site, &html, &query);
                    // gog-games fallback: request AJAX JSON/fragment when DOM is empty
                    if page_results.is_empty()
                        && site.name.eq_ignore_ascii_case("gog-games")
                        && let Some(r) = fetch_gog_games_ajax_json(
                            &client,
                            &site,
                            &query,
                            use_cf,
                            &cf_url,
                            cookie_headers.clone(),
                            debug,
                        )
                        .await
                        && !r.is_empty()
                    {
                        page_results = r;
                    }
                    // csrin fallback: parse Atom feed when page body is minimal or selectors miss
                    if page_results.is_empty()
                        && site.name.eq_ignore_ascii_case("csrin")
                        && let Some(r) = fetch_csrin_feed(
                            &client,
                            &site,
                            &query,
                            use_cf,
                            &cf_url,
                            cookie_headers.clone(),
                            debug,
                        )
                        .await
                        && !r.is_empty()
                    {
                        page_results = r;
                    }
                    // Extra filtering for gog-games to avoid unrelated pages/cards
                    if site.name.eq_ignore_ascii_case("gog-games") {
                        filter_results_by_query_strict(&mut page_results, &query);
                    }
                    results.extend(page_results);
                    if results.len() >= 5000 {
                        // safety cap
                        break;
                    }
                }
            }
            // csrin: Automatic Playwright fallback if listing/feed produced nothing and user didn't explicitly request it
            if site.name.eq_ignore_ascii_case("csrin") && results.is_empty() && !no_playwright {
                let cookie_val = cookie_headers
                    .as_ref()
                    .and_then(|h| h.get(COOKIE))
                    .and_then(|v| v.to_str().ok())
                    .map(|s| s.to_string());
                if let Some(html) = fetch_csrin_playwright_html(&query, cookie_val).await {
                    if debug {
                        eprintln!(
                            "[debug] site={} via Playwright (auto) html_len={}",
                            site.name,
                            html.len()
                        );
                    }
                    let rs = parse_results(&site, &html, &query);
                    if !rs.is_empty() {
                        results = rs;
                    }
                }
            }
            if debug {
                eprintln!(
                    "[debug] site={} results={} (pre-truncate)",
                    site.name,
                    results.len()
                );
                if results.is_empty() {
                    // compute debug stats in a tight scope so non-Send Html is dropped before awaits
                    let (anchors_total, matched_samples, article_count, entry_title_count) = {
                        let mut anchors_total = 0usize;
                        let mut matched_samples: Vec<(String, String)> = Vec::new();
                        let mut article_count = 0usize;
                        let mut entry_title_count = 0usize;
                        // For pagination, doc stats should be computed on the last page html if available
                        let doc = Html::parse_document("");
                        if let Ok(a_sel) = Selector::parse("a[href]") {
                            anchors_total = doc.select(&a_sel).count();
                            let ql = query.to_lowercase();
                            for a in doc.select(&a_sel) {
                                let text = a.text().collect::<String>();
                                let href = a.value().attr("href").unwrap_or("");
                                if text.to_lowercase().contains(&ql) {
                                    matched_samples.push((text, href.to_string()));
                                    if matched_samples.len() >= 5 {
                                        break;
                                    }
                                }
                            }
                        }
                        if let Ok(article_sel) = Selector::parse("article") {
                            article_count = doc.select(&article_sel).count();
                        }
                        if let Ok(h2_sel) =
                            Selector::parse("h2.entry-title, h1.entry-title, .entry-title")
                        {
                            entry_title_count = doc.select(&h2_sel).count();
                        }
                        (
                            anchors_total,
                            matched_samples,
                            article_count,
                            entry_title_count,
                        )
                    };

                    eprintln!(
                        "[debug] site={} anchors_total={} anchors_with_query_sample={}",
                        site.name,
                        anchors_total,
                        matched_samples.len()
                    );
                    for (i, (t, h)) in matched_samples.into_iter().enumerate() {
                        let t_short = t.trim().chars().take(80).collect::<String>();
                        let h_short = h.chars().take(120).collect::<String>();
                        eprintln!("[debug]  [{}] text='{}' href='{}'", i, t_short, h_short);
                    }
                    eprintln!("[debug] site={} article_count={}", site.name, article_count);
                    eprintln!(
                        "[debug] site={} entry_title_nodes={}",
                        site.name, entry_title_count
                    );

                    // write html to debug file
                    let _ = tokio::fs::create_dir_all("debug").await;
                    let path = format!("debug/{}_sample.html", site.name);
                    if let Err(e) = tokio::fs::write(&path, "").await {
                        eprintln!("[debug] failed to write {}: {}", path, e);
                    } else {
                        eprintln!("[debug] wrote {}", path);
                    }
                }
            }
            if matches!(
                site.search_kind,
                SearchKind::FrontPage | SearchKind::ListingPage | SearchKind::PhpBBSearch
            ) {
                // csrin: keep only topic pages, and avoid URL-based query matches (phpBB adds
                // hilit=<query> to every result link). Only keep titles that include the query.
                let q_lower = query.to_lowercase();
                if site.name.eq_ignore_ascii_case("csrin") {
                    results.retain(|r| r.url.contains("viewtopic.php"));
                    results.retain(|r| r.title.to_lowercase().contains(&q_lower));
                } else {
                    let q_dash = q_lower.replace(' ', "-");
                    let q_plus = q_lower.replace(' ', "+");
                    let q_enc = q_lower.replace(' ', "%20");
                    let q_strip = q_lower.replace(' ', "");
                    results.retain(|r| {
                        let tl = r.title.to_lowercase();
                        let ul = r.url.to_lowercase();
                        tl.contains(&q_lower)
                            || ul.contains(&q_lower)
                            || ul.contains(&q_dash)
                            || ul.contains(&q_plus)
                            || ul.contains(&q_enc)
                            || ul.contains(&q_strip)
                    });
                }
            }
            // Normalize titles for nicer output
            for r in &mut results {
                r.title = normalize_title(site.name, &r.title);
            }
            if !results.is_empty() {
                results.truncate(cli.limit);
            }
            results
        }));
    }

    let mut combined: Vec<SearchResult> = Vec::new();
    while let Some(joined) = tasks.next().await {
        if let Ok(mut site_results) = joined {
            combined.append(&mut site_results);
        }
    }

    // Deduplicate by (site, url) then sort
    combined.sort_by(|a, b| a.site.cmp(&b.site).then_with(|| a.title.cmp(&b.title)));
    combined.dedup_by(|a, b| a.site == b.site && a.url == b.url);

    // Save to cache (unless disabled)
    if !cli.no_cache && !combined.is_empty() {
        search_cache.add(normalized.clone(), combined.clone());
        if let Err(e) = search_cache.save_to_file_sync(&cache_path) {
            if cli.debug {
                eprintln!("[debug] Failed to save cache: {}", e);
            }
        } else if cli.debug {
            eprintln!(
                "[debug] Cached {} results for \"{}\"",
                combined.len(),
                normalized
            );
        }
    }

    let out_format = if cli.query.is_none() {
        OutputFormat::Table
    } else {
        cli.format
    };
    // Keep TUI only for interactive mode (no query provided). If user explicitly passes
    // --format table with a query, print classic table output instead of TUI.
    let interactive_tui =
        cli.query.is_none() && std::io::stdin().is_terminal() && std::io::stdout().is_terminal();
    if interactive_tui && matches!(out_format, OutputFormat::Table) {
        run_live_tui(&combined)?;
    } else {
        match out_format {
            OutputFormat::Json => output::print_pretty_json(&combined),
            OutputFormat::Table => output::print_table_grouped(&combined),
        }
    }
    Ok(())
}

fn run_live_tui(results: &[SearchResult]) -> anyhow::Result<()> {
    // Setup terminal
    let mut stdout = stdout();
    execute!(stdout, terminal::EnterAlternateScreen)?;
    terminal::enable_raw_mode()?;
    let backend = ratatui::backend::CrosstermBackend::new(stdout);
    let mut terminal = ratatui::Terminal::new(backend)?;

    // Prepare grouped entries and state (group by site, render boxed groups)
    use std::collections::BTreeMap;
    let mut by_site: BTreeMap<&str, Vec<&SearchResult>> = BTreeMap::new();
    for r in results {
        by_site.entry(&r.site).or_default().push(r);
    }
    // Meta for navigation and opening: one None for box top, Some(url) per item line, one None for box bottom
    let mut entry_urls: Vec<Option<String>> = Vec::new();
    // Keep ordered groups for rendering
    let groups: Vec<(String, Vec<(String, String)>)> = by_site
        .into_iter()
        .map(|(site, items)| {
            let list: Vec<(String, String)> = items
                .into_iter()
                .map(|r| (r.title.clone(), r.url.clone()))
                .collect();
            (site.to_string(), list)
        })
        .collect();
    for (_site, items) in &groups {
        entry_urls.push(None); // top border
        for (_t, u) in items {
            entry_urls.push(Some(u.clone()));
        }
        entry_urls.push(None); // bottom border
    }
    let mut state = ListState::default();
    // Select first selectable row
    let first_sel = entry_urls.iter().position(|u| u.is_some()).unwrap_or(0);
    if !entry_urls.is_empty() {
        state.select(Some(first_sel));
    }

    // Drain any pending keystrokes (e.g., the Enter used to run the command)
    while event::poll(std::time::Duration::from_millis(0))? {
        let _ = event::read()?;
    }

    let mut should_quit = false;
    while !should_quit {
        terminal.draw(|f| {
            let area = f.area();
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(1), Constraint::Length(1)])
                .split(area);

            // Build rendered lines based on available width to draw ASCII boxes per site
            let width = chunks[0].width.max(2) as usize;
            let inner = width.saturating_sub(2);
            let mut rendered: Vec<String> = Vec::with_capacity(entry_urls.len());
            for (site, items) in &groups {
                // Top border with centered-ish site name
                let title = format!(" {} ", site);
                let mut top = String::new();
                top.push('┌');
                if title.len() + 2 <= inner {
                    // pad with dashes before and after title
                    let left = 1usize;
                    let right = inner.saturating_sub(left + title.len());
                    top.push_str(&"─".repeat(left));
                    top.push_str(&title);
                    top.push_str(&"─".repeat(right));
                } else {
                    top.push_str(&"─".repeat(inner));
                }
                top.push('┐');
                rendered.push(top);

                for (t, u) in items {
                    let mut mid = String::new();
                    mid.push('│');
                    let content = format!(" - {} ({})", t, u);
                    // ensure at least inner chars, pad or truncate
                    if content.len() >= inner {
                        mid.push_str(&content[..inner.min(content.len())]);
                    } else {
                        mid.push_str(&content);
                        mid.push_str(&" ".repeat(inner - content.len()));
                    }
                    mid.push('│');
                    rendered.push(mid);
                }

                let mut bot = String::new();
                bot.push('└');
                bot.push_str(&"─".repeat(inner));
                bot.push('┘');
                rendered.push(bot);
            }

            let items: Vec<ListItem> = rendered
                .iter()
                .map(|text| ListItem::new(Line::from(text.as_str())))
                .collect();

            let title = format!(
                "Results ({}). ↑/↓ move, PgUp/PgDn scroll, Enter/o open, q quit",
                results.len()
            );
            let list = List::new(items)
                .block(Block::default().title(title).borders(Borders::ALL))
                .highlight_symbol("> ")
                .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
                .repeat_highlight_symbol(false);
            f.render_stateful_widget(list, chunks[0], &mut state);

            // Footer/help with selected URL
            let sel = state
                .selected()
                .unwrap_or(0)
                .min(entry_urls.len().saturating_sub(1));
            let footer = if entry_urls.is_empty() {
                String::new()
            } else {
                entry_urls[sel].clone().unwrap_or_default()
            };
            let foot = Paragraph::new(footer)
                .block(Block::default().borders(Borders::TOP))
                .wrap(Wrap { trim: false });
            f.render_widget(foot, chunks[1]);
        })?;

        // Handle input & resize; non-blocking poll
        if event::poll(std::time::Duration::from_millis(150))? {
            match event::read()? {
                event::Event::Key(k) => {
                    if k.kind != KeyEventKind::Press {
                        continue;
                    }
                    match k.code {
                        event::KeyCode::Char('q') | event::KeyCode::Esc => should_quit = true,
                        event::KeyCode::Up => {
                            let mut i = state.selected().unwrap_or(0);
                            i = i.saturating_sub(1);
                            // skip non-selectable lines (borders)
                            while i > 0 && entry_urls.get(i).and_then(|u| u.as_ref()).is_none() {
                                i = i.saturating_sub(1);
                            }
                            state.select(Some(i));
                        }
                        event::KeyCode::Down => {
                            let mut i = state.selected().unwrap_or(0);
                            let max = entry_urls.len().saturating_sub(1);
                            if i < max {
                                i += 1;
                            }
                            while i < max && entry_urls.get(i).and_then(|u| u.as_ref()).is_none() {
                                i += 1;
                            }
                            state.select(Some(i));
                        }
                        event::KeyCode::PageUp => {
                            let i = state.selected().unwrap_or(0);
                            let step = 10usize;
                            let next = i.saturating_sub(step);
                            state.select(Some(next));
                        }
                        event::KeyCode::PageDown => {
                            let i = state.selected().unwrap_or(0);
                            let step = 10usize;
                            let max = entry_urls.len().saturating_sub(1);
                            let next = (i + step).min(max);
                            state.select(Some(next));
                        }
                        event::KeyCode::Home => {
                            state.select(Some(0));
                        }
                        event::KeyCode::End => {
                            let max = entry_urls.len().saturating_sub(1);
                            state.select(Some(max));
                        }
                        event::KeyCode::Enter | event::KeyCode::Char('o') => {
                            if let Some(i) = state.selected()
                                && let Some(Some(url)) = entry_urls.get(i)
                            {
                                let _ = open_url(url);
                            }
                        }
                        _ => {}
                    }
                }
                event::Event::Resize(_, _) => {}
                _ => {}
            }
        }
    }

    // Restore terminal
    terminal::disable_raw_mode()?;
    crossterm::execute!(terminal.backend_mut(), terminal::LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
}

fn open_url(url: &str) -> anyhow::Result<()> {
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("cmd")
            .args(["/C", "start", "", url])
            .spawn()
            .map(|_| ())?;
        return Ok(());
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(url)
            .spawn()
            .map(|_| ())?;
        return Ok(());
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        std::process::Command::new("xdg-open")
            .arg(url)
            .spawn()
            .map(|_| ())?;
        return Ok(());
    }
    #[allow(unreachable_code)]
    Ok(())
}

fn filter_results_by_query_strict(results: &mut Vec<SearchResult>, query: &str) {
    let ql = query.to_lowercase();
    let ql_dash = ql.replace(' ', "-");
    let ql_plus = ql.replace(' ', "+");
    let ql_encoded = ql.replace(' ', "%20");
    let ql_stripped = ql.replace(' ', "");
    results.retain(|r| {
        let tl = r.title.to_lowercase();
        let ul = r.url.to_lowercase();
        let matches = tl.contains(&ql)
            || ul.contains(&ql)
            || ul.contains(&ql_dash)
            || ul.contains(&ql_plus)
            || ul.contains(&ql_encoded)
            || ul.contains(&ql_stripped);
        let gog_path_ok = ul.contains("/game/") || ul.contains("/games/");
        matches && gog_path_ok
    });
}

async fn fetch_gog_games_ajax_json(
    client: &reqwest::Client,
    site: &website_searcher_core::models::SiteConfig,
    query: &str,
    use_cf: bool,
    cf_url: &str,
    cookie_headers: Option<ReqHeaderMap>,
    debug: bool,
) -> Option<Vec<SearchResult>> {
    let qenc = urlencoding::encode(query);
    let urls = vec![
        format!(
            "https://gog-games.to/search?search={}&page=1&den_filter=none",
            qenc
        ),
        format!("https://gog-games.to/search?page=1&search={}", qenc),
        format!("https://gog-games.to/?search={}", qenc),
    ];
    // build headers
    let mut headers = ReqHeaderMap::new();
    headers.insert(
        ACCEPT,
        HeaderValue::from_static("application/json, text/plain, */*"),
    );
    headers.insert(
        HeaderName::from_static("x-requested-with"),
        HeaderValue::from_static("XMLHttpRequest"),
    );
    headers.insert(
        REFERER,
        HeaderValue::from_str(&format!("https://gog-games.to/?search={}", qenc))
            .unwrap_or(HeaderValue::from_static("https://gog-games.to/")),
    );
    if let Some(ch) = &cookie_headers {
        for (k, v) in ch.iter() {
            headers.insert(k, v.clone());
        }
    }

    for (i, u) in urls.into_iter().enumerate() {
        let body: String = if use_cf {
            (cf::fetch_via_solver_with_headers(client, &u, cf_url, Some(headers.clone())).await)
                .unwrap_or_default()
        } else {
            (fetcher::fetch_with_retry_headers(client, &u, Some(headers.clone())).await)
                .unwrap_or_default()
        };
        if body.is_empty() {
            continue;
        }
        if debug {
            let _ = tokio::fs::create_dir_all("debug").await;
            let _ = tokio::fs::write(format!("debug/gog-games_ajax_{}.txt", i), &body).await;
        }
        let trimmed = body.trim_start();
        if trimmed.starts_with('<') {
            // Try to extract JSON inside <pre>...</pre>
            if let (Some(sidx), Some(eidx)) = (trimmed.find("<pre>"), trimmed.find("</pre>")) {
                let s = sidx + 5;
                if s < eidx {
                    let json_inner = &trimmed[s..eidx];
                    if let Ok(v) = serde_json::from_str::<Value>(json_inner) {
                        let mut results: Vec<SearchResult> = Vec::new();
                        collect_title_url_pairs(&v, &mut results);
                        if !results.is_empty() {
                            return Some(results);
                        }
                    }
                }
            }
            // else treat as HTML fragment
            let rs = parse_results(site, &body, query);
            if !rs.is_empty() {
                return Some(rs);
            }
            continue;
        }
        if let Ok(v) = serde_json::from_str::<Value>(&body) {
            if let Some(html) = v.get("html").and_then(|x| x.as_str()) {
                let rs = parse_results(site, html, query);
                if !rs.is_empty() {
                    return Some(rs);
                }
            }
            if let Some(html) = v
                .get("data")
                .and_then(|x| x.get("html"))
                .and_then(|x| x.as_str())
            {
                let rs = parse_results(site, html, query);
                if !rs.is_empty() {
                    return Some(rs);
                }
            }
            let mut results: Vec<SearchResult> = Vec::new();
            collect_title_url_pairs(&v, &mut results);
            if !results.is_empty() {
                return Some(results);
            }
        }
    }
    None
}

async fn fetch_csrin_feed(
    client: &reqwest::Client,
    site: &website_searcher_core::models::SiteConfig,
    query: &str,
    _use_cf: bool,
    cf_url: &str,
    _cookie_headers: Option<ReqHeaderMap>,
    debug: bool,
) -> Option<Vec<SearchResult>> {
    // Try forum feed which lists topics
    let feed_url = "https://cs.rin.ru/forum/feed.php?f=10";
    // Never route feeds via solver for csrin to avoid solver blacklisting/redirect noise
    let body = if false {
        cf::fetch_via_solver(client, feed_url, cf_url)
            .await
            .unwrap_or_default()
    } else {
        fetcher::fetch_with_retry(client, feed_url)
            .await
            .unwrap_or_default()
    };
    if body.is_empty() {
        return None;
    }
    // Some endpoints wrap Atom XML inside HTML <pre> with escaped entities; unwrap and decode
    let mut xml = body.clone();
    if let Some(pre_idx) = xml.find("<pre")
        && let Some(tag_end) = xml[pre_idx..].find('>')
    {
        let content_start = pre_idx + tag_end + 1;
        if let Some(close_rel) = xml[content_start..].find("</pre>") {
            let content_end = content_start + close_rel;
            let inner = &xml[content_start..content_end];
            xml = inner
                .replace("&lt;", "<")
                .replace("&gt;", ">")
                .replace("&amp;", "&")
                .replace("&quot;", "\"")
                .replace("&#39;", "'");
        }
    }
    if debug {
        let _ = tokio::fs::create_dir_all("debug").await;
        let _ = tokio::fs::write("debug/csrin_feed.xml", &xml).await;
    }
    // Very light XML parse: find <entry><title> and <link href="...viewtopic.php?..."/>
    let mut results: Vec<SearchResult> = Vec::new();
    let ql = query.to_lowercase();
    let mut i = 0usize;
    while let Some(tidx) = xml[i..].find("<entry>") {
        let start = i + tidx;
        let end = xml[start..]
            .find("</entry>")
            .map(|e| start + e + 8)
            .unwrap_or(xml.len());
        let entry = &xml[start..end];
        // Extract <title ...>...</title>, allowing attributes and CDATA
        let mut title = "";
        if let Some(t_open_rel) = entry.find("<title") {
            let after_tag_rel = entry[t_open_rel..].find('>').map(|p| t_open_rel + p + 1);
            if let Some(content_start) = after_tag_rel
                && let Some(close_rel) = entry[content_start..].find("</title>")
            {
                let raw = &entry[content_start..content_start + close_rel];
                let raw = raw.trim();
                // Unwrap CDATA if present
                if let Some(inner) = raw.strip_prefix("<![CDATA[") {
                    if let Some(inner2) = inner.strip_suffix("]]>") {
                        title = inner2.trim();
                    } else {
                        title = inner.trim();
                    }
                } else {
                    title = raw;
                }
            }
        }
        if title.is_empty() {
            title = entry
                .split_once("<title>")
                .and_then(|(_, rest)| rest.split_once("</title>").map(|(t, _)| t))
                .unwrap_or("")
                .trim();
        }
        let href = entry
            .split_once("<link href=\"")
            .and_then(|(_, rest)| rest.split_once('\"').map(|(u, _)| u))
            .unwrap_or("");
        if !title.is_empty() && href.contains("viewtopic.php") {
            let tl = title.to_lowercase();
            if tl.contains(&ql) || href.to_lowercase().contains(&ql.replace(' ', "+")) {
                let url = if href.starts_with("http") {
                    href.to_string()
                } else {
                    format!("https://cs.rin.ru/forum/{}", href.trim_start_matches('/'))
                };
                results.push(SearchResult {
                    site: site.name.to_string(),
                    title: title.to_string(),
                    url,
                });
            }
        }
        i = end;
        if results.len() >= 50 {
            break;
        }
    }
    if results.is_empty() {
        None
    } else {
        Some(results)
    }
}

// Spawn Node + Playwright helper to fetch rendered HTML for cs.rin search
async fn fetch_csrin_playwright_html(query: &str, cookie: Option<String>) -> Option<String> {
    // Test/CI fast path: if CS_PLAYWRIGHT_HTML is provided, return it without spawning Node
    if let Ok(fake) = std::env::var("CS_PLAYWRIGHT_HTML")
        && !fake.trim().is_empty()
    {
        return Some(fake);
    }
    let script = "../../scripts/csrin_search.cjs";
    let mut cmd = Command::new("node");
    cmd.arg(script).arg(query);
    if let Some(c) = cookie {
        cmd.env("PLAYWRIGHT_COOKIE", c);
    }
    // Allow page count override from CLI pages setting via env
    if let Ok(p) = std::env::var("CSRIN_PAGES")
        && !p.trim().is_empty()
    {
        cmd.env("CSRIN_PAGES", p);
    }
    cmd.stdin(Stdio::null());
    cmd.stderr(Stdio::inherit());
    cmd.stdout(Stdio::piped());
    let mut child = match cmd.spawn() {
        Ok(c) => c,
        Err(_) => return None,
    };
    let mut out = String::new();
    if let Some(mut so) = child.stdout.take() {
        let _ = so.read_to_string(&mut out).await;
    }
    let _ = child.wait().await;
    if out.trim().is_empty() {
        None
    } else {
        Some(out)
    }
}

#[allow(clippy::collapsible_if)]
fn collect_title_url_pairs(v: &Value, out: &mut Vec<SearchResult>) {
    match v {
        Value::Object(map) => {
            let title = map
                .get("title")
                .and_then(|x| x.as_str())
                .or_else(|| map.get("name").and_then(|x| x.as_str()));
            let mut url: Option<String> = map
                .get("url")
                .and_then(|x| x.as_str())
                .map(|s| s.to_string())
                .or_else(|| {
                    map.get("permalink")
                        .and_then(|x| x.as_str())
                        .map(|s| s.to_string())
                })
                .or_else(|| {
                    map.get("href")
                        .and_then(|x| x.as_str())
                        .map(|s| s.to_string())
                })
                .or_else(|| {
                    map.get("path")
                        .and_then(|x| x.as_str())
                        .map(|s| s.to_string())
                });
            if url.is_none() {
                if let Some(slug) = map.get("slug").and_then(|x| x.as_str()) {
                    url = Some(format!("https://gog-games.to/game/{}", slug));
                }
            }
            if let (Some(t), Some(u)) = (title, url) {
                let u_abs = if u.starts_with('/') {
                    format!("https://gog-games.to{}", u)
                } else {
                    u
                };
                out.push(SearchResult {
                    site: "gog-games".to_string(),
                    title: t.to_string(),
                    url: u_abs,
                });
            }
            for val in map.values() {
                collect_title_url_pairs(val, out);
            }
        }
        Value::Array(arr) => {
            for val in arr {
                collect_title_url_pairs(val, out);
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_title_collapses_whitespace() {
        let s = "\n   Elden\nRing   \n";
        assert_eq!(normalize_title("fitgirl", s), "Elden");
    }

    #[test]
    fn normalize_title_trims_ankergames_size_suffix() {
        let s = "Some Game Deluxe Edition 64.91 GB";
        assert_eq!(normalize_title("ankergames", s), "Some Game Deluxe Edition");
    }

    #[test]
    fn collect_title_url_pairs_extracts_nested_objects_and_arrays() {
        let v = serde_json::json!({
            "title": "One",
            "url": "/game/one",
            "nested": {
                "name": "Two",
                "permalink": "https://gog-games.to/game/two"
            },
            "arr": [
                {"title": "Three", "href": "/game/three"},
                {"name": "Four", "slug": "four"}
            ]
        });
        let mut out = Vec::new();
        collect_title_url_pairs(&v, &mut out);
        let titles: Vec<_> = out.iter().map(|r| r.title.as_str()).collect();
        let urls: Vec<_> = out.iter().map(|r| r.url.as_str()).collect();
        assert!(titles.contains(&"One"));
        assert!(urls.contains(&"https://gog-games.to/game/one"));
        assert!(titles.contains(&"Two"));
        assert!(urls.contains(&"https://gog-games.to/game/two"));
        assert!(titles.contains(&"Three"));
        assert!(urls.contains(&"https://gog-games.to/game/three"));
        assert!(titles.contains(&"Four"));
        assert!(urls.contains(&"https://gog-games.to/game/four"));
    }

    #[test]
    fn normalize_title_csrin_removes_forum_prefix() {
        let s = "Main Forum • Elden Ring";
        assert_eq!(normalize_title("csrin", s), "Elden Ring");
    }

    #[test]
    fn normalize_title_csrin_removes_re_prefix() {
        let s = "Re: Elden Ring Discussion";
        assert_eq!(normalize_title("csrin", s), "Elden Ring Discussion");
    }

    #[test]
    fn normalize_title_csrin_combined() {
        let s = "Main Forum • Re: Some Game Title";
        assert_eq!(normalize_title("csrin", s), "Some Game Title");
    }

    #[test]
    fn filter_results_strict_requires_gog_path() {
        let mut results = vec![
            SearchResult {
                site: "gog-games".into(),
                title: "Elden Ring".into(),
                url: "https://gog-games.to/game/elden-ring".into(),
            },
            SearchResult {
                site: "gog-games".into(),
                title: "Elden Ring".into(),
                url: "https://gog-games.to/search?q=elden".into(),
            },
        ];
        filter_results_by_query_strict(&mut results, "elden ring");
        assert_eq!(results.len(), 1);
        assert!(results[0].url.contains("/game/"));
    }

    #[test]
    fn filter_results_strict_handles_encoded_queries() {
        let mut results = vec![SearchResult {
            site: "gog-games".into(),
            title: "Some Title".into(),
            url: "https://gog-games.to/games/elden%20ring-deluxe".into(),
        }];
        filter_results_by_query_strict(&mut results, "elden ring");
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn normalize_title_other_site_passthrough() {
        let s = "Some Game Title";
        assert_eq!(normalize_title("fitgirl", s), "Some Game Title");
    }

    #[test]
    fn normalize_title_ankergames_no_gb_suffix() {
        let s = "Game Without Size";
        assert_eq!(normalize_title("ankergames", s), "Game Without Size");
    }

    #[test]
    fn normalize_title_csrin_re_without_space() {
        let s = "Re:Some Topic";
        assert_eq!(normalize_title("csrin", s), "Some Topic");
    }

    #[test]
    fn collect_title_url_pairs_handles_href_field() {
        let v = serde_json::json!({
            "title": "Game Href",
            "href": "/game/href-game"
        });
        let mut out = Vec::new();
        collect_title_url_pairs(&v, &mut out);
        assert_eq!(out.len(), 1);
        assert!(out[0].url.contains("href-game"));
    }

    #[test]
    fn collect_title_url_pairs_handles_path_field() {
        let v = serde_json::json!({
            "name": "Path Game",
            "path": "/game/path-game"
        });
        let mut out = Vec::new();
        collect_title_url_pairs(&v, &mut out);
        assert_eq!(out.len(), 1);
        assert!(out[0].url.contains("path-game"));
    }

    #[test]
    fn collect_title_url_pairs_ignores_invalid_types() {
        let v = serde_json::json!(null);
        let mut out = Vec::new();
        collect_title_url_pairs(&v, &mut out);
        assert!(out.is_empty());
    }

    #[test]
    fn collect_title_url_pairs_ignores_boolean() {
        let v = serde_json::json!(true);
        let mut out = Vec::new();
        collect_title_url_pairs(&v, &mut out);
        assert!(out.is_empty());
    }

    #[test]
    fn collect_title_url_pairs_ignores_number() {
        let v = serde_json::json!(42);
        let mut out = Vec::new();
        collect_title_url_pairs(&v, &mut out);
        assert!(out.is_empty());
    }

    #[test]
    fn collect_pairs_skips_missing_url_and_title() {
        let v = serde_json::json!({
            "other_field": "value"
        });
        let mut out = Vec::new();
        collect_title_url_pairs(&v, &mut out);
        assert!(out.is_empty());
    }

    #[test]
    fn filter_results_strict_stripped_query_match() {
        let mut results = vec![SearchResult {
            site: "gog-games".into(),
            title: "Some Title".into(),
            url: "https://gog-games.to/game/eldenring".into(),
        }];
        filter_results_by_query_strict(&mut results, "elden ring");
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn filter_results_strict_games_path() {
        let mut results = vec![SearchResult {
            site: "gog-games".into(),
            title: "Elden Ring".into(),
            url: "https://gog-games.to/games/elden-ring".into(),
        }];
        filter_results_by_query_strict(&mut results, "elden ring");
        assert_eq!(results.len(), 1);
    }

    #[tokio::test]
    async fn fetch_csrin_playwright_html_returns_env_var() {
        // Set env var for test
        unsafe { std::env::set_var("CS_PLAYWRIGHT_HTML", "<html>mock</html>") };
        let result = fetch_csrin_playwright_html("test", None).await;
        unsafe { std::env::remove_var("CS_PLAYWRIGHT_HTML") };
        assert!(result.is_some());
        assert!(result.unwrap().contains("mock"));
    }

    #[tokio::test]
    async fn fetch_csrin_playwright_html_empty_env_returns_none() {
        unsafe { std::env::set_var("CS_PLAYWRIGHT_HTML", "   ") };
        let result = fetch_csrin_playwright_html("test", None).await;
        unsafe { std::env::remove_var("CS_PLAYWRIGHT_HTML") };
        // Script won't be found since we're in test, so None
        assert!(result.is_none());
    }
}
