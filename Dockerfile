# syntax=docker/dockerfile:1
FROM rust:1.89 as builder
WORKDIR /app
COPY . .
# Optionally run the full test suite during image build (enable with --build-arg RUN_TESTS=true)
ARG RUN_TESTS=false
RUN if [ "$RUN_TESTS" = "true" ]; then cargo test --all; fi
# Build optimized binary
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=builder /app/target/release/website-searcher /usr/local/bin/websearcher
ENV RUST_LOG=info
# Default CF URL (can be overridden)
ENV CF_URL=http://localhost:8191/v1
# Default command runs interactive mode
CMD ["websearcher"]
