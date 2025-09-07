function Invoke-Build {
cargo build -p website-searcher --release
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
if (-not (Test-Path "scripts/ws.cmd")) { throw "Missing scripts/ws.cmd" }
New-Item -ItemType Directory -Force -Path "src-tauri/wix/bin" | Out-Null
Copy-Item -Force "target/release/website-searcher.exe" "src-tauri/wix/bin/website-searcher.exe"
Copy-Item -Force "scripts/ws.cmd" "src-tauri/wix/bin/ws.cmd"
New-Item -ItemType Directory -Force -Path "src-tauri/bin" | Out-Null
$hostTriple = (rustc -Vv | Select-String 'host:' | ForEach-Object { ($_ -split '\s+')[1] })
if (-not $hostTriple) { $hostTriple = "x86_64-pc-windows-msvc" }
Copy-Item -Force "target/release/website-searcher.exe" ("src-tauri/bin/website-searcher-" + $hostTriple + ".exe")
cargo clippy --all-targets -- -D warnings
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
cargo build -p website-searcher
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
cargo test --locked --workspace
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
cargo audit
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
cargo build --workspace --release
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
cargo tauri build
if ($LASTEXITCODE -ne 0) {
Write-Host "Tauri CLI build failed or not installed; falling back to 'cargo build -p website-searcher-gui --release'"
cargo build -p website-searcher-gui --release
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
}
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