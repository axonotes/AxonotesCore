# Multi-stage build for minimal production image
FROM rust:1.91-slim AS builder

WORKDIR /app

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Copy manifests
COPY Cargo.toml Cargo.lock ./

# Create dummy files to build dependencies only
RUN mkdir -p src benches tests && \
    echo "fn main() {}" > src/main.rs && \
    echo "fn main() {}" > benches/api_benchmarks.rs && \
    echo "fn main() {}" > tests/integration_tests.rs

# Build dependencies (this layer will be cached)
RUN cargo build --release && \
    rm -rf target/release/deps/axonotes* target/release/axonotes*

# Copy real source code (benches/tests not needed in production)
COPY src ./src

# Build release binary (only rebuilds changed code)
RUN cargo build --release

# Runtime stage (use same base as builder for GLIBC compatibility)
FROM debian:trixie-slim

WORKDIR /app

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Copy binary from builder
COPY --from=builder /app/target/release/axonotes-storage /usr/local/bin/axonotes-storage

# Create directories for data and logs
RUN mkdir -p /data /var/log/axonotes-storage

# Expose port
EXPOSE 8080

# Set default environment variables
ENV CONFIG_PATH=/etc/axonotes/config.toml
ENV RUST_LOG=info

# Run the binary
CMD ["axonotes-storage"]
