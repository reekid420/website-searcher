use std::sync::Arc;

use futures::stream::{FuturesUnordered, StreamExt};
use reqwest::header::{
    ACCEPT, COOKIE, HeaderMap as ReqHeaderMap, HeaderName, HeaderValue, REFERER,
};
use tokio::sync::Semaphore;
use website_searcher_core::cache::{MIN_CACHE_SIZE, SearchCache};
use website_searcher_core::rate_limiter::RateLimiter;
use website_searcher_core::{cf, config, fetcher, models, parser, query};

/// Get the shared cache file path (same as CLI uses)
fn get_cache_path() -> std::path::PathBuf {
    dirs::cache_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("website-searcher")
        .join("search_cache.json")
}

#[derive(serde::Deserialize)]
struct SearchArgs {
    query: String,
    limit: Option<usize>,
    cutoff: Option<usize>,
    sites: Option<Vec<String>>, // names
    debug: Option<bool>,
    no_cf: Option<bool>,
    cf_url: Option<String>,
    cookie: Option<String>,
    csrin_pages: Option<usize>,
    csrin_search: Option<bool>,
    no_playwright: Option<bool>,
    no_rate_limit: Option<bool>,
}

#[tauri::command]
async fn list_sites() -> Result<Vec<String>, String> {
    let names: Vec<String> = config::site_configs()
        .into_iter()
        .map(|s| s.name.to_string())
        .collect();
    Ok(names)
}

/// Cache entry for serialization to frontend
#[derive(serde::Serialize, Clone)]
struct CacheEntryResponse {
    query: String,
    result_count: usize,
    timestamp: u64,
}

/// Get all cached searches
#[tauri::command]
async fn get_cache() -> Result<Vec<CacheEntryResponse>, String> {
    let path = get_cache_path();
    if !path.exists() {
        return Ok(vec![]);
    }
    let cache = SearchCache::load_from_file(&path)
        .await
        .map_err(|e| e.to_string())?;

    let entries: Vec<CacheEntryResponse> = cache
        .entries()
        .iter()
        .rev() // newest first
        .map(|e| CacheEntryResponse {
            query: e.query.clone(),
            result_count: e.results.len(),
            timestamp: e.timestamp,
        })
        .collect();
    Ok(entries)
}

/// Get cached results for a specific query
#[tauri::command]
async fn get_cached_results(query: String) -> Result<Option<Vec<models::SearchResult>>, String> {
    let path = get_cache_path();
    if !path.exists() {
        return Ok(None);
    }
    let cache = SearchCache::load_from_file(&path)
        .await
        .map_err(|e| e.to_string())?;

    if let Some(entry) = cache.get(&query) {
        Ok(Some(entry.results.clone()))
    } else {
        Ok(None)
    }
}

/// Add search results to cache
#[tauri::command]
async fn add_to_cache(query: String, results: Vec<models::SearchResult>) -> Result<(), String> {
    let path = get_cache_path();
    let mut cache = if path.exists() {
        SearchCache::load_from_file(&path)
            .await
            .unwrap_or_else(|_| SearchCache::with_default_size())
    } else {
        SearchCache::with_default_size()
    };

    cache.add(query, results);
    cache.save_to_file(&path).await.map_err(|e| e.to_string())?;
    Ok(())
}

/// Remove a specific cache entry by query
#[tauri::command]
async fn remove_cache_entry(query: String) -> Result<bool, String> {
    let path = get_cache_path();
    if !path.exists() {
        return Ok(false);
    }
    let mut cache = SearchCache::load_from_file(&path)
        .await
        .map_err(|e| e.to_string())?;

    let removed = cache.remove(&query);
    cache.save_to_file(&path).await.map_err(|e| e.to_string())?;
    Ok(removed)
}

