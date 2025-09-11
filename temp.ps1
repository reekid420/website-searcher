$log = Join-Path $env:TEMP 'wix-light.txt'
$env:WIX_TOOLSET_TRACE_PATH = $log
$env:WIX_TOOLSET_TRACE = 'verbose'
cargo tauri build --bundles msi
Get-Content $log -Tail 120