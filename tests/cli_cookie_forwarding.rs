use assert_cmd::prelude::*;
use mockito::{Matcher, Server};
use std::process::Command;

#[tokio::test]
async fn cli_cookie_forwarded_to_solver_payload() {
    let mut server = Server::new_async().await;
    let m = server
        .mock("POST", "/")
        .match_body(Matcher::Regex("cf_clearance=abc; a=b".into()))
        .with_status(200)
        .with_body(r#"{"solution":{"response":"<html><h2 class=\"entry-title\"><a href=\"https://fitgirl-repacks.site/elden-ring\">Elden Ring</a></h2></html>"},"status":"ok"}"#)
        .create_async()
        .await;

    let mut cmd = Command::cargo_bin("website-searcher").expect("binary built");
    cmd.args([
        "elden ring",
        "--sites",
        "fitgirl",
        "--cf-url",
        &server.url(),
        "--cookie",
        "cf_clearance=abc; a=b",
        "--format",
        "json",
    ]);
    cmd.env("NO_COLOR", "1");

    let assert = cmd.assert().success();
    let out = String::from_utf8(assert.get_output().stdout.clone()).expect("utf8");
    let v: serde_json::Value = serde_json::from_str(&out).expect("json");
    assert!(v["count"].as_u64().unwrap_or(0) >= 1);
    let sites: Vec<String> = v["results"].as_array().unwrap().iter().map(|r| r["site"].as_str().unwrap().to_string()).collect();
    assert!(sites.iter().any(|s| s == "fitgirl"));
    // Ensure mock was hit with the matched cookie
    m.assert();
}
