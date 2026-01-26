use scraper::{Html, Selector};
use urlencoding::decode;

use crate::models::{SearchResult, SiteConfig};

pub fn parse_results(site: &SiteConfig, html: &str, query: &str) -> Vec<SearchResult> {
    if html.is_empty() {
        return Vec::new();
    }

    // csrin phpBB search page: topics are anchors with class topictitle
    if site.name.eq_ignore_ascii_case("csrin") && html.contains("search.php") {
        let document = Html::parse_document(html);
        if let Ok(sel) = Selector::parse("a.topictitle") {
            let mut out = Vec::new();
            for a in document.select(&sel) {
                let href = a.value().attr("href").unwrap_or("");
                if href.is_empty() {
                    continue;
                }
                let mut url = href.to_string();
                let is_http = url.starts_with("http://")
                    || url.starts_with("https://")
                    || url.starts_with("//");
                if !is_http {
                    let base = site.base_url.trim_end_matches('/');
                    if url.starts_with('/') {
                        url = format!("{base}{url}");
                    } else {
                        url = format!("{}/{}", base, url.trim_start_matches('/'));
                    }
                }
                let mut title = a.text().collect::<String>().trim().to_string();
                if title.is_empty()
                    && let Some(derived) = derive_title_from_href(&url)
                {
                    title = derived;
                }
                if !title.is_empty() {
                    out.push(SearchResult {
                        site: site.name.to_string(),
                        title,
                        url,
                    });
                }
            }
            if !out.is_empty() {
                return out;
            }
        }
    }

    // Site-specific parser for elamigos: titles are in the heading text, link text is "DOWNLOAD"
    if site.name.eq_ignore_ascii_case("elamigos") {
        return parse_elamigos(site, html, query);
    }

    // Site-specific parser for f95zone: parse forum thread listings
    if site.name.eq_ignore_ascii_case("f95zone") {
        return parse_f95zone(site, html, query);
    }

    // Site-specific parser for nswpedia: filter WordPress search results
    if site.name.eq_ignore_ascii_case("nswpedia") {
        return parse_nswpedia(site, html, query);
    }
    let document = Html::parse_document(html);

    // Primary: use provided selector
    if let Ok(sel) = Selector::parse(&site.result_selector) {
        let mut primary: Vec<SearchResult> = Vec::new();
        for el in document.select(&sel) {
            let mut title = el.text().collect::<String>().trim().to_string();
            // Extract href; if empty, try parent element (some cards wrap anchors)
            let href_attr = el.value().attr("href").or_else(|| {
                el.parent()
                    .and_then(|p| p.value().as_element())
                    .and_then(|pel| pel.attr("href"))
            });
            let href = href_attr.unwrap_or("");
            let mut url = href.to_string();
            // Build absolute URL if relative
            if !url.is_empty() {
                let is_http = url.starts_with("http://")
                    || url.starts_with("https://")
                    || url.starts_with("//");
                if !is_http {
                    let base = site.base_url.trim_end_matches('/');
                    if url.starts_with('/') {
                        url = format!("{base}{url}");
                    } else if url.starts_with('#') {
                        url = format!("{}{}", site.base_url, url);
                    } else {
                        url = format!("{}/{}", base, url.trim_start_matches('/'));
                    }
                }
            }
            if url.is_empty() {
                continue;
            }
            if title.is_empty() {
                title = derive_title_from_href(&url).unwrap_or(title);
            }
            if site.name.eq_ignore_ascii_case("fitgirl") {
                if let Some(clean) = filter_and_normalize_fitgirl(&url, &title) {
                    title = clean;
                } else {
                    continue;
                }
            } else if site.name.eq_ignore_ascii_case("steamrip") {
                if let Some(clean) = filter_and_normalize_steamrip(&url, &title) {
                    title = clean;
                } else {
                    continue;
                }
            }
            if !title.is_empty() {
                primary.push(SearchResult {
                    site: site.name.to_string(),
                    title,
                    url,
                });
            }
        }
        if !primary.is_empty() {
            // Filter by query presence in title or URL to drop unrelated items
            let ql = query.to_lowercase();
            let ql_dash = ql.replace(' ', "-");
            let ql_plus = ql.replace(' ', "+");
            let ql_encoded = ql.replace(' ', "%20");
            let ql_stripped = ql.replace(' ', "");
            primary.retain(|r| {
                let tl = r.title.to_lowercase();
                let ul = r.url.to_lowercase();
                let basic = tl.contains(&ql)
                    || ul.contains(&ql)
                    || ul.contains(&ql_dash)
                    || ul.contains(&ql_plus)
                    || ul.contains(&ql_encoded)
                    || ul.contains(&ql_stripped);
                if site.name.eq_ignore_ascii_case("gog-games") {
                    // Tighten for gog-games: require a game-like path
                    basic && (ul.contains("/game/") || ul.contains("/games/"))
                } else {
                    basic
                }
            });
            if !primary.is_empty() {
                return primary;
            }
        }
    }

    // Fallback: scan all anchors and filter by query presence
    let Ok(a_sel) = Selector::parse("a[href]") else {
        return Vec::new();
    };
    let ql = query.to_lowercase();
    let ql_dash = ql.replace(' ', "-");
    let ql_plus = ql.replace(' ', "+");
    let ql_encoded = ql.replace(' ', "%20");
    let ql_stripped = ql.replace(' ', "");
    document
        .select(&a_sel)
        .filter_map(|el| {
            let text = el.text().collect::<String>();
            let href = el.value().attr("href").unwrap_or("");
            if href.is_empty() {
                return None;
            }
            let text_l = text.to_lowercase();
            let href_l = href.to_lowercase();
            let matches_query = text_l.contains(&ql)
                || href_l.contains(&ql)
                || href_l.contains(&ql_dash)
                || href_l.contains(&ql_plus)
                || href_l.contains(&ql_encoded)
                || href_l.contains(&ql_stripped);
            if !matches_query {
                return None;
            }
            // treat non-slashed hrefs like "post-slug/" as relative too
            let is_http = href.starts_with("http://")
                || href.starts_with("https://")
                || href.starts_with("//");
            let is_relative = href.starts_with('/') || href.starts_with('#') || !is_http;

            let mut url = href.to_string();
            if is_relative {
                let base = site.base_url.trim_end_matches('/');
                if href.starts_with('/') {
                    url = format!("{base}{href}");
                } else if href.starts_with('#') {
                    url = format!("{}{}", site.base_url, href);
                } else {
                    url = format!("{}/{}", base, href.trim_start_matches('/'));
                }
            }

            let mut title = text.trim().to_string();
            if title.is_empty() {
                if let Some(derived) = derive_title_from_href(&url) {
                    title = derived;
                }
                if title.is_empty() {
                    return None;
                }
            }
            if site.name.eq_ignore_ascii_case("fitgirl") {
                if let Some(clean) = filter_and_normalize_fitgirl(&url, &title) {
                    title = clean;
                } else {
                    return None;
                }
            } else if site.name.eq_ignore_ascii_case("steamrip") {
                if let Some(clean) = filter_and_normalize_steamrip(&url, &title) {
                    title = clean;
                } else {
                    return None;
                }
            }

            Some(SearchResult {
                site: site.name.to_string(),
                title,
                url: url.replace("/./", "/"),
            })
        })
        .collect()
}

