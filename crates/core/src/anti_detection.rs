//! Anti-detection module for evading bot detection mechanisms.
//!
//! This module provides user agent rotation, proxy support, and header
//! randomization to help avoid detection when scraping websites.

use rand::Rng;
use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicUsize, Ordering};

/// Modern browser user agents for rotation
static USER_AGENTS: &[&str] = &[
    // Chrome on Windows
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/119.0.0.0 Safari/537.36",
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/121.0.0.0 Safari/537.36",
    // Chrome on macOS
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/119.0.0.0 Safari/537.36",
    // Firefox on Windows
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:121.0) Gecko/20100101 Firefox/121.0",
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:120.0) Gecko/20100101 Firefox/120.0",
    // Firefox on macOS
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10.15; rv:121.0) Gecko/20100101 Firefox/121.0",
    // Edge on Windows
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36 Edg/120.0.0.0",
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/119.0.0.0 Safari/537.36 Edg/119.0.0.0",
    // Safari on macOS
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.2 Safari/605.1.15",
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.1 Safari/605.1.15",
    // Chrome on Linux
    "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
    "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/119.0.0.0 Safari/537.36",
];

/// Accept-Language headers for randomization
static ACCEPT_LANGUAGES: &[&str] = &[
    "en-US,en;q=0.9",
    "en-GB,en;q=0.9",
    "en-US,en;q=0.9,es;q=0.8",
    "en-US,en;q=0.9,de;q=0.8",
    "en-US,en;q=0.9,fr;q=0.8",
    "en,en-US;q=0.9,en-GB;q=0.8",
];

/// Referer patterns for header randomization
static REFERERS: &[&str] = &[
    "https://www.google.com/",
    "https://www.bing.com/",
    "https://duckduckgo.com/",
    "https://www.google.co.uk/",
    "https://search.yahoo.com/",
];

/// Proxy type for configuration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProxyType {
    Http,
    Https,
    Socks5,
}

impl Default for ProxyType {
    fn default() -> Self {
        Self::Socks5
    }
}

impl std::fmt::Display for ProxyType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProxyType::Http => write!(f, "http"),
            ProxyType::Https => write!(f, "https"),
            ProxyType::Socks5 => write!(f, "socks5"),
        }
    }
}

/// Proxy configuration for requests
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyConfig {
    /// Proxy server URL (host:port)
    pub url: String,
    /// Type of proxy (HTTP, HTTPS, SOCKS5)
    pub proxy_type: ProxyType,
    /// Optional authentication (username, password)
    pub auth: Option<(String, String)>,
}

impl ProxyConfig {
    /// Create a new proxy config
    pub fn new(url: String, proxy_type: ProxyType) -> Self {
        Self {
            url,
            proxy_type,
            auth: None,
        }
    }

    /// Create a proxy config with authentication
    pub fn with_auth(
        url: String,
        proxy_type: ProxyType,
        username: String,
        password: String,
    ) -> Self {
        Self {
            url,
            proxy_type,
            auth: Some((username, password)),
        }
    }

    /// Parse a proxy URL string (e.g., "socks5://user:pass@host:port")
    pub fn parse(url: &str) -> Option<Self> {
        let url = url.trim();

        // Determine proxy type from scheme
        let (proxy_type, rest) = if url.starts_with("socks5://") {
            (ProxyType::Socks5, &url[9..])
        } else if url.starts_with("https://") {
            (ProxyType::Https, &url[8..])
        } else if url.starts_with("http://") {
            (ProxyType::Http, &url[7..])
        } else {
            // Default to SOCKS5 if no scheme
            (ProxyType::Socks5, url)
        };

        // Check for auth
        if let Some(at_pos) = rest.find('@') {
            let auth_part = &rest[..at_pos];
            let host_part = &rest[at_pos + 1..];

            if let Some(colon_pos) = auth_part.find(':') {
                let username = auth_part[..colon_pos].to_string();
                let password = auth_part[colon_pos + 1..].to_string();
                Some(Self::with_auth(
                    host_part.to_string(),
                    proxy_type,
                    username,
                    password,
                ))
            } else {
                None
            }
        } else {
            Some(Self::new(rest.to_string(), proxy_type))
        }
    }

    /// Convert to a full URL with scheme
    pub fn to_url(&self) -> String {
        let scheme = self.proxy_type.to_string();
        if let Some((user, pass)) = &self.auth {
            format!("{}://{}:{}@{}", scheme, user, pass, self.url)
        } else {
            format!("{}://{}", scheme, self.url)
        }
    }
}

