# Website Searcher

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/Rust-1.89+-orange.svg)](https://www.rust-lang.org/)
[![CI](https://github.com/reekid420/website-searcher/actions/workflows/ci.yml/badge.svg)](https://github.com/reekid420/website-searcher/actions/workflows/ci.yml)

Cross-platform CLI and GUI application that queries multiple game download sites in parallel, scrapes results, and displays them as pretty JSON or grouped tables.

## Features

- **Parallel Search** - Query 13+ sites simultaneously
- **Multiple Outputs** - JSON, table, or interactive TUI
- **Cloudflare Bypass** - Integrated FlareSolverr support
- **Cross-Platform** - Windows, macOS, Linux (x64 & ARM)
- **GUI & CLI** - Tauri-based desktop app or terminal tool
- **Playwright Support** - JavaScript rendering for complex sites

## Quick Start

```bash
# Install prerequisites
python quickstart.py

# Build
python compile.py

# Search
websearcher "elden ring" --format table
```

## Installation

See [docs/INSTALLATION.md](docs/INSTALLATION.md) for detailed platform-specific instructions.

### Pre-built Packages

| Platform | Package                                       |
| -------- | --------------------------------------------- |
| Windows  | `.msi` installer                              |
| macOS    | `.dmg` bundle                                 |
| Linux    | AppImage, `.deb`, `.rpm`, Arch `.pkg.tar.zst` |

## Usage

```bash
# Basic search (JSON output)
websearcher "baldur's gate 3"

# Table output with limit
websearcher "elden ring" --limit 5 --format table

# Search specific sites
websearcher "starfield" --sites fitgirl,dodi,steamrip

# Interactive mode
websearcher

# GUI mode
ws --gui
```

### CLI Options

| Flag                   | Description                    |
| ---------------------- | ------------------------------ |
| `--limit N`            | Results per site (default: 10) |
| `--sites a,b,c`        | Filter sites                   |
| `--format json\|table` | Output format                  |
| `--debug`              | Write HTML samples to `debug/` |
| `--no-cf`              | Disable Cloudflare solver      |

See [docs/CLI.md](docs/CLI.md) for complete reference.

## Supported Sites

| Site      | Type          | Notes                              |
| --------- | ------------- | ---------------------------------- |
| fitgirl   | Repacks       | Cloudflare protected               |
| dodi      | Repacks       | Cloudflare protected               |
| steamrip  | Pre-installed |                                    |
| gog-games | GOG DRM-free  |                                    |
| csrin     | Forum         | Uses Playwright                    |
| + 8 more  | Various       | See [docs/SITES.md](docs/SITES.md) |

## Docker

```bash
# Build and run
docker build -t websearcher .
docker run --rm -it websearcher "elden ring"

# With FlareSolverr (optional Cloudflare bypass)
docker compose --profile cf up -d
```

See [docs/DOCKER.md](docs/DOCKER.md) for advanced usage.

## Documentation

| Document                             | Description                 |
| ------------------------------------ | --------------------------- |
| [ARCHITECTURE](docs/ARCHITECTURE.md) | System design and data flow |
| [INSTALLATION](docs/INSTALLATION.md) | Setup for all platforms     |
| [CLI](docs/CLI.md)                   | Command-line reference      |
| [GUI](docs/GUI.md)                   | Desktop application guide   |
| [SITES](docs/SITES.md)               | Supported sites details     |
| [DEVELOPMENT](docs/DEVELOPMENT.md)   | Contributing guide          |
| [DOCKER](docs/DOCKER.md)             | Container usage             |
| [API](docs/API.md)                   | Core library reference      |
| [TESTING](docs/TESTING.md)           | Test suite documentation    |
| [PACKAGING](docs/PACKAGING.md)       | Build and packaging         |

## Development

```bash
# Format
cargo fmt --all

# Lint
cargo clippy --all-targets

# Test
cargo test --workspace

# Coverage
cargo llvm-cov --workspace --html
```

See [docs/DEVELOPMENT.md](docs/DEVELOPMENT.md) for setup instructions.

## Architecture

```
website-searcher/
├── crates/
│   ├── core/      # Shared library (scraping, parsing)
│   └── cli/       # CLI binary with TUI
├── src-tauri/     # Tauri backend
├── gui/           # React frontend
└── scripts/       # Playwright helpers
```

See [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) for detailed design.

## Contributing

1. Fork the repository
2. Create a feature branch
3. Run `cargo fmt && cargo clippy && cargo test`
4. Submit a pull request

See [docs/DEVELOPMENT.md](docs/DEVELOPMENT.md) for guidelines.

## License

MIT License - see [LICENSE](LICENSE) for details.
