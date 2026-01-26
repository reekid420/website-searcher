use assert_cmd::prelude::*;
use mockito::{Matcher, Server};
use std::process::Command;

#[test]
fn interactive_empty_input_errors() {
    use std::io::Write;
    use std::process::Stdio;
    // Use assert_cmd to locate the test-built binary reliably
    let mut cmd = Command::cargo_bin("website-searcher").expect("binary built");
    cmd.stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = cmd.spawn().expect("spawn interactive");
    {
        let stdin = child.stdin.as_mut().expect("stdin");
        stdin.write_all(b"\n").expect("write stdin");
    }
    let status = child.wait().expect("wait");
    assert!(!status.success());
}

#[tokio::test]
async fn multi_site_table_grouping() {
    let mut server = Server::new_async().await;
    let m_fit = server
        .mock("POST", "/")
        .match_body(Matcher::Regex("fitgirl-repacks.site".into()))
        .with_status(200)
        .with_body(r#"{"solution":{"response":"<html><h2 class=\"entry-title\"><a href=\"https://fitgirl-repacks.site/elden-one\">Elden Ring One</a></h2></html>"},"status":"ok"}"#)
        .create_async()
        .await;
    let m_dodi = server
        .mock("POST", "/")
        .match_body(Matcher::Regex("dodi-repacks.download".into()))
        .with_status(200)
        .with_body(r#"{"solution":{"response":"<html><h2 class=\"entry-title\"><a href=\"https://dodi-repacks.download/elden-two\">Elden Ring Two</a></h2></html>"},"status":"ok"}"#)
        .create_async()
        .await;

    let mut cmd = Command::cargo_bin("website-searcher").unwrap();
    cmd.args([
        "elden ring",
        "--sites",
        "fitgirl,dodi",
        "--cf-url",
        &server.url(),
        "--format",
        "table",
        "--no-cache",
    ]);
    cmd.env("NO_COLOR", "1");

    cmd.env("NO_TABLE", "1");
    let assert = cmd.assert().success();
    let out = String::from_utf8(assert.get_output().stdout.clone()).expect("utf8");
    assert!(out.contains("fitgirl:"));
    assert!(out.contains("dodi:"));
    assert!(out.contains("Elden Ring One (https://fitgirl-repacks.site/elden-one)"));
    assert!(out.contains("Elden Ring Two (https://dodi-repacks.download/elden-two)"));

    m_fit.assert();
    m_dodi.assert();
}

#[tokio::test]
async fn sites_filter_only_selected_sites_queried() {
    let mut server = Server::new_async().await;
    let m_fit = server
        .mock("POST", "/")
        .match_body(Matcher::Regex("fitgirl-repacks.site".into()))
        .with_status(200)
        .with_body(r#"{"solution":{"response":"<html></html>"},"status":"ok"}"#)
        .create_async()
        .await;
    let m_dodi = server
        .mock("POST", "/")
        .match_body(Matcher::Regex("dodi-repacks.download".into()))
        .with_status(200)
        .with_body(r#"{"solution":{"response":"<html></html>"},"status":"ok"}"#)
        .create_async()
        .await;

    let mut cmd = Command::cargo_bin("website-searcher").unwrap();
    cmd.args([
        "elden ring",
        "--sites",
        "fitgirl,dodi",
        "--cf-url",
        &server.url(),
        "--no-cache",
    ]);
    cmd.env("NO_COLOR", "1");
    cmd.assert().success();

    m_fit.assert();
    m_dodi.assert();
}

#[test]
fn unknown_sites_graceful_json_empty() {
    let mut cmd = Command::cargo_bin("website-searcher").unwrap();
    cmd.args(["elden ring", "--sites", "nosuchsite", "--format", "json", "--no-cache"]);
    cmd.env("NO_COLOR", "1");

    let assert = cmd.assert().success();
    let out = String::from_utf8(assert.get_output().stdout.clone()).expect("utf8");
    let v: serde_json::Value = serde_json::from_str(&out).expect("json");
    assert_eq!(v["count"].as_u64().unwrap_or(999), 0);
}

#[tokio::test]
async fn debug_file_written_on_empty_results() {
    let mut server = Server::new_async().await;
    let _m = server
        .mock("POST", "/")
        .match_body(Matcher::Regex("fitgirl-repacks.site".into()))
        .with_status(200)
        .with_body(r#"{"solution":{"response":"<html></html>"},"status":"ok"}"#)
        .create_async()
        .await;

    let path = "debug/fitgirl_sample.html";
    let _ = std::fs::remove_file(path);

    let mut cmd = Command::cargo_bin("website-searcher").unwrap();
    cmd.args([
        "elden ring",
        "--sites",
        "fitgirl",
        "--format",
        "table",
        "--cf-url",
        &server.url(),
        "--debug",
        "--no-cache",
    ]);
    cmd.env("NO_COLOR", "1");

    let assert = cmd.assert().success();
    let out = String::from_utf8(assert.get_output().stdout.clone()).expect("utf8");
    let err = String::from_utf8(assert.get_output().stderr.clone()).unwrap_or_default();
    assert!(out.contains("No results."));
    assert!(err.contains("wrote debug/fitgirl_sample.html"));
    assert!(std::path::Path::new(path).exists());
}

#[tokio::test]
async fn per_site_limit_across_multiple_sites_json() {
    let mut server = Server::new_async().await;
    let _fit = server
        .mock("POST", "/")
        .match_body(Matcher::Regex("fitgirl-repacks.site".into()))
        .with_status(200)
        .with_body(r#"{"solution":{"response":"<html><h2 class=\"entry-title\"><a href=\"https://fitgirl-repacks.site/elden-ring-a\">Elden Ring A</a></h2><h2 class=\"entry-title\"><a href=\"https://fitgirl-repacks.site/elden-ring-b\">Elden Ring B</a></h2></html>"},"status":"ok"}"#)
        .create_async()
        .await;
    let _dodi = server
        .mock("POST", "/")
        .match_body(Matcher::Regex("dodi-repacks.download".into()))
        .with_status(200)
        .with_body(r#"{"solution":{"response":"<html><h2 class=\"entry-title\"><a href=\"https://dodi-repacks.download/elden-ring-x\">Elden Ring X</a></h2><h2 class=\"entry-title\"><a href=\"https://dodi-repacks.download/elden-ring-y\">Elden Ring Y</a></h2></html>"},"status":"ok"}"#)
        .create_async()
        .await;

    let mut cmd = Command::cargo_bin("website-searcher").unwrap();
    cmd.args([
        "elden ring",
        "--limit",
        "1",
        "--sites",
        "fitgirl,dodi",
        "--cf-url",
        &server.url(),
        "--format",
        "json",
        "--no-cache",
    ]);
    cmd.env("NO_COLOR", "1");

    let assert = cmd.assert().success();
    let out = String::from_utf8(assert.get_output().stdout.clone()).expect("utf8");
    let v: serde_json::Value = serde_json::from_str(&out).expect("json");
    assert_eq!(v["count"].as_u64().unwrap_or(0), 2);
    let sites: Vec<String> = v["results"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["site"].as_str().unwrap().to_string())
        .collect();
    assert!(sites.iter().any(|s| s == "fitgirl"));
    assert!(sites.iter().any(|s| s == "dodi"));
}
