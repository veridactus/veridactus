# =============================================================================
# VERIDACTUS Core (Rust Data Plane) Dockerfile
# Multi-stage build for minimal production image
# Optimized for multi-architecture builds with enhanced caching
# =============================================================================

# -----------------------------------------------------------------------------
# Stage 1: Builder
# -----------------------------------------------------------------------------
FROM rust:1.91-bullseye AS builder

# Set working directory
WORKDIR /build

# Set environment variables for caching
ENV CARGO_HOME=/cargo
ENV PATH=/cargo/bin:$PATH

# Create directories for cargo cache
RUN mkdir -p /cargo/registry /cargo/git

# Copy dependency manifests first for better layer caching
COPY Cargo.toml Cargo.lock ./

# Create dummy source files to satisfy Cargo.toml requirements
RUN mkdir -p src && echo "fn main() {}" > src/main.rs && \
    mkdir -p benches && echo "fn main() {}" > benches/journal_bench.rs && \
    mkdir -p tests && echo "fn main() {}" > tests/integration_test.rs

# Build dependencies with cache mount (this layer will be cached if Cargo.toml/Cargo.lock don't change)
# Using --mount to cache cargo registry and git dependencies
RUN --mount=type=cache,target=/cargo/registry \
    --mount=type=cache,target=/cargo/git \
    cargo build --release --bin veridactus-core

# Remove dummy source files
RUN rm -rf src benches tests

# Copy actual source code
COPY src/ ./src/
COPY benches/ ./benches/
COPY tests/ ./tests/

# Build the final binary with cache mount
RUN --mount=type=cache,target=/cargo/registry \
    --mount=type=cache,target=/cargo/git \
    cargo build --release --bin veridactus-core

# -----------------------------------------------------------------------------
# Stage 2: Runtime
# -----------------------------------------------------------------------------
FROM debian:bullseye-slim AS runtime

# Install runtime dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    tzdata \
    && rm -rf /var/lib/apt/lists/*

# Add labels for OCI image spec
LABEL org.opencontainers.image.title="VERIDACTUS Core Data Plane" \
      org.opencontainers.image.description="Trusted AI Execution Governance - Rust Data Plane" \
      org.opencontainers.image.source="https://github.com/veridactus/veridactus" \
      org.opencontainers.image.licenses="Apache-2.0" \
      org.opencontainers.image.version="0.2.1" \
      org.opencontainers.image.vendor="VERIDACTUS Authors"

# Create non-root user for security
RUN groupadd -r veridactus --gid=1000 && \
    useradd -r -g veridactus --uid=1000 veridactus

# Set working directory
WORKDIR /app

# Copy binary from builder
COPY --from=builder /build/target/release/veridactus-core /app/

# Copy configuration (if exists)
RUN if [ -f /build/veridactus.toml ]; then cp /build/veridactus.toml /app/; fi

# Set ownership
RUN chown -R veridactus:veridactus /app

# Switch to non-root user
USER veridactus

# Expose port
EXPOSE 8080

# Health check
HEALTHCHECK --interval=30s --timeout=5s --start-period=5s --retries=3 \
    CMD wget --no-verbose --tries=1 --spider http://localhost:8080/health || exit 1

# Environment defaults
ENV RUST_LOG=info
ENV RUST_BACKTRACE=1
ENV VERIDACTUS_PORT=8080

# Run the binary
ENTRYPOINT ["/app/veridactus-core"]