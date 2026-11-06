# syntax=docker/dockerfile:1

# ---- build stage ----
# Uses a current stable Rust (1.x). The pinned deps in Cargo.lock require
# rustc >= 1.88 (image/time) and >= 1.86 (icu_*). To pin exactly, use e.g.
# `rust:1.97-bookworm`.
FROM rust:1-bookworm AS builder

WORKDIR /app

# Colored cargo output in CI logs. Does not affect the binary.
ENV CARGO_TERM_COLOR=always

# Cache dependencies in their own layer. GitHub Actions restores this layer
# per architecture, so source-only changes do not rebuild the crate graph.
# A failing dependency build must fail the image: swallowing it with `|| true`
# would cache a broken layer and hide the error.
COPY Cargo.toml Cargo.lock ./
RUN mkdir -p src static/css static/js static/pages migrations \
    && echo 'fn main() {}' > src/main.rs \
    && cargo build --release --locked \
    && rm -rf /usr/local/cargo/registry/src /usr/local/cargo/git

# Real source + embedded assets (static/ and migrations/ are read at compile time).
# Drop the dummy binary fingerprint so cargo cannot reuse `fn main() {}`.
COPY src ./src
COPY migrations ./migrations
COPY static ./static
RUN rm -rf target/release/.fingerprint/rangpic-* \
        target/release/deps/rangpic-* \
        target/release/rangpic \
    && cargo build --release --locked

# ---- runtime stage ----
# Postgres is installed here and listens only on 127.0.0.1.
# Operators publish one port and mount one directory at /data.
FROM debian:bookworm-slim

ENV DEBIAN_FRONTEND=noninteractive \
    LANG=C.UTF-8

RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        ca-certificates \
        postgresql-15 \
        tini \
    && rm -rf /var/lib/apt/lists/* \
    && install -d -o postgres -g postgres /var/run/postgresql

WORKDIR /app

COPY --from=builder /app/target/release/rangpic /app/rangpic
COPY data /app/data
COPY docker/entrypoint.sh /app/entrypoint.sh
RUN chmod +x /app/entrypoint.sh

ENV BIND_ADDR=0.0.0.0:17777 \
    STORAGE_DIR=/data/storage \
    DATA_DIR=/app \
    ADMIN_USERNAME=admin \
    ADMIN_PASSWORD=changeme \
    RUST_LOG=info

EXPOSE 17777
VOLUME ["/data"]

ENTRYPOINT ["tini", "--", "/app/entrypoint.sh"]
