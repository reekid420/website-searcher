use assert_cmd::prelude::*;
use mockito::Server;
use std::process::Command;

#[tokio::test]
async fn cli_no_results_prints_table_message() {
    let mut server = Server::new_async().await;
    // Return empty HTML response from solver
    let _m = server
        .mock("POST", "/")
        .with_status(200)
        .with_body(r#"{"solution":{"response":"<html></html>"},"status":"ok"}"#)
        .create_async()
        .await;

    let mut cmd = Command::cargo_bin("website-searcher").expect("binary built");
    cmd.args([
        "elden ring",
        "--sites",
        "fitgirl",
        "--cf-url",
        &server.url(),
        "--format",
        "table",
    ]);
    cmd.env("NO_COLOR", "1");

    let assert = cmd.assert().success();
    let out = String::from_utf8(assert.get_output().stdout.clone()).expect("utf8");
    assert!(out.contains("No results."));
}
