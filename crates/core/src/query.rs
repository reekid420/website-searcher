use crate::models::{SearchKind, SiteConfig};

pub fn normalize_query(input: &str) -> String {
    input.split_whitespace().collect::<Vec<_>>().join(" ")
}

pub fn build_search_url(site: &SiteConfig, query: &str) -> String {
    match site.search_kind {
        SearchKind::QueryParam => {
            let param = site.query_param.as_deref().unwrap_or("s");
            let qs = serde_urlencoded::to_string([(param, query)])
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
        SearchKind::PhpBBSearch => {
            // phpBB forum search: search.php?keywords=...&fid[]=10&sr=topics&sf=firstpost
            let encoded = urlencoding::encode(query);
            format!(
                "{}search.php?keywords={}&fid%5B%5D=10&sr=topics&sf=firstpost",
                site.base_url, encoded
            )
        }
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
    fn normalize_handles_empty_and_tabs() {
        assert_eq!(normalize_query("\t\t"), "");
        assert_eq!(normalize_query("a\t\tb"), "a b");
        assert_eq!(normalize_query(" a \n b \r\n c "), "a b c");
    }

    #[test]
    fn build_queryparam_s() {
        let cfg = SiteConfig {
            name: "x".to_string(),
            base_url: "https://example.com/".to_string(),
            search_kind: SearchKind::QueryParam,
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
        let url = build_search_url(&cfg, &normalize_query("elden ring"));
        assert!(url.starts_with("https://example.com/?s="));
        assert!(url.contains("elden+ring"));
    }

    #[test]
    fn build_pathencoded_spaces() {
        let cfg = SiteConfig {
            name: "x".to_string(),
            base_url: "https://ankergames.net/search/".to_string(),
            search_kind: SearchKind::PathEncoded,
            query_param: None,
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
        let url = build_search_url(&cfg, &normalize_query("elden ring"));
        assert_eq!(url, "https://ankergames.net/search/elden%20ring");
    }

    #[test]
    fn build_frontpage_returns_base() {
        let cfg = SiteConfig {
            name: "front".to_string(),
            base_url: "https://front.example/".to_string(),
            search_kind: SearchKind::FrontPage,
            query_param: None,
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
        let url = build_search_url(&cfg, &normalize_query("anything"));
        assert_eq!(url, "https://front.example/");
    }

    #[test]
    fn build_listingpage_returns_base() {
        let cfg = SiteConfig {
            name: "list".to_string(),
            base_url: "https://list.example/".to_string(),
            search_kind: SearchKind::ListingPage,
            query_param: None,
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
        let url = build_search_url(&cfg, &normalize_query("anything"));
        assert_eq!(url, "https://list.example/");
    }

    #[test]
    fn build_phpbbsearch_creates_forum_search_url() {
        let cfg = SiteConfig {
            name: "csrin".to_string(),
            base_url: "https://cs.rin.ru/forum/".to_string(),
            search_kind: SearchKind::PhpBBSearch,
            query_param: Some("keywords".to_string()),
            listing_path: Some("https://cs.rin.ru/forum/viewforum.php?f=10".to_string()),
            result_selector: "a.topictitle".to_string(),
            title_attr: "text".to_string(),
            url_attr: "href".to_string(),
            requires_js: false,
            requires_cloudflare: false,
            timeout_seconds: 30,
            retry_attempts: 3,
            rate_limit_delay_ms: 1000,
        };
        let url = build_search_url(&cfg, &normalize_query("elden ring"));
        assert!(url.starts_with("https://cs.rin.ru/forum/search.php?"));
        assert!(url.contains("keywords=elden%20ring"));
        assert!(url.contains("fid%5B%5D=10"));
        assert!(url.contains("sr=topics"));
        assert!(url.contains("sf=firstpost"));
    }
}
