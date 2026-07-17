# syntax=docker/dockerfile:1

# ---- build stage ----
# Uses a current stable Rust (1.x). The pinned deps in Cargo.lock require
# rustc >= 1.88 (image/time) and >= 1.86 (icu_*). To pin exactly, use e.g.
# `rust:1.97-bookworm`.
FROM rust:1-bookworm AS builder

WORKDIR /app

# Cache dependencies separately from source changes.
COPY Cargo.toml Cargo.lock ./
RUN mkdir -p src static/css static/js static/pages migrations \
    && echo 'fn main() {}' > src/main.rs \
    && cargo build --release --locked || true

# Real source + embedded assets (static/ and migrations/ are read at compile time).
COPY src ./src
COPY migrations ./migrations
COPY static ./static
RUN touch src/main.rs && cargo build --release --locked

# ---- runtime stage ----
FROM debian:bookworm-slim

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates tini \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# The frontend is embedded in the binary; only the seed file is needed at runtime.
COPY --from=builder /app/target/release/rangpic /app/rangpic
COPY data ./data

ENV BIND_ADDR=0.0.0.0:17777 \
    STORAGE_DIR=/app/storage \
    DATA_DIR=/app \
    RUST_LOG=info

EXPOSE 17777
VOLUME ["/app/storage"]

ENTRYPOINT ["tini", "--", "/app/rangpic"]
