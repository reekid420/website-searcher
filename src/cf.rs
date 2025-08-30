use anyhow::{Context, Result};
use reqwest::Client;
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


