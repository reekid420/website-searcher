## Website Searcher

Cross-platform CLI that queries multiple game-download sites in parallel, scrapes results, and prints pretty JSON or a grouped table.

### Build

1. Install Rust via `https://rustup.rs`.
2. Clone this repo, then run:
   - `cargo build --release`

### Usage

```powershell
# Basic
websearcher "elden ring" --limit 10
websearcher --sites steamrip,reloadedsteam "baldur's gate 3"
websearcher --debug "starfield"

# Table output grouped by site
websearcher "elden ring" --sites "fitgirl,dodi,steamrip" --limit 5 --format table

# Interactive mode (no args): prompts for query and sites, prints a table
websearcher

# Cloudflare
# CF solver is ON by default. Disable globally with:
websearcher "elden ring" --no-cf
# Override solver endpoint if needed
websearcher "elden ring" --cf_url http://localhost:8191/v1

# gog-games specifics: solver is ignored; optional cookies help
websearcher "cyberpunk" --sites gog-games --format table --cookie "cf_clearance=...; gog_games_download_free_gog_pc_games_session=...; XSRF-TOKEN=..."
```

Flags:
- `--limit N` per-site cap (default 10)
- `--sites a,b,c` restrict to listed sites (default all)
- `--debug` print diagnostics and write HTML samples to `debug/`
- `--format [json|table]` output format (default json)
 - `--no-cf` disable Cloudflare solver (enabled by default)
 - `--cf_url URL` override FlareSolverr endpoint (default `http://localhost:8191/v1`)
 - `--cookie "key=value; other=value2"` forward cookies to requests and solver

Interactive mode:
- Run `websearcher` with no arguments to be prompted for a search phrase and site selection.
- Type `all` or press Enter to search all sites; or input names/numbers comma-separated.

### Notes

- Concurrency limited to 3 requests.
- CF is enabled by default. Run FlareSolverr locally if needed:
  - `docker run -d --name flaresolverr -p 8191:8191 ghcr.io/flaresolverr/flaresolverr:latest`
- `gog-games`: solver is ignored; uses normal fetch plus AJAX JSON/HTML fragment fallback. Cookies often improve results.
- `ankergames`: uses path-encoded search (spaces as `%20`) and improved selectors; listing page parsing supported.
- Selectors are best-effort with fallbacks; contributions to improve coverage are welcome.

### Docker / Devcontainer

- One-shot container (builds binary, runs interactive):
```bash
docker build -t websearcher .
docker run --rm -it -e CF_URL=http://host.docker.internal:8191/v1 websearcher
```

- Devcontainer / Compose (starts FlareSolverr and the app):
```bash
docker compose -f docker-compose.dev.yml up --build
# App runs interactively inside the container; FlareSolverr available at http://flaresolverr:8191/v1
```

- VS Code devcontainer: open the folder and "Reopen in Container".

### Development

```powershell
cargo fmt
cargo clippy
cargo test
```


