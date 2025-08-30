use scraper::{Html, Selector};
use urlencoding::decode;

use crate::models::{SearchResult, SiteConfig};

pub fn parse_results(site: &SiteConfig, html: &str, query: &str) -> Vec<SearchResult> {
    if html.is_empty() {
        return Vec::new();
    }

    // Site-specific parser for elamigos: titles are in the heading text, link text is "DOWNLOAD"
    if site.name.eq_ignore_ascii_case("elamigos") {
        return parse_elamigos(site, html, query);
    }
    let document = Html::parse_document(html);

    // Primary: use provided selector
    if let Ok(sel) = Selector::parse(site.result_selector) {
        let mut primary: Vec<SearchResult> = Vec::new();
        for el in document.select(&sel) {
            let mut title = el.text().collect::<String>().trim().to_string();
            let url = el.value().attr("href").unwrap_or("").to_string();
            if url.is_empty() {
                continue;
            }
            if title.is_empty() {
                if let Some(derived) = derive_title_from_href(&url) {
                    title = derived;
                }
            }
            if site.name.eq_ignore_ascii_case("fitgirl") {
                if let Some(clean) = filter_and_normalize_fitgirl(&url, &title) {
                    title = clean;
                } else {
                    continue;
                }
            }
            if !title.is_empty() {
                primary.push(SearchResult { site: site.name.to_string(), title, url });
            }
        }
        if !primary.is_empty() {
            return primary;
        }
    }

    // Fallback: scan all anchors and filter by query presence
    let Ok(a_sel) = Selector::parse("a[href]") else { return Vec::new() };
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
            let is_http = href.starts_with("http://") || href.starts_with("https://") || href.starts_with("//");
            let is_relative = href.starts_with('/') || href.starts_with('#') || !is_http;

            let mut url = href.to_string();
            if is_relative {
                let base = site.base_url.trim_end_matches('/');
                if href.starts_with('/') {
                    url = format!("{}{}", base, href);
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
                if title.is_empty() { return None; }
            }
            if site.name.eq_ignore_ascii_case("fitgirl") {
                if let Some(clean) = filter_and_normalize_fitgirl(&url, &title) {
                    title = clean;
                } else {
                    return None;
                }
            }

            Some(SearchResult { site: site.name.to_string(), title, url })
        })
        .collect()
}

fn derive_title_from_href(href: &str) -> Option<String> {
    // Try last path segment
    let mut segment = href;
    if let Some(idx) = href.rfind('/') { segment = &href[idx+1..]; }
    // strip anchors/query
    if let Some(q) = segment.find(['?', '#']) { segment = &segment[..q]; }
    if segment.is_empty() { return None; }
    let decoded = decode(segment).ok()?.to_string();
    let replaced = decoded.replace('-', " ").replace('_', " ");
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
    if t.len() < 8 || t.len() > 10 { return false; }
    if t.chars().filter(|c| *c == '/').count() != 2 { return false; }
    t.chars().all(|c| c.is_ascii_digit() || c == '/')
}

fn filter_and_normalize_fitgirl(url: &str, title: &str) -> Option<String> {
    // Drop pagination and comment anchors
    if url.contains("/page/") || url.contains("#respond") || url.contains("?s=") {
        return None;
    }
    let t = title.trim();
    if t.is_empty() { return None; }
    if t.chars().all(|c| c.is_ascii_digit()) { return None; }
    if t.to_lowercase().contains("comments") { return None; }
    if looks_like_date_ddmmyyyy(t) { return None; }

    // Drop "Continue reading ..." teaser links (we keep the main post link instead)
    if t.to_lowercase().starts_with("continue reading") {
        return None;
    }
    Some(t.to_string())
}

fn parse_elamigos(site: &SiteConfig, html: &str, query: &str) -> Vec<SearchResult> {
    let document = Html::parse_document(html);
    let Ok(sel) = Selector::parse("h3, h5") else { return Vec::new() };
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
        if let Ok(a_sel) = Selector::parse("a[href]") {
            if let Some(a) = heading.select(&a_sel).next() {
                let href = a.value().attr("href").unwrap_or("");
                if href.is_empty() { continue; }
                // Build absolute URL
                let mut url = href.to_string();
                if !(href.starts_with("http://") || href.starts_with("https://") || href.starts_with("//")) {
                    let base = site.base_url.trim_end_matches('/');
                    if href.starts_with('/') {
                        url = format!("{}{}", base, href);
                    } else {
                        url = format!("{}/{}", base, href.trim_start_matches('/'));
                    }
                }
                // Title: remove trailing DOWNLOAD and trim
                let title = text_norm.replace("DOWNLOAD", "").trim().to_string();
                results.push(SearchResult { site: site.name.to_string(), title, url });
            }
        }
    }

    results
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg() -> SiteConfig {
        SiteConfig {
            name: "example",
            base_url: "https://example.com/",
            search_kind: crate::models::SearchKind::QueryParam,
            query_param: Some("s"),
            listing_path: None,
            result_selector: "h2.entry-title a", // won't match our fixture; triggers fallback
            title_attr: "text",
            url_attr: "href",
            requires_js: false,
            requires_cloudflare: false,
        }
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
        assert!(results.len() >= 3, "expected at least 3 results, got {}", results.len());
        let urls: Vec<String> = results.into_iter().map(|r| r.url).collect();
        assert!(urls.contains(&"https://example.com/post-slug/".to_string()));
        assert!(urls.contains(&"https://example.com/absolute-path/".to_string()));
        assert!(urls.contains(&"https://other.com/x".to_string()));
    }
}