fn derive_title_from_href(href: &str) -> Option<String> {
    // Try last path segment
    let mut segment = href;
    if let Some(idx) = href.rfind('/') {
        segment = &href[idx + 1..];
    }
    // strip anchors/query
    if let Some(q) = segment.find(['?', '#']) {
        segment = &segment[..q];
    }
    if segment.is_empty() {
        return None;
    }
    let decoded = decode(segment).ok()?.to_string();
    let replaced = decoded.replace(['-', '_'], " ");
    let words: Vec<String> = replaced
        .split_whitespace()
        .map(|w| {
            let mut chrs = w.chars();
            match chrs.next() {
                Some(c) => format!("{}{}", c.to_uppercase(), chrs.as_str().to_lowercase()),
                None => String::new(),
            }
        })
        .collect();
    let title = words.join(" ").trim().to_string();
    if title.is_empty() { None } else { Some(title) }
}

fn looks_like_date_ddmmyyyy(s: &str) -> bool {
    let t = s.trim();
    if t.len() < 8 || t.len() > 10 {
        return false;
    }
    if t.chars().filter(|c| *c == '/').count() != 2 {
        return false;
    }
    t.chars().all(|c| c.is_ascii_digit() || c == '/')
}

fn filter_and_normalize_fitgirl(url: &str, title: &str) -> Option<String> {
    // Drop pagination and comment anchors
    if url.contains("/page/") || url.contains("#respond") || url.contains("?s=") {
        return None;
    }
    // Drop tag/category/archive and inquiry pages
    let url_l = url.to_lowercase();
    if url_l.contains("/tag/") || url_l.contains("/category/") || url_l.contains("/categories/") {
        return None;
    }
    if url_l.contains("/inquiry") || url_l.contains("/inquery") {
        return None;
    }
    let t = title.trim();
    if t.is_empty() {
        return None;
    }
    if t.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    if t.to_lowercase().contains("comments") {
        return None;
    }
    if looks_like_date_ddmmyyyy(t) {
        return None;
    }

    // Drop "Continue reading ..." teaser links (we keep the main post link instead)
    if t.to_lowercase().starts_with("continue reading") {
        return None;
    }
    Some(t.to_string())
}

