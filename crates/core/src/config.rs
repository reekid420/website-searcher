use crate::models::{SiteConfig, SitesConfig};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Configuration manager that handles loading and hot-reloading of site configurations
pub struct ConfigManager {
    config_path: PathBuf,
    sites: Arc<RwLock<Vec<SiteConfig>>>,
}

impl ConfigManager {
    /// Create a new configuration manager
    pub fn new(config_path: PathBuf) -> anyhow::Result<Self> {
        let sites = Arc::new(RwLock::new(Vec::new()));

        // Load initial configuration
        let initial_sites = Self::load_sites(&config_path)?;
        *sites.blocking_write() = initial_sites;

        Ok(Self { config_path, sites })
    }

    /// Load site configurations from file
    fn load_sites(path: &PathBuf) -> anyhow::Result<Vec<SiteConfig>> {
        // If the config file doesn't exist, fall back to hardcoded configs
        if !path.exists() {
            tracing::warn!("Configuration file not found at {:?}, using defaults", path);
            return Ok(hardcoded_site_configs());
        }

        let sites_config = SitesConfig::load_from_file(path)?;
        let mut sites = sites_config.get_site_configs();

        // Apply global defaults where needed
        if let Some(global) = sites_config.global {
            for site in &mut sites {
                if site.timeout_seconds == 0 {
                    site.timeout_seconds = global.default_timeout_seconds;
                }
                if site.retry_attempts == 0 {
                    site.retry_attempts = global.default_retry_attempts;
                }
                if site.rate_limit_delay_ms == 0 {
                    site.rate_limit_delay_ms = global.default_rate_limit_delay_ms;
                }
            }
        }

        // Validate configurations
        validate_sites(&sites)?;

        Ok(sites)
    }

    /// Get all site configurations
    pub async fn get_sites(&self) -> Vec<SiteConfig> {
        self.sites.read().await.clone()
    }

    /// Get a specific site configuration by name
    pub async fn get_site(&self, name: &str) -> Option<SiteConfig> {
        self.sites
            .read()
            .await
            .iter()
            .find(|s| s.name == name)
            .cloned()
    }

    /// Reload configuration from file
    pub async fn reload(&self) -> anyhow::Result<()> {
        let new_sites = Self::load_sites(&self.config_path)?;
        *self.sites.write().await = new_sites;
        Ok(())
    }
}

/// Validate site configurations
fn validate_sites(sites: &[SiteConfig]) -> anyhow::Result<()> {
    for site in sites {
        if site.name.is_empty() {
            anyhow::bail!("Site name cannot be empty");
        }
        if site.base_url.is_empty() {
            anyhow::bail!("Base URL for site '{}' cannot be empty", site.name);
        }
        if site.result_selector.is_empty() {
            anyhow::bail!("Result selector for site '{}' cannot be empty", site.name);
        }
        if site.timeout_seconds == 0 {
            anyhow::bail!("Timeout for site '{}' must be greater than 0", site.name);
        }
    }
    Ok(())
}

