use std::time::Duration;

use crate::monitoring::get_metrics;
use crate::rate_limiter::RateLimiter;
use anyhow::{Context, Result};
use reqwest::{Client, StatusCode, header::HeaderMap};
use tokio::time::sleep;
use tracing::{debug, error, info, instrument, warn};

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

#[instrument(skip(client, rate_limiter))]
pub async fn fetch_with_retry(
    client: &Client,
    url: &str,
    mut rate_limiter: Option<&mut RateLimiter>,
    site_name: Option<&str>,
) -> Result<String> {
    let site = site_name.unwrap_or("unknown");
    let mut attempt: u32 = 0;
    let max_attempts: u32 = 3;

    info!(site = site, url = url, "Starting fetch with retry");
    let mut last_err: Option<anyhow::Error> = None;

    while attempt < max_attempts {
        // Apply rate limiting if provided
        if let Some(limiter) = rate_limiter.as_mut()
            && let Err(e) = limiter.wait_for_site(site).await
        {
            return Err(anyhow::anyhow!("Rate limit error: {}", e));
        }

        let start_time = std::time::Instant::now();
        info!(site = site, attempt = attempt + 1, "Sending HTTP request");
        let resp = client.get(url).send().await;
        let response_time = start_time.elapsed();

        // Record metrics
        get_metrics()
            .record_request(site, response_time, resp.is_ok())
            .await;

        match resp {
            Ok(r) => {
                let status = r.status();
                info!(
                    site = site,
                    status = status.as_u16(),
                    response_time_ms = response_time.as_millis(),
                    "Received response"
                );

                match status {
                    StatusCode::OK => {
                        let body = r.text().await.context("Failed to read response body")?;
                        debug!(
                            site = site,
                            body_length = body.len(),
                            "Successfully fetched body"
                        );
                        return Ok(body);
                    }
                    StatusCode::TOO_MANY_REQUESTS => {
                        warn!(site = site, "Rate limited (429), backing off");
                        last_err = Some(anyhow::anyhow!("Rate limited: {}", status));
                        // Exponential backoff for rate limiting
                        let backoff = Duration::from_millis(1000 * (2_u64.pow(attempt)));
                        sleep(backoff).await;
                    }
                    StatusCode::FORBIDDEN | StatusCode::UNAUTHORIZED => {
                        warn!(site = site, status = status.as_u16(), "Access denied");
                        // Return empty string for access denied errors
                        return Ok(String::new());
                    }
                    StatusCode::NOT_FOUND => {
                        debug!(site = site, "Resource not found (404)");
                        // Return empty string for not found errors
                        return Ok(String::new());
                    }
                    StatusCode::SERVICE_UNAVAILABLE
                    | StatusCode::BAD_GATEWAY
                    | StatusCode::GATEWAY_TIMEOUT => {
                        warn!(
                            site = site,
                            status = status.as_u16(),
                            "Server error, will retry"
                        );
                        last_err = Some(anyhow::anyhow!("Server error: {}", status));
                        // Exponential backoff for server errors
                        let backoff = Duration::from_millis(500 * (2_u64.pow(attempt)));
                        sleep(backoff).await;
                    }
                    _ => {
                        // Handle redirection codes by returning empty string
                        if status.is_redirection() {
                            debug!(
                                site = site,
                                status = status.as_u16(),
                                "Redirection received"
                            );
                            return Ok(String::new());
                        }
                        warn!(site = site, status = status.as_u16(), "Unexpected status");
                        last_err = Some(anyhow::anyhow!("Unexpected status: {}", status));
                        // Linear backoff for other errors
                        sleep(Duration::from_millis(500)).await;
                    }
                }
            }
            Err(e) => {
                error!(site = site, error = %e, "HTTP request failed");
                last_err = Some(anyhow::anyhow!("Request failed: {}", e));
                // Exponential backoff for network errors
                let backoff = Duration::from_millis(200 * (2_u64.pow(attempt)));
                sleep(backoff).await;
            }
        }

        // Exponential backoff with jitter (handled by RateLimiter's wait_for_site)
        // But we still need a small delay for retries when rate limiter is not used
        if rate_limiter.is_none() {
            let backoff_ms = 300u64.saturating_mul(1u64 << attempt);
            sleep(Duration::from_millis(backoff_ms)).await;
        }

        attempt += 1;
    }

    Err(last_err.unwrap_or_else(|| anyhow::anyhow!("unknown error fetching {}", url)))
}

