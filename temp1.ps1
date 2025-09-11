# Build to generate main.wixobj
cargo tauri build --bundles msi

# Manually link to the same output path Tauri uses
$cfg = (Test-Path target\release\wix\x64\main.wixobj) ? 'release' : 'debug'
$wix = "$env:LOCALAPPDATA\tauri\WixTools314"
New-Item -ItemType Directory -Force "target\$cfg\bundle\msi" | Out-Null
& "$wix\light.exe" -v -ext WixUIExtension -ext WixUtilExtension `
  "target\$cfg\wix\x64\main.wixobj" `
  -out "target\$cfg\bundle\msi\website-searcher_0.1.0_x64_en-US.msi"