/// Anti-detection configuration
#[derive(Debug, Default)]
pub struct AntiDetectionConfig {
    /// Enable user agent rotation
    pub rotate_user_agent: bool,
    /// Enable header randomization
    pub randomize_headers: bool,
    /// Optional proxy configuration
    pub proxy: Option<ProxyConfig>,
    /// Index for round-robin UA selection
    ua_index: AtomicUsize,
}

impl AntiDetectionConfig {
    /// Create a new anti-detection config with defaults
    pub fn new() -> Self {
        Self::default()
    }

    /// Enable user agent rotation
    pub fn with_ua_rotation(mut self) -> Self {
        self.rotate_user_agent = true;
        self
    }

    /// Enable header randomization
    pub fn with_header_randomization(mut self) -> Self {
        self.randomize_headers = true;
        self
    }

    /// Set proxy configuration
    pub fn with_proxy(mut self, proxy: ProxyConfig) -> Self {
        self.proxy = Some(proxy);
        self
    }

    /// Get the next user agent (round-robin selection)
    pub fn get_user_agent(&self) -> &'static str {
        if self.rotate_user_agent {
            let index = self.ua_index.fetch_add(1, Ordering::Relaxed) % USER_AGENTS.len();
            USER_AGENTS[index]
        } else {
            // Default user agent
            USER_AGENTS[0]
        }
    }

    /// Get a random user agent
    pub fn random_user_agent(&self) -> &'static str {
        if self.rotate_user_agent {
            let mut rng = rand::thread_rng();
            USER_AGENTS
                .choose(&mut rng)
                .copied()
                .unwrap_or(USER_AGENTS[0])
        } else {
            USER_AGENTS[0]
        }
    }

    /// Get randomized Accept-Language header
    pub fn get_accept_language(&self) -> &'static str {
        if self.randomize_headers {
            let mut rng = rand::thread_rng();
            ACCEPT_LANGUAGES
                .choose(&mut rng)
                .copied()
                .unwrap_or(ACCEPT_LANGUAGES[0])
        } else {
            ACCEPT_LANGUAGES[0]
        }
    }

    /// Get a randomized referer header
    pub fn get_referer(&self) -> &'static str {
        if self.randomize_headers {
            let mut rng = rand::thread_rng();
            REFERERS.choose(&mut rng).copied().unwrap_or(REFERERS[0])
        } else {
            REFERERS[0]
        }
    }

    /// Generate randomized headers for a request
    pub fn generate_headers(&self) -> Vec<(&'static str, String)> {
        let mut headers = Vec::new();

        if self.randomize_headers {
            headers.push(("Accept-Language", self.get_accept_language().to_string()));
            headers.push((
                "Accept",
                "text/html,application/xhtml+xml,application/xml;q=0.9,image/webp,*/*;q=0.8"
                    .to_string(),
            ));
            headers.push(("Accept-Encoding", "gzip, deflate, br".to_string()));

            // Random referer (50% chance)
            let mut rng = rand::thread_rng();
            if rng.gen_bool(0.5) {
                headers.push(("Referer", self.get_referer().to_string()));
            }

            // DNT header (random)
            if rng.gen_bool(0.3) {
                headers.push(("DNT", "1".to_string()));
            }

            // Upgrade-Insecure-Requests
            headers.push(("Upgrade-Insecure-Requests", "1".to_string()));

            // Sec-Fetch headers (modern browsers)
            headers.push(("Sec-Fetch-Dest", "document".to_string()));
            headers.push(("Sec-Fetch-Mode", "navigate".to_string()));
            headers.push(("Sec-Fetch-Site", "none".to_string()));
            headers.push(("Sec-Fetch-User", "?1".to_string()));
        }

        headers
    }
}

/// Get the default user agent
pub fn default_user_agent() -> &'static str {
    USER_AGENTS[0]
}

