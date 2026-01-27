# Website Searcher

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/Rust-1.89+-orange.svg)](https://www.rust-lang.org/)
[![CI](https://github.com/reekid420/website-searcher/actions/workflows/ci.yml/badge.svg)](https://github.com/reekid420/website-searcher/actions/workflows/ci.yml)

Cross-platform CLI and GUI application that queries multiple game download sites in parallel, scrapes results, and displays them as pretty JSON or grouped tables.

## Features

- **Parallel Search** - Query 13+ sites simultaneously
- **Multiple Outputs** - JSON, table, or interactive TUI
- **Live Progress TUI** - Real-time per-site status during search
- **Advanced Search Operators** - Filter by site, exclude terms, exact phrases, regex
- **Multi-Query Syntax** - Search different games on different sites with `|` separator
- **Invert Site Selection** - Exclude specific sites with `--invert-sites`
- **Cloudflare Bypass** - Integrated FlareSolverr support
- **Cross-Platform** - Windows, macOS, Linux (x64 & ARM)
- **GUI & CLI** - Tauri-based desktop app or terminal tool
- **Playwright Support** - JavaScript rendering for complex sites
- **Smart Caching** - TTL-based cache with configurable expiration
- **Rate Limiting** - Exponential backoff and per-site rate controls
- **Monitoring** - Prometheus metrics and structured logging
- **Configurable Logging** - Verbose and debug log levels

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

# Search all sites EXCEPT fitgirl
websearcher "elden ring" --sites fitgirl --invert-sites

# Advanced search operators
websearcher "elden ring site:fitgirl -deluxe"
websearcher '"shadow of the erdtree" site:dodi'
websearcher "cyberpunk regex:v[0-9]+\.[0-9]+"

# Multi-query: different games on different sites
websearcher "elden ring site:fitgirl | minecraft site:csrin"

# Verbose logging
websearcher "elden ring" --verbose

# Interactive mode
websearcher

# GUI mode
ws --gui
```

### CLI Options

| Flag                   | Description                              |
| ---------------------- | ---------------------------------------- |
| `--limit N`            | Results per site (default: 10)           |
| `--sites a,b,c`        | Filter sites                             |
| `--invert-sites`       | Invert site selection (exclude listed)   |
| `--format json\|table` | Output format                            |
| `-v, --verbose`        | Enable info-level logging                |
| `--debug`              | Write HTML samples to `debug/`           |
| `--no-cf`              | Disable Cloudflare solver                |
| `--no-cache`           | Skip cache for fresh results             |
| `--json`               | Alias for `--format json`                |

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

| Document                                   | Description                 |
| ------------------------------------------ | --------------------------- |
| [ARCHITECTURE](docs/ARCHITECTURE.md)       | System design and data flow |
| [INSTALLATION](docs/INSTALLATION.md)       | Setup for all platforms     |
| [CLI](docs/CLI.md)                         | Command-line reference      |
| [GUI](docs/GUI.md)                         | Desktop application guide   |
| [ADVANCED_SEARCH](docs/ADVANCED_SEARCH.md) | Search operators guide      |
| [SITES](docs/SITES.md)                     | Supported sites details     |
| [DEVELOPMENT](docs/DEVELOPMENT.md)         | Contributing guide          |
| [DOCKER](docs/DOCKER.md)                   | Container usage             |
| [API](docs/API.md)                         | Core library reference      |
| [TESTING](docs/TESTING.md)                 | Test suite documentation    |
| [PACKAGING](docs/PACKAGING.md)             | Build and packaging         |
| [MONITORING](docs/MONITORING.md)           | Metrics and logging guide   |

## Development

```bash
# Format
cargo fmt --all

# Lint
cargo clippy --all-targets

# Test
cargo nextest --workspace

# Test with logging
python test.py --log

# Coverage
cargo llvm-cov --workspace --html
```

See [docs/DEVELOPMENT.md](docs/DEVELOPMENT.md) for setup instructions.

## Architecture

```
website-searcher/
├── crates/
│   ├── core/      # Shared library (scraping, parsing, caching, monitoring)
│   └── cli/       # CLI binary with TUI
├── src-tauri/     # Tauri backend
├── gui/           # React frontend
├── scripts/       # Playwright helpers
├── config/        # External configuration (sites.toml)
└── .cargo/        # Cargo configuration (test environment)
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