fn filter_and_normalize_steamrip(url: &str, title: &str) -> Option<String> {
    // Drop obvious pagination and search navigational links
    if url.contains("/page/") || url.contains("?s=") {
        return None;
    }
    let t = title.trim();
    if t.is_empty() {
        return None;
    }
    let tl = t.to_lowercase();
    if tl == "next" || tl == "previous" || tl.starts_with("next") || tl.starts_with("prev") {
        return None;
    }
    if t.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    Some(t.to_string())
}

fn parse_elamigos(site: &SiteConfig, html: &str, query: &str) -> Vec<SearchResult> {
    let document = Html::parse_document(html);
    let Ok(sel) = Selector::parse("h3, h5") else {
        return Vec::new();
    };
    let ql = query.to_lowercase();
    let mut results: Vec<SearchResult> = Vec::new();

    for heading in document.select(&sel) {
        let text = heading.text().collect::<String>();
        let text_norm = text.trim();
        if text_norm.is_empty() {
            continue;
        }
        if !text_norm.to_lowercase().contains(&ql) {
            continue;
        }
        // Find the first link in this heading
        let a_sel = match Selector::parse("a[href]") {
            Ok(s) => s,
            Err(_) => continue,
        };
        if let Some(a) = heading.select(&a_sel).next() {
            let href = a.value().attr("href").unwrap_or("");
            if href.is_empty() {
                continue;
            }
            // Build absolute URL
            let mut url = href.to_string();
            if !(href.starts_with("http://")
                || href.starts_with("https://")
                || href.starts_with("//"))
            {
                let base = site.base_url.trim_end_matches('/');
                if href.starts_with('/') {
                    url = format!("{base}{href}");
                } else {
                    url = format!("{}/{}", base, href.trim_start_matches('/'));
                }
            }
            // Title: remove trailing DOWNLOAD and trim
            let title = text_norm.replace("DOWNLOAD", "").trim().to_string();
            results.push(SearchResult {
                site: site.name.to_string(),
                title,
                url,
            });
        }
    }

    results
}

/// Parse F95zone forum thread listings
/// Extracts game titles from thread links like [Game Name [vX.X] [Developer]]
fn parse_f95zone(site: &SiteConfig, html: &str, query: &str) -> Vec<SearchResult> {
    let document = Html::parse_document(html);
    let Ok(sel) = Selector::parse("a[href*='/threads/']") else {
        return Vec::new();
    };
    let ql = query.to_lowercase();
    let ql_parts: Vec<&str> = ql.split_whitespace().collect();
    let mut results: Vec<SearchResult> = Vec::new();
    let mut seen_urls: std::collections::HashSet<String> = std::collections::HashSet::new();

    for a in document.select(&sel) {
        let href = a.value().attr("href").unwrap_or("");
        if href.is_empty() {
            continue;
        }

        // Skip pagination, member links, and non-game thread links
        if href.contains("/page-")
            || href.contains("/members/")
            || href.contains("/latest")
            || href.contains("#")
        {
            continue;
        }

        let mut url = href.to_string();
        // Build absolute URL
        if !url.starts_with("http") {
            url = format!("{}{}", site.base_url.trim_end_matches('/'), url);
        }

        // Deduplicate
        if seen_urls.contains(&url) {
            continue;
        }

        let title = a.text().collect::<String>().trim().to_string();
        if title.is_empty() {
            continue;
        }

        // Skip navigational text
        let tl = title.to_lowercase();
        if tl.len() < 3
            || tl == "threads"
            || tl == "games"
            || tl.starts_with("page ")
            || tl.parse::<u32>().is_ok()
        {
            continue;
        }

        // Check if query matches (all words must be present)
        let matches = ql_parts.iter().all(|part| tl.contains(part));
        if !matches {
            continue;
        }

        seen_urls.insert(url.clone());
        results.push(SearchResult {
            site: site.name.to_string(),
            title,
            url,
        });

        if results.len() >= 50 {
            break;
        }
    }

    results
}

