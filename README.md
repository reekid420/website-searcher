## Website Searcher

Cross-platform CLI that queries multiple game-download sites in parallel, scrapes results, and prints pretty JSON.

### Build

1. Install Rust via `https://rustup.rs`.
2. Clone this repo, then run:
   - `cargo build --release`

### Usage

```powershell
websearcher "elden ring" --limit 10
websearcher --sites steamrip,reloadedsteam "baldur's gate 3"
websearcher --debug "starfield"
# Cloudflare-protected sites (FitGirl, DODI):
websearcher --use-cf --sites fitgirl,dodi "elden ring"
```

Flags:
- `--limit N` per-site cap (default 10)
- `--sites a,b,c` restrict to listed sites (default all)
- `--debug` print diagnostics and write HTML samples to `debug/`

### Notes

- Concurrency limited to 3 requests.
- Some sites may be protected (403/JS challenge); those return zero results.
- For Cloudflare-protected sites, you can optionally run FlareSolverr locally and pass `--use-cf`:
  - `docker run -d --name flaresolverr -p 8191:8191 ghcr.io/flaresolverr/flaresolverr:latest`
  - Override endpoint with `--cf_url http://localhost:8191/v1` if needed.
- Selectors are best-effort with fallbacks; contributions to improve coverage are welcome.

### Development

```powershell
cargo fmt
cargo clippy
cargo test
```


