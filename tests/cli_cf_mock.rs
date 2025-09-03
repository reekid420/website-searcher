use assert_cmd::prelude::*;
use mockito::Server;
use std::process::Command;

#[tokio::test]
async fn cli_with_cf_mock_produces_results() {
    // Mock FlareSolverr endpoint
    let mut server = Server::new_async().await;
    let _m = server
        .mock("POST", "/")
        .with_status(200)
        .with_body(r#"{"solution":{"response":"<html><h2 class=\"entry-title\"><a href=\"https://fitgirl-repacks.site/game\">Elden Ring Game Page</a></h2></html>"},"status":"ok"}"#)
        .create_async()
        .await;

    let mut cmd = Command::cargo_bin("website-searcher").expect("binary built");
    cmd.args([
        "elden ring",
        "--limit",
        "1",
        "--sites",
        "fitgirl",
        "--cf-url",
        &server.url(),
    ]);
    // Avoid colored output ambiguity
    cmd.env("NO_COLOR", "1");
    cmd.env("NO_TABLE", "1");

    let assert = cmd.assert().success();
    let out = String::from_utf8(assert.get_output().stdout.clone()).expect("utf8");
    assert!(out.contains("\"results\""));
    assert!(out.contains("\"site\": \"fitgirl\""));
}

#[tokio::test]
async fn cli_table_format_groups_by_site() {
    let mut server = Server::new_async().await;
    let _m = server
        .mock("POST", "/")
        .with_status(200)
        .with_body(r#"{"solution":{"response":"<html><h2 class=\"entry-title\"><a href=\"https://fitgirl-repacks.site/game1\">Elden Ring One</a></h2><h2 class=\"entry-title\"><a href=\"https://fitgirl-repacks.site/game2\">Elden Ring Two</a></h2></html>"},"status":"ok"}"#)
        .create_async()
        .await;

    let mut cmd = Command::cargo_bin("website-searcher").expect("binary built");
    cmd.args([
        "elden ring",
        "--limit",
        "2",
        "--sites",
        "fitgirl",
        "--cf-url",
        &server.url(),
        "--format",
        "table",
    ]);
    cmd.env("NO_COLOR", "1");
    cmd.env("NO_TABLE", "1");

    let assert = cmd.assert().success();
    let out = String::from_utf8(assert.get_output().stdout.clone()).expect("utf8");
    eprintln!("\nTABLE OUTPUT:\n{}\n", out);
    assert!(out.contains("fitgirl:"));
    assert!(out.contains("- Elden Ring One (https://fitgirl-repacks.site/game1)"));
    assert!(out.contains("- Elden Ring Two (https://fitgirl-repacks.site/game2)"));
}

#[tokio::test]
async fn cli_csrin_listing_via_solver() {
    let mut server = Server::new_async().await;
    let body = r#"{"solution":{"response":"<html>\n<a class=\"topictitle\" href=\"viewtopic.php?t=111\">Elden Ring One</a>\n<a class=\"topictitle\" href=\"viewtopic.php?t=222\">Elden Ring Two</a>\n</html>"},"status":"ok"}"#;
    let _m = server
        .mock("POST", "/")
        .with_status(200)
        .with_body(body)
        .create_async()
        .await;

    let mut cmd = Command::cargo_bin("website-searcher").expect("binary built");
    cmd.args([
        "elden ring",
        "--sites",
        "csrin",
        "--cf-url",
        &server.url(),
        "--format",
        "table",
    ]);
    cmd.env("NO_COLOR", "1");
    cmd.env("NO_TABLE", "1");

    let assert = cmd.assert().success();
    let out = String::from_utf8(assert.get_output().stdout.clone()).expect("utf8");
    assert!(out.contains("csrin:"));
    assert!(out.contains("Elden Ring One (https://cs.rin.ru/forum/viewtopic.php?t=111)"));
    assert!(out.contains("Elden Ring Two (https://cs.rin.ru/forum/viewtopic.php?t=222)"));
}