/// Parse NSWpedia WordPress search results
/// Filters navigation links and extracts game titles
fn parse_nswpedia(site: &SiteConfig, html: &str, query: &str) -> Vec<SearchResult> {
    let document = Html::parse_document(html);
    // Match h2 elements that contain links (search result cards)
    let Ok(sel) = Selector::parse("h2 a, article a, .post-title a") else {
        return Vec::new();
    };
    let ql = query.to_lowercase();
    let ql_parts: Vec<&str> = ql.split_whitespace().collect();
    let mut results: Vec<SearchResult> = Vec::new();
    let mut seen_urls: std::collections::HashSet<String> = std::collections::HashSet::new();

    for a in document.select(&sel) {
        let href = a.value().attr("href").unwrap_or("");
        if href.is_empty() {
            continue;
        }

        // Skip pagination, navigation, and category links
        if href.contains("/page/")
            || href.contains("/category/")
            || href.contains("/tag/")
            || href.contains("/badge/")
            || href.contains("/tutorials/")
            || href.contains("/about")
            || href.contains("/contact")
            || href.contains("/privacy")
            || !href.contains("nswpedia.com")
        {
            continue;
        }

        let url = href.to_string();

        // Deduplicate
        if seen_urls.contains(&url) {
            continue;
        }

        let title = a.text().collect::<String>().trim().to_string();
        if title.is_empty() {
            continue;
        }

        // Skip nav elements
        let tl = title.to_lowercase();
        if tl == "nswpedia.com"
            || tl == "switch roms"
            || tl == "exclusives"
            || tl == "tutorials"
            || tl == "more"
            || tl == "home"
        {
            continue;
        }

        // Check if query matches
        let matches = ql_parts.iter().all(|part| tl.contains(part));
        if !matches {
            continue;
        }

        seen_urls.insert(url.clone());
        results.push(SearchResult {
            site: site.name.to_string(),
            title,
            url,
        });

        if results.len() >= 50 {
            break;
        }
    }

    results
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg() -> SiteConfig {
        SiteConfig {
            name: "example".to_string(),
            base_url: "https://example.com/".to_string(),
            search_kind: crate::models::SearchKind::QueryParam,
            query_param: Some("s".to_string()),
            listing_path: None,
            result_selector: "h2.entry-title a".to_string(),
            title_attr: "text".to_string(),
            url_attr: "href".to_string(),
            requires_js: false,
            requires_cloudflare: false,
            timeout_seconds: 30,
            retry_attempts: 3,
            rate_limit_delay_ms: 1000,
        }
    }

    fn cfg_with_selector(selector: &str) -> SiteConfig {
        SiteConfig {
            name: "example".to_string(),
            base_url: "https://example.com/".to_string(),
            search_kind: crate::models::SearchKind::QueryParam,
            query_param: Some("s".to_string()),
            listing_path: None,
            result_selector: selector.to_string(),
            title_attr: "text".to_string(),
            url_attr: "href".to_string(),
            requires_js: false,
            requires_cloudflare: false,
            timeout_seconds: 30,
            retry_attempts: 3,
            rate_limit_delay_ms: 1000,
        }
    }

    #[test]
    fn primary_selector_is_filtered_by_query() {
        let cfg = cfg_with_selector("a");
        let html = r#"<html><body>
            <a href="/one">Something else</a>
            <a href="/cyberpunk-2077">Cyberpunk 2077</a>
        </body></html>"#;
        let results = parse_results(&cfg, html, "cyberpunk");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title.to_lowercase(), "cyberpunk 2077");
        assert!(results[0].url.ends_with("/cyberpunk-2077"));
    }

    #[test]
    fn primary_relative_href_becomes_absolute() {
        let cfg = cfg_with_selector("a.topictitle"); // simulate csrin selector
        let html = r#"<html><body>
            <a class="topictitle" href="viewtopic.php?t=12345">Elden Ring</a>
        </body></html>"#;
        let results = parse_results(&cfg, html, "elden ring");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].url, "https://example.com/viewtopic.php?t=12345");
    }

    #[test]
    fn fallback_finds_query_text() {
        let html = r#"<html><body>
            <a href="post-slug/">Elden Ring Deluxe Edition Free Download</a>
            <a href="/absolute-path/">ELDEN RING NIGHTREIGN</a>
            <a href="https://other.com/x">Elden Ring external</a>
            <a href="/unrelated">Something else</a>
        </body></html>"#;
        let results = parse_results(&cfg(), html, "elden ring");
        assert!(
            results.len() >= 3,
            "expected at least 3 results, got {}",
            results.len()
        );
        let urls: Vec<String> = results.into_iter().map(|r| r.url).collect();
        assert!(urls.contains(&"https://example.com/post-slug/".to_string()));
        assert!(urls.contains(&"https://example.com/absolute-path/".to_string()));
        assert!(urls.contains(&"https://other.com/x".to_string()));
    }

    #[test]
    fn derives_title_from_empty_anchor_text() {
        let html = r#"<html><body>
            <a href="elden-ring_nightreign">   </a>
        </body></html>"#;
        let results = parse_results(&cfg(), html, "elden ring");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title, "Elden Ring Nightreign");
        assert_eq!(results[0].url, "https://example.com/elden-ring_nightreign");
    }

    #[test]
    fn relative_hash_anchor_builds_absolute() {
        let html = r##"<html><body>
            <a href="#respond">Elden Ring Comments</a>
        </body></html>"##;
        let results = parse_results(&cfg(), html, "elden ring");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].url, "https://example.com/#respond");
    }

    #[test]
    fn fitgirl_filters_and_normalizes() {
        let mut cfg = cfg();
        cfg.name = "fitgirl".to_string();
        let html = r#"<html><body>
            <a href="/page/2">Elden Ring Page</a>
            <a href="/post/1">12345</a>
            <a href="/post/2">21/07/2023</a>
            <a href="/post/3?search=s">Continue reading Elden Ring</a>
            <a href="/post/4#respond">Elden Ring Comments</a>
            <a href="/post/5">Proper Elden Ring Release</a>
        </body></html>"#;
        let results = parse_results(&cfg, html, "elden ring");
        // Only the last one should survive filters
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title, "Proper Elden Ring Release");
    }

    #[test]
    fn parse_elamigos_headings_extract_title_and_absolute_url() {
        let cfg = SiteConfig {
            name: "elamigos".to_string(),
            base_url: "https://elamigos.site/".to_string(),
            search_kind: crate::models::SearchKind::FrontPage,
            query_param: None,
            listing_path: None,
            result_selector: "ignored".to_string(),
            title_attr: "text".to_string(),
            url_attr: "href".to_string(),
            requires_js: false,
            requires_cloudflare: false,
            timeout_seconds: 30,
            retry_attempts: 3,
            rate_limit_delay_ms: 1000,
        };
        let html = r#"<html><body>
            <h3><a href="/post/elden-ring">ELDEN RING DOWNLOAD</a></h3>
            <h5><a href="https://elamigos.site/post/other">Other Game DOWNLOAD</a></h5>
        </body></html>"#;
        let results = parse_results(&cfg, html, "elden ring");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title, "ELDEN RING");
        assert_eq!(results[0].url, "https://elamigos.site/post/elden-ring");
    }

    #[test]
    fn parse_f95zone_extracts_thread_links() {
        let cfg = SiteConfig {
            name: "f95zone".to_string(),
            base_url: "https://f95zone.to".to_string(),
            search_kind: crate::models::SearchKind::QueryParam,
            query_param: Some("q".to_string()),
            listing_path: None,
            result_selector: "a".to_string(),
            title_attr: "text".to_string(),
            url_attr: "href".to_string(),
            requires_js: false,
            requires_cloudflare: false,
            timeout_seconds: 30,
            retry_attempts: 3,
            rate_limit_delay_ms: 1000,
        };
        let html = r#"<html><body>
            <a href="/threads/elden-ring-nightreign.12345/">Elden Ring Nightreign [v1.0] [FromSoft]</a>
            <a href="/threads/other-game.54321/">Other Game</a>
            <a href="/members/user.123/">User Profile</a>
        </body></html>"#;
        let results = parse_results(&cfg, html, "elden ring");
        assert_eq!(results.len(), 1);
        assert!(results[0].title.contains("Elden Ring"));
        assert!(results[0].url.contains("/threads/elden-ring"));
    }

    #[test]
    fn parse_f95zone_deduplicates_urls() {
        let cfg = SiteConfig {
            name: "f95zone".to_string(),
            base_url: "https://f95zone.to".to_string(),
            search_kind: crate::models::SearchKind::QueryParam,
            query_param: Some("q".to_string()),
            listing_path: None,
            result_selector: "a".to_string(),
            title_attr: "text".to_string(),
            url_attr: "href".to_string(),
            requires_js: false,
            requires_cloudflare: false,
            timeout_seconds: 30,
            retry_attempts: 3,
            rate_limit_delay_ms: 1000,
        };
        let html = r#"<html><body>
            <a href="/threads/elden-ring.12345/">Elden Ring</a>
            <a href="/threads/elden-ring.12345/">Elden Ring (duplicate)</a>
        </body></html>"#;
        let results = parse_results(&cfg, html, "elden ring");
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn parse_nswpedia_extracts_game_links() {
        let cfg = SiteConfig {
            name: "nswpedia".to_string(),
            base_url: "https://nswpedia.com".to_string(),
            search_kind: crate::models::SearchKind::QueryParam,
            query_param: Some("s".to_string()),
            listing_path: None,
            result_selector: "h2 a".to_string(),
            title_attr: "text".to_string(),
            url_attr: "href".to_string(),
            requires_js: false,
            requires_cloudflare: false,
            timeout_seconds: 30,
            retry_attempts: 3,
            rate_limit_delay_ms: 1000,
        };
        let html = r#"<html><body>
            <h2><a href="https://nswpedia.com/zelda-tears-kingdom/">Zelda Tears of the Kingdom</a></h2>
            <h2><a href="https://nswpedia.com/category/games/">Games</a></h2>
            <h2><a href="https://nswpedia.com/zelda-botw/">Zelda BOTW</a></h2>
        </body></html>"#;
        let results = parse_results(&cfg, html, "zelda");
        assert_eq!(results.len(), 2);
        assert!(results.iter().any(|r| r.title.contains("Tears")));
        assert!(results.iter().any(|r| r.title.contains("BOTW")));
    }

    #[test]
    fn parse_nswpedia_skips_nav_elements() {
        let cfg = SiteConfig {
            name: "nswpedia".to_string(),
            base_url: "https://nswpedia.com".to_string(),
            search_kind: crate::models::SearchKind::QueryParam,
            query_param: Some("s".to_string()),
            listing_path: None,
            result_selector: "h2 a".to_string(),
            title_attr: "text".to_string(),
            url_attr: "href".to_string(),
            requires_js: false,
            requires_cloudflare: false,
            timeout_seconds: 30,
            retry_attempts: 3,
            rate_limit_delay_ms: 1000,
        };
        let html = r#"<html><body>
            <h2><a href="https://nswpedia.com/about">About</a></h2>
            <h2><a href="https://nswpedia.com/tag/games/">Tag Games</a></h2>
        </body></html>"#;
        let results = parse_results(&cfg, html, "games");
        assert!(results.is_empty());
    }

    #[test]
    fn csrin_topictitle_parses_relative_url_with_query() {
        let cfg = SiteConfig {
            name: "csrin".to_string(),
            base_url: "https://cs.rin.ru/forum".to_string(),
            search_kind: crate::models::SearchKind::PhpBBSearch,
            query_param: Some("keywords".to_string()),
            listing_path: None,
            result_selector: "a.topictitle".to_string(),
            title_attr: "text".to_string(),
            url_attr: "href".to_string(),
            requires_js: false,
            requires_cloudflare: false,
            timeout_seconds: 30,
            retry_attempts: 3,
            rate_limit_delay_ms: 1000,
        };
        // Simulate search.php results page
        let html = r#"<html><body>search.php
            <a class="topictitle" href="./viewtopic.php?t=12345&hilit=elden">Elden Ring</a>
        </body></html>"#;
        let results = parse_results(&cfg, html, "elden ring");
        assert_eq!(results.len(), 1);
        assert!(results[0].url.starts_with("https://cs.rin.ru/forum"));
        assert!(results[0].url.contains("viewtopic.php"));
    }

    #[test]
    fn derive_title_handles_query_strings() {
        // Test that query strings and anchors are stripped
        let result = derive_title_from_href("https://example.com/elden-ring?ref=search#main");
        assert!(result.is_some());
        let title = result.unwrap();
        assert_eq!(title, "Elden Ring");
    }

    #[test]
    fn derive_title_handles_special_characters() {
        let result = derive_title_from_href("https://example.com/game_name-here");
        assert!(result.is_some());
        let title = result.unwrap();
        assert_eq!(title, "Game Name Here");
    }

    #[test]
    fn steamrip_filter_drops_nav_links() {
        let mut cfg = cfg();
        cfg.name = "steamrip".to_string();
        let html = r#"<html><body>
            <a href="/page/2">Next</a>
            <a href="/game?s=test">Previous</a>
            <a href="/elden-ring-free">Elden Ring Free Download</a>
        </body></html>"#;
        let results = parse_results(&cfg, html, "elden ring");
        assert_eq!(results.len(), 1);
        assert!(results[0].title.contains("Elden Ring"));
    }

    #[test]
    fn steamrip_filter_drops_numeric_titles() {
        let mut cfg = cfg();
        cfg.name = "steamrip".to_string();
        let html = r#"<html><body>
            <a href="/elden-ring">12345</a>
            <a href="/elden-ring-deluxe">Elden Ring Deluxe</a>
        </body></html>"#;
        let results = parse_results(&cfg, html, "elden ring");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title, "Elden Ring Deluxe");
    }

    #[test]
    fn looks_like_date_detects_date_format() {
        assert!(looks_like_date_ddmmyyyy("21/07/2023"));
        assert!(looks_like_date_ddmmyyyy("1/1/2023"));
        assert!(!looks_like_date_ddmmyyyy("not a date"));
        assert!(!looks_like_date_ddmmyyyy("2023-07-21"));
    }

    #[test]
    fn empty_html_returns_empty_results() {
        let results = parse_results(&cfg(), "", "query");
        assert!(results.is_empty());
    }

    #[test]
    fn gog_games_filtering_requires_game_path() {
        let mut cfg = cfg();
        cfg.name = "gog-games".to_string();
        let html = r#"<html><body>
            <a href="/game/elden-ring">Elden Ring</a>
            <a href="/search?q=elden">Search Results</a>
        </body></html>"#;
        let results = parse_results(&cfg, html, "elden ring");
        assert_eq!(results.len(), 1);
        assert!(results[0].url.contains("/game/"));
    }

    #[test]
    fn fitgirl_filters_category_and_tag_urls() {
        let mut cfg = cfg();
        cfg.name = "fitgirl".to_string();
        let html = r#"<html><body>
            <a href="/category/games">Elden Ring Category</a>
            <a href="/tag/rpg">Elden Ring RPG Tag</a>
            <a href="/post/elden-ring">Elden Ring Download</a>
        </body></html>"#;
        let results = parse_results(&cfg, html, "elden ring");
        assert_eq!(results.len(), 1);
        assert!(results[0].url.contains("/post/"));
    }

    #[test]
    fn fitgirl_filters_inquiry_pages() {
        let mut cfg = cfg();
        cfg.name = "fitgirl".to_string();
        let html = r#"<html><body>
            <a href="/inquiry/elden-ring">Elden Ring Inquiry</a>
            <a href="/inquery/elden">Elden Inquery</a>
            <a href="/game/elden-ring-proper">Elden Ring Proper</a>
        </body></html>"#;
        let results = parse_results(&cfg, html, "elden ring");
        assert_eq!(results.len(), 1);
        assert!(results[0].url.contains("/game/"));
    }

    #[test]
    fn elamigos_empty_query_returns_empty() {
        let cfg = SiteConfig {
            name: "elamigos".to_string(),
            base_url: "https://elamigos.site/".to_string(),
            search_kind: crate::models::SearchKind::FrontPage,
            query_param: None,
            listing_path: None,
            result_selector: "h3 a".to_string(),
            title_attr: "text".to_string(),
            url_attr: "href".to_string(),
            requires_js: false,
            requires_cloudflare: false,
            timeout_seconds: 30,
            retry_attempts: 3,
            rate_limit_delay_ms: 1000,
        };
        let html = r#"<html><body>
            <h3><a href="/game/other">Other Game DOWNLOAD</a></h3>
        </body></html>"#;
        let results = parse_results(&cfg, html, "elden ring");
        assert!(results.is_empty());
    }

    #[test]
    fn f95zone_skips_pagination_and_hash_links() {
        let cfg = SiteConfig {
            name: "f95zone".to_string(),
            base_url: "https://f95zone.to".to_string(),
            search_kind: crate::models::SearchKind::QueryParam,
            query_param: Some("q".to_string()),
            listing_path: None,
            result_selector: "a".to_string(),
            title_attr: "text".to_string(),
            url_attr: "href".to_string(),
            requires_js: false,
            requires_cloudflare: false,
            timeout_seconds: 30,
            retry_attempts: 3,
            rate_limit_delay_ms: 1000,
        };
        let html = r#"<html><body>
            <a href="/threads/elden-ring.12345/page-2">Page 2</a>
            <a href="/threads/elden-ring.12345/#post-1">Elden Ring Post</a>
            <a href="/threads/elden-ring.12345/">Elden Ring Main</a>
        </body></html>"#;
        let results = parse_results(&cfg, html, "elden ring");
        assert_eq!(results.len(), 1);
        assert!(!results[0].url.contains("page-"));
    }

    #[test]
    fn derive_title_empty_segment_returns_none() {
        let result = derive_title_from_href("https://example.com/");
        assert!(result.is_none());
    }

    #[test]
    fn derive_title_url_encoded_segment() {
        let result = derive_title_from_href("https://example.com/elden%20ring%20deluxe");
        assert!(result.is_some());
        assert!(result.unwrap().to_lowercase().contains("elden"));
    }

    #[test]
    fn primary_selector_with_parent_href() {
        // Tests the case where anchor doesn't have href but parent does
        let cfg = SiteConfig {
            name: "example".to_string(),
            base_url: "https://example.com/".to_string(),
            search_kind: crate::models::SearchKind::QueryParam,
            query_param: Some("s".to_string()),
            listing_path: None,
            result_selector: "a".to_string(),
            title_attr: "text".to_string(),
            url_attr: "href".to_string(),
            requires_js: false,
            requires_cloudflare: false,
            timeout_seconds: 30,
            retry_attempts: 3,
            rate_limit_delay_ms: 1000,
        };
        let html = r#"<html><body>
            <a href="/elden-ring"><span class="title">Elden Ring</span></a>
        </body></html>"#;
        let results = parse_results(&cfg, html, "elden ring");
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn csrin_relative_url_without_leading_slash() {
        let cfg = SiteConfig {
            name: "csrin".to_string(),
            base_url: "https://cs.rin.ru/forum".to_string(),
            search_kind: crate::models::SearchKind::PhpBBSearch,
            query_param: Some("keywords".to_string()),
            listing_path: None,
            result_selector: "a.topictitle".to_string(),
            title_attr: "text".to_string(),
            url_attr: "href".to_string(),
            requires_js: false,
            requires_cloudflare: false,
            timeout_seconds: 30,
            retry_attempts: 3,
            rate_limit_delay_ms: 1000,
        };
        let html = r#"<html><body>search.php
            <a class="topictitle" href="viewtopic.php?t=99">Elden Ring</a>
        </body></html>"#;
        let results = parse_results(&cfg, html, "elden ring");
        assert_eq!(results.len(), 1);
        assert!(results[0].url.starts_with("https://cs.rin.ru/forum"));
    }

    #[test]
    fn steamrip_filter_empty_title_returns_none() {
        let result = filter_and_normalize_steamrip("/game", "   ");
        assert!(result.is_none());
    }

    #[test]
    fn fitgirl_filter_empty_title_returns_none() {
        let result = filter_and_normalize_fitgirl("/game", "   ");
        assert!(result.is_none());
    }

    #[test]
    fn nswpedia_skips_non_domain_links() {
        let cfg = SiteConfig {
            name: "nswpedia".to_string(),
            base_url: "https://nswpedia.com".to_string(),
            search_kind: crate::models::SearchKind::QueryParam,
            query_param: Some("s".to_string()),
            listing_path: None,
            result_selector: "h2 a".to_string(),
            title_attr: "text".to_string(),
            url_attr: "href".to_string(),
            requires_js: false,
            requires_cloudflare: false,
            timeout_seconds: 30,
            retry_attempts: 3,
            rate_limit_delay_ms: 1000,
        };
        let html = r#"<html><body>
            <h2><a href="https://other-site.com/zelda">Zelda on Other</a></h2>
            <h2><a href="https://nswpedia.com/zelda-totk">Zelda TOTK</a></h2>
        </body></html>"#;
        let results = parse_results(&cfg, html, "zelda");
        assert_eq!(results.len(), 1);
        assert!(results[0].url.contains("nswpedia.com"));
    }

    #[test]
    fn looks_like_date_edge_cases() {
        // Too short
        assert!(!looks_like_date_ddmmyyyy("1/1/23"));
        // Too long
        assert!(!looks_like_date_ddmmyyyy("01/01/20230"));
        // Not enough slashes
        assert!(!looks_like_date_ddmmyyyy("01-01-2023"));
    }
}
