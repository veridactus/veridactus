# VERIDACTUS

**Trusted AI Execution Governance Framework**

VERIDACTUS is a cloud-native AI proxy gateway that provides comprehensive governance, traceability, and cryptographic proof for large language model (LLM) interactions.

## Overview

VERIDACTUS implements a three-tier service architecture with a monitoring layer:

```
┌─────────────────────────────────────────────────────────────────────────┐
│                              Clients                                     │
│                    curl / SDK / Web UI / CI/CD Pipeline                  │
└──────────────────────┬──────────────────────────────┬───────────────────┘
                      │                              │
                      ▼                              ▼
┌──────────────────────────────────┐    ┌─────────────────────────────────┐
│        veridactus-ui (:3000)     │    │      veridactus-core (:8080)    │
│        React + Vite Frontend     │    │      Rust Data Plane            │
│                                  │    │      AI Proxy Gateway           │
│  • Dashboard                     │    │                                 │
│  • Pipeline Designer             │    │  ┌─────────────────────────────┐ │
│  • Model Routing                 │    │  │  Synchronous (ms)           │ │
│  • API Key Management            │    │  │  1. Request Parsing         │ │
│  • Plugin Market                 │    │  │  2. Idempotency Check       │ │
│  • Audit Center                  │    │  │  3. Version Negotiation     │ │
│  • System Settings               │    │  │  4. DSL Compilation         │ │
└──────────────────┬───────────────┘    │  │  5. Constraint Detection    │ │
                   │ REST API          │  │  6. Plugin Execution        │ │
                   ▼                   │  │  7. Upstream LLM Proxy      │ │
┌──────────────────────────────────┐    │  │  8. G2 Output Scanning      │ │
│    veridactus-cp (:8081)         │    │  │  9. L0/L2A Proof Generation│ │
│    Go Control Plane              │    │  └─────────────────────────────┘ │
│    Configuration Management       │    │                                 │
│                                  │    │  Asynchronous (Background):      │
│  • Pipelines CRUD               │    │  • L2B ZK Proof Framework       │
│  • Models CRUD                  │    │  • C-SafeGen Certified Guarantee│ │
│  • API Keys CRUD                │    │  • Fairness Audit               │
│  • Plugins/Policies             │    │  • Compliance Report Generation  │
│  • Config Version Polling        │    │  • Redis Stream Task Dispatch   │
│  • SQLite Persistence           │    │                                 │
└──────────────────────────────────┘    └──────────────┬──────────────────┘
                                                      │ HTTP/gRPC
                                                      ▼
                                    ┌─────────────────────────────────────┐
                                    │     veridactus-python-worker (:8002) │
                                    │     Python + FastAPI                 │
                                    │     Enhanced Computation (Optional)   │
                                    │                                      │
                                    │  • Deep PII Detection                │
                                    │  • C-SafeGen Multi-dimensional      │
                                    │    Safety Scoring                    │
                                    │  • Semantic Drift Detection          │
                                    │  • Redis Stream Async Consumer       │
                                    └─────────────────────────────────────┘
```

## Key Features

### Governance Layer

| Feature | Description |
|---------|-------------|
| **Budget Control** | Per-request and daily budget limits with configurable strategies |
| **Privacy Protection** | PII detection and masking for personal identifiers |
| **Guardrails** | Input injection prevention, output content filtering |
| **Semantic Consistency** | Cross-validation of outputs against baselines |

### Traceability

| Feature | Description |
|---------|-------------|
| **Execution Traces** | Complete request/response logging with cryptographic signatures |
| **Replay Engine** | Reproducible execution with branching support |
| **Replay Verification** | Cryptographic proof of trace integrity |

### Compliance

| Feature | Description |
|---------|-------------|
| **GDPR Compliance** | Right to deletion with cryptographic proof |
| **EU AI Act GPAI** | Compliance profile for General Purpose AI |
| **NIST AI RMP** | Risk management framework alignment |

