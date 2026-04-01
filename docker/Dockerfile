# ── Stage 1: Build ────────────────────────────────────────────────────────────
FROM rust:1.82-bookworm AS builder
WORKDIR /workspace

# Copy workspace files needed for the server crate
COPY Cargo.toml Cargo.lock ./
COPY crates/ crates/
COPY apps/localmessenger_server/ apps/localmessenger_server/

# Build only the relay server binary
RUN cargo build --release -p localmessenger_server

# ── Stage 2: Runtime ──────────────────────────────────────────────────────────
FROM debian:bookworm-slim
RUN apt-get update \
    && apt-get install -y --no-install-recommends \
       libssl3 ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /workspace/target/release/localmessenger_server /usr/local/bin/localmessenger_server
COPY docker/entrypoint.sh /entrypoint.sh
RUN chmod +x /entrypoint.sh

# Persistent data: SQLite DB, cert, key
VOLUME ["/data"]

# QUIC runs over UDP
EXPOSE 7443/udp

ENTRYPOINT ["/entrypoint.sh"]
