# =============================================================================
# VERIDACTUS Control Plane (Go) Dockerfile
# Multi-stage build for minimal production image
# =============================================================================

# -----------------------------------------------------------------------------
# Stage 1: Builder
# -----------------------------------------------------------------------------
FROM golang:1.21-alpine AS builder

# Install build dependencies
RUN apk add --no-cache git ca-certificates

# Set working directory
WORKDIR /app

# Copy dependency manifests
COPY go.mod go.sum ./

# Download dependencies (cached layer)
RUN go mod download

# Copy source code
COPY . .

# Build the binary with version info
ARG VERSION=dev
ARG COMMIT_SHA=unknown
ARG BUILD_DATE=unknown
RUN CGO_ENABLED=0 GOOS=linux GOARCH=amd64 \
    go build -ldflags="\
        -s -w \
        -X main.version=${VERSION} \
        -X main.commit=${COMMIT_SHA} \
        -X main.buildDate=${BUILD_DATE} \
    " \
    -o veridactus-cp ./cmd/server/

# -----------------------------------------------------------------------------
# Stage 2: Runtime
# -----------------------------------------------------------------------------
FROM alpine:3.19 AS runtime

# Install runtime dependencies
RUN apk add --no-cache ca-certificates tzdata wget

# Add labels for OCI image spec
LABEL org.opencontainers.image.title="VERIDACTUS Control Plane" \
      org.opencontainers.image.description="Trusted AI Execution Governance - Go Control Plane" \
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
COPY --from=builder /app/veridactus-cp /app/

# Copy database schema (if exists)
RUN mkdir -p /app/migrations && touch /app/migrations/.keep || true

# Set ownership
RUN chown -R veridactus:veridactus /app

# Switch to non-root user
USER veridactus

# Expose port
EXPOSE 8081

# Health check
HEALTHCHECK --interval=30s --timeout=5s --start-period=5s --retries=3 \
    CMD wget --no-verbose --tries=1 --spider http://localhost:8081/api/v1/health || exit 1

# Environment defaults
ENV VERIDACTUS_CP_PORT=8081
ENV VERIDACTUS_CP_DB_PATH=/app/veridactus.db

# Run the binary
ENTRYPOINT ["/app/veridactus-cp"]
