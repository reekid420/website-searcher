use anyhow::{Context, Result};
use reqwest::{Client, header::HeaderMap};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct FlareResponseSolution {
    response: String,
}

#[derive(Debug, Deserialize)]
struct FlareResponse {
    solution: FlareResponseSolution,
    status: String,
}

pub async fn fetch_via_solver(client: &Client, url: &str, solver_url: &str) -> Result<String> {
    // POST {cmd: request.get, url}
    let payload = serde_json::json!({
        "cmd": "request.get",
        "url": url,
        "maxTimeout": 20000
    });

    let resp = client
        .post(solver_url)
        .header("content-type", "application/json")
        .json(&payload)
        .send()
        .await
        .context("send flaresolverr request")?;

    let status = resp.status();
    if !status.is_success() {
        anyhow::bail!("flaresolverr http status {}", status);
    }

    let fr: FlareResponse = resp.json().await.context("decode flaresolverr json")?;
    Ok(fr.solution.response)
}

pub async fn fetch_via_solver_with_headers(client: &Client, url: &str, solver_url: &str, headers: Option<HeaderMap>) -> Result<String> {
    let mut payload = serde_json::json!({
        "cmd": "request.get",
        "url": url,
        "maxTimeout": 20000
    });
    if let Some(hm) = headers {
        let mut map = serde_json::Map::new();
        for (k, v) in hm.iter() {
            if let Ok(vs) = v.to_str() { map.insert(k.to_string(), serde_json::Value::String(vs.to_string())); }
        }
        payload["headers"] = serde_json::Value::Object(map);
    }

    let resp = client
        .post(solver_url)
        .header("content-type", "application/json")
        .json(&payload)
        .send()
        .await
        .context("send flaresolverr request")?;

    let status = resp.status();
    if !status.is_success() {
        anyhow::bail!("flaresolverr http status {}", status);
    }

    let fr: FlareResponse = resp.json().await.context("decode flaresolverr json")?;
    Ok(fr.solution.response)
}


