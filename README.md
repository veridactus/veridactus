# VERIDACTUS — Trusted AI Execution Governance Infrastructure

[![License](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](LICENSE)
[![Protocol](https://img.shields.io/badge/Protocol-v0.3.0-blue)](SPECIFICATION.md)
[![Rust](https://img.shields.io/badge/Rust-1.75%2B-orange)](core/)
[![Go](https://img.shields.io/badge/Go-1.21%2B-00ADD8)](control-plane/)
[![React](https://img.shields.io/badge/React-18-61DAFB)](veridactus-ui/)

**VERIDACTUS** transforms every LLM interaction into a **cryptographically-verifiable, independently-auditable, and tamper-evident engineering event**. It is the governance layer that sits between your application and any AI model — ensuring compliance, safety, and accountability without sacrificing developer experience.

> 📖 [Full Specification](SPECIFICATION.md) | 🏗 [Architecture Guide](docs/architecture/ARCHITECTURE.md) | 📡 [OpenAPI Spec](docs/api/openapi.yaml)

---

## Why VERIDACTUS?

**The Problem**: Organizations are deploying LLMs into production without any audit trail, cost control, or safety guarantees. Every AI call is a black box — you don't know what happened, who paid for it, or whether it was safe.

**The Solution**: VERIDACTUS acts as a **cryptographic proxy** between your applications and AI models. Every request passes through a governance pipeline that enforces safety policies, tracks costs, and generates immutable proof chains. The result: **every AI interaction is independently verifiable — no trust required.**

### Business Scenarios

| Scenario | VERIDACTUS Role |
|----------|----------------|
| **Regulated Enterprise** (Finance, Healthcare, Legal) | L0-L2B cryptographic audit trail satisfies EU AI Act, GDPR, and NIST AI 600-1 compliance requirements |
| **SaaS AI Platform** | Multi-tenant isolation + per-tenant budgets + white-label branding for embedding AI into customer-facing products |
| **AI Safety Team** | Dynamic PII detection, budget guardrails, and OWASP ASI Top 10-aligned safety filters prevent prompt injection and data exfiltration |
| **DevOps/MLOps Pipeline** | Deterministic replay engine enables regression testing of AI behavior across model versions |
| **API Reseller / Aggregator** | Dual-key system (BYOK + Platform Pool) with micro-dollar FinOps billing and per-customer key management |

---

## Architecture

```
┌─────────────────────────────────────────────────────────────────────────┐
│                          VERIDACTUS v0.3.0                               │
├────────────────┬──────────────────────┬──────────────────────────────────┤
│  React UI      │  Go Control Plane    │  Rust Data Plane                 │
│  (:3000)       │  (:8081)             │  (:8080)                         │
│                │                      │                                  │
│  • Chat 沙箱   │  • Multi-Tenant      │  • AI Proxy Gateway (OpenAI API) │
│  • Dev Hub     │  • Organization/WS   │  • Governance Pipeline Executor  │
│  • Trace Vault │  • JWT Auth (OAuth)  │  • L0/L2A/L2B Crypto Proofs     │
│  • Pipeline UI │  • Virtual Keys      │  • Budget Guard (streaming)      │
│  • Model Mgmt  │  • Wallet/Billing    │  • Dynamic Safety Filters        │
│  • API Keys    │  • RBAC (5 roles)    │  • Deterministic Replay Engine   │
│  • Settings    │  • Config Poll       │  • ZK Proof Framework            │
│                │  • Conversations     │  • Config Sync Client            │
└────────────────┴──────────────────────┴──────────────┬───────────────────┘
                                                       │
                                          ┌────────────┴────────────┐
                                          │      PostgreSQL          │
                                          │  (unified data store)    │
                                          └─────────────────────────┘
```

**Storage**: PostgreSQL is the unified data store for CP business data (organizations, workspaces, users, pipelines, models, API keys, wallets, conversations) and DP trace data (dp_traces with JSONB trace_data). SQLite is available as a lightweight development option.

---

## Multi-Tenant Data Isolation

VERIDACTUS enforces **strict tenant isolation** at every layer:

```
Organization (org)
  ├── Workspace A (ws-A)      ← strictly isolated from ws-B
  │   ├── User 1 (personal)
  │   ├── Pipelines
  │   ├── Models
  │   └── Traces (tenant_id = ws-A)
  │
  └── Workspace B (ws-B)
      ├── User 2 (personal)
      ├── Pipelines
      ├── Models
      └── Traces (tenant_id = ws-B)
```

| Isolation Layer | Mechanism |
|----------------|-----------|
| **Traces** | `VERIDACTUS-Workspace-Id` header → DP stores `tenant_id = workspace_id` → CP queries `WHERE tenant_id = $1` |
| **Conversations** | CP API validates `conv.UserID == request.UserID` at handler level |
| **Pipelines / Models** | CP API validates `existing.WorkspaceID == request.WorkspaceID` at handler level |
| **Enterprise** | Enterprise admin sees all traces across org workspaces via `WHERE tenant_id = ANY($1)` |
| **Personal** | Personal users see **only** their own workspace data — never other tenants' |

---

## Proof Levels

| Level | Type | Description | Status |
|:---|:---|:---|:---|
| **L0** | Hash Chain | SHA-256 JCS canonicalization + signature chain | ✅ Production |
| **L1** | TEE Attestation | Hardware enclave attestation (Intel TDX / AMD SEV) | 🟡 Planned |
| **L2A** | Sampling Verification | Merkle tree random sampling verification | ✅ Production |
| **L2B** | Zero-Knowledge | NANOZK-style layered commitment proofs | ✅ Production |

---

## Key Features

### 🔐 AI Governance Pipeline
7 production plugins in a composable pipeline: **Budget Guard**, **PII Detector**, **Input Sanitizer**, **Response Validator**, **G1 Input Filter**, **G2 Output Filter**, **G3 Semantic Guard**. Each plugin runs in configurable stages (pre-request, post-response, async).

### 📊 Holo-Trace Vault (Audit)
Every AI call generates an immutable **Trace** with full cryptographic proof chain. Browse by conversation session, drill into individual traces, verify L0 signatures, view compliance reports. Traces are grouped by conversation for intuitive navigation.

### 🛡️ Dynamic Safety Shield
Real-time PII detection with animated safety shield in the Chat interface. Detects 6 PII types (email, phone, credit card, SSN, IP address, API key) and 5 injection patterns.

### 💰 FinOps Budget Control
Micro-dollar precision cost tracking with streaming budget guard. Configurable per-request budget limits with automatic cutoff when exceeded.

### 🔑 Dual-Key System
- **BYOK** (Bring Your Own Key): AES-256-GCM encrypted, per-workspace API keys
- **Platform Pool**: Shared model pool managed by platform admin

### 🏢 Enterprise SSO & RBAC
5 role-based access control levels: **platform_admin**, **org_admin**, **workspace_admin**, **developer**, **auditor**. Email/password registration with JWT authentication (HS256, 15-min access + refresh tokens).

---

## Quick Start

### Prerequisites
- **Rust 1.75+** (Data Plane)
- **Go 1.21+** (Control Plane)
- **Node.js 20+** (UI)
- **PostgreSQL 14+** (production storage)

### One-Command Start (Recommended)
```bash
# Start everything: PostgreSQL, Control Plane, Data Plane, UI
./scripts/start-all.sh
```

### Manual Start

**1. PostgreSQL** (skip if using SQLite dev mode):
```bash
docker run -d --name veridactus-pg -e POSTGRES_USER=veridactus -e POSTGRES_PASSWORD=veridactus -e POSTGRES_DB=veridactus -p 5432:5432 postgres:14
```

**2. Control Plane** (:8081):
```bash
cd control-plane
cp .env.example .env  # edit with your config
go build -o bin/control-plane ./cmd/server/
./bin/control-plane
```

**3. Data Plane** (:8080):
```bash
cd core
# Set your LLM API key (Zhipu GLM-5.1 or compatible)
export ZHIPU_API_KEY="your-api-key"
cargo run --release --bin veridactus-core
```

**4. Frontend UI** (:3000):
```bash
cd veridactus-ui
npm install && npm run dev
```

Open **http://localhost:3000** — register an account and start chatting through the governance pipeline.

### Environment Variables

| Variable | Service | Description | Default |
|----------|---------|-------------|---------|
| `DATABASE_URL` | CP | PostgreSQL connection string | `postgres://veridactus:veridactus@localhost:5432/veridactus` |
| `STORE_BACKEND` | CP | Storage backend (`postgres` or `sqlite`) | `postgres` |
| `JWT_SECRET` | CP | HMAC-SHA256 signing key for JWT | auto-generated (persist in `.env`) |
| `ZHIPU_API_KEY` | DP | Zhipu GLM-5.1 API key | required for GLM routing |
| `VERIDACTUS_ADMIN_KEY` | CP+DP | Admin API key for internal calls | `veridactus-admin-dev-2026` |

---

## Project Structure

```
veridactus/
├── core/                         # Rust Data Plane
│   └── src/
│       ├── http/                 # Axum HTTP/SSE server + OpenAI-compatible API
│       │   ├── server.rs         # Main handler: chat completion + traces
│       │   └── headers.rs        # VERIDACTUS protocol header parsing
│       ├── plugin/               # 7 governance plugins (budget, PII, sanitizer, etc.)
│       ├── pipeline/             # Pipeline compiler + executor
│       ├── crypto/               # JCS canonicalization + L0/L2A/L2B proofs
│       ├── store/                # PostgreSQL trace storage adapter
│       ├── configsync/           # CP config poll client (pipelines + models)
│       └── types/                # Trace, Journal, Proof, ExecutionState
├── control-plane/                # Go Control Plane
│   ├── cmd/server/               # Router: 30+ REST endpoints
│   │   ├── router.go             # Route registration + handlers
│   │   └── enterprise.go         # SSO, audit events, compliance
│   └── internal/
│       ├── auth/                 # JWT middleware, OAuth, RBAC, email auth
│       ├── store/                # PostgreSQL + SQLite dual backend
│       │   ├── postgres.go       # 800+ lines of SQL queries
│       │   └── facade.go         # StoreFacade interface (40+ methods)
│       └── model/                # Organization, Workspace, Pipeline, Key, Wallet, etc.
├── veridactus-ui/                # React Frontend (Vite + TypeScript)
│   ├── auth/                     # Login, Onboarding, AuthGuard, useAuth
│   ├── engines/chat/             # VERIDACTUS Chat + SafetyShield
│   ├── engines/vault/            # Holo-Trace Vault + session grouping + crypto verify
│   ├── engines/devhub/           # Developer Hub Playground + XRayPanel
│   ├── pages/                    # Dashboard, Pipelines, Models, API Keys, Settings
│   ├── api/                      # API client + data transformers
│   └── components/               # Shared UI: Sidebar, GlassCard, Toast, Dialog, etc.
├── python-worker/                # Python async workers (compliance reports, ZK proofs)
├── proto/                        # gRPC/Protobuf definitions
├── deploy/                       # Docker Compose, Helm charts, SQL schemas
├── scripts/                      # start-all.sh, e2e tests, security audit
└── docs/                         # Architecture, API specs, guides
```

---

## Safety & Compliance Alignment

VERIDACTUS aligns with the **OWASP Top 10 for Agentic AI (ASI)**:

| OWASP ASI Risk | VERIDACTUS Mitigation |
|:---|:---|
| ASI-01: Prompt Injection | Input Sanitizer + G1 Input Filter |
| ASI-02: Sensitive Data Exposure | PII Detector (6 types) + Dynamic Shield |
| ASI-03: Supply Chain Risk | Pipeline versioning + Config Poll integrity |
| ASI-04: Excessive Agency | Budget Guard + action allowlisting |
| ASI-05: Output Handling | Response Validator + G2 Output Filter |
| ASI-06: Model Theft | API Key encryption + rate limiting |
| ASI-07: Insecure Plugin Design | Native plugins (compiled) + sandboxed execution |
| ASI-08: Data Poisoning | Deterministic replay for regression testing |
| ASI-09: Overreliance | Human-in-the-loop with audit trail |
| ASI-10: Unbounded Consumption | Streaming budget guard with real-time cutoff |

---

## Contributing

VERIDACTUS follows an open governance model. See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines and [GOVERNANCE.md](GOVERNANCE.md) for project roles.

---

## License

Apache License 2.0 — See [LICENSE](LICENSE).  
Copyright 2026 The VERIDACTUS Authors.
