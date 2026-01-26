# Packaging Guide

This guide covers building and packaging website-searcher for distribution.

## Build Script

The `compile.py` script handles the full build and packaging pipeline.

### Usage

```bash
python compile.py [OPTIONS]
```

### Options

| Flag | Description |
|------|-------------|
| `-v, --verbose` | Show full command output |
| `-l, --log` | Enable logging to timestamped file |
| `-d, --deb` | Force .deb bundle (Linux) |
| `-r, --rpm` | Force .rpm bundle (Linux) |
| `-p, --pacman` | Build Arch Linux .pkg.tar.zst |
| `--nogui` | Exclude GUI from Arch package |

### Build Pipeline

1. **Format check** - Verify/fix code formatting
2. **Build CLI** - Release binary
3. **Stage sidecar** - Copy CLI to Tauri bin/
4. **Stage WiX inputs** - Windows MSI assets
5. **Clippy** - Lint checks
6. **Debug build** - Development binary
7. **Tests** - Full test suite
8. **Audit** - Security check
9. **Workspace build** - All crates release
10. **Tauri build** - Platform bundles
11. **Arch package** - If applicable

## Platform Packages

### Windows MSI

```bash
python compile.py
# Output: target/release/bundle/msi/website-searcher_0.1.0_x64_en-US.msi
```

**Contents:**
- `website-searcher.exe` - CLI binary
- `ws.cmd` - Alias script
- PATH entry for `INSTALLDIR\bin`

**WiX Template:** `src-tauri/wix/main.wxs`

**Manual WiX Build:**
If Tauri build fails, compile.py attempts manual WiX linking using cached WiX tools.

### Windows Portable GUI (Zipped)

```bash
python compile.py
# Output: target/release/website-searcher-gui.exe (zipped in CI)
```

**Notes:**
- GUI executable zipped for standalone distribution
- No installer required - just unzip and run
- Equivalent to Linux AppImage for Windows users

### macOS DMG

```bash
python compile.py
# Output: target/release/bundle/dmg/website-searcher_0.1.0_aarch64.dmg
```

**Contents:**
- `website-searcher.app` - Application bundle
- `ws` - Alias script in Contents/MacOS

### macOS App Bundle (Standalone GUI)

```bash
python compile.py
# Output: target/release/bundle/macos/website-searcher.app
```

**Notes:**
- Built alongside DMG automatically
- Self-contained `.app` folder
- Can be zipped for standalone distribution
- Equivalent to Linux AppImage for macOS users

### Linux AppImage

```bash
python compile.py
# Output: target/release/bundle/appimage/website-searcher_0.1.0_amd64.AppImage
```

**Notes:**
- Always built on Linux
- Self-contained, portable
- Uses `APPIMAGE_EXTRACT_AND_RUN=1` environment

### Linux DEB

```bash
python compile.py --deb
# Output: target/release/bundle/deb/website-searcher_0.1.0_amd64.deb
```

**Contents:**
- `/usr/bin/website-searcher` - CLI binary
- `/usr/bin/website-searcher-gui` - GUI binary
- `/usr/local/bin/ws` - Alias script
- postinst.sh, prerm.sh install scripts

**Auto-detection:** Built automatically on Debian/Ubuntu.

### Linux RPM

```bash
python compile.py --rpm
# Output: target/release/bundle/rpm/website-searcher-0.1.0-1.x86_64.rpm
```

**Contents:**
- Same as DEB package

**Auto-detection:** Built automatically on Fedora/RHEL/SUSE.

### Arch Linux Package

```bash
python compile.py --pacman
# Or auto-detected on Arch:
python compile.py
# Output: pkg/website-searcher-0.1.0-1-x86_64.pkg.tar.zst
```

**Contents:**
- `/usr/bin/website-searcher` - CLI binary
- `/usr/bin/websearcher` - Symlink
- `/usr/bin/ws` - Alias script
- `/usr/bin/website-searcher-gui` - GUI (unless `--nogui`)
- `/usr/share/licenses/website-searcher/LICENSE`

**PKGBUILD Generation:**
compile.py generates `pkg/PKGBUILD` automatically.

**Install:**
```bash
sudo pacman -U pkg/website-searcher-0.1.0-1-x86_64.pkg.tar.zst
```

## Sidecar Staging

Tauri bundles the CLI as an external binary (sidecar):

```bash
# Stage sidecar manually
mkdir -p src-tauri/bin
cp target/release/website-searcher src-tauri/bin/website-searcher-x86_64-pc-windows-msvc.exe
```

compile.py handles this automatically, detecting the host triple:
- Windows: `website-searcher-x86_64-pc-windows-msvc.exe`
- macOS: `website-searcher-aarch64-apple-darwin`
- Linux: `website-searcher-x86_64-unknown-linux-gnu`

## Logging

Enable logging for build diagnostics:

```bash
python compile.py --log
# Creates: compile-script-YYYYMMDD-HHMMSS.log
```

**Log rotation:** Only 3 most recent logs are kept.

## Troubleshooting

### WiX link failure (Windows)

If Tauri build fails on WiX step:
1. compile.py attempts manual linking
2. Requires WiX tools in `%LOCALAPPDATA%\tauri\WixTools314`
3. Install manually if missing: `cargo tauri build` caches them

### Missing Linux deps

```bash
# Debian/Ubuntu
sudo apt install libgtk-3-dev libwebkit2gtk-4.1-dev

# Arch
sudo pacman -S gtk3 webkit2gtk-4.1
```

### Arch package fails

```bash
# Check makepkg is available
which makepkg

# Ensure release binary exists
ls -la target/release/website-searcher
```

### GUI not included

Use `--nogui` only if intentionally excluding:
```bash
python compile.py --pacman --nogui
```

## CI/CD

GitHub Actions builds packages automatically:

| Workflow | Platforms |
|----------|-----------|
| `ci.yml` | All (test only) |
| `build-windows.yml` | MSI, Portable GUI (.zip) |
| `build-macos.yml` | DMG, App Bundle (.app.zip) |
| `build-linux.yml` | AppImage, deb, rpm |
| `release-all.yml` | All platforms |

See `.github/workflows/` for configurations.

## Version Bumping

Update version in these files:
1. `Cargo.toml` (workspace.package.version)
2. `src-tauri/tauri.conf.json` (version)
3. `package.json` (version)
4. `gui/package.json` (version)

## Signing (Optional)

### Windows Code Signing

Set environment variables for MSI signing:
```powershell
$env:TAURI_SIGNING_PRIVATE_KEY = "path/to/key.pfx"
$env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD = "password"
```

### macOS Code Signing

Set for notarization:
```bash
export APPLE_SIGNING_IDENTITY="Developer ID Application: Name (TEAMID)"
export APPLE_ID="appleid@example.com"
export APPLE_PASSWORD="app-specific-password"
```
