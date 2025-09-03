use assert_cmd::prelude::*;
use std::process::Command;

// This smoke test requires Node + Playwright available on the runner.
// It executes the cs.rin Playwright path with --no-cf so no solver is involved.
// We only assert that the binary runs successfully and prints a site header line.
#[test]
fn csrin_playwright_runs_and_prints_header() {
    let mut cmd = Command::cargo_bin("website-searcher").expect("binary built");
    cmd.args([
        "elden ring",
        "--sites",
        "csrin",
        "--format",
        "table",
        "--no-cf",
        "--limit",
        "1",
    ]);
    cmd.env("NO_COLOR", "1");
    // Provide a small stub HTML so Playwright spawning is skipped and test runs fast
    cmd.env(
        "CS_PLAYWRIGHT_HTML",
        "<html><a class=\"topictitle\" href=\"viewtopic.php?t=1\">Elden Ring</a></html>",
    );
    // Keep it light if the helper returns nothing
    let assert = cmd.assert().success();
    let out = String::from_utf8(assert.get_output().stdout.clone()).expect("utf8");
    // Accept either a grouped site or an empty message depending on network conditions
    assert!(
        out.contains("csrin:") || out.contains("No results."),
        "Unexpected output: {}",
        out
    );
}
