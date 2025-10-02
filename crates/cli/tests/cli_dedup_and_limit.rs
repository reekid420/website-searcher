use assert_cmd::prelude::*;
use mockito::Server;
use std::process::Command;

#[tokio::test]
async fn cli_dedup_and_limit_json() {
    let mut server = Server::new_async().await;
    // Duplicate same URL twice + a third unique one
    let body = r#"{"solution":{"response":"<html><h2 class=\"entry-title\"><a href=\"https://fitgirl-repacks.site/game1\">Elden Ring One</a></h2><h2 class=\"entry-title\"><a href=\"https://fitgirl-repacks.site/game1\">Elden Ring One</a></h2><h2 class=\"entry-title\"><a href=\"https://fitgirl-repacks.site/game2\">Elden Ring Two</a></h2></html>"},"status":"ok"}"#;
    let _m = server
        .mock("POST", "/")
        .with_status(200)
        .with_body(body)
        .create_async()
        .await;

    // Limit 1 => only 1 result
    let mut cmd = Command::cargo_bin("website-searcher").expect("binary built");
    cmd.args([
        "elden ring",
        "--limit",
        "1",
        "--sites",
        "fitgirl",
        "--cf-url",
        &server.url(),
        "--format",
        "json",
        "--debug",
    ]);
    cmd.env("NO_COLOR", "1");

    let assert = cmd.assert().success();
    let out = String::from_utf8(assert.get_output().stdout.clone()).expect("utf8");
    eprintln!("FIRST OUT:\n{}\n", out);
    let err = String::from_utf8(assert.get_output().stderr.clone()).unwrap_or_default();
    eprintln!("FIRST ERR:\n{}\n", err);

    let v: serde_json::Value = serde_json::from_str(&out).expect("valid json");
    assert_eq!(v["count"].as_u64().unwrap_or(0), 1);
    assert_eq!(v["results"].as_array().map(|a| a.len()).unwrap_or(0), 1);
    assert_eq!(v["results"][0]["site"].as_str().unwrap_or(""), "fitgirl");

    // With higher limit, duplicates should be removed and both unique URLs remain
    let mut cmd2 = Command::cargo_bin("website-searcher").expect("binary built");
    cmd2.args([
        "elden ring",
        "--limit",
        "5",
        "--sites",
        "fitgirl",
        "--cf-url",
        &server.url(),
        "--format",
        "json",
    ]);
    cmd2.env("NO_COLOR", "1");
    let assert2 = cmd2.assert().success();
    let out2 = String::from_utf8(assert2.get_output().stdout.clone()).expect("utf8");
    let v2: serde_json::Value = serde_json::from_str(&out2).expect("valid json");
    assert_eq!(v2["count"].as_u64().unwrap_or(0), 2);
    let urls: Vec<String> = v2["results"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["url"].as_str().unwrap().to_string())
        .collect();
    assert!(urls.contains(&"https://fitgirl-repacks.site/game1".to_string()));
    assert!(urls.contains(&"https://fitgirl-repacks.site/game2".to_string()));
}
