param(
    [switch]$CI
)

$ErrorActionPreference = 'SilentlyContinue'

function Invoke-Step([string]$name, [ScriptBlock]$block) {
    Write-Host "==> $name"
    & $block
}

function Ensure-File([string]$path, [string]$message) {
    if (-not (Test-Path $path)) { throw $message }
}

function Stage-Windows-WiX-Inputs() {
    Ensure-File "scripts/ws.cmd" "Missing scripts/ws.cmd"
    New-Item -ItemType Directory -Force -Path "src-tauri/wix/bin" | Out-Null
    Copy-Item -Force "target/release/website-searcher.exe" "src-tauri/wix/bin/website-searcher.exe"
    Copy-Item -Force "scripts/ws.cmd" "src-tauri/wix/bin/ws.cmd"
}

function Stage-Tauri-Sidecar() {
    New-Item -ItemType Directory -Force -Path "src-tauri/bin" | Out-Null
    $hostTriple = (rustc -Vv | Select-String 'host:' | ForEach-Object { ($_ -split '\s+')[1] })
    if (-not $hostTriple) { $hostTriple = "x86_64-pc-windows-msvc" }
    Copy-Item -Force "target/release/website-searcher.exe" ("src-tauri/bin/website-searcher-" + $hostTriple + ".exe") -ErrorAction SilentlyContinue
}

function Manual-Link-MSI() {
    $wixRoot = Join-Path $env:LOCALAPPDATA 'tauri/WixTools314'
    if (-not (Test-Path $wixRoot)) { Write-Warning "WiX tools not found at $wixRoot"; return }

    $cfg  = 'release'
    $obj  = "target/$cfg/wix/x64/main.wixobj"
    $wxs  = "wix/main.wxs"
    $outD = "target/$cfg/bundle/msi"
    $outF = "$outD/website-searcher_0.1.0_x64_en-US.msi"

    New-Item -ItemType Directory -Force -Path (Split-Path $obj) | Out-Null
    New-Item -ItemType Directory -Force -Path $outD | Out-Null

    $repoRoot = Get-Location
    Push-Location "src-tauri"
    try {
        Write-Host "Manual WiX compile (candle) -> $obj"
        $outDir = (Split-Path (Join-Path $repoRoot $obj) -Resolve)
        if (-not (Test-Path $outDir)) { New-Item -ItemType Directory -Force -Path $outDir | Out-Null }
        & (Join-Path $wixRoot 'candle.exe') -ext WixUIExtension -ext WixUtilExtension -arch x64 -out ("$outDir\") $wxs
        if ($LASTEXITCODE -ne 0) { Write-Warning "candle.exe failed"; return }

        Write-Host "Manual WiX link (light) -> $outF"
        & (Join-Path $wixRoot 'light.exe') -ext WixUIExtension -ext WixUtilExtension (Join-Path $repoRoot $obj) -out (Join-Path $repoRoot $outF)
        if ($LASTEXITCODE -ne 0) { Write-Warning "light.exe failed" }
    } finally {
        Pop-Location
    }
}

function Invoke-Build {
    Invoke-Step "Build CLI (release)" { cargo build -p website-searcher --release }
    if ($IsWindows) { Invoke-Step "Stage WiX inputs" { Stage-Windows-WiX-Inputs } }
    Invoke-Step "Stage Tauri sidecar" { Stage-Tauri-Sidecar }

    Invoke-Step "Clippy" { cargo clippy --all-targets -- -D warnings }
    Invoke-Step "Build CLI (debug)" { cargo build -p website-searcher }
    Invoke-Step "Tests" {
        $nextest = Get-Command cargo-nextest -ErrorAction SilentlyContinue
        if ($null -eq $nextest) {
            Write-Host "Nextest not found, falling back to cargo test"
            cargo test --locked --workspace
        } else {
            cargo nextest run --locked --workspace
        }
    }
    Invoke-Step "Audit" { try { cargo audit } catch { Write-Host "cargo audit failed, continuing..." } }
    Invoke-Step "Build workspace (release)" { cargo build --workspace --release }

    $tauriArgs = @('--bundles','msi')
    $tauriFailed = $false
    try {
        Invoke-Step "Tauri build" {
            cargo tauri build @tauriArgs
            if ($LASTEXITCODE -ne 0) {
                throw "cargo tauri build failed with exit code $LASTEXITCODE"
            }
        }
    } catch {
        $tauriFailed = $true
        Write-Warning "Tauri build failed: $($_.Exception.Message)"
    }

    if ($tauriFailed) {
        Write-Host "Falling back: manually link MSI (if wixobj present)"
        Manual-Link-MSI
    }
}

# Format, then build
$cargoFmtResult = cargo fmt --all -- --check
if ($LASTEXITCODE -eq 0) {
    Write-Host "Formatting is correct"
    Invoke-Build
} else {
    Write-Host "Formatting is incorrect, fixing..."
    cargo fmt --all
    Invoke-Build
}
exit 0