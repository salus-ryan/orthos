# ORTHOS Daemon Docker Image
# Multi-stage build for minimal runtime image

# Stage 1: Build environment with Z3 and Rust
FROM rust:1.75-bookworm AS builder

# Install Z3 dependencies
RUN apt-get update && apt-get install -y \
    cmake \
    python3 \
    libz3-dev \
    && rm -rf /var/lib/apt/lists/*

# Create workspace
WORKDIR /orthos

# Copy workspace Cargo files first for dependency caching
COPY Cargo.toml ./
COPY orthos-kernel/Cargo.toml ./orthos-kernel/
COPY orthos-daemon/Cargo.toml ./orthos-daemon/

# Create dummy source files to build dependencies
RUN mkdir -p orthos-kernel/src && \
    echo "pub fn dummy() {}" > orthos-kernel/src/lib.rs && \
    mkdir -p orthos-daemon/src && \
    echo "fn main() {}" > orthos-daemon/src/main.rs

# Build dependencies (this layer is cached)
RUN cargo build --release --package orthos-daemon || true

# Copy actual source code
COPY orthos-kernel/src ./orthos-kernel/src
COPY orthos-daemon/src ./orthos-daemon/src

# Touch source files to invalidate cache and rebuild
RUN touch orthos-kernel/src/lib.rs orthos-daemon/src/main.rs

# Build the daemon
RUN cargo build --release --package orthos-daemon

# Stage 2: Minimal runtime image
FROM debian:bookworm-slim AS runtime

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    libz3-4 \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user for security
RUN useradd -m -u 1000 orthos
USER orthos
WORKDIR /home/orthos

# Copy the built binary
COPY --from=builder /orthos/target/release/orthos-daemon /usr/local/bin/orthos-daemon

# Copy standard library
COPY --chown=orthos:orthos std/ /home/orthos/std/

# Expose no ports - daemon uses stdin/stdout
# For network mode, we'll add a wrapper

# Health check via simple echo test
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD echo '{"jsonrpc":"2.0","method":"ping","id":1}' | timeout 2 orthos-daemon || exit 1

# Default entrypoint
ENTRYPOINT ["orthos-daemon"]
