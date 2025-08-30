use crate::models::{SearchKind, SiteConfig};
use serde_urlencoded;

pub fn normalize_query(input: &str) -> String {
    input
        .trim()
        .split_whitespace()
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn build_search_url(site: &SiteConfig, query: &str) -> String {
    match site.search_kind {
        SearchKind::QueryParam => {
            let param = site.query_param.unwrap_or("s");
            let qs = serde_urlencoded::to_string(&[(param, query)])
                .unwrap_or_else(|_| format!("{}={}", param, query.replace(' ', "+")));
            format!("{}?{}", site.base_url, qs)
        }
        SearchKind::PathEncoded => {
            // Special: spaces must be %20 per PLAN.md
            let path = query.replace(' ', "%20");
            format!("{}{}", site.base_url, path)
        }
        SearchKind::FrontPage => site.base_url.to_string(),
        SearchKind::ListingPage => site.base_url.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_collapses_spaces() {
        assert_eq!(normalize_query("  hello   world  "), "hello world");
    }

    #[test]
    fn build_queryparam_s() {
        let cfg = SiteConfig {
            name: "x",
            base_url: "https://example.com/",
            search_kind: SearchKind::QueryParam,
            query_param: Some("s"),
            listing_path: None,
            result_selector: "a",
            title_attr: "text",
            url_attr: "href",
            requires_js: false,
            requires_cloudflare: false,
        };
        let url = build_search_url(&cfg, &normalize_query("elden ring"));
        assert!(url.starts_with("https://example.com/?s="));
        assert!(url.contains("elden+ring"));
    }

    #[test]
    fn build_pathencoded_spaces() {
        let cfg = SiteConfig {
            name: "x",
            base_url: "https://ankergames.net/search/",
            search_kind: SearchKind::PathEncoded,
            query_param: None,
            listing_path: None,
            result_selector: "a",
            title_attr: "text",
            url_attr: "href",
            requires_js: false,
            requires_cloudflare: false,
        };
        let url = build_search_url(&cfg, &normalize_query("elden ring"));
        assert_eq!(url, "https://ankergames.net/search/elden%20ring");
    }
}