/// Clear all cache entries
#[tauri::command]
async fn clear_cache() -> Result<(), String> {
    let path = get_cache_path();
    if path.exists() {
        std::fs::remove_file(&path).map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Get cache settings (size)
#[tauri::command]
async fn get_cache_settings() -> Result<usize, String> {
    let path = get_cache_path();
    if path.exists() {
        let cache = SearchCache::load_from_file(&path)
            .await
            .unwrap_or_else(|_| SearchCache::with_default_size());
        Ok(cache.max_size())
    } else {
        Ok(MIN_CACHE_SIZE)
    }
}

/// Set cache size
#[tauri::command]
async fn set_cache_size(size: usize) -> Result<(), String> {
    let path = get_cache_path();
    let mut cache = if path.exists() {
        SearchCache::load_from_file(&path)
            .await
            .unwrap_or_else(|_| SearchCache::with_default_size())
    } else {
        SearchCache::with_default_size()
    };

    cache.set_max_size(size);
    cache.save_to_file(&path).await.map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
async fn search_gui(args: SearchArgs) -> Result<Vec<models::SearchResult>, String> {
    if args.query.trim().is_empty() {
        return Err("empty search phrase".to_string());
    }
    let limit = args.limit.unwrap_or(10);
    let _debug = args.debug.unwrap_or(false);
    let use_cf = !args.no_cf.unwrap_or(false);
    let mut cf_url = args
        .cf_url
        .unwrap_or_else(|| "http://localhost:8191/v1".to_string());
    if cf_url == "http://localhost:8191/v1"
        && let Ok(env_cf) = std::env::var("CF_URL")
        && !env_cf.trim().is_empty()
    {
        cf_url = env_cf;
    }

    let normalized = query::normalize_query(&args.query);
    let all_sites = config::site_configs();
    let selected_sites: Vec<models::SiteConfig> = if let Some(names) = args.sites {
        let wanted: Vec<String> = names
            .into_iter()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        all_sites
            .into_iter()
            .filter(|s| wanted.iter().any(|w| w.eq_ignore_ascii_case(&s.name)))
            .collect()
    } else {
        all_sites
    };

    let client = fetcher::build_http_client();
    let semaphore = Arc::new(Semaphore::new(3));
    let rate_limiter = if !args.no_rate_limit.unwrap_or(false) {
        Some(Arc::new(tokio::sync::Mutex::new(RateLimiter::new())))
    } else {
        None
    };

    // Optional Cookie header
    let cookie_headers: Option<ReqHeaderMap> = if let Some(c) = args.cookie.as_deref() {
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

    let mut tasks = FuturesUnordered::new();
    for site in selected_sites {
        let permit = semaphore
            .clone()
            .acquire_owned()
            .await
            .map_err(|e| e.to_string())?;
        let client = client.clone();
        let query = normalized.clone();
        let cf_url = cf_url.clone();
        let cookie_headers = cookie_headers.clone();
        let csrin_pages = args.csrin_pages.unwrap_or(1);
        let csrin_search = args.csrin_search.unwrap_or(false);
        let no_playwright = args.no_playwright.unwrap_or(false);
        let rate_limiter = rate_limiter.clone();
        tasks.push(tokio::spawn(async move {
            let _permit = permit;
            let base_url = match site.search_kind {
                models::SearchKind::ListingPage => site
                    .listing_path
                    .clone()
                    .unwrap_or(site.base_url.clone())
                    .to_string(),
                _ => query::build_search_url(&site, &query),
            };
            let page_urls: Vec<String> = if site.name.eq_ignore_ascii_case("csrin") {
                let mut urls = Vec::new();
                if csrin_search {
                    let qenc = serde_urlencoded::to_string([
                        ("keywords", query.as_str()),
                        ("sr", "topics"),
                    ])
                    .unwrap_or_else(|_| format!("keywords={}&sr=topics", query.replace(' ', "+")));
                    let search_base = "https://cs.rin.ru/forum/search.php";
                    urls.push(format!("{}?{}&fid%5B%5D=10", search_base, qenc));
                } else {
                    let pages = csrin_pages.max(1);
                    urls.push(base_url.clone());
                    for i in 1..pages {
                        let start = i * 100;
                        if base_url.contains('?') {
                            urls.push(format!("{}&start={}", base_url, start));
                        } else {
                            urls.push(format!("{}?start={}", base_url, start));
                        }
                    }
                }
                urls
            } else {
                vec![base_url.clone()]
            };

            let mut results: Vec<models::SearchResult> = Vec::new();
            // Try Playwright path for csrin only when solver not explicitly preferred
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
                    results = parser::parse_results(&site, &html, &query);
                }
            }
            if results.is_empty() {
                for url in page_urls {
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
                        (if cookie_headers.is_some() {
                            cf::fetch_via_solver_with_headers(
                                &client,
                                &url,
                                &cf_url,
                                cookie_headers.clone(),
                            )
                            .await
                        } else {
                            cf::fetch_via_solver(&client, &url, &cf_url).await
                        })
                        .unwrap_or_default()
                    } else {
                        let rate_limiter_ref = if let Some(ref rl) = rate_limiter {
                            Some(&mut *rl.lock().await)
                        } else {
                            None
                        };

                        (if cookie_headers.is_some() {
                            fetcher::fetch_with_retry_headers(
                                &client,
                                &url,
                                cookie_headers.clone(),
                                rate_limiter_ref,
                                Some(&site.name),
                            )
                            .await
                        } else {
                            fetcher::fetch_with_retry(
                                &client,
                                &url,
                                rate_limiter_ref,
                                Some(&site.name),
                            )
                            .await
                        })
                        .unwrap_or_default()
                    };
                    let mut page_results = parser::parse_results(&site, &html, &query);
                    // gog-games: try AJAX/JSON fragment fallbacks when DOM parse is empty
                    if page_results.is_empty() && site.name.eq_ignore_ascii_case("gog-games") {
                        let rate_limiter_ref = if let Some(ref rl) = rate_limiter {
                            Some(&mut *rl.lock().await)
                        } else {
                            None
                        };

                        if let Some(r) = fetch_gog_games_ajax_json(
                            &client,
                            &site,
                            &query,
                            use_cf,
                            &cf_url,
                            cookie_headers.clone(),
                            rate_limiter_ref,
                        )
                        .await
                            && !r.is_empty()
                        {
                            page_results = r;
                        }
                    }
                    if site.name.eq_ignore_ascii_case("gog-games") {
                        filter_results_by_query_strict(&mut page_results, &query);
                    }
                    // csrin: Atom feed fallback
                    if page_results.is_empty() && site.name.eq_ignore_ascii_case("csrin") {
                        let rate_limiter_ref = if let Some(ref rl) = rate_limiter {
                            Some(&mut *rl.lock().await)
                        } else {
                            None
                        };

                        if let Some(feed_results) =
                            fetch_csrin_feed(&client, &site, &query, rate_limiter_ref).await
                        {
                            page_results = feed_results;
                        }
                    }
                    results.extend(page_results);
                    if results.len() >= 5000 {
                        break;
                    }
                }
            }
            // Final csrin Playwright fallback if still empty
            if site.name.eq_ignore_ascii_case("csrin") && results.is_empty() && !no_playwright {
                let cookie_val = cookie_headers
                    .as_ref()
                    .and_then(|h| h.get(COOKIE))
                    .and_then(|v| v.to_str().ok())
                    .map(|s| s.to_string());
                if let Some(html) = fetch_csrin_playwright_html(&query, cookie_val).await {
                    let rs = parser::parse_results(&site, &html, &query);
                    if !rs.is_empty() {
                        results = rs;
                    }
                }
            }
            // Filter csrin results: only keep viewtopic.php links with title matching query
            // This removes sticky posts like "FAQ", "Forum rules", "Donations", etc.
            if site.name.eq_ignore_ascii_case("csrin") {
                let q_lower = query.to_lowercase();
                results.retain(|r| r.url.contains("viewtopic.php"));
                results.retain(|r| r.title.to_lowercase().contains(&q_lower));
            }
            // Truncate per-site
            if !results.is_empty() {
                results.truncate(limit);
            }
            results
        }));
    }

    let mut combined: Vec<models::SearchResult> = Vec::new();
    while let Some(joined) = tasks.next().await {
        if let Ok(mut site_results) = joined {
            combined.append(&mut site_results);
        }
    }
    // Dedup + sort
    combined.sort_by(|a, b| a.site.cmp(&b.site).then_with(|| a.title.cmp(&b.title)));
    combined.dedup_by(|a, b| a.site == b.site && a.url == b.url);

    // Apply overall cutoff if specified (0 means no cutoff)
    if let Some(cutoff) = args.cutoff
        && cutoff > 0
        && combined.len() > cutoff
    {
        combined.truncate(cutoff);
    }

    Ok(combined)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            search_gui,
            list_sites,
            get_cache,
            get_cached_results,
            add_to_cache,
            remove_cache_entry,
            clear_cache,
            get_cache_settings,
            set_cache_size
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

// Minimal feed + playwright helpers adapted for GUI context
async fn fetch_csrin_feed(
    client: &reqwest::Client,
    site: &website_searcher_core::models::SiteConfig,
    query: &str,
    rate_limiter: Option<&mut RateLimiter>,
) -> Option<Vec<models::SearchResult>> {
    let feed_url = "https://cs.rin.ru/forum/feed.php?f=10";
    let body = fetcher::fetch_with_retry(client, feed_url, rate_limiter, Some("csrin"))
        .await
        .ok()?;
    if body.is_empty() {
        return None;
    }
    let mut results: Vec<models::SearchResult> = Vec::new();
    let ql = query.to_lowercase();
    let mut i = 0usize;
    let mut xml = body;
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
    while let Some(tidx) = xml[i..].find("<entry>") {
        let start = i + tidx;
        let end = xml[start..]
            .find("</entry>")
            .map(|e| start + e + 8)
            .unwrap_or(xml.len());
        let entry = &xml[start..end];
        let mut title = "";
        if let Some(t_open_rel) = entry.find("<title") {
            let after_tag_rel = entry[t_open_rel..].find('>').map(|p| t_open_rel + p + 1);
            if let Some(content_start) = after_tag_rel
                && let Some(close_rel) = entry[content_start..].find("</title>")
            {
                let raw = &entry[content_start..content_start + close_rel];
                let raw = raw.trim();
                if let Some(inner) = raw.strip_prefix("<![CDATA[") {
                    if let Some(inner2) = inner.strip_suffix("]]") {
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
            .and_then(|(_, rest)| rest.split_once('"').map(|(u, _)| u))
            .unwrap_or("");
        if !title.is_empty() && href.contains("viewtopic.php") {
            let tl = title.to_lowercase();
            if tl.contains(&ql) || href.to_lowercase().contains(&ql.replace(' ', "+")) {
                let url = if href.starts_with("http") {
                    href.to_string()
                } else {
                    format!("https://cs.rin.ru/forum/{}", href.trim_start_matches('/'))
                };
                results.push(models::SearchResult {
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

async fn fetch_csrin_playwright_html(query: &str, cookie: Option<String>) -> Option<String> {
    // Allow tests/dev to inject HTML
    if let Ok(fake) = std::env::var("CS_PLAYWRIGHT_HTML")
        && !fake.trim().is_empty()
    {
        return Some(fake);
    }
    let script = "../scripts/csrin_search.cjs";
    let mut cmd = tokio::process::Command::new("node");
    use std::process::Stdio;
    cmd.arg(script).arg(query);
    if let Some(c) = cookie {
        cmd.env("PLAYWRIGHT_COOKIE", c);
    }
    if let Ok(p) = std::env::var("CSRIN_PAGES")
        && !p.trim().is_empty()
    {
        cmd.env("CSRIN_PAGES", p);
    }
    cmd.stdin(Stdio::null());
    cmd.stderr(Stdio::inherit());
    cmd.stdout(Stdio::piped());
    let mut child = cmd.spawn().ok()?;
    use tokio::io::AsyncReadExt;
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

// gog-games fallback: request AJAX JSON/HTML fragment when primary DOM selectors yield nothing
async fn fetch_gog_games_ajax_json(
    client: &reqwest::Client,
    site: &website_searcher_core::models::SiteConfig,
    query: &str,
    use_cf: bool,
    cf_url: &str,
    cookie_headers: Option<ReqHeaderMap>,
    mut rate_limiter: Option<&mut RateLimiter>,
) -> Option<Vec<models::SearchResult>> {
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

    for u in urls.into_iter() {
        let body: String = if use_cf {
            (cf::fetch_via_solver_with_headers(client, &u, cf_url, Some(headers.clone())).await)
                .unwrap_or_default()
        } else {
            (fetcher::fetch_with_retry_headers(
                client,
                &u,
                Some(headers.clone()),
                rate_limiter.as_deref_mut(),
                Some("gog-games"),
            )
            .await)
                .unwrap_or_default()
        };
        if body.is_empty() {
            continue;
        }
        let trimmed = body.trim_start();
        if trimmed.starts_with('<') {
            // Try to extract JSON inside <pre>...</pre>
            if let (Some(sidx), Some(eidx)) = (trimmed.find("<pre>"), trimmed.find("</pre>")) {
                let s = sidx + 5;
                if s < eidx {
                    let json_inner = &trimmed[s..eidx];
                    if let Ok(v) = serde_json::from_str::<serde_json::Value>(json_inner) {
                        let mut results: Vec<models::SearchResult> = Vec::new();
                        collect_title_url_pairs(&v, &mut results);
                        if !results.is_empty() {
                            return Some(results);
                        }
                    }
                }
            }
            // else treat as HTML fragment
            let rs = parser::parse_results(site, &body, query);
            if !rs.is_empty() {
                return Some(rs);
            }
            continue;
        }
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&body) {
            if let Some(html) = v.get("html").and_then(|x| x.as_str()) {
                let rs = parser::parse_results(site, html, query);
                if !rs.is_empty() {
                    return Some(rs);
                }
            }
            if let Some(html) = v
                .get("data")
                .and_then(|x| x.get("html"))
                .and_then(|x| x.as_str())
            {
                let rs = parser::parse_results(site, html, query);
                if !rs.is_empty() {
                    return Some(rs);
                }
            }
            let mut results: Vec<models::SearchResult> = Vec::new();
            collect_title_url_pairs(&v, &mut results);
            if !results.is_empty() {
                return Some(results);
            }
        }
    }
    None
}

#[allow(clippy::collapsible_if)]
fn collect_title_url_pairs(v: &serde_json::Value, out: &mut Vec<models::SearchResult>) {
    match v {
        serde_json::Value::Object(map) => {
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
                out.push(models::SearchResult {
                    site: "gog-games".to_string(),
                    title: t.to_string(),
                    url: u_abs,
                });
            }
            for val in map.values() {
                collect_title_url_pairs(val, out);
            }
        }
        serde_json::Value::Array(arr) => {
            for val in arr {
                collect_title_url_pairs(val, out);
            }
        }
        _ => {}
    }
}

fn filter_results_by_query_strict(results: &mut Vec<models::SearchResult>, query: &str) {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn list_sites_returns_all_configs() {
        let sites = list_sites().await.unwrap();
        assert!(!sites.is_empty());
        // Should contain well-known sites
        assert!(sites.iter().any(|s| s.eq_ignore_ascii_case("fitgirl")));
        assert!(sites.iter().any(|s| s.eq_ignore_ascii_case("dodi")));
    }

    #[test]
    fn filter_results_by_query_strict_removes_unrelated() {
        let mut results = vec![
            models::SearchResult {
                site: "gog-games".into(),
                title: "Elden Ring".into(),
                url: "https://gog-games.to/game/elden-ring".into(),
            },
            models::SearchResult {
                site: "gog-games".into(),
                title: "Other Game".into(),
                url: "https://gog-games.to/game/other".into(),
            },
        ];
        filter_results_by_query_strict(&mut results, "elden ring");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title, "Elden Ring");
    }

    #[test]
    fn filter_results_by_query_strict_handles_dash_encoding() {
        let mut results = vec![models::SearchResult {
            site: "gog-games".into(),
            title: "A Long Title".into(),
            url: "https://gog-games.to/game/elden-ring".into(),
        }];
        filter_results_by_query_strict(&mut results, "elden ring");
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn collect_title_url_pairs_extracts_from_array() {
        let json = serde_json::json!([
            {"title": "Game A", "url": "https://gog-games.to/game/a"},
            {"title": "Game B", "permalink": "https://gog-games.to/game/b"}
        ]);
        let mut out = Vec::new();
        collect_title_url_pairs(&json, &mut out);
        assert_eq!(out.len(), 2);
        assert!(out.iter().any(|r| r.title == "Game A"));
        assert!(out.iter().any(|r| r.title == "Game B"));
    }

    #[test]
    fn collect_title_url_pairs_extracts_from_nested_object() {
        let json = serde_json::json!({
            "data": {
                "items": [{"title": "Nested Game", "slug": "nested-game"}]
            }
        });
        let mut out = Vec::new();
        collect_title_url_pairs(&json, &mut out);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].title, "Nested Game");
        assert!(out[0].url.contains("nested-game"));
    }

    #[test]
    fn collect_title_url_pairs_handles_slug_to_url() {
        let json = serde_json::json!({"title": "My Game", "slug": "my-game"});
        let mut out = Vec::new();
        collect_title_url_pairs(&json, &mut out);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].url, "https://gog-games.to/game/my-game");
    }

    #[test]
    fn collect_title_url_pairs_handles_relative_urls() {
        let json = serde_json::json!({"title": "Rel Game", "url": "/game/relative"});
        let mut out = Vec::new();
        collect_title_url_pairs(&json, &mut out);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].url, "https://gog-games.to/game/relative");
    }

    #[test]
    fn filter_gog_path_must_include_game_segment() {
        let mut results = vec![
            models::SearchResult {
                site: "gog-games".into(),
                title: "Elden Ring".into(),
                url: "https://gog-games.to/game/elden-ring".into(),
            },
            models::SearchResult {
                site: "gog-games".into(),
                title: "Other".into(),
                url: "https://gog-games.to/search?q=elden".into(),
            },
        ];
        filter_results_by_query_strict(&mut results, "elden ring");
        assert_eq!(results.len(), 1);
        assert!(results[0].url.contains("/game/"));
    }

    #[test]
    fn filter_results_handles_url_variants() {
        let mut results = vec![
            models::SearchResult {
                site: "gog-games".into(),
                title: "Some Title".into(),
                url: "https://gog-games.to/game/elden%20ring".into(),
            },
            models::SearchResult {
                site: "gog-games".into(),
                title: "Some Title".into(),
                url: "https://gog-games.to/games/elden+ring".into(),
            },
        ];
        filter_results_by_query_strict(&mut results, "elden ring");
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn collect_pairs_handles_empty_value() {
        let json = serde_json::json!({});
        let mut out = Vec::new();
        collect_title_url_pairs(&json, &mut out);
        assert!(out.is_empty());
    }

    #[test]
    fn collect_pairs_uses_name_fallback() {
        let json = serde_json::json!({"name": "My Game", "slug": "my-game"});
        let mut out = Vec::new();
        collect_title_url_pairs(&json, &mut out);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].title, "My Game");
    }

    #[test]
    fn collect_pairs_uses_path_fallback() {
        let json = serde_json::json!({"title": "Path Game", "path": "/game/path-game"});
        let mut out = Vec::new();
        collect_title_url_pairs(&json, &mut out);
        assert_eq!(out.len(), 1);
        assert!(out[0].url.contains("path-game"));
    }

    #[tokio::test]
    async fn fetch_csrin_playwright_uses_env_var() {
        // SAFETY: Test-only, single-threaded; no other code reads this env var concurrently
        unsafe { std::env::set_var("CS_PLAYWRIGHT_HTML", "<html>test content</html>") };
        let result = fetch_csrin_playwright_html("test", None).await;
        // SAFETY: Cleaning up test env var
        unsafe { std::env::remove_var("CS_PLAYWRIGHT_HTML") };
        assert!(result.is_some());
        assert!(result.unwrap().contains("test content"));
    }

    #[tokio::test]
    async fn search_gui_empty_query_returns_error() {
        let args = SearchArgs {
            query: "   ".to_string(),
            limit: None,
            cutoff: None,
            sites: None,
            debug: None,
            no_cf: None,
            cf_url: None,
            cookie: None,
            csrin_pages: None,
            csrin_search: None,
            no_playwright: None,
            no_rate_limit: None,
        };
        let result = search_gui(args).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("empty"));
    }

    #[test]
    fn collect_pairs_handles_string_value() {
        let json = serde_json::json!("just a string");
        let mut out = Vec::new();
        collect_title_url_pairs(&json, &mut out);
        assert!(out.is_empty());
    }

    #[test]
    fn collect_pairs_handles_null_value() {
        let json = serde_json::json!(null);
        let mut out = Vec::new();
        collect_title_url_pairs(&json, &mut out);
        assert!(out.is_empty());
    }

    #[test]
    fn collect_pairs_handles_boolean_value() {
        let json = serde_json::json!(true);
        let mut out = Vec::new();
        collect_title_url_pairs(&json, &mut out);
        assert!(out.is_empty());
    }

    #[test]
    fn collect_pairs_handles_number_value() {
        let json = serde_json::json!(123.45);
        let mut out = Vec::new();
        collect_title_url_pairs(&json, &mut out);
        assert!(out.is_empty());
    }

    #[test]
    fn filter_results_plus_encoding() {
        let mut results = vec![models::SearchResult {
            site: "gog-games".into(),
            title: "Some Title".into(),
            url: "https://gog-games.to/game/elden+ring".into(),
        }];
        filter_results_by_query_strict(&mut results, "elden ring");
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn filter_results_stripped_spaces() {
        let mut results = vec![models::SearchResult {
            site: "gog-games".into(),
            title: "Some Title".into(),
            url: "https://gog-games.to/game/eldenring".into(),
        }];
        filter_results_by_query_strict(&mut results, "elden ring");
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn collect_pairs_uses_href_field() {
        let json = serde_json::json!({"title": "Href Game", "href": "/game/href"});
        let mut out = Vec::new();
        collect_title_url_pairs(&json, &mut out);
        assert_eq!(out.len(), 1);
        assert!(out[0].url.contains("href"));
    }

    #[tokio::test]
    async fn search_gui_with_site_filter() {
        // Test with a specific site filter that should return immediately
        let args = SearchArgs {
            query: "test".to_string(),
            limit: Some(1),
            cutoff: None,
            sites: Some(vec!["unknown-fake-site".to_string()]),
            debug: None,
            no_cf: Some(true),
            cf_url: None,
            cookie: None,
            csrin_pages: None,
            csrin_search: None,
            no_playwright: Some(true),
            no_rate_limit: None,
        };
        let result = search_gui(args).await;
        // Should succeed but return empty (no matching site)
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[tokio::test]
    async fn fetch_csrin_playwright_with_cookie() {
        unsafe { std::env::set_var("CS_PLAYWRIGHT_HTML", "<html>cookie test</html>") };
        let result = fetch_csrin_playwright_html("test", Some("session=abc".to_string())).await;
        unsafe { std::env::remove_var("CS_PLAYWRIGHT_HTML") };
        assert!(result.is_some());
        assert!(result.unwrap().contains("cookie test"));
    }

    #[tokio::test]
    async fn fetch_csrin_playwright_empty_env_returns_none() {
        unsafe { std::env::set_var("CS_PLAYWRIGHT_HTML", "   ") };
        let result = fetch_csrin_playwright_html("test", None).await;
        unsafe { std::env::remove_var("CS_PLAYWRIGHT_HTML") };
        // Empty env is treated as not set, script doesn't exist in test env
        assert!(result.is_none());
    }
}