### Plugin Architecture

| Type | Latency | Technology | Use Cases |
|------|---------|------------|-----------|
| **Native** | <10μs | Compiled into Rust binary | Budget Guard, Auth Validator |
| **WASM** | 50-200μs | WASM runtime with sandbox | Keyword Guardrail, PII Masking |
| **gRPC** | 5-500ms | External service calls | Drift Detector, TEE Attestation |

## Quick Start

### Prerequisites

- Docker and Docker Compose
- Redis (for production)
- PostgreSQL (for production)

### Using Docker Compose

```bash
# Clone the repository
git clone https://github.com/veridactus/veridactus.git
cd veridactus

# Start all services
docker-compose -f deploy/docker-compose.yml up -d

# Verify health
curl http://localhost:8080/health
curl http://localhost:8081/api/v1/health
```

### Manual Deployment

See [Deployment Guide](docs/deployment/DEPLOYMENT.md) for detailed instructions.

## Documentation Structure

```
docs/
├── README.md                    # This file
├── architecture/
│   ├── ARCHITECTURE.md         # System architecture overview
│   ├── PLUGIN_SYSTEM.md        # Plugin development guide
│   └── SECURITY.md             # Security model and threat analysis
├── deployment/
│   ├── DEPLOYMENT.md           # Deployment guide
│   ├── DOCKER.md               # Docker deployment
│   └── KUBERNETES.md           # Kubernetes deployment
├── api/
│   ├── OVERVIEW.md             # API overview
│   ├── data-plane/             # Data plane API specs
│   ├── control-plane/           # Control plane API specs
│   └── python-worker/           # Python worker API specs
├── development/
│   ├── CONTRIBUTING.md         # Contribution guidelines
│   ├── CODING_STANDARDS.md     # Coding standards
│   └── TESTING.md              # Testing guide
└── specification/
    └── v0.2.1/                 # Protocol specifications
```

## Architecture Highlights

### Three-Layer Constraint Configuration

```
Request Header (Most Flexible) > DSL (Request Body) > Pipeline Preset (Admin) > System Default
```

Clients can dynamically set constraints per request:

```bash
curl -X POST http://localhost:8080/v1/chat/completions \
  -H "VERIDACTUS-Budget-Limit: 0.10" \
  -H "VERIDACTUS-Privacy-Level: masked" \
  -H "VERIDACTUS-Guardrails: G1,G2" \
  -d '{"model": "glm-5.1", "messages": [...]}'
```

### Four-Stage Pipeline

```
Pre-Request (Serial)          Streaming (Parallel)         Post-Response (Serial)    Async (Background)
─────────────────────────────────────────────────────────────────────────────────────────────────────────
┌─────────────────────┐   ┌─────────────────────┐   ┌─────────────────────┐   ┌─────────────────────┐
│ Budget Guard        │   │ Keyword Guardrail   │   │ Trace Finalizer     │   │ Drift Detector      │
│ Auth Validator      │   │ PII Masking         │   │ Response Validator  │   │ C-SafeGen           │
│ Route Selector      │   │                     │   │                     │   │ TEE Attestation      │
└─────────────────────┘   └─────────────────────┘   └─────────────────────┘   └─────────────────────┘
```

### Cryptographic Proof

Every trace is cryptographically signed using:
- JCS (JSON Canonicalization Scheme)
- SHA-256 hashing
- Merkle tree aggregation

## Docker Images

All images are published to Docker Hub:

| Image | Description |
|-------|-------------|
| `veridactus/veridactus-core` | Rust data plane |
| `veridactus/veridactus-cp` | Go control plane |
| `veridactus/veridactus-ui` | React frontend |
| `veridactus/veridactus-python-worker` | Python worker (optional) |

## License

Apache License 2.0 - See [LICENSE](LICENSE) for details.

## Contributing

See [Contributing Guide](docs/development/CONTRIBUTING.md) for development setup and contribution guidelines.
