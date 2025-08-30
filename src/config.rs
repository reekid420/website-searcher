use crate::models::{SearchKind, SiteConfig};

pub fn site_configs() -> Vec<SiteConfig> {
    vec![
        // 1. steamgg.net – WP search results entries often under h2.entry-title a
        SiteConfig {
            name: "steamgg",
            base_url: "https://steamgg.net/",
            search_kind: SearchKind::QueryParam,
            query_param: Some("s"),
            listing_path: None,
            result_selector: "h2.entry-title a, h3.entry-title a, .post-title a",
            title_attr: "text",
            url_attr: "href",
            requires_js: false,
            requires_cloudflare: false,
        },
        // 2. gog-games.to – uses ?search= query
        SiteConfig {
            name: "gog-games",
            base_url: "https://gog-games.to/",
            search_kind: SearchKind::QueryParam,
            query_param: Some("search"),
            listing_path: None,
            result_selector: "a.card, .games-list a, article a",
            title_attr: "text",
            url_attr: "href",
            requires_js: false,
            requires_cloudflare: false,
        },
        // 3. atopgames.com – WordPress-like ?s= query
        SiteConfig {
            name: "atopgames",
            base_url: "https://atopgames.com/",
            search_kind: SearchKind::QueryParam,
            query_param: Some("s"),
            listing_path: None,
            result_selector: "h2.entry-title a, h3.entry-title a, .post-box-title a",
            title_attr: "text",
            url_attr: "href",
            requires_js: false,
            requires_cloudflare: false,
        },
        // 4. elamigos.site – no search; parse front page
        SiteConfig {
            name: "elamigos",
            base_url: "https://elamigos.site/",
            search_kind: SearchKind::FrontPage,
            query_param: None,
            listing_path: None,
            result_selector: "h2.entry-title a, .card-title a, .entry-title a",
            title_attr: "text",
            url_attr: "href",
            requires_js: false,
            requires_cloudflare: false,
        },
        // 5. fitgirl-repacks.site – ?s= guarded sometimes by DDoS – still fetch HTML
        SiteConfig {
            name: "fitgirl",
            base_url: "https://fitgirl-repacks.site/",
            search_kind: SearchKind::QueryParam,
            query_param: Some("s"),
            listing_path: None,
            result_selector: "h2.entry-title a, h1.post-title a, .post-title a",
            title_attr: "text",
            url_attr: "href",
            requires_js: false,
            requires_cloudflare: true,
        },
        // 6. dodi-repacks.download – ?s= (may be Cloudflare challenged)
        SiteConfig {
            name: "dodi",
            base_url: "https://dodi-repacks.download/",
            search_kind: SearchKind::QueryParam,
            query_param: Some("s"),
            listing_path: None,
            result_selector: "h2.entry-title a, .entry-title a",
            title_attr: "text",
            url_attr: "href",
            requires_js: false,
            requires_cloudflare: true,
        },
        // 7. skidrowrepacks.com – ?s=
        SiteConfig {
            name: "skidrowrepacks",
            base_url: "https://skidrowrepacks.com/",
            search_kind: SearchKind::QueryParam,
            query_param: Some("s"),
            listing_path: None,
            result_selector: "h2.entry-title a, h1.entry-title a, .entry-title a, .entry-title > a, article h2 a",
            title_attr: "text",
            url_attr: "href",
            requires_js: false,
            requires_cloudflare: false,
        },
        // 8. steamrip.com – ?s=
        SiteConfig {
            name: "steamrip",
            base_url: "https://steamrip.com/",
            search_kind: SearchKind::QueryParam,
            query_param: Some("s"),
            listing_path: None,
            result_selector: "h2.entry-title a, h3.entry-title a, .post-title a, article h2 a",
            title_attr: "text",
            url_attr: "href",
            requires_js: false,
            requires_cloudflare: false,
        },
        // 9. reloadedsteam.com – ?s=
        SiteConfig {
            name: "reloadedsteam",
            base_url: "https://reloadedsteam.com/",
            search_kind: SearchKind::QueryParam,
            query_param: Some("s"),
            listing_path: None,
            result_selector: "h2.entry-title a, .post-title a, article h2 a",
            title_attr: "text",
            url_attr: "href",
            requires_js: false,
            requires_cloudflare: false,
        },
        // 10. ankergames.net – path-encoded with %20 spaces; try listing-page when search fails
        SiteConfig {
            name: "ankergames",
            base_url: "https://ankergames.net/search/",
            search_kind: SearchKind::ListingPage,
            query_param: None,
            listing_path: Some("https://ankergames.net/games-list"),
            result_selector: "div a[href^='/game/'], a.game-card, h2 a, h3 a",
            title_attr: "text",
            url_attr: "href",
            requires_js: false,
            requires_cloudflare: false,
        },
    ]
}


