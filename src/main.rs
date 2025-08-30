use anyhow::Result;
use clap::Parser;
use futures::stream::{FuturesUnordered, StreamExt};
use scraper::{Html, Selector};
use std::sync::Arc;
use tokio::sync::Semaphore;

mod config;
mod cf;
mod fetcher;
mod models;
mod output;
mod parser;
mod query;

use config::site_configs;
use cf::fetch_via_solver;
use fetcher::{build_http_client, fetch_with_retry};
use models::{SearchResult, SearchKind};
use parser::parse_results;
use query::{build_search_url, normalize_query};

fn normalize_title(site: &str, title: &str) -> String {
    // Collapse whitespace
    let mut cleaned = title.lines().find(|l| !l.trim().is_empty()).unwrap_or(title).to_string();
    cleaned = cleaned.replace('\n', " ").replace('\r', " ");
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
    query: String,

    /// Limit results per site
    #[arg(long, default_value_t = 10)]
    limit: usize,

    /// Comma-separated site list to include (default: all)
    #[arg(long)]
    sites: Option<String>,

    /// Print per-site debug info
    #[arg(long, default_value_t = false)]
    debug: bool,

    /// Use FlareSolverr Cloudflare solver (http://localhost:8191/v1 by default)
    #[arg(long, default_value_t = false)]
    use_cf: bool,
    /// FlareSolverr endpoint
    #[arg(long, default_value = "http://localhost:8191/v1")]
    cf_url: String,
}

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let normalized = normalize_query(&cli.query);

    let selected_sites = if let Some(sites_csv) = cli.sites.as_deref() {
        let wanted: Vec<String> = sites_csv
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        site_configs()
            .into_iter()
            .filter(|s| wanted.iter().any(|w| w.eq_ignore_ascii_case(s.name)))
            .collect()
    } else {
        site_configs()
    };

    let client = build_http_client();
    let semaphore = Arc::new(Semaphore::new(3));
    let mut tasks = FuturesUnordered::new();

    for site in selected_sites {
        let permit = semaphore.clone().acquire_owned().await.unwrap();
        let client = client.clone();
        let query = normalized.clone();
        let debug = cli.debug;
        let use_cf = cli.use_cf;
        let cf_url = cli.cf_url.clone();
        tasks.push(tokio::spawn(async move {
            let _permit = permit; // hold until task end
            let url = match site.search_kind {
                SearchKind::ListingPage => site.listing_path.unwrap_or(site.base_url).to_string(),
                _ => build_search_url(&site, &query),
            };
            let html = if site.requires_cloudflare && use_cf {
                if debug { eprintln!("[debug] site={} using FlareSolverr {}", site.name, cf_url); }
                match fetch_via_solver(&client, &url, &cf_url).await {
                    Ok(h) => h,
                    Err(_) => String::new(),
                }
            } else {
                match fetch_with_retry(&client, &url).await {
                    Ok(h) => h,
                    Err(_) => String::new(),
                }
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
            if debug {
                eprintln!("[debug] site={} results={} (pre-truncate)", site.name, results.len());
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
                                    if matched_samples.len() >= 5 { break; }
                                }
                            }
                        }
                        if let Ok(article_sel) = Selector::parse("article") {
                            article_count = doc.select(&article_sel).count();
                        }
                        if let Ok(h2_sel) = Selector::parse("h2.entry-title, h1.entry-title, .entry-title") {
                            entry_title_count = doc.select(&h2_sel).count();
                        }
                        (anchors_total, matched_samples, article_count, entry_title_count)
                    };

                    eprintln!("[debug] site={} anchors_total={} anchors_with_query_sample={}", site.name, anchors_total, matched_samples.len());
                    for (i, (t, h)) in matched_samples.into_iter().enumerate() {
                        let t_short = t.trim().chars().take(80).collect::<String>();
                        let h_short = h.chars().take(120).collect::<String>();
                        eprintln!("[debug]  [{}] text='{}' href='{}'", i, t_short, h_short);
                    }
                    eprintln!("[debug] site={} article_count={}", site.name, article_count);
                    eprintln!("[debug] site={} entry_title_nodes={}", site.name, entry_title_count);

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
            if matches!(site.search_kind, SearchKind::FrontPage | SearchKind::ListingPage) {
                let q_lower = query.to_lowercase();
                let q_dash = q_lower.replace(' ', "-");
                let q_plus = q_lower.replace(' ', "+");
                let q_enc = q_lower.replace(' ', "%20");
                let q_strip = q_lower.replace(' ', "");
                results.retain(|r| {
                    let tl = r.title.to_lowercase();
                    let ul = r.url.to_lowercase();
                    tl.contains(&q_lower) || ul.contains(&q_lower) || ul.contains(&q_dash) || ul.contains(&q_plus) || ul.contains(&q_enc) || ul.contains(&q_strip)
                });
            }
            // Normalize titles for nicer output
            for r in &mut results {
                r.title = normalize_title(site.name, &r.title);
            }
            if results.len() > 0 {
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

    output::print_pretty_json(&combined);
    Ok(())
}
