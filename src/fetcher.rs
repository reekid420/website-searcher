use std::time::Duration;

use anyhow::{Context, Result};
use reqwest::{Client, StatusCode, header::HeaderMap};
use tokio::time::sleep;

pub fn build_http_client() -> Client {
    Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/127.0.0.0 Safari/537.36 website-searcher/0.1")
        .gzip(true)
        .brotli(true)
        // leave HTTP/2 settings at defaults
        .http2_adaptive_window(true)
        .pool_idle_timeout(Duration::from_secs(30))
        .pool_max_idle_per_host(2)
        .timeout(Duration::from_secs(15))
        .build()
        .expect("failed to build reqwest client")
}

pub async fn fetch_with_retry(client: &Client, url: &str) -> Result<String> {
    let mut attempt: u32 = 0;
    let max_attempts: u32 = 3;
    let mut last_err: Option<anyhow::Error> = None;

    while attempt < max_attempts {
        let resp = client.get(url).send().await;
        match resp {
            Ok(r) => {
                if r.status() == StatusCode::OK {
                    let text = r.text().await.context("read body text")?;
                    return Ok(text);
                } else if r.status().is_redirection() || r.status() == StatusCode::FORBIDDEN {
                    // Likely protected by anti-bot; return empty rather than hard fail
                    return Ok(String::new());
                } else {
                    last_err = Some(anyhow::anyhow!("HTTP status {} for {}", r.status(), url));
                }
            }
            Err(e) => {
                last_err = Some(e.into());
            }
        }

        // Backoff 300ms * 2^attempt
        let backoff_ms = 300u64.saturating_mul(1u64 << attempt);
        sleep(Duration::from_millis(backoff_ms)).await;
        attempt += 1;
    }

    Err(last_err.unwrap_or_else(|| anyhow::anyhow!("unknown error fetching {}", url)))
}

pub async fn fetch_with_retry_headers(client: &Client, url: &str, headers: Option<HeaderMap>) -> Result<String> {
    let mut attempt: u32 = 0;
    let max_attempts: u32 = 3;
    let mut last_err: Option<anyhow::Error> = None;

    while attempt < max_attempts {
        let mut rb = client.get(url);
        if let Some(h) = headers.clone() { rb = rb.headers(h); }
        let resp = rb.send().await;
        match resp {
            Ok(r) => {
                if r.status() == StatusCode::OK {
                    let text = r.text().await.context("read body text")?;
                    return Ok(text);
                } else if r.status().is_redirection() || r.status() == StatusCode::FORBIDDEN {
                    return Ok(String::new());
                } else {
                    last_err = Some(anyhow::anyhow!("HTTP status {} for {}", r.status(), url));
                }
            }
            Err(e) => {
                last_err = Some(e.into());
            }
        }

        let backoff_ms = 300u64.saturating_mul(1u64 << attempt);
        sleep(Duration::from_millis(backoff_ms)).await;
        attempt += 1;
    }

    Err(last_err.unwrap_or_else(|| anyhow::anyhow!("unknown error fetching {}", url)))
}