/// Get all available user agents
pub fn all_user_agents() -> &'static [&'static str] {
    USER_AGENTS
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_agent_rotation() {
        let config = AntiDetectionConfig::new().with_ua_rotation();

        let ua1 = config.get_user_agent();
        let ua2 = config.get_user_agent();
        let ua3 = config.get_user_agent();

        // Should cycle through different user agents
        assert!(USER_AGENTS.contains(&ua1));
        assert!(USER_AGENTS.contains(&ua2));
        assert!(USER_AGENTS.contains(&ua3));

        // With rotation, sequential calls should give different UAs
        assert_ne!(ua1, ua2);
        assert_ne!(ua2, ua3);
    }

    #[test]
    fn test_user_agent_no_rotation() {
        let config = AntiDetectionConfig::new();

        let ua1 = config.get_user_agent();
        let ua2 = config.get_user_agent();

        // Without rotation, should always return the same UA
        assert_eq!(ua1, ua2);
        assert_eq!(ua1, USER_AGENTS[0]);
    }

    #[test]
    fn test_random_user_agent() {
        let config = AntiDetectionConfig::new().with_ua_rotation();

        // Get multiple random UAs
        let uas: Vec<_> = (0..20).map(|_| config.random_user_agent()).collect();

        // All should be valid
        for ua in &uas {
            assert!(USER_AGENTS.contains(ua));
        }
    }

    #[test]
    fn test_proxy_config_parse_socks5() {
        let config = ProxyConfig::parse("socks5://127.0.0.1:1080").unwrap();
        assert_eq!(config.proxy_type, ProxyType::Socks5);
        assert_eq!(config.url, "127.0.0.1:1080");
        assert!(config.auth.is_none());
    }

    #[test]
    fn test_proxy_config_parse_http() {
        let config = ProxyConfig::parse("http://proxy.example.com:8080").unwrap();
        assert_eq!(config.proxy_type, ProxyType::Http);
        assert_eq!(config.url, "proxy.example.com:8080");
    }

    #[test]
    fn test_proxy_config_parse_with_auth() {
        let config = ProxyConfig::parse("socks5://user:pass@127.0.0.1:1080").unwrap();
        assert_eq!(config.proxy_type, ProxyType::Socks5);
        assert_eq!(config.url, "127.0.0.1:1080");
        assert_eq!(config.auth, Some(("user".to_string(), "pass".to_string())));
    }

    #[test]
    fn test_proxy_config_to_url() {
        let config = ProxyConfig::new("127.0.0.1:1080".to_string(), ProxyType::Socks5);
        assert_eq!(config.to_url(), "socks5://127.0.0.1:1080");

        let config_auth = ProxyConfig::with_auth(
            "127.0.0.1:1080".to_string(),
            ProxyType::Http,
            "user".to_string(),
            "pass".to_string(),
        );
        assert_eq!(config_auth.to_url(), "http://user:pass@127.0.0.1:1080");
    }

    #[test]
    fn test_header_randomization() {
        let config = AntiDetectionConfig::new().with_header_randomization();

        let headers = config.generate_headers();

        // Should have multiple headers
        assert!(!headers.is_empty());

        // Should include Accept-Language
        assert!(headers.iter().any(|(k, _)| *k == "Accept-Language"));

        // Should include Sec-Fetch headers
        assert!(headers.iter().any(|(k, _)| *k == "Sec-Fetch-Dest"));
    }

    #[test]
    fn test_accept_language_randomization() {
        let config = AntiDetectionConfig::new().with_header_randomization();

        let lang = config.get_accept_language();
        assert!(ACCEPT_LANGUAGES.contains(&lang));
    }

    #[test]
    fn test_referer_randomization() {
        let config = AntiDetectionConfig::new().with_header_randomization();

        let referer = config.get_referer();
        assert!(REFERERS.contains(&referer));
    }

    #[test]
    fn test_default_user_agent() {
        let ua = default_user_agent();
        assert!(!ua.is_empty());
        assert!(ua.contains("Mozilla"));
    }

    #[test]
    fn test_all_user_agents() {
        let all = all_user_agents();
        assert!(all.len() >= 10);

        for ua in all {
            assert!(ua.contains("Mozilla"));
        }
    }

    #[test]
    fn test_config_builder_pattern() {
        let proxy = ProxyConfig::parse("socks5://127.0.0.1:1080").unwrap();

        let config = AntiDetectionConfig::new()
            .with_ua_rotation()
            .with_header_randomization()
            .with_proxy(proxy);

        assert!(config.rotate_user_agent);
        assert!(config.randomize_headers);
        assert!(config.proxy.is_some());
    }

    #[test]
    fn test_proxy_parse_no_scheme() {
        let config = ProxyConfig::parse("127.0.0.1:1080").unwrap();
        assert_eq!(config.proxy_type, ProxyType::Socks5); // Default
        assert_eq!(config.url, "127.0.0.1:1080");
    }
}
