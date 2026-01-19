use std::sync::Arc;

use futures::stream::{FuturesUnordered, StreamExt};
use reqwest::header::{
    ACCEPT, COOKIE, HeaderMap as ReqHeaderMap, HeaderName, HeaderValue, REFERER,
};
use tokio::sync::Semaphore;
use website_searcher_core::{cf, config, fetcher, models, parser, query};

#[derive(serde::Deserialize)]
struct SearchArgs {
    query: String,
    limit: Option<usize>,
    sites: Option<Vec<String>>, // names
    debug: Option<bool>,
    no_cf: Option<bool>,
    cf_url: Option<String>,
    cookie: Option<String>,
    csrin_pages: Option<usize>,
    csrin_search: Option<bool>,
    no_playwright: Option<bool>,
}

#[tauri::command]
async fn list_sites() -> Result<Vec<String>, String> {
    let names: Vec<String> = config::site_configs()
        .into_iter()
        .map(|s| s.name.to_string())
        .collect();
    Ok(names)
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
            .filter(|s| wanted.iter().any(|w| w.eq_ignore_ascii_case(s.name)))
            .collect()
    } else {
        all_sites
    };

    let client = fetcher::build_http_client();
    let semaphore = Arc::new(Semaphore::new(3));

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
        tasks.push(tokio::spawn(async move {
            let _permit = permit;
            let base_url = match site.search_kind {
                models::SearchKind::ListingPage => {
                    site.listing_path.unwrap_or(site.base_url).to_string()
                }
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
                        (if cookie_headers.is_some() {
                            fetcher::fetch_with_retry_headers(&client, &url, cookie_headers.clone())
                                .await
                        } else {
                            fetcher::fetch_with_retry(&client, &url).await
                        })
                        .unwrap_or_default()
                    };
                    let mut page_results = parser::parse_results(&site, &html, &query);
                    // gog-games: try AJAX/JSON fragment fallbacks when DOM parse is empty
                    if page_results.is_empty()
                        && site.name.eq_ignore_ascii_case("gog-games")
                        && let Some(r) = fetch_gog_games_ajax_json(
                            &client,
                            &site,
                            &query,
                            use_cf,
                            &cf_url,
                            cookie_headers.clone(),
                        )
                        .await
                        && !r.is_empty()
                    {
                        page_results = r;
                    }
                    if site.name.eq_ignore_ascii_case("gog-games") {
                        filter_results_by_query_strict(&mut page_results, &query);
                    }
                    // csrin: Atom feed fallback
                    if page_results.is_empty()
                        && site.name.eq_ignore_ascii_case("csrin")
                        && let Some(feed_results) = fetch_csrin_feed(&client, &site, &query).await
                    {
                        page_results = feed_results;
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
        .invoke_handler(tauri::generate_handler![search_gui, list_sites])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

// Minimal feed + playwright helpers adapted for GUI context
async fn fetch_csrin_feed(
    client: &reqwest::Client,
    site: &website_searcher_core::models::SiteConfig,
    query: &str,
) -> Option<Vec<models::SearchResult>> {
    let feed_url = "https://cs.rin.ru/forum/feed.php?f=10";
    let body = fetcher::fetch_with_retry(client, feed_url).await.ok()?;
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
            (fetcher::fetch_with_retry_headers(client, &u, Some(headers.clone())).await)
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
