# VERIDACTUS Deployment Guide

This document provides comprehensive deployment instructions for VERIDACTUS across different environments.

## Table of Contents

1. [Prerequisites](#prerequisites)
2. [Quick Start with Docker Compose](#quick-start-with-docker-compose)
3. [Production Deployment](#production-deployment)
4. [Kubernetes Deployment](#kubernetes-deployment)
5. [Configuration Reference](#configuration-reference)
6. [Monitoring and Logging](#monitoring-and-logging)
7. [Troubleshooting](#troubleshooting)

---

## Prerequisites

### System Requirements

| Component | CPU | Memory | Storage |
|-----------|-----|--------|---------|
| veridactus-core | 2 cores | 2 GB | 10 GB |
| veridactus-cp | 1 core | 256 MB | 1 GB |
| veridactus-ui | 1 core | 512 MB | 1 GB |
| veridactus-python-worker | 2 cores | 1 GB | 5 GB |
| Redis | 1 core | 256 MB | - |
| PostgreSQL | 1 core | 512 MB | 20 GB |

### Required Services

- **Docker** 20.10+ or **Podman** 4.0+
- **Docker Compose** 2.0+
- **Redis** 7.0+ (for caching and idempotency)
- **PostgreSQL** 16+ (for trace persistence)

### Network Ports

| Port | Service | Description |
|------|---------|-------------|
| 3000 | veridactus-ui | Admin dashboard |
| 8080 | veridactus-core | Data plane API |
| 8081 | veridactus-cp | Control plane API |
| 8002 | veridactus-python-worker | Python worker (optional) |
| 6379 | Redis | Cache |
| 5432 | PostgreSQL | Database |

---

## Quick Start with Docker Compose

### 1. Clone Repository

```bash
git clone https://github.com/veridactus/veridactus.git
cd veridactus
```

### 2. Configure Environment

Create a `.env` file:

```bash
# Control Plane
VERIDACTUS_ADMIN_KEY=your-secure-admin-key

# Data Plane
VERIDACTUS_UPSTREAM_URL=https://open.bigmodel.cn
VERIDACTUS_UPSTREAM_KEY=your-llm-api-key
VERIDACTUS_CONTROL_PLANE_URL=http://localhost:8081

# Storage
VERIDACTUS_STORE_BACKEND=postgres
POSTGRES_PASSWORD=your-postgres-password
MINIO_ROOT_USER=veridactus
MINIO_ROOT_PASSWORD=your-minio-password

# S3/MinIO Configuration (for large object storage)
VERIDACTUS_STORE_S3_ENDPOINT=http://minio:9000
VERIDACTUS_STORE_S3_BUCKET=veridactus-traces
VERIDACTUS_STORE_S3_ACCESS_KEY=veridactus
VERIDACTUS_STORE_S3_SECRET_KEY=your-minio-password
VERIDACTUS_STORE_S3_REGION=us-east-1

# CORS
CORS_ORIGINS=http://localhost:3000,http://localhost:8080
```

### 3. Start Services

```bash
# Start all services
docker-compose -f deploy/docker-compose.yml up -d

# Verify health
curl http://localhost:8080/health
curl http://localhost:8081/api/v1/health

# View logs
docker-compose -f deploy/docker-compose.yml logs -f
```

### 4. Access Dashboard

Open [http://localhost:3000](http://localhost:3000) in your browser.

---

## Production Deployment

### Architecture

```
┌─────────────────────────────────────────────────────────────────────────┐
│                         Production Architecture                          │
└─────────────────────────────────────────────────────────────────────────┘

                        ┌─────────────────┐
                        │   Load Balancer  │
                        │   (nginx/Traefik)│
                        └────────┬────────┘
                                 │
              ┌──────────────────┼──────────────────┐
              │                  │                  │
              ▼                  ▼                  ▼
    ┌─────────────────┐ ┌─────────────────┐ ┌─────────────────┐
    │  veridactus-ui  │ │ veridactus-core │ │  veridactus-cp  │
    │    (×2)         │ │    (×3)         │ │    (×2)         │
    │  :3000          │ │  :8080          │ │  :8081          │
    └─────────────────┘ └─────────────────┘ └─────────────────┘
              │                  │                  │
              └──────────────────┴──────────────────┘
                                 │
                    ┌────────────┴────────────┐
                    ▼                         ▼
           ┌─────────────────┐      ┌─────────────────┐
           │     Redis        │      │   PostgreSQL     │
           │     Cluster      │      │     Replica      │
           │     (×3)         │      │     Set (×3)     │
           └─────────────────┘      └─────────────────┘
```

### 1. Build Images

```bash
# Build all images
docker build -t veridactus/veridactus-core:latest ./core
docker build -t veridactus/veridactus-cp:latest ./control-plane
docker build -t veridactus/veridactus-ui:latest ./veridactus-ui
docker build -t veridactus/veridactus-python-worker:latest ./python-worker

# Push to registry
docker push veridactus/veridactus-core:latest
docker push veridactus/veridactus-cp:latest
docker push veridactus/veridactus-ui:latest
docker push veridactus/veridactus-python-worker:latest
```

### 2. External Services Setup

#### Redis Cluster

```bash
# Redis Sentinel for HA
docker run -d --name redis-sentinel \
  -e REDIS_MASTER_HOST=redis-primary \
  -p 26379:26379 \
  bitnami/redis-sentinel:latest
```

#### PostgreSQL Replica Set

```sql
-- Primary node
CREATE DATABASE veridactus;
CREATE USER veridactus WITH PASSWORD 'secure-password';
GRANT ALL PRIVILEGES ON DATABASE veridactus TO veridactus;

-- Enable replication
ALTER USER veridactus REPLICATION;
```

### 3. Environment Configuration

```bash
# .env.production

# Control Plane
VERIDACTUS_ADMIN_KEY=production-admin-key-min-32-chars
VERIDACTUS_CP_PORT=8081
VERIDACTUS_CP_DB_PATH=/app/veridactus.db

# Data Plane
VERIDACTUS_UPSTREAM_URL=https://api.openai.com/v1
VERIDACTUS_UPSTREAM_KEY=sk-prod-...
VERIDACTUS_MODE=governance
RUST_LOG=info
VERIDACTUS_CONTROL_PLANE_URL=http://veridactus-cp:8081

# Storage
VERIDACTUS_STORE_REDIS_HOST=redis-cluster
VERIDACTUS_STORE_REDIS_PORT=6379
VERIDACTUS_STORE_POSTGRES_HOST=postgres-primary
VERIDACTUS_STORE_POSTGRES_PORT=5432
VERIDACTUS_STORE_POSTGRES_DB=veridactus
VERIDACTUS_STORE_POSTGRES_USER=veridactus
VERIDACTUS_STORE_POSTGRES_PASSWORD=secure-password
```

### 4. Start Services

```bash
# Production compose
docker-compose -f deploy/docker-compose.yml -f deploy/docker-compose.prod.yml up -d
```

---

## Kubernetes Deployment

### Prerequisites

- Kubernetes 1.28+
- Helm 3.12+
- Ingress controller (nginx-ingress or traefik)

### 1. Add Helm Repository

```bash
helm repo add veridactus https://charts.veridactus.io
helm repo update
```

### 2. Install Charts

```bash
# Install PostgreSQL (dependency)
helm install postgres bitnami/postgresql \
  --namespace veridactus \
  --create-namespace \
  --set auth.postgresPassword=secure-password \
  --set auth.database=veridactus

# Install Redis
helm install redis bitnami/redis \
  --namespace veridactus \
  --set auth.password=secure-password

# Install veridactus-core
helm install veridactus-core veridactus/veridactus-core \
  --namespace veridactus \
  --set image.repository=veridactus/veridactus-core \
  --set image.tag=latest \
  --set config.adminKey=production-admin-key \
  --set config.upstreamLLM.url=https://api.openai.com/v1 \
  --set config.upstreamLLM.apiKey=sk-prod-... \
  --set controlPlane.url=http://veridactus-cp:8081 \
  --set store.postgres.host=postgres-primary \
  --set store.redis.host=redis

# Install veridactus-control-plane
helm install veridactus-cp veridactus/veridactus-control-plane \
  --namespace veridactus \
  --set image.repository=veridactus/veridactus-cp \
  --set image.tag=latest \
  --set config.adminKey=production-admin-key

# Install veridactus-ui
helm install veridactus-ui veridactus/veridactus-ui \
  --namespace veridactus \
  --set image.repository=veridactus/veridactus-ui \
  --set image.tag=latest
```

### 3. Configure Ingress

```yaml
# ingress.yaml
apiVersion: networking.k8s.io/v1
kind: Ingress
metadata:
  name: veridactus-ingress
  namespace: veridactus
  annotations:
    nginx.ingress.kubernetes.io/rewrite-target: /
spec:
  rules:
  - host: veridactus.example.com
    http:
      paths:
      - path: /
        pathType: Prefix
        backend:
          service:
            name: veridactus-ui
            port:
              number: 3000
      - path: /v1
        pathType: Prefix
        backend:
          service:
            name: veridactus-core
            port:
              number: 8080
      - path: /api
        pathType: Prefix
        backend:
          service:
            name: veridactus-cp
            port:
              number: 8081
```

---

## Configuration Reference

### Environment Variables

#### veridactus-core

| Variable | Default | Description |
|----------|---------|-------------|
| `VERIDACTUS_ADMIN_KEY` | - | Admin authentication key |
| `VERIDACTUS_MODE` | `passthrough` | Operation mode: passthrough/governance |
| `VERIDACTUS_UPSTREAM_URL` | - | LLM API base URL |
| `VERIDACTUS_UPSTREAM_KEY` | - | LLM API key |
| `VERIDACTUS_CONTROL_PLANE_URL` | `http://localhost:8081` | Control plane URL |
| `RUST_LOG` | `info` | Log level |
| `VERIDACTUS_STORE_REDIS_HOST` | `localhost` | Redis host |
| `VERIDACTUS_STORE_REDIS_PORT` | `6379` | Redis port |
| `VERIDACTUS_STORE_POSTGRES_HOST` | `localhost` | PostgreSQL host |
| `VERIDACTUS_STORE_POSTGRES_PORT` | `5432` | PostgreSQL port |
| `VERIDACTUS_STORE_POSTGRES_DB` | `veridactus` | Database name |
| `VERIDACTUS_STORE_POSTGRES_USER` | `veridactus` | Database user |
| `VERIDACTUS_STORE_POSTGRES_PASSWORD` | - | Database password |
| `VERIDACTUS_STORE_BACKEND` | `memory` | Storage backend: memory/file/postgres |
| `VERIDACTUS_STORE_S3_ENDPOINT` | - | S3/MinIO endpoint (e.g., http://minio:9000) |
| `VERIDACTUS_STORE_S3_BUCKET` | - | S3 bucket name |
| `VERIDACTUS_STORE_S3_ACCESS_KEY` | - | S3 access key |
| `VERIDACTUS_STORE_S3_SECRET_KEY` | - | S3 secret key |
| `VERIDACTUS_STORE_S3_REGION` | `us-east-1` | S3 region |

#### veridactus-cp

| Variable | Default | Description |
|----------|---------|-------------|
| `VERIDACTUS_CP_PORT` | `8081` | HTTP server port |
| `VERIDACTUS_CP_DB_PATH` | `./veridactus.db` | SQLite database path |
| `VERIDACTUS_CP_ADMIN_KEY` | - | Admin authentication key |
| `VERIDACTUS_CP_CORS_ORIGINS` | `*` | Allowed CORS origins |

---

## Monitoring and Logging

### Health Checks

```bash
# Data plane
curl http://localhost:8080/health

# Control plane
curl http://localhost:8081/api/v1/health

# Python worker
curl http://localhost:8002/health
```

### Metrics Endpoint

```bash
# Prometheus metrics
curl http://localhost:8080/metrics
```

### Structured Logging

```json
{
  "timestamp": "2024-01-15T10:30:00Z",
  "level": "info",
  "service": "veridactus-core",
  "trace_id": "abc123",
  "message": "Request processed",
  "duration_ms": 150,
  "model": "gpt-4",
  "status": "success"
}
```

---

## Troubleshooting

### Common Issues

#### 1. Control Plane Not Reaching Data Plane

```bash
# Check connectivity
docker exec veridactus-cp curl http://veridactus-core:8080/health

# Check logs
docker logs veridactus-cp | grep "data plane"
```

#### 2. Database Connection Issues

```bash
# Test PostgreSQL connection
docker exec veridactus-core psql \
  -h postgres -U veridactus -d veridactus \
  -c "SELECT 1"
```

#### 3. Budget Not Enforcing

```bash
# Check pipeline configuration
curl -H "X-Admin-Key: your-key" \
  http://localhost:8081/api/v1/pipelines

# Verify plugin execution in logs
docker logs veridactus-core | grep "BudgetGuard"
```

### Debug Mode

```bash
# Enable debug logging
RUST_LOG=debug docker-compose -f deploy/docker-compose.yml up -d

# View detailed logs
docker logs -f veridactus-core
```

---

## Next Steps

- [Plugin Development](../architecture/PLUGIN_SYSTEM.md) - Create custom plugins
- [API Reference](../api/OVERVIEW.md) - API documentation
- [Architecture Overview](../architecture/ARCHITECTURE.md) - System design
