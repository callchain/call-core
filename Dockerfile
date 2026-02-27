# Call-core Docker image
# Multi-stage build for optimized production image

# =============================================================================
# Stage 1: Builder
# =============================================================================
FROM rust:1.75-slim-bookworm AS builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    cmake \
    libclang-dev \
    llvm-dev \
    clang \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Set working directory
WORKDIR /usr/src/call-core

# Copy workspace files
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
COPY tests ./tests

# Build the release binary
RUN cargo build --release --package node

# =============================================================================
# Stage 2: Runtime
# =============================================================================
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN useradd -m -u 1000 callchain

# Create data directory
RUN mkdir -p /data && chown -R callchain:callchain /data

# Copy binary from builder
COPY --from=builder /usr/src/call-core/target/release/call-core /usr/local/bin/

# Set working directory
WORKDIR /data

# Expose P2P and RPC ports
EXPOSE 51235 5005

# Switch to non-root user
USER callchain

# Default entrypoint
ENTRYPOINT ["/usr/local/bin/call-core"]

# Default command - start the node
CMD ["start"]
