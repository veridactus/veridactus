# =============================================================================
# VERIDACTUS Python Worker Dockerfile
# For PII detection and other Python-based processing
# =============================================================================

# -----------------------------------------------------------------------------
# Stage 1: Builder
# -----------------------------------------------------------------------------
FROM python:3.11-slim AS builder

# Set working directory
WORKDIR /app

# Copy dependency files
COPY python-worker/requirements.txt .

# Install dependencies
RUN pip install --no-cache-dir --user -r requirements.txt

# Copy source code
COPY python-worker/ .

# -----------------------------------------------------------------------------
# Stage 2: Runtime
# -----------------------------------------------------------------------------
FROM python:3.11-slim AS runtime

# Install runtime dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Add labels for OCI image spec
LABEL org.opencontainers.image.title="VERIDACTUS Python Worker" \
      org.opencontainers.image.description="Trusted AI Execution Governance - Python Worker" \
      org.opencontainers.image.source="https://github.com/veridactus/veridactus" \
      org.opencontainers.image.licenses="Apache-2.0" \
      org.opencontainers.image.version="0.2.1" \
      org.opencontainers.image.vendor="VERIDACTUS Authors"

# Create non-root user
RUN groupadd -g 1000 veridactus && \
    useradd -u 1000 -g veridactus -s /bin/bash veridactus

# Set working directory
WORKDIR /app

# Copy Python packages from builder
COPY --from=builder /root/.local /home/veridactus/.local

# Copy application code
COPY --from=builder /app /app

# Set ownership
RUN chown -R veridactus:veridactus /app

# Switch to non-root user
USER veridactus

# Set Python path
ENV PATH=/home/veridactus/.local/bin:$PATH
ENV PYTHONUNBUFFERED=1

# Health check
HEALTHCHECK --interval=30s --timeout=5s --start-period=5s --retries=3 \
    CMD python -c "import requests; requests.get('http://localhost:8082/health')" 2>/dev/null || exit 1

# Default command (can be overridden in docker-compose)
CMD ["python", "-m", "veridactus_worker"]
