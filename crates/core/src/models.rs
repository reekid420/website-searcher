use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SearchResult {
    pub site: String,
    pub title: String,
    pub url: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum SearchKind {
    QueryParam,
    FrontPage,
    PathEncoded,
    ListingPage,
    /// phpBB forum search with keywords, fid[], sr params (e.g., cs.rin.ru)
    PhpBBSearch,
}

impl From<&str> for SearchKind {
    fn from(s: &str) -> Self {
        match s {
            "QueryParam" => SearchKind::QueryParam,
            "FrontPage" => SearchKind::FrontPage,
            "PathEncoded" => SearchKind::PathEncoded,
            "ListingPage" => SearchKind::ListingPage,
            "PhpBBSearch" => SearchKind::PhpBBSearch,
            _ => SearchKind::QueryParam, // Default fallback
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SiteConfig {
    pub name: String,
    pub base_url: String,
    pub search_kind: SearchKind,
    pub query_param: Option<String>,
    pub listing_path: Option<String>,
    pub result_selector: String,
    pub title_attr: String,
    pub url_attr: String,
    pub requires_js: bool,
    pub requires_cloudflare: bool,
    pub timeout_seconds: u64,
    pub retry_attempts: u32,
    pub rate_limit_delay_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalConfig {
    pub default_timeout_seconds: u64,
    pub default_retry_attempts: u32,
    pub default_rate_limit_delay_ms: u64,
}

impl Default for GlobalConfig {
    fn default() -> Self {
        Self {
            default_timeout_seconds: 30,
            default_retry_attempts: 3,
            default_rate_limit_delay_ms: 1000,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SitesConfig {
    pub global: Option<GlobalConfig>,
    pub sites: std::collections::HashMap<String, SiteConfig>,
}

impl SitesConfig {
    pub fn load_from_file(path: &PathBuf) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: SitesConfig = toml::from_str(&content)?;
        Ok(config)
    }

    pub fn get_site_configs(&self) -> Vec<SiteConfig> {
        self.sites.values().cloned().collect()
    }
}
