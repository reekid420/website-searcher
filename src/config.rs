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
            search_kind: SearchKind::PathEncoded,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fitgirl_and_dodi_require_cloudflare() {
        let cfgs = site_configs();
        let fitgirl = cfgs.iter().find(|c| c.name == "fitgirl").unwrap();
        let dodi = cfgs.iter().find(|c| c.name == "dodi").unwrap();
        assert!(fitgirl.requires_cloudflare);
        assert!(dodi.requires_cloudflare);
    }

    #[test]
    fn gog_games_not_cloudflare_and_has_queryparam() {
        let cfgs = site_configs();
        let gog = cfgs.iter().find(|c| c.name == "gog-games").unwrap();
        assert!(!gog.requires_cloudflare);
        assert!(matches!(
            gog.search_kind,
            crate::models::SearchKind::QueryParam
        ));
        assert_eq!(gog.query_param, Some("search"));
    }

    #[test]
    fn elamigos_is_frontpage_and_ankergames_pathencoded() {
        let cfgs = site_configs();
        let ela = cfgs.iter().find(|c| c.name == "elamigos").unwrap();
        let anker = cfgs.iter().find(|c| c.name == "ankergames").unwrap();
        assert!(matches!(
            ela.search_kind,
            crate::models::SearchKind::FrontPage
        ));
        assert!(matches!(
            anker.search_kind,
            crate::models::SearchKind::PathEncoded
        ));
        assert!(anker.listing_path.is_some());
    }
}
