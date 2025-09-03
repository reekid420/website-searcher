# syntax=docker/dockerfile:1
FROM rust:1.89 AS builder
WORKDIR /app
# Cache dependencies
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo "fn main(){}" > src/main.rs && \
    cargo build --release --locked && \
    rm -rf src
# Copy source and build
COPY . .
# Optionally run the full test suite during image build (enable with --build-arg RUN_TESTS=true)
ARG RUN_TESTS=false
RUN if [ "$RUN_TESTS" = "true" ]; then cargo test --all --locked; fi
## Build optimized static binary (musl) to avoid glibc version issues at runtime
RUN apt-get update && apt-get install -y --no-install-recommends musl-tools && rm -rf /var/lib/apt/lists/*
RUN rustup target add x86_64-unknown-linux-musl
RUN cargo build --release --locked --target x86_64-unknown-linux-musl && \
    ( strip /app/target/x86_64-unknown-linux-musl/release/website-searcher || true )

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates tzdata && rm -rf /var/lib/apt/lists/*
# Create non-root user
RUN useradd -r -u 10001 -g users appuser
WORKDIR /app
COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/website-searcher /usr/local/bin/websearcher
# Note: Playwright is not bundled in this image to keep it small and secure.
# Use docker-compose 'playwright' service or run locally with Node installed
# when using --csrin-playwright.
ENV RUST_LOG=info
# Default CF URL (can be overridden)
ENV CF_URL=http://localhost:8191/v1
USER 10001
# Default command runs interactive mode
ENTRYPOINT ["/usr/local/bin/websearcher"]
CMD []