/// Hardcoded fallback site configurations (original implementation)
fn hardcoded_site_configs() -> Vec<SiteConfig> {
    vec![
        // 1. steamgg.net
        SiteConfig {
            name: "steamgg".to_string(),
            base_url: "https://steamgg.net/".to_string(),
            search_kind: crate::models::SearchKind::QueryParam,
            query_param: Some("s".to_string()),
            listing_path: None,
            result_selector: "h2.entry-title a, h3.entry-title a, .post-title a".to_string(),
            title_attr: "text".to_string(),
            url_attr: "href".to_string(),
            requires_js: false,
            requires_cloudflare: false,
            timeout_seconds: 30,
            retry_attempts: 3,
            rate_limit_delay_ms: 1000,
        },
        // 2. gog-games.to
        SiteConfig {
            name: "gog-games".to_string(),
            base_url: "https://gog-games.to/".to_string(),
            search_kind: crate::models::SearchKind::QueryParam,
            query_param: Some("search".to_string()),
            listing_path: None,
            result_selector: "a.card, .games-list a, article a".to_string(),
            title_attr: "text".to_string(),
            url_attr: "href".to_string(),
            requires_js: false,
            requires_cloudflare: false,
            timeout_seconds: 30,
            retry_attempts: 3,
            rate_limit_delay_ms: 1000,
        },
        // 3. atopgames.com
        SiteConfig {
            name: "atopgames".to_string(),
            base_url: "https://atopgames.com/".to_string(),
            search_kind: crate::models::SearchKind::QueryParam,
            query_param: Some("s".to_string()),
            listing_path: None,
            result_selector: "h2.entry-title a, h3.entry-title a, .post-box-title a".to_string(),
            title_attr: "text".to_string(),
            url_attr: "href".to_string(),
            requires_js: false,
            requires_cloudflare: false,
            timeout_seconds: 30,
            retry_attempts: 3,
            rate_limit_delay_ms: 1000,
        },
        // 4. elamigos.site
        SiteConfig {
            name: "elamigos".to_string(),
            base_url: "https://elamigos.site/".to_string(),
            search_kind: crate::models::SearchKind::FrontPage,
            query_param: None,
            listing_path: None,
            result_selector: "h2.entry-title a, .card-title a, .entry-title a".to_string(),
            title_attr: "text".to_string(),
            url_attr: "href".to_string(),
            requires_js: false,
            requires_cloudflare: false,
            timeout_seconds: 30,
            retry_attempts: 3,
            rate_limit_delay_ms: 1000,
        },
        // 5. fitgirl-repacks.site
        SiteConfig {
            name: "fitgirl".to_string(),
            base_url: "https://fitgirl-repacks.site/".to_string(),
            search_kind: crate::models::SearchKind::QueryParam,
            query_param: Some("s".to_string()),
            listing_path: None,
            result_selector: "h2.entry-title a, h1.post-title a, .post-title a".to_string(),
            title_attr: "text".to_string(),
            url_attr: "href".to_string(),
            requires_js: false,
            requires_cloudflare: true,
            timeout_seconds: 30,
            retry_attempts: 3,
            rate_limit_delay_ms: 1000,
        },
        // 6. dodi-repacks.download
        SiteConfig {
            name: "dodi".to_string(),
            base_url: "https://dodi-repacks.download/".to_string(),
            search_kind: crate::models::SearchKind::QueryParam,
            query_param: Some("s".to_string()),
            listing_path: None,
            result_selector: "h2.entry-title a, .entry-title a".to_string(),
            title_attr: "text".to_string(),
            url_attr: "href".to_string(),
            requires_js: false,
            requires_cloudflare: true,
            timeout_seconds: 30,
            retry_attempts: 3,
            rate_limit_delay_ms: 1000,
        },
        // 7. skidrowrepacks.com
        SiteConfig {
            name: "skidrowrepacks".to_string(),
            base_url: "https://skidrowrepacks.com/".to_string(),
            search_kind: crate::models::SearchKind::QueryParam,
            query_param: Some("s".to_string()),
            listing_path: None,
            result_selector:
                "h2.entry-title a, h1.entry-title a, .entry-title a, .entry-title > a, article h2 a"
                    .to_string(),
            title_attr: "text".to_string(),
            url_attr: "href".to_string(),
            requires_js: false,
            requires_cloudflare: false,
            timeout_seconds: 30,
            retry_attempts: 3,
            rate_limit_delay_ms: 1000,
        },
        // 8. steamrip.com
        SiteConfig {
            name: "steamrip".to_string(),
            base_url: "https://steamrip.com/".to_string(),
            search_kind: crate::models::SearchKind::QueryParam,
            query_param: Some("s".to_string()),
            listing_path: None,
            result_selector: "h2.entry-title a, h3.entry-title a, .post-title a, article h2 a"
                .to_string(),
            title_attr: "text".to_string(),
            url_attr: "href".to_string(),
            requires_js: false,
            requires_cloudflare: false,
            timeout_seconds: 30,
            retry_attempts: 3,
            rate_limit_delay_ms: 1000,
        },
        // 9. reloadedsteam.com
        SiteConfig {
            name: "reloadedsteam".to_string(),
            base_url: "https://reloadedsteam.com/".to_string(),
            search_kind: crate::models::SearchKind::QueryParam,
            query_param: Some("s".to_string()),
            listing_path: None,
            result_selector: "h2.entry-title a, .post-title a, article h2 a".to_string(),
            title_attr: "text".to_string(),
            url_attr: "href".to_string(),
            requires_js: false,
            requires_cloudflare: false,
            timeout_seconds: 30,
            retry_attempts: 3,
            rate_limit_delay_ms: 1000,
        },
        // 10. ankergames.net
        SiteConfig {
            name: "ankergames".to_string(),
            base_url: "https://ankergames.net/search/".to_string(),
            search_kind: crate::models::SearchKind::PathEncoded,
            query_param: None,
            listing_path: Some("https://ankergames.net/games-list".to_string()),
            result_selector: "div a[href^='/game/'], a.game-card, h2 a, h3 a".to_string(),
            title_attr: "text".to_string(),
            url_attr: "href".to_string(),
            requires_js: false,
            requires_cloudflare: false,
            timeout_seconds: 30,
            retry_attempts: 3,
            rate_limit_delay_ms: 1000,
        },
        // 11. cs.rin.ru forum
        SiteConfig {
            name: "csrin".to_string(),
            base_url: "https://cs.rin.ru/forum/".to_string(),
            search_kind: crate::models::SearchKind::PhpBBSearch,
            query_param: Some("keywords".to_string()),
            listing_path: Some("https://cs.rin.ru/forum/viewforum.php?f=10".to_string()),
            result_selector: "a.topictitle, a[href^='viewtopic.php']".to_string(),
            title_attr: "text".to_string(),
            url_attr: "href".to_string(),
            requires_js: false,
            requires_cloudflare: cfg!(test),
            timeout_seconds: 30,
            retry_attempts: 3,
            rate_limit_delay_ms: 1000,
        },
        // 12. nswpedia.com
        SiteConfig {
            name: "nswpedia".to_string(),
            base_url: "https://nswpedia.com/".to_string(),
            search_kind: crate::models::SearchKind::QueryParam,
            query_param: Some("s".to_string()),
            listing_path: None,
            result_selector: "h2 a, article h2 a, .post-title a".to_string(),
            title_attr: "text".to_string(),
            url_attr: "href".to_string(),
            requires_js: false,
            requires_cloudflare: false,
            timeout_seconds: 30,
            retry_attempts: 3,
            rate_limit_delay_ms: 1000,
        },
        // 13. f95zone.to
        SiteConfig {
            name: "f95zone".to_string(),
            base_url: "https://f95zone.to/".to_string(),
            search_kind: crate::models::SearchKind::ListingPage,
            query_param: None,
            listing_path: Some("https://f95zone.to/forums/games.2/".to_string()),
            result_selector: "a[href*='/threads/']".to_string(),
            title_attr: "text".to_string(),
            url_attr: "href".to_string(),
            requires_js: false,
            requires_cloudflare: false,
            timeout_seconds: 30,
            retry_attempts: 3,
            rate_limit_delay_ms: 1000,
        },
    ]
}

