use serde::{Deserialize, Serialize};

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
}

#[derive(Debug, Clone)]
pub struct SiteConfig {
    pub name: &'static str,
    pub base_url: &'static str,
    pub search_kind: SearchKind,
    pub query_param: Option<&'static str>,
    pub listing_path: Option<&'static str>,
    pub result_selector: &'static str,
    pub title_attr: &'static str,
    pub url_attr: &'static str,
    pub requires_js: bool,
    pub requires_cloudflare: bool,
}
