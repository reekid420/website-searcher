## Website Searcher

Cross-platform CLI that queries multiple game-download sites in parallel, scrapes results, and prints pretty JSON or a grouped table.

### Build
```bash
# If on linux
chmod +x ./compile.sh
./compile.sh
```

```powershell
# If on windows
./compile.ps1
```

### Usage

```bash
# Basic
websearcher "elden ring" --limit 10
websearcher --sites steamrip,reloadedsteam "baldur's gate 3"
websearcher --debug "starfield"

# Table output grouped by site
websearcher "elden ring" --sites "fitgirl,dodi,steamrip" --limit 5 --format table

# Interactive mode (no args): prompts for query and sites, prints a table
websearcher

# TUI is default for table output when interactive (press q to quit)
websearcher "elden ring" --sites fitgirl,dodi --format table

# Cloudflare
# CF solver is ON by default. Disable globally with:
websearcher "elden ring" --no-cf
# Override solver endpoint if needed
websearcher "elden ring" --cf_url http://localhost:8191/v1

# gog-games specifics: solver is ignored; optional cookies help
websearcher "cyberpunk" --sites gog-games --format table --cookie "cf_clearance=...; gog_games_download_free_gog_pc_games_session=...; XSRF-TOKEN=..."

# cs.rin.ru via Playwright (auto by default). Use --no-playwright to force non-PW fallbacks
# Requires Node + Playwright locally, or use docker-compose 'playwright' service
websearcher "elden ring" --sites csrin --format table --debug --no-cf
```

Flags:
- `--limit N` per-site cap (default 10)
- `--sites a,b,c` restrict to listed sites (default all)
- `--debug` print diagnostics and write HTML samples to `debug/`
- `--format [json|table]` output format (default json)
- `--no-cf` disable Cloudflare solver (enabled by default)
- `--cf_url URL` override FlareSolverr endpoint (default `http://localhost:8191/v1`)
- `--cookie "key=value; other=value2"` forward cookies to requests and solver
- TUI is enabled automatically for table output when running interactively
- `--no-playwright` disable automatic Playwright fallback for cs.rin

Interactive mode:
- Run `websearcher` with no arguments to be prompted for a search phrase and site selection.
- Type `all` or press Enter to search all sites; or input names/numbers comma-separated.

### Notes

- Concurrency limited to 3 requests.
- CF is enabled by default. Run FlareSolverr locally if needed:
  - `docker run -d --name flaresolverr -p 8191:8191 ghcr.io/flaresolverr/flaresolverr:latest`
- `gog-games`: solver is ignored; uses normal fetch plus AJAX JSON/HTML fragment fallback. Cookies often improve results.
- `ankergames`: uses path-encoded search (spaces as `%20`) and improved selectors; listing page parsing supported.
- `csrin`: parses the forum Releases listing; each topic is treated as a game entry.
- Table output auto-wraps titles to your terminal width; resizing the terminal updates layout in `--live` mode.

#### Windows packaging (MSI)

- MSI bundling uses a custom WiX template that installs the CLI into `INSTALLDIR\bin`, adds it to `PATH`, and deploys a `ws.cmd` alias.
- The build script `compile.ps1` will attempt `cargo tauri build`. If the WiX link step fails, it automatically falls back to manually linking the MSI using the locally installed WiX tools (candle/light).
- Ensure WiX tools are present locally (Tauri caches them under `%LOCALAPPDATA%\tauri\WixTools*`).

#### Arch Linux / Manjaro packaging

On Arch-based distributions, the compile script auto-detects and builds a `.pkg.tar.zst` package:

```bash
python compile.py                    # Auto-detects Arch, builds with CLI + GUI
python compile.py --pacman           # Force pacman package build
python compile.py --pacman --nogui   # Build without GUI binary
```

The package includes:
- `website-searcher` - CLI binary
- `websearcher` - symlink alias
- `ws` - short alias script (use `ws --gui` to launch GUI)
- `website-searcher-gui` - GUI binary (excluded with `--nogui`)

Install the resulting package:
```bash
sudo pacman -U pkg/website-searcher-*.pkg.tar.zst
```


### Docker / Devcontainer

- One-shot container (builds binary, runs interactive):
```bash
docker build -t websearcher .
docker run --rm -it -e CF_URL=http://host.docker.internal:8191/v1 websearcher
```

- Devcontainer / Compose (starts FlareSolverr and the app):
```bash
docker compose up --build
# App runs interactively inside the container; FlareSolverr available at http://flaresolverr:8191/v1
# The runtime container runs as a non-root user for improved security.
```

- VS Code devcontainer: open the folder and "Reopen in Container".

### Development

```bash
cargo fmt
cargo clippy
cargo test
```

### Tests

- Unit tests cover:
  - `src/query.rs`: normalization and search URL building for all SearchKind variants
  - `src/parser.rs`: fallback anchor parsing, ElAmigos headings, FitGirl filters, relative/absolute URLs, href title derivation
  - `src/fetcher.rs`: retry/backoff behavior, 200/302/403 handling, header forwarding
  - `src/cf.rs`: FlareSolverr success/error JSON handling, header forwarding in payload
  - `src/main.rs`: `normalize_title` cleanup, JSON traversal for gog-games AJAX fallbacks
  - `src/config.rs`: site invariants (SearchKind, Cloudflare flags)

- Integration tests (mocked FlareSolverr; no external network):
  - `tests/cli_cf_mock.rs`: solver on by default; JSON and table output grouping
  - `tests/cli_cookie_forwarding.rs`: `--cookie` forwarded in solver payload
  - `tests/cli_dedup_and_limit.rs`: deduplication and per-site `--limit`
  - `tests/cli_no_results_table.rs`: table output prints "No results." when empty
  - `tests/integration_smoke.rs`: interactive empty input error; multi-site grouping; site filtering; unknown sites → empty JSON; `--debug` writes `debug/fitgirl_sample.html`; per-site limit across sites

Playwright:
- Local: `npm i -D playwright && npx playwright install --with-deps`
- Compose: `docker compose run --rm playwright bash -lc "npm i -D playwright && node scripts/csrin_search.cjs 'elden ring'"`
- CI: A smoke run executes `--no-cf` with stubbed HTML for Playwright path.

Run tests:
```powershell
cargo test
# Single file
cargo test --test integration_smoke
# Show stdout/stderr for a test
cargo test --test cli_dedup_and_limit -- --nocapture
```

Notes:
- Tests mock FlareSolverr with `mockito` and override the endpoint via `--cf-url`; CF is enabled by default.
- Some tests enable `--debug` and write sample HTML into `debug/`. Clean with:
```powershell
Remove-Item -Force -ErrorAction SilentlyContinue debug\*.html
```
- To avoid color codes in assertions, tests set `NO_COLOR=1`.


### Coverage (Rust workspace)

You can generate coverage across `crates/core`, `crates/cli`, and `src-tauri` using LLVM source-based coverage. Install the tool once:

```bash
cargo install cargo-llvm-cov
```

Run coverage (HTML report will open location in the output):

```bash
# From the repo root
cargo llvm-cov --workspace --exclude examples --html
```

Tips:
- On Windows, ensure LLVM tools are available via the installed `cargo-llvm-cov`; MSVC toolchain is supported.
- If you only want a quick summary without HTML: `cargo llvm-cov --workspace --summary-only`.
- Linux users may optionally use tarpaulin: `cargo install cargo-tarpaulin && cargo tarpaulin --out Html` (Linux-only).