/// Get the default configuration file path
pub fn default_config_path() -> PathBuf {
    if let Ok(config_dir) = std::env::var("WEBSITE_SEARCHER_CONFIG_DIR") {
        PathBuf::from(config_dir).join("sites.toml")
    } else {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("website-searcher")
            .join("sites.toml")
    }
}

/// Get the local configuration file path (for development)
pub fn local_config_path() -> PathBuf {
    PathBuf::from("config").join("sites.toml")
}

/// Legacy function for backward compatibility
pub fn site_configs() -> Vec<SiteConfig> {
    hardcoded_site_configs()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_load_config_from_file() {
        let dir = tempdir().unwrap();
        let config_path = dir.path().join("test_sites.toml");

        let config_content = r#"
[global]
default_timeout_seconds = 60
default_retry_attempts = 5
default_rate_limit_delay_ms = 2000

[sites.test-site]
name = "test-site"
base_url = "https://example.com/"
search_kind = "QueryParam"
query_param = "q"
result_selector = "a.result"
title_attr = "text"
url_attr = "href"
requires_js = false
requires_cloudflare = false
timeout_seconds = 60
retry_attempts = 5
rate_limit_delay_ms = 2000
"#;

        std::fs::write(&config_path, config_content).unwrap();

        let sites = ConfigManager::load_sites(&config_path).unwrap();
        assert_eq!(sites.len(), 1);
        assert_eq!(sites[0].name, "test-site");
        assert_eq!(sites[0].timeout_seconds, 60);
        assert_eq!(sites[0].retry_attempts, 5);
        assert_eq!(sites[0].rate_limit_delay_ms, 2000);
    }

    #[test]
    fn test_hardcoded_fallback() {
        let non_existent_path = PathBuf::from("/non/existent/path.toml");
        let sites = ConfigManager::load_sites(&non_existent_path).unwrap();
        assert!(!sites.is_empty());
        assert!(sites.iter().any(|s| s.name == "fitgirl"));
    }
}
