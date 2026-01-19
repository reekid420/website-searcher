# Installation

## Prerequisites

| Tool | Version | Purpose |
|------|---------|---------|
| Rust | 1.89+ | Core compilation |
| Node.js | 20+ | GUI frontend, Playwright |
| pnpm | 10+ | Package management |
| Python | 3.8+ | Build scripts |

## Quick Install (All Platforms)

The quickstart script installs all prerequisites automatically:

```bash
python quickstart.py
```

This installs:
- Rust toolchain via rustup
- Node.js and pnpm
- Tauri CLI, cargo-audit, cargo-nextest, cargo-llvm-cov
- Platform-specific build tools (Linux only)

## Platform-Specific Setup

### Windows

1. **Prerequisites** (installed automatically by quickstart.py):
   - Visual Studio Build Tools with C++ workload
   - Rust via rustup

2. **Build**:
   ```powershell
   python compile.py
   ```

3. **Install from MSI**:
   ```powershell
   # After build, find MSI in:
   # target/release/bundle/msi/website-searcher_0.1.0_x64_en-US.msi
   ```

### macOS

1. **Prerequisites**:
   ```bash
   # Install Xcode command line tools
   xcode-select --install
   
   # Install Homebrew packages (optional, quickstart.py handles this)
   brew install node rust
   ```

2. **Build**:
   ```bash
   python quickstart.py
   python compile.py
   ```

3. **Install from DMG**:
   ```bash
   # After build, find DMG in:
   # target/release/bundle/dmg/
   ```

### Linux (Debian/Ubuntu)

1. **Prerequisites**:
   ```bash
   sudo apt-get update
   sudo apt-get install -y \
     build-essential pkg-config libssl-dev \
     libgtk-3-dev libwebkit2gtk-4.1-dev \
     libayatana-appindicator3-dev librsvg2-dev
   ```

2. **Build**:
   ```bash
   python quickstart.py
   python compile.py
   ```

3. **Install from .deb**:
   ```bash
   sudo dpkg -i target/release/bundle/deb/website-searcher_*.deb
   ```

### Linux (Arch/Manjaro)

1. **Prerequisites**:
   ```bash
   sudo pacman -Sy base-devel pkg-config openssl \
     gtk3 webkit2gtk-4.1 libappindicator-gtk3 librsvg
   ```

2. **Build with Arch package**:
   ```bash
   python quickstart.py
   python compile.py --pacman
   ```

3. **Install**:
   ```bash
   sudo pacman -U pkg/website-searcher-*.pkg.tar.zst
   ```

### Linux (Fedora/RHEL)

1. **Prerequisites**:
   ```bash
   sudo dnf groupinstall -y "Development Tools"
   sudo dnf install -y pkg-config openssl-devel gtk3-devel \
     webkit2gtk4.1-devel libappindicator-gtk3-devel librsvg2-devel
   ```

2. **Build**:
   ```bash
   python quickstart.py
   python compile.py --rpm
   ```

3. **Install from .rpm**:
   ```bash
   sudo rpm -i target/release/bundle/rpm/website-searcher-*.rpm
   ```

## Docker Installation

For containerized usage without local build:

```bash
# Build image
docker build -t websearcher .

# Run interactively
docker run --rm -it websearcher

# With FlareSolverr
docker compose up --build
```

See [DOCKER.md](DOCKER.md) for detailed container usage.

## Verifying Installation

```bash
# Check CLI
website-searcher --version
websearcher --version  # alias
ws --version           # short alias

# Test search
websearcher "elden ring" --limit 3
```

## Installed Files

After installation, these files are available:

| File | Location | Description |
|------|----------|-------------|
| `website-searcher` | System bin | Main CLI binary |
| `websearcher` | System bin | Symlink alias |
| `ws` | System bin | Short alias (launches GUI with `--gui`) |
| `website-searcher-gui` | System bin | GUI binary (Linux pkg only) |

## Updating

To update to a new version:

```bash
# Pull latest source
git pull

# Rebuild
python compile.py

# Reinstall package (platform-specific)
```

## Uninstalling

### Windows
Use "Add or Remove Programs" or:
```powershell
msiexec /x website-searcher_0.1.0_x64_en-US.msi
```

### Linux (deb)
```bash
sudo apt remove website-searcher
```

### Linux (Arch)
```bash
sudo pacman -R website-searcher
```

### Linux (rpm)
```bash
sudo rpm -e website-searcher
```
