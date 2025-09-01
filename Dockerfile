# syntax=docker/dockerfile:1
FROM rust:1.79 as builder
WORKDIR /app
COPY . .
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
