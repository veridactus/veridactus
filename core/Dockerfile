# =============================================================================
# VERIDACTUS Core (Rust Data Plane) Dockerfile
# Multi-stage build for minimal production image
# =============================================================================

# -----------------------------------------------------------------------------
# Stage 1: Builder
# -----------------------------------------------------------------------------
FROM rust:1.75-alpine AS builder

# Install build dependencies
RUN apk add --no-cache musl-dev pkgconfig openssl-dev openssl-libs-static

# Set working directory
WORKDIR /build

# Copy dependency manifests
COPY Cargo.toml Cargo.lock ./

# Create dummy source for dependency caching
RUN mkdir -p src benches && \
    echo "fn main() {}" > src/main.rs && \
    echo "fn main() {}" > src/lib.rs && \
    echo "fn main() {}" > benches/journal_bench.rs

# Build dependencies only (cached layer)
RUN cargo build --release --target x86_64-unknown-linux-musl && \
    rm -rf src

# Copy actual source code
COPY src ./src

# Build the final binary
RUN cargo build --release --target x86_64-unknown-linux-musl

# -----------------------------------------------------------------------------
# Stage 2: Runtime
# -----------------------------------------------------------------------------
FROM alpine:3.19 AS runtime

# Install runtime dependencies
RUN apk add --no-cache ca-certificates tzdata

# Add labels for OCI image spec
LABEL org.opencontainers.image.title="VERIDACTUS Core Data Plane" \
      org.opencontainers.image.description="Trusted AI Execution Governance - Rust Data Plane" \
      org.opencontainers.image.source="https://github.com/veridactus/veridactus" \
      org.opencontainers.image.licenses="Apache-2.0" \
      org.opencontainers.image.version="0.2.1" \
      org.opencontainers.image.vendor="VERIDACTUS Authors"

# Create non-root user for security
RUN addgroup -g 1000 veridactus && \
    adduser -u 1000 -G veridactus -s /bin/sh -D veridactus

# Set working directory
WORKDIR /app

# Copy binary from builder
COPY --from=builder /build/target/x86_64-unknown-linux-musl/release/veridactus-core /app/

# Copy configuration (if exists)
COPY --from=builder /build/veridactus.toml /app/ 2>/dev/null || true

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
