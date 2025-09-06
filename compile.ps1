function Invoke-Build {
    cargo clippy --all-targets -- -D warnings
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
    cargo build -p website-searcher
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
    cargo test -p website-searcher --locked
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
    try { 
        cargo test --locked --workspace --exclude website_searcher_core --exclude app 
    } catch { 
        Write-Host "Workspace tests failed (non-blocking)" 
    }
    cargo audit
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
    cargo build -p website-searcher --release
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
    cargo build -p app --release
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
}
$cargoFmtResult = cargo fmt --all -- --check
if ($LASTEXITCODE -eq 0) {
    echo "Formatting is correct"
    Invoke-Build
} else {
    echo "Formatting is incorrect, fixing..."
    cargo fmt --all
    Invoke-Build
}
exit 0