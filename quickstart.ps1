param(
    [switch]$InstallCoverage
)

$ErrorActionPreference = 'Continue'

function Write-Info([string]$msg) { Write-Host "[info] $msg" -ForegroundColor Cyan }
function Write-Warn([string]$msg) { Write-Host "[warn] $msg" -ForegroundColor Yellow }
function Write-Err([string]$msg)  { Write-Host "[err ] $msg" -ForegroundColor Red }

function Test-Command([string]$name) {
    $null -ne (Get-Command $name -ErrorAction SilentlyContinue)
}

function Ensure-PathRefresh() {
    $cargoBin = Join-Path $HOME ".cargo\bin"
    if ((Test-Path $cargoBin) -and (($env:PATH -split ';') -notcontains $cargoBin)) {
        Write-Info "Adding $cargoBin to PATH for current session"
        $env:PATH = "$cargoBin;$env:PATH"
    }
}

function Install-Node {
    if (Test-Command node) { return }
    Write-Info "Installing Node.js LTS (using winget if available)"
    if (Test-Command winget) {
        try {
            winget install -e --id OpenJS.NodeJS.LTS --accept-package-agreements --accept-source-agreements
        } catch {
            Write-Warn "winget NodeJS install failed: $($_.Exception.Message)"
        }
    } else {
        Write-Warn "winget not found. Please install Node.js LTS from https://nodejs.org/ and re-run."
    }
}

function Activate-PnpmViaCorepack {
    if (-not (Test-Command node)) { return }
    if (Test-Command pnpm) { return }
    Write-Info "Activating pnpm via Corepack"
    try {
        node -v | Out-Null
        corepack enable | Out-Null
        corepack prepare pnpm@latest --activate | Out-Null
    } catch {
        Write-Warn "Corepack activation failed. Ensure Node >= 16.13 is installed."
    }
}

function Install-Rustup {
    if (Test-Command cargo) { return }
    Write-Info "Installing Rust (rustup)"
    if (Test-Command winget) {
        try {
            winget install -e --id Rustlang.Rustup --accept-package-agreements --accept-source-agreements
        } catch {
            Write-Warn "winget rustup install failed: $($_.Exception.Message)"
        }
    }
    if (-not (Test-Command cargo)) {
        try {
            $tmp = Join-Path $env:TEMP "rustup-init.exe"
            Invoke-WebRequest -Uri "https://win.rustup.rs/x86_64" -OutFile $tmp -UseBasicParsing
            & $tmp -y | Out-Null
        } catch {
            Write-Err "Failed to install rustup automatically. Install from https://rustup.rs and re-run."
        }
    }
    Ensure-PathRefresh
}

function Ensure-RustComponents {
    if (-not (Test-Command cargo)) { return }
    Write-Info "Ensuring rustfmt and clippy components"
    try { rustup component add rustfmt | Out-Null } catch { Write-Warn "rustfmt add failed" }
    try { rustup component add clippy  | Out-Null } catch { Write-Warn "clippy add failed" }
}

function Ensure-CargoTool([string]$display, [object[]]$checkCmd, [string]$installCrate, [string]$args = "") {
    $needs = $false
    try {
        if ($checkCmd -is [System.Array] -and $checkCmd.Length -gt 0) {
            $cmdName = [string]$checkCmd[0]
            $cmdArgs = @()
            if ($checkCmd.Length -gt 1) { $cmdArgs = $checkCmd[1..($checkCmd.Length - 1)] }
            if ($cmdArgs.Count -gt 0) {
                & $cmdName @cmdArgs | Out-Null
            } else {
                & $cmdName | Out-Null
            }
        } else {
            & $checkCmd | Out-Null
        }
    } catch { $needs = $true }
    if ($needs) {
        if (-not (Test-Command cargo)) {
            Write-Warn "$display requires Cargo; skipping (Cargo not found)"
            return
        }
        Write-Info "Installing $display ($installCrate)"
        try {
            if ([string]::IsNullOrWhiteSpace($args)) {
                cargo install $installCrate
            } else {
                cargo install $installCrate $args
            }
        } catch {
            Write-Warn "Failed to install ${display}: $($_.Exception.Message)"
        }
    }
}

Write-Host "==> Quickstart: installing prerequisites" -ForegroundColor Green

Install-Node
Activate-PnpmViaCorepack

Install-Rustup
Ensure-PathRefresh
Ensure-RustComponents

# Cargo tools used by scripts/build/tests
Ensure-CargoTool -display "Tauri CLI"      -checkCmd "cargo","tauri","--version"     -installCrate "tauri-cli" -args "--locked"
Ensure-CargoTool -display "cargo-audit"    -checkCmd "cargo","audit","-V"           -installCrate "cargo-audit"
Ensure-CargoTool -display "cargo-nextest"  -checkCmd "cargo","nextest","--version"   -installCrate "cargo-nextest"
if ($InstallCoverage -or $true) {
    Ensure-CargoTool -display "cargo-llvm-cov" -checkCmd "cargo","llvm-cov","--version" -installCrate "cargo-llvm-cov"
}

# Final report
Write-Host ""; Write-Host "==> Versions" -ForegroundColor Green
if (Test-Command node)  { Write-Host ("node    : " + (node -v)) }
if (Test-Command pnpm)  { Write-Host ("pnpm    : " + (pnpm -v)) }
if (Test-Command cargo) { Write-Host ("cargo   : " + (cargo -V)) }
if (Test-Command rustc) { Write-Host ("rustc   : " + (rustc -V)) }
try { cargo tauri --version | ForEach-Object { Write-Host ("tauri   : " + $_) } } catch {}
try { cargo audit -V       | ForEach-Object { Write-Host ("audit   : " + $_) } } catch {}
try { cargo nextest --version | ForEach-Object { Write-Host ("nextest : " + $_) } } catch {}
try { cargo llvm-cov --version | ForEach-Object { Write-Host ("llvm-cov: " + $_) } } catch {}

Write-Host ""; Write-Info "Quickstart complete. You can now run ./compile.ps1"


