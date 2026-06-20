# =============================================================================
# VERIDACTUS Frontend (React) Dockerfile
# Multi-stage build for minimal production image
# =============================================================================

# -----------------------------------------------------------------------------
# Stage 1: Builder
# -----------------------------------------------------------------------------
FROM node:20-alpine AS builder

# Set working directory
WORKDIR /app

# Copy package files
COPY package.json package-lock.json ./

# Install dependencies
RUN npm ci --legacy-peer-deps

# Copy source code
COPY . .

# Build arguments
ARG VITE_API_URL=http://localhost:8081
ARG VITE_WS_URL=ws://localhost:8080
ARG VERSION=dev

# Set build environment
ENV VITE_API_URL=${VITE_API_URL}
ENV VITE_WS_URL=${VITE_WS_URL}

# Build the application
RUN npm run build

# -----------------------------------------------------------------------------
# Stage 2: Runtime (Nginx)
# -----------------------------------------------------------------------------
FROM nginx:alpine AS runtime

# Install runtime dependencies
RUN apk add --no-cache ca-certificates

# Add labels for OCI image spec
LABEL org.opencontainers.image.title="VERIDACTUS Frontend" \
      org.opencontainers.image.description="Trusted AI Execution Governance - React Frontend" \
      org.opencontainers.image.source="https://github.com/veridactus/veridactus" \
      org.opencontainers.image.licenses="Apache-2.0" \
      org.opencontainers.image.version="0.2.1" \
      org.opencontainers.image.vendor="VERIDACTUS Authors"

# Copy custom nginx configuration
COPY --from=builder /app/nginx.conf /etc/nginx/conf.d/default.conf

# Copy built assets from builder
COPY --from=builder /app/dist /usr/share/nginx/html

# Create non-root user for security
RUN addgroup -g 1000 veridactus && \
    adduser -u 1000 -G veridactus -s /bin/sh -D veridactus && \
    chown -R veridactus:veridactus /usr/share/nginx/html && \
    chown -R veridactus:veridactus /var/cache/nginx && \
    chown -R veridactus:veridactus /var/log/nginx && \
    touch /var/run/nginx.pid && \
    chown -R veridactus:veridactus /var/run/nginx.pid

# Switch to non-root user
USER veridactus

# Expose port
EXPOSE 3000

# Health check
HEALTHCHECK --interval=30s --timeout=5s --start-period=5s --retries=3 \
    CMD wget --no-verbose --tries=1 --spider http://localhost:3000/ || exit 1

# Start nginx
EXPOSE 3000
CMD ["nginx", "-g", "daemon off;"]