pub async fn fetch_with_retry_headers(
    client: &Client,
    url: &str,
    headers: Option<HeaderMap>,
    mut rate_limiter: Option<&mut RateLimiter>,
    site_name: Option<&str>,
) -> Result<String> {
    let site = site_name.unwrap_or("unknown");
    let mut attempt: u32 = 0;
    let max_attempts: u32 = 3;
    let mut last_err: Option<anyhow::Error> = None;

    while attempt < max_attempts {
        // Apply rate limiting if provided
        if let Some(limiter) = rate_limiter.as_mut()
            && let Err(e) = limiter.wait_for_site(site).await
        {
            return Err(anyhow::anyhow!("Rate limit error: {}", e));
        }

        let start_time = std::time::Instant::now();
        let mut rb = client.get(url);
        if let Some(h) = headers.clone() {
            rb = rb.headers(h);
        }
        let resp = rb.send().await;
        let response_time = start_time.elapsed();

        match resp {
            Ok(r) => {
                let status = r.status();
                info!(
                    site = site,
                    status = status.as_u16(),
                    response_time_ms = response_time.as_millis(),
                    "Received response"
                );

                match status {
                    StatusCode::OK => {
                        let body = r.text().await.context("Failed to read response body")?;
                        debug!(
                            site = site,
                            body_length = body.len(),
                            "Successfully fetched body"
                        );
                        return Ok(body);
                    }
                    StatusCode::TOO_MANY_REQUESTS => {
                        warn!(site = site, "Rate limited (429), backing off");
                        last_err = Some(anyhow::anyhow!("Rate limited: {}", status));
                        // Exponential backoff for rate limiting
                        let backoff = Duration::from_millis(1000 * (2_u64.pow(attempt)));
                        sleep(backoff).await;
                    }
                    StatusCode::FORBIDDEN | StatusCode::UNAUTHORIZED => {
                        warn!(site = site, status = status.as_u16(), "Access denied");
                        // Return empty string for access denied errors
                        return Ok(String::new());
                    }
                    StatusCode::NOT_FOUND => {
                        debug!(site = site, "Resource not found (404)");
                        // Return empty string for not found errors
                        return Ok(String::new());
                    }
                    StatusCode::SERVICE_UNAVAILABLE
                    | StatusCode::BAD_GATEWAY
                    | StatusCode::GATEWAY_TIMEOUT => {
                        warn!(
                            site = site,
                            status = status.as_u16(),
                            "Server error, will retry"
                        );
                        last_err = Some(anyhow::anyhow!("Server error: {}", status));
                        // Exponential backoff for server errors
                        let backoff = Duration::from_millis(500 * (2_u64.pow(attempt)));
                        sleep(backoff).await;
                    }
                    _ => {
                        // Handle redirection codes by returning empty string
                        if status.is_redirection() {
                            debug!(
                                site = site,
                                status = status.as_u16(),
                                "Redirection received"
                            );
                            return Ok(String::new());
                        }
                        warn!(site = site, status = status.as_u16(), "Unexpected status");
                        last_err = Some(anyhow::anyhow!("Unexpected status: {}", status));
                        // Linear backoff for other errors
                        sleep(Duration::from_millis(500)).await;
                    }
                }
            }
            Err(e) => {
                error!(site = site, error = %e, "HTTP request failed");
                last_err = Some(anyhow::anyhow!("Request failed: {}", e));
                // Exponential backoff for network errors
                let backoff = Duration::from_millis(200 * (2_u64.pow(attempt)));
                sleep(backoff).await;
            }
        }

        if rate_limiter.is_none() {
            let backoff_ms = 300u64.saturating_mul(1u64 << attempt);
            sleep(Duration::from_millis(backoff_ms)).await;
        }

        attempt += 1;
    }

    Err(last_err.unwrap_or_else(|| anyhow::anyhow!("unknown error fetching {}", url)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockito::Server;

    #[tokio::test]
    async fn fetch_ok_returns_body() {
        let mut server = Server::new_async().await;
        let _m = server
            .mock("GET", "/ok")
            .with_status(200)
            .with_body("hello")
            .create_async()
            .await;
        let client = build_http_client();
        let body = fetch_with_retry(&client, &format!("{}/ok", server.url()), None, Some("test"))
            .await
            .unwrap();
        assert_eq!(body, "hello");
    }

    #[tokio::test]
    async fn fetch_redirection_returns_empty() {
        let mut server = Server::new_async().await;
        let _m = server
            .mock("GET", "/redir")
            .with_status(302)
            .create_async()
            .await;
        let client = build_http_client();
        let body = fetch_with_retry(
            &client,
            &format!("{}/redir", server.url()),
            None,
            Some("test"),
        )
        .await
        .unwrap();
        assert_eq!(body, "");
    }

    #[tokio::test]
    async fn fetch_forbidden_returns_empty() {
        let mut server = Server::new_async().await;
        let _m = server
            .mock("GET", "/forbid")
            .with_status(403)
            .create_async()
            .await;
        let client = build_http_client();
        let body = fetch_with_retry(
            &client,
            &format!("{}/forbid", server.url()),
            None,
            Some("test"),
        )
        .await
        .unwrap();
        assert_eq!(body, "");
    }

    #[tokio::test]
    async fn fetch_retries_then_errors() {
        let mut server = Server::new_async().await;
        // Three failures to exhaust retries
        let _m1 = server
            .mock("GET", "/fail")
            .with_status(500)
            .create_async()
            .await;
        let _m2 = server
            .mock("GET", "/fail")
            .with_status(500)
            .create_async()
            .await;
        let _m3 = server
            .mock("GET", "/fail")
            .with_status(500)
            .create_async()
            .await;
        let client = build_http_client();
        let res = fetch_with_retry(
            &client,
            &format!("{}/fail", server.url()),
            None,
            Some("test"),
        )
        .await;
        assert!(res.is_err());
    }

    #[tokio::test]
    async fn fetch_with_headers_forwards_header() {
        use mockito::Matcher;
        let mut server = Server::new_async().await;
        let _m = server
            .mock("GET", "/hdr")
            .match_header("x-test", Matcher::Exact("1".into()))
            .with_status(200)
            .with_body("ok")
            .create_async()
            .await;
        let client = build_http_client();
        let mut hm = HeaderMap::new();
        hm.insert(
            reqwest::header::HeaderName::from_static("x-test"),
            reqwest::header::HeaderValue::from_static("1"),
        );
        let body = fetch_with_retry_headers(
            &client,
            &format!("{}/hdr", server.url()),
            Some(hm),
            None,
            Some("test"),
        )
        .await
        .unwrap();
        assert_eq!(body, "ok");
    }

    #[tokio::test]
    async fn fetch_with_headers_none_works() {
        let mut server = Server::new_async().await;
        let _m = server
            .mock("GET", "/no-hdr")
            .with_status(200)
            .with_body("no header")
            .create_async()
            .await;
        let client = build_http_client();
        let body = fetch_with_retry_headers(
            &client,
            &format!("{}/no-hdr", server.url()),
            None,
            None,
            Some("test"),
        )
        .await
        .unwrap();
        assert_eq!(body, "no header");
    }

    #[tokio::test]
    async fn fetch_with_headers_redirection_returns_empty() {
        let mut server = Server::new_async().await;
        let _m = server
            .mock("GET", "/hdr-redir")
            .with_status(302)
            .create_async()
            .await;
        let client = build_http_client();
        let body = fetch_with_retry_headers(
            &client,
            &format!("{}/hdr-redir", server.url()),
            None,
            None,
            Some("test"),
        )
        .await
        .unwrap();
        assert_eq!(body, "");
    }

    #[tokio::test]
    async fn fetch_with_headers_forbidden_returns_empty() {
        let mut server = Server::new_async().await;
        let _m = server
            .mock("GET", "/hdr-forbid")
            .with_status(403)
            .create_async()
            .await;
        let client = build_http_client();
        let body = fetch_with_retry_headers(
            &client,
            &format!("{}/hdr-forbid", server.url()),
            None,
            None,
            Some("test"),
        )
        .await
        .unwrap();
        assert_eq!(body, "");
    }

    #[tokio::test]
    async fn fetch_with_headers_retries_then_errors() {
        let mut server = Server::new_async().await;
        let _m1 = server
            .mock("GET", "/hdr-fail")
            .with_status(500)
            .create_async()
            .await;
        let _m2 = server
            .mock("GET", "/hdr-fail")
            .with_status(500)
            .create_async()
            .await;
        let _m3 = server
            .mock("GET", "/hdr-fail")
            .with_status(500)
            .create_async()
            .await;
        let client = build_http_client();
        let res = fetch_with_retry_headers(
            &client,
            &format!("{}/hdr-fail", server.url()),
            None,
            None,
            Some("test"),
        )
        .await;
        assert!(res.is_err());
    }
}
