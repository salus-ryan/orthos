# ORTHOS Bridge Docker Image
# Provides TCP/WebSocket access to the daemon
# Multi-stage build including daemon

FROM rust:1.75-bookworm AS builder

# Install Z3 dependencies
RUN apt-get update && apt-get install -y \
    cmake \
    python3 \
    libz3-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /orthos

# Copy workspace Cargo files
COPY Cargo.toml ./
COPY orthos-kernel/Cargo.toml ./orthos-kernel/
COPY orthos-daemon/Cargo.toml ./orthos-daemon/
COPY orthos-bridge/Cargo.toml ./orthos-bridge/

# Create dummy sources for dependency caching
RUN mkdir -p orthos-kernel/src && \
    echo "pub fn dummy() {}" > orthos-kernel/src/lib.rs && \
    mkdir -p orthos-daemon/src && \
    echo "fn main() {}" > orthos-daemon/src/main.rs && \
    mkdir -p orthos-bridge/src && \
    echo "fn main() {}" > orthos-bridge/src/main.rs

# Build dependencies
RUN cargo build --release --package orthos-bridge --package orthos-daemon || true

# Copy actual source
COPY orthos-kernel/src ./orthos-kernel/src
COPY orthos-daemon/src ./orthos-daemon/src
COPY orthos-bridge/src ./orthos-bridge/src

# Rebuild with real sources
RUN touch orthos-kernel/src/lib.rs orthos-daemon/src/main.rs orthos-bridge/src/main.rs && \
    cargo build --release --package orthos-bridge --package orthos-daemon

# Runtime image
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    libz3-4 \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Copy both binaries
COPY --from=builder /orthos/target/release/orthos-bridge /usr/local/bin/orthos-bridge
COPY --from=builder /orthos/target/release/orthos-daemon /usr/local/bin/orthos-daemon

RUN useradd -m -u 1000 orthos
USER orthos

EXPOSE 7777 7778

ENTRYPOINT ["orthos-bridge"]
