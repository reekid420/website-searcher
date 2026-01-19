# syntax=docker/dockerfile:1

# =============================================================================
# Stage 1: Chef - Prepare dependency recipe for caching
# =============================================================================
FROM lukemathwalker/cargo-chef:latest-rust-1.89-alpine AS chef
WORKDIR /app

# =============================================================================
# Stage 2: Planner - Analyze dependencies
# =============================================================================
FROM chef AS planner
COPY Cargo.toml Cargo.lock ./
COPY crates/ crates/
# Remove src-tauri from workspace (GUI not needed in Docker)
RUN sed -i '/"src-tauri",/d' Cargo.toml
# Regenerate lockfile after modifying workspace
RUN cargo generate-lockfile
RUN cargo chef prepare --recipe-path recipe.json

# =============================================================================
# Stage 3: Builder - Compile dependencies and application
# =============================================================================
FROM chef AS builder

# Install musl target for static linking
RUN rustup target add x86_64-unknown-linux-musl

# Copy modified Cargo.toml and regenerated Cargo.lock from planner
COPY --from=planner /app/Cargo.toml Cargo.toml
COPY --from=planner /app/Cargo.lock Cargo.lock
COPY --from=planner /app/recipe.json recipe.json

# Build dependencies first (cached layer)
RUN cargo chef cook --release --target x86_64-unknown-linux-musl --recipe-path recipe.json

# Copy source (but don't overwrite already-modified Cargo.toml)
COPY crates/ crates/

# Optionally run tests during build (enable with --build-arg RUN_TESTS=true)
ARG RUN_TESTS=false
RUN if [ "$RUN_TESTS" = "true" ]; then cargo test --all; fi

# Build the final binary (CLI only, src-tauri excluded from workspace)
RUN cargo build --release --target x86_64-unknown-linux-musl && \
    strip /app/target/x86_64-unknown-linux-musl/release/website-searcher 2>/dev/null || true

# =============================================================================
# Stage 4: Runtime - Minimal production image
# =============================================================================
FROM alpine:3.21 AS runtime

# OCI Image Labels
LABEL org.opencontainers.image.source="https://github.com/reekid420/website-searcher"
LABEL org.opencontainers.image.description="Cross-platform CLI tool for searching game download sites"
LABEL org.opencontainers.image.licenses="MIT"
LABEL org.opencontainers.image.title="website-searcher"

# Install runtime dependencies
RUN apk add --no-cache ca-certificates tzdata

# Create non-root user for security
RUN adduser -D -u 10001 appuser

WORKDIR /app

# Copy the compiled binary
COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/website-searcher /usr/local/bin/websearcher

# Environment configuration
ENV RUST_LOG=info
ENV CF_URL=http://localhost:8191/v1

# Switch to non-root user
USER 10001

# Healthcheck - verify binary is functional
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD ["/usr/local/bin/websearcher", "--version"]

# Default entrypoint
ENTRYPOINT ["/usr/local/bin/websearcher"]
CMD []
