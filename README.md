# VERIDACTUS — Trusted AI Execution Governance Infrastructure

[![License](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](LICENSE)
[![Protocol](https://img.shields.io/badge/Protocol-v0.3.0-blue)](SPECIFICATION.md)
[![Rust](https://img.shields.io/badge/Rust-1.75%2B-orange)](core/)
[![Go](https://img.shields.io/badge/Go-1.21%2B-00ADD8)](control-plane/)

**VERIDACTUS** is the constitutional blueprint for trustworthy AI governance. It transforms probabilistic LLM interactions into **independently-auditable, replay-deterministic, and delegation-traceable engineering events** through a standardized, cryptographically-verifiable interface.

> 📖 [Full Specification](SPECIFICATION.md) | 🏗 [Architecture Guide](docs/architecture/ARCHITECTURE.md) | 📋 [Changelog](CHANGELOG.md) | 📡 [OpenAPI Spec](docs/api/openapi.yaml)

---

## Architecture

```
┌──────────────────────────────────────────────────────────────────┐
│                        VERIDACTUS v0.3.0                          │
├──────────────┬─────────────────────┬──────────────────────────────┤
│  React UI    │  Go Control Plane   │  Rust Data Plane             │
│  (:3000)     │  (:8081)            │  (:8080)                     │
│              │                     │                              │
│  Chat 沙箱   │  Multi-tenant       │  AI Proxy Gateway            │
│  Developer   │  Org/Workspace      │  Governance Pipeline         │
│  Hub         │  Virtual Keys       │  L0/L2A/L2B Crypto Proofs    │
│  Trace Vault │  Wallet/Billing     │  Stream Budget Guard         │
│  Pipeline    │  Audit/Compliance   │  Key Resolution              │
│  Studio      │  SSO/Brand          │  Redis Budget (Lua)          │
└──────────────┴─────────────────────┴──────────────┬───────────────┘
                                                     │
                                    ┌────────────────┼────────────────┐
                                    │ PostgreSQL │ Redis │ MinIO(S3)  │
                                    │ (业务数据) │ (预算) │ (对象存储) │
                                    └────────────────┴────────────────┘
```

## Key Features

- **Multi-Tenant Architecture**: Organization → Workspace → Virtual Key 三级隔离
- **Dual-Engine Frontend**: VERIDACTUS Chat (安全沙箱) + Developer Hub (全息调试台) + Holo-Trace Vault
- **Cryptographic Proof Chain (L0 → L2B)**: Hash chain integrity (L0) → Merkle tree sampling (L2A) → Zero-Knowledge proofs (L2B)
- **Dynamic Safety Shield**: Real-time PII detection with animated shield (6 PII types + 5 injection patterns)
- **Crypto Self-Verification**: Browser-side Web Crypto API JCS+SHA-256 verification with particle burst animation
- **Dual-Key System**: BYOK (Bring Your Own Key) with AES-256-GCM encryption + Platform Unified Pool
- **FinOps Engine**: Micro-dollar precision wallet + Redis Lua atomic budget deduction + Stream guard
- **Enterprise SSO**: GitHub/Email/Phone registration + Okta/Azure/Feishu SSO configuration
- **Compliance Mapping**: EU AI Act / GDPR / NIST AI 600-1 auto-generated compliance reports
- **Auditor Command Center**: 7 event types + risk distribution visualization
- **OWASP ASI Top 10 Aligned**: Full coverage of OWASP Agentic AI Security risks ASI01-ASI10

## Quick Start

### Prerequisites
- Rust 1.75+ (data plane)
- Go 1.21+ (control plane)
- Node.js 20+ (UI)
- Docker + Docker Compose (PostgreSQL, Redis, MinIO)

### 1. Start Infrastructure
```bash
docker compose -f deploy/docker-compose.yml up -d postgres redis minio
```

### 2. Start Control Plane
```bash
cd control-plane
go build -o bin/control-plane ./cmd/server/
STORE_BACKEND=postgres PG_HOST=localhost PG_PORT=5432 ./bin/control-plane
# API available at http://localhost:8081
```

### 3. Start Data Plane
```bash
cd core
ZHIPU_API_KEY="your-key" UPSTREAM_URL="https://open.bigmodel.cn" cargo run --bin veridactus-core
# Proxy available at http://localhost:8080
```

### 4. Start UI
```bash
cd veridactus-ui
npm install && npm run dev
# UI available at http://localhost:3000
```

## Proof Levels

| Level | Type | Description | Status |
|:---|:---|:---|:---|
| **L0** | Hash Chain | SHA-256 integrity via JCS canonicalization | ✅ Production |
| **L1** | TEE Attestation | Hardware enclave attestation | 🟡 Type defs |
| **L2A** | Sampling Verification | Merkle tree random sampling | ✅ Production |
| **L2B** | Zero-Knowledge | NANOZK-style layered commitment proofs | ✅ Production |

## Project Structure

```
veridactus/
├── core/                    # Rust Data Plane
│   ├── src/http/            # Axum HTTP/SSE server
│   ├── src/plugin/          # 7 production governance plugins
│   ├── src/pipeline/        # Pipeline compiler + executor
│   ├── src/crypto/          # JCS + L0/L2A/L2B proofs
│   ├── src/budget/          # StreamBudgetGuard + Redis Lua
│   ├── src/keymanager/      # Key resolution client
│   └── src/observability/   # Prometheus metrics
├── control-plane/           # Go Control Plane
│   ├── internal/
│   │   ├── auth/            # JWT, OAuth, RBAC, Email/Phone auth
│   │   ├── crypto/          # AES-256-GCM envelope encryption
│   │   ├── store/           # PostgreSQL + SQLite dual backend
│   │   └── model/           # 20+ data models
│   └── cmd/server/          # Router + 30+ REST endpoints
├── veridactus-ui/           # React Frontend (Vite)
│   ├── auth/                # Login + Onboarding
│   ├── engines/chat/        # VERIDACTUS Chat + SafetyShield + ABCompare
│   ├── engines/vault/       # Holo-Trace Vault + CryptoVerify
│   ├── engines/devhub/      # Developer Hub Playground + XRayPanel
│   └── admin/               # Brand Settings
├── python-worker/           # Python enhanced computation
├── scripts/
│   ├── redis/               # budget_decr.lua + rate_limit.lua
│   ├── security-audit.sh    # Security audit script
│   ├── e2e-smoke.sh         # E2E smoke test
│   └── pre-commit           # Pre-commit hook
└── deploy/
    ├── docker-compose.yml   # Full stack deployment
    ├── clickhouse-schema.sql # OLAP analytics schema
    └── helm/                # Kubernetes Helm charts
```

## License

Apache License 2.0 — See [LICENSE](LICENSE) for details.  
Copyright 2026 The VERIDACTUS Authors.
