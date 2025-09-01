use anyhow::Result;
use clap::{Parser, ValueEnum};
use futures::stream::{FuturesUnordered, StreamExt};
use scraper::{Html, Selector};
use std::sync::Arc;
use tokio::sync::Semaphore;

mod cf;
mod config;
mod fetcher;
mod models;
mod output;
mod parser;
mod query;

use cf::fetch_via_solver;
use config::site_configs;
use fetcher::{build_http_client, fetch_with_retry};
use models::{SearchKind, SearchResult};
use parser::parse_results;
use query::{build_search_url, normalize_query};
use reqwest::header::{
    ACCEPT, COOKIE, HeaderMap as ReqHeaderMap, HeaderName, HeaderValue, REFERER,
};
use serde_json::Value;

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
}

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Interactive prompt when query omitted
    let query_value: String = match &cli.query {
        Some(q) => q.clone(),
        None => {
            println!("Website Searcher (interactive)\n");
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
    };
    let normalized = normalize_query(&query_value);

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
        tasks.push(tokio::spawn(async move {
            let _permit = permit; // hold until task end
            let url = match site.search_kind {
                SearchKind::ListingPage => site.listing_path.unwrap_or(site.base_url).to_string(),
                _ => build_search_url(&site, &query),
            };
            let html = if site.requires_cloudflare && use_cf {
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
                    fetcher::fetch_with_retry_headers(&client, &url, cookie_headers.clone()).await
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
            let mut results = parse_results(&site, &html, &query);
            // gog-games fallback: request AJAX JSON/fragment when DOM is empty
            if results.is_empty() && site.name.eq_ignore_ascii_case("gog-games") {
                match fetch_gog_games_ajax_json(
                    &client,
                    &site,
                    &query,
                    use_cf,
                    &cf_url,
                    cookie_headers.clone(),
                    debug,
                )
                .await
                {
                    Some(r) if !r.is_empty() => results = r,
                    _ => {}
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
                        let doc = Html::parse_document(&html);
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
                    if let Err(e) = tokio::fs::write(&path, &html).await {
                        eprintln!("[debug] failed to write {}: {}", path, e);
                    } else {
                        eprintln!("[debug] wrote {}", path);
                    }
                }
            }
            if matches!(
                site.search_kind,
                SearchKind::FrontPage | SearchKind::ListingPage
            ) {
                let q_lower = query.to_lowercase();
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

    let out_format = if cli.query.is_none() {
        OutputFormat::Table
    } else {
        cli.format
    };
    match out_format {
        OutputFormat::Json => output::print_pretty_json(&combined),
        OutputFormat::Table => output::print_table_grouped(&combined),
    }
    Ok(())
}

async fn fetch_gog_games_ajax_json(
    client: &reqwest::Client,
    site: &crate::models::SiteConfig,
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
                .or_else(|| map.get("permalink").and_then(|x| x.as_str()).map(|s| s.to_string()))
                .or_else(|| map.get("href").and_then(|x| x.as_str()).map(|s| s.to_string()))
                .or_else(|| map.get("path").and_then(|x| x.as_str()).map(|s| s.to_string()));
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
}
