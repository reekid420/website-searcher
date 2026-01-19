# Docker Guide

Website-searcher provides Docker support for containerized builds and deployment.

## Quick Start

```bash
# Build and run standalone
docker build -t websearcher .
docker run --rm -it websearcher "elden ring"

# With FlareSolverr (for Cloudflare bypass)
docker compose --profile cf up -d
docker attach websearcher
```

## Image Details

| Property   | Value                    |
| ---------- | ------------------------ |
| Base Image | `alpine:3.21`            |
| Final Size | ~15MB                    |
| User       | Non-root (UID 10001)     |
| Binary     | Statically linked (musl) |

## Dockerfile

The Dockerfile uses a 4-stage build with `cargo-chef` for efficient caching:

```
┌─────────────┐     ┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│    chef     │ ──▶ │   planner   │ ──▶ │   builder   │ ──▶ │   runtime   │
│  (base)     │     │  (analyze)  │     │  (compile)  │     │  (alpine)   │
└─────────────┘     └─────────────┘     └─────────────┘     └─────────────┘
```

### Build Arguments

| Arg         | Default | Description                 |
| ----------- | ------- | --------------------------- |
| `RUN_TESTS` | `false` | Run test suite during build |

```bash
# Build with tests
docker build --build-arg RUN_TESTS=true -t websearcher .
```

## Docker Compose

### Services

| Service        | Profile      | Description                      |
| -------------- | ------------ | -------------------------------- |
| `app`          | (always)     | Main application                 |
| `flaresolverr` | `cf`         | Cloudflare bypass proxy          |
| `playwright`   | `playwright` | Browser automation for cs.rin.ru |

### Usage

```bash
# App only (no Cloudflare bypass)
docker compose up -d

# App + FlareSolverr
docker compose --profile cf up -d

# Attach to interactive app
docker attach websearcher

# One-shot search
docker compose run --rm app "elden ring" --format table

# Run Playwright search
docker compose --profile playwright run --rm playwright \
  bash -c "npm i -D playwright && node scripts/csrin_search.cjs 'elden ring'"
```

### Environment Variables

| Variable      | Service      | Description                   |
| ------------- | ------------ | ----------------------------- |
| `CF_URL`      | app          | FlareSolverr endpoint         |
| `RUST_LOG`    | app          | Log level (debug, info, warn) |
| `CSRIN_PAGES` | playwright   | Result pages for cs.rin.ru    |
| `LOG_LEVEL`   | flaresolverr | FlareSolverr log level        |

## Security Features

The Docker configuration includes several security hardening measures:

- **Non-root user**: Runs as UID 10001
- **Read-only filesystem**: Container filesystem is immutable
- **Dropped capabilities**: All Linux capabilities removed
- **Resource limits**: Memory and CPU constraints
- **Healthcheck**: Automatic liveness verification

## Standalone Usage

```bash
# Build image
docker build -t websearcher .

# Interactive mode
docker run --rm -it websearcher

# One-shot search
docker run --rm websearcher "elden ring" --format table

# With external FlareSolverr
docker run --rm -it \
  -e CF_URL=http://host.docker.internal:8191/v1 \
  websearcher "elden ring"
```

## Troubleshooting

### FlareSolverr not connecting

```bash
# Check if flaresolverr is running and healthy
docker compose --profile cf ps
docker compose --profile cf logs flaresolverr

# Verify endpoint
curl http://localhost:8191/
```

### Build fails with cargo-chef

```bash
# Clean build cache and rebuild
docker builder prune
docker build --no-cache -t websearcher .
```

### Alpine compatibility issues

If you encounter glibc-related errors:

```bash
# Verify the binary is statically linked
docker run --rm websearcher sh -c "ldd /usr/local/bin/websearcher"
# Should output: "not a dynamic executable"
```

### Permission issues on volumes

```bash
# Linux: Fix ownership
sudo chown -R $(id -u):$(id -g) .
```

## DevContainer

For VS Code development:

1. Install "Dev Containers" extension
2. Open project folder
3. Click "Reopen in Container"

See `.devcontainer/devcontainer.json` for configuration.
