# VERIDACTUS — Trusted AI Execution Governance Infrastructure

[![License](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](LICENSE)
[![Protocol](https://img.shields.io/badge/Protocol-v0.2.1-blue)](veridactus/docs/specification/v0.2.1/)
[![Rust](https://img.shields.io/badge/Rust-1.75%2B-orange)](core/)
[![Go](https://img.shields.io/badge/Go-1.21%2B-00ADD8)](control-plane/)

**VERIDACTUS** is the constitutional blueprint for trustworthy AI governance. It transforms probabilistic LLM interactions into **independently-auditable, replay-deterministic, and delegation-traceable engineering events** through a standardized, cryptographically-verifiable interface.

> 📖 [Full Protocol Specification](veridactus/docs/specification/v0.2.1/) | 🏗 [Architecture Guide](veridactus/docs/) | 📋 [Changelog](veridactus/CHANGELOG.md)

---

## Architecture

```
┌─────────────┐     ┌──────────────────┐     ┌──────────────┐
│  React UI   │────▶│  Go Control Plane │────▶│ Rust Data    │
│  (:3000)    │     │  (:8081)          │     │ Plane (:8080)│
│  veridactus-│     │  control-plane/   │     │ core/        │
│  ui/        │     │  SQLite + REST    │     │ Pipeline +   │
└─────────────┘     └──────────────────┘     │ L0/L2A/L2B   │
                                             └──────┬───────┘
                                                    │
                                             ┌──────▼───────┐
                                             │ Upstream LLM │
                                             │ (OpenAI API) │
                                             └──────────────┘
```

## Key Features

- **Cryptographic Proof Chain (L0 → L2B)**: Hash chain integrity (L0) → Merkle tree sampling (L2A) → Zero-Knowledge proofs (L2B)
- **Streaming Budget Control**: Micro-dollar precision real-time SSE budget enforcement with awareness events
- **Active Prevention**: Token-level constrained decoding via DFA pattern matching (PII, credentials, dangerous code, agent hijack)
- **Privacy Tiers**: Raw → Masked → Hash-Only → TEE-Private with GDPR right-to-erasure
- **Governance Pipeline**: 7 production plugins (BudgetGuard, PiiDetector, InputSanitizer, ResponseValidator, G1-G3 guardrails)
- **OWASP ASI Top 10 Aligned**: Full coverage of OWASP Agentic AI Security risks ASI01-ASI10
- **Delegation Chain**: Composite attestation (Ed25519 + TEE + ZK) for multi-agent trust delegation
- **Compliance Mapping**: Auto-generated EU AI Act / NIST AI 600-1 compliance reports per inference

## Quick Start

### Prerequisites
- Rust 1.75+ (data plane)
- Go 1.21+ (control plane)
- Node.js 20+ (UI)
- Docker + Docker Compose (infrastructure: PostgreSQL, Redis, MinIO)

### 1. Start Infrastructure
```bash
docker-compose -f scripts/docker-compose.yml up -d
```

### 2. Start Control Plane
```bash
cd control-plane
go run cmd/server/main.go
# API available at http://localhost:8081
```

### 3. Start Data Plane
```bash
cd core
cargo run --release
# Proxy available at http://localhost:8080
```

### 4. Start UI (optional)
```bash
cd veridactus-ui
npm install && npm run dev
# UI available at http://localhost:3000
```

### 5. Quick Test
```bash
curl -X POST http://localhost:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $VERIDACTUS_ADMIN_KEY" \
  -d '{
    "model": "deepseek-r1:14b",
    "messages": [{"role":"user","content":"hello"}],
    "max_tokens": 10
  }'
```

## Project Structure

```
veridactus/
├── core/                 # Rust Data Plane (proxy + pipeline + crypto)
│   ├── src/http/         # Axum HTTP/SSE server
│   ├── src/plugin/       # Governance plugins (7 production plugins)
│   ├── src/pipeline/     # Pipeline compiler + executor
│   ├── src/crypto/       # JCS + L0 signature + Merkle + ZK proofs
│   ├── src/types/        # Trace, Proof, Constraints, Error types
│   └── src/store/        # Trace/config/budget/cache/object stores
├── control-plane/        # Go Control Plane (REST API + SQLite)
├── veridactus-ui/        # React frontend (ReactFlow pipeline designer)
├── veridactus/           # Protocol specification + docs + RFCs
├── python-worker/        # Python PII detection worker
├── deploy/               # Helm charts for Kubernetes
└── scripts/              # Docker Compose + E2E test scripts
```

## Proof Levels

| Level | Type | Description | Status |
|:---|:---|:---|:---|
| **L0** | Hash Chain | SHA-256 integrity via JCS canonicalization | ✅ Production |
| **L1** | TEE Attestation | Hardware enclave attestation (Intel TDX, AMD SEV-SNP, NVIDIA CC) | 🟡 Type defs |
| **L2A** | Sampling Verification | Merkle tree random sampling (IMMACULATE framework) | ✅ Production |
| **L2B** | Zero-Knowledge | NANOZK-style layered commitment proofs | ✅ Production |

## Governance Plugins

| Plugin | Stage | Description |
|:---|:---|:---|
| `BudgetGuard` | pre_request | Micro-dollar budget control ($0.000001 precision) |
| `PiiDetector` | pre_request | PII detection & masking (regex + NER patterns) |
| `InputSanitizer` | pre_request | Prompt injection & jailbreak defense |
| `G1InputFilter` | pre_request | OWASP-aligned input safety guard |
| `G2OutputFilter` | post_response | Harmful content output guard |
| `G3SemanticGuard` | post_response | Factual consistency & domain rules |
| `ResponseValidator` | post_response | JSON schema validation for structured outputs |

## Documentation

- [Protocol Specification v0.2.1](veridactus/docs/specification/v0.2.1/)
- [Architecture Guide](docs/architecture/ARCHITECTURE.md)
- [Plugin System Guide](docs/architecture/PLUGIN_SYSTEM.md)
- [Deployment Guide](docs/deployment/DEPLOYMENT.md)
- [API Overview](docs/api/OVERVIEW.md)
- [Contributing Guide](CONTRIBUTING.md)
- [Security Policy](SECURITY.md)
- [Code of Conduct](CODE_OF_CONDUCT.md)
- [Changelog](veridactus/CHANGELOG.md)

## Docker Images

All images are published to Docker Hub under the `veridactus` organization:

| Image | Description | Tags |
|-------|-------------|------|
| `veridactus/veridactus-core` | Rust data plane (AI proxy gateway) | `latest`, `main-*`, `develop-*` |
| `veridactus/veridactus-cp` | Go control plane (configuration management) | `latest`, `main-*`, `develop-*` |
| `veridactus/veridactus-ui` | React frontend (admin dashboard) | `latest`, `main-*`, `develop-*` |
| `veridactus/veridactus-python-worker` | Python worker (enhanced PII detection) | `latest`, `main-*`, `develop-*` |

### Quick Deployment

```bash
# Pull and run with Docker Compose
curl -O https://raw.githubusercontent.com/veridactus/veridactus/main/deploy/docker-compose.yml
docker-compose up -d
```

## License

Apache License 2.0 — See [LICENSE](LICENSE) for details.

Copyright 2026 The VERIDACTUS Authors.
