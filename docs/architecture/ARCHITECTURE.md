# VERIDACTUS Architecture

This document provides a comprehensive overview of the VERIDACTUS system architecture, design decisions, and component interactions.

## Table of Contents

1. [System Overview](#system-overview)
2. [Component Architecture](#component-architecture)
3. [Data Flow](#data-flow)
4. [Configuration System](#configuration-system)
5. [Security Model](#security-model)
6. [Storage Architecture](#storage-architecture)

---

## System Overview

VERIDACTUS implements a **microservices architecture** with four primary components:

### Components

| Component | Technology | Port | Responsibility |
|-----------|------------|------|----------------|
| **veridactus-core** | Rust + Axum | 8080 | AI proxy gateway (OpenAI-compatible), governance pipeline execution, cryptographic trace signing, model routing |
| **veridactus-cp** | Go + PostgreSQL | 8081 | Multi-tenant management (Org/Workspace), JWT auth + RBAC, pipeline/model CRUD, virtual keys, wallets, conversations, config polling |
| **veridactus-ui** | React + Vite + TypeScript | 3000 | Chat safety sandbox, Dev Hub playground, Holo-Trace Vault, pipeline designer, model & API key management, admin settings |
| **veridactus-python-worker** | Python + FastAPI | 8001 | Enhanced computation: compliance reports, ZK proof generation (optional) |

### Design Principles

1. **Separation of Concerns**: Control plane handles configuration; data plane handles execution
2. **Multi-Tenant Isolation**: Strict workspace-level data isolation at every API layer (CP handler + DP trace tenant_id)
3. **Fail-Safe Defaults**: Plugins degrade gracefully when external services fail
4. **Cryptographic Audit**: All traces are cryptographically signed with JCS+SHA-256 for tamper evidence
5. **Dynamic Configuration**: Pipeline changes take effect without restart via config polling

---

## Component Architecture

### Data Plane (veridactus-core)

The data plane is implemented in Rust for high performance and memory safety.

```
┌─────────────────────────────────────────────────────────────────────────┐
│                           Data Plane Architecture                        │
└─────────────────────────────────────────────────────────────────────────┘

  HTTP Request
       │
       ▼
┌─────────────────┐
│   Axum Router   │
│  /v1/chat/*    │
│  /v1/traces/*  │
│  /health       │
└────────┬────────┘
         │
         ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                      Request Processing Pipeline                         │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│  ┌─────────────┐   ┌─────────────┐   ┌─────────────┐   ┌─────────────┐ │
│  │  Auth       │ → │  Idempotency│ → │  Budget     │ → │  DSL        │ │
│  │  Validation │   │  Guard      │   │  Check      │   │  Compiler   │ │
│  └─────────────┘   └─────────────┘   └─────────────┘   └─────────────┘ │
│         │                                                      │         │
│         │              ┌──────────────────────────────────────┘         │
│         │              │                                                │
│         ▼              ▼                                                │
│  ┌─────────────────────────────────────────────────────────────────┐   │
│  │                    Pipeline Executor                             │   │
│  │  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐       │   │
│  │  │ Pre-Req  │→ │Streaming │→ │Post-Rsp  │→ │ Async    │       │   │
│  │  │ (Serial) │  │(Parallel)│  │(Serial)  │  │(Parallel)│       │   │
│  │  └──────────┘  └──────────┘  └──────────┘  └──────────┘       │   │
│  └─────────────────────────────────────────────────────────────────┘   │
│         │                                                              │
│         ▼                                                              │
│  ┌─────────────┐   ┌─────────────┐   ┌─────────────┐                   │
│  │  Upstream  │ → │  Response   │ → │  Trace      │                   │
│  │  LLM Proxy │   │  Scanner    │   │  Signer     │                   │
│  └─────────────┘   └─────────────┘   └─────────────┘                   │
│                                                                          │
└─────────────────────────────────────────────────────────────────────────┘
```

### Control Plane (veridactus-cp)

The control plane is implemented in Go for simplicity and uses PostgreSQL for production persistence (with SQLite available as a lightweight development option).

```
┌─────────────────────────────────────────────────────────────────────────┐
│                         Control Plane Architecture                       │
└─────────────────────────────────────────────────────────────────────────┘

  Admin Dashboard
       │
       │ REST API (X-Admin-Key auth)
       ▼
┌─────────────────┐
│   HTTP Router   │
│  /api/v1/*     │
└────────┬────────┘
         │
         ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                         Service Layer                                    │
├─────────────────────────────────────────────────────────────────────────┤
│  ┌───────────┐  ┌───────────┐  ┌───────────┐  ┌───────────┐            │
│  │ Pipeline  │  │  Model    │  │  Plugin   │  │   API     │            │
│  │ Service   │  │  Service  │  │  Service  │  │   Key     │            │
│  │           │  │           │  │           │  │  Service  │            │
│  └─────┬─────┘  └─────┬─────┘  └─────┬─────┘  └─────┬─────┘            │
│        │              │              │              │                   │
│        └──────────────┴──────────────┴──────────────┘                   │
│                              │                                          │
│                              ▼                                          │
│                     ┌────────────────┐                                  │
│                     │   Store Layer  │                                  │
│                     │  (PostgreSQL) │                                  │
│                     └────────────────┘                                  │
└─────────────────────────────────────────────────────────────────────────┘
         │
         │ Push/Poll Config
         ▼
┌─────────────────┐
│  Config Sync    │
│  Service        │
└─────────────────┘
```

### Frontend (veridactus-ui)

The frontend is a React 18 SPA (Vite + TypeScript) with multiple engines:

- **VERIDACTUS Chat**: Safety sandbox with dynamic PII detection shield, pipeline selector, model picker, multi-conversation sidebar, and real-time streaming with VERIDACTUS governance protocol headers
- **Dev Hub Playground**: Three-panel developer workspace with prompt editor, streaming output, and X-Ray diagnostics panel
- **Holo-Trace Vault**: Audit center with session-grouped trace browsing, cryptographic signature verification, compliance status, cost/token tracking, and per-trace detail view
- **Pipeline Studio**: Visual pipeline designer and editor for governance pipeline stages
- **Model Management**: CRUD for LLM model configurations with workspace-level isolation
- **API Key Management**: Create, view, and rotate API keys per workspace

---

## Data Flow

### Request Processing Flow

```
┌─────────────────────────────────────────────────────────────────────────┐
│                        Request Processing Flow                            │
└─────────────────────────────────────────────────────────────────────────┘

  Client Request
       │
       ├─ HTTP Headers (VERIDACTUS-*)
       │   • VERIDACTUS-Budget-Limit
       │   • VERIDACTUS-Privacy-Level
       │   • VERIDACTUS-Guardrails
       │   • VERIDACTUS-Action
       │
       └─ Request Body (JSON)
           • model
           • messages
           • veridactus_dsl (optional)
               • intents
               • constraints
               • preferences

       │
       ▼
┌─────────────────────────────────────────────────────────────────────────┐
│ Phase 1: Request Preprocessing                                            │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│  1. Auth Validation                                                       │
│     • Extract API key from Authorization header                          │
│     • Validate key exists and is active                                  │
│     • Check tenant permissions                                           │
│                                                                          │
│  2. Idempotency Check                                                    │
│     • Extract X-Idempotency-Key header                                   │
│     • Check Redis for existing response                                  │
│     • Return cached response if found                                    │
│                                                                          │
│  3. Budget Check (Pre-Request Plugins)                                   │
│     • Check daily/request budget limits                                 │
│     • Determine strategy (hard_stop/degrade/adaptive)                   │
│                                                                          │
│  4. Privacy Processing                                                   │
│     • Detect PII in input messages                                      │
│     • Apply configured privacy level                                    │
│                                                                          │
└─────────────────────────────────────────────────────────────────────────┘
       │
       ▼
┌─────────────────────────────────────────────────────────────────────────┐
│ Phase 2: Upstream Proxy                                                  │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│  5. Model Routing                                                        │
│     • Select upstream model based on routing rules                      │
│     • Apply API key for upstream                                        │
│     • Transform request to upstream format                              │
│                                                                          │
│  6. Streaming Proxy                                                      │
│     • Forward request to upstream                                       │
│     • Stream response back to client                                    │
│     • Execute streaming plugins in real-time                            │
│                                                                          │
└─────────────────────────────────────────────────────────────────────────┘
       │
       ▼
┌─────────────────────────────────────────────────────────────────────────┐
│ Phase 3: Response Processing                                             │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│  7. Output Scanning                                                      │
│     • G2 Content filtering                                              │
│     • PII detection in response                                         │
│     • Safety event generation                                           │
│                                                                          │
│  8. Trace Finalization                                                   │
│     • Create trace record with all metadata                            │
│     • Compute L0 cryptographic signature                               │
│     • Store trace in configured backend                                │
│                                                                          │
│  9. Async Processing (Background)                                       │
│     • Submit tasks to Redis Stream                                      │
│     • Python worker consumes and processes                             │
│     • Update trace with results                                         │
│                                                                          │
└─────────────────────────────────────────────────────────────────────────┘
       │
       ▼
  Client Response
       │
       ├─ HTTP Response Headers
       │   • VERIDACTUS-Trace-Id
       │   • VERIDACTUS-Proof-Levels
       │   • VERIDACTUS-Cost-Consumed
       │   • VERIDACTUS-Version
       │
       └─ Response Body (JSON)
           • id
           • choices
           • usage
           • (streamed tokens)
```

### Configuration Synchronization Flow

```
┌─────────────────────────────────────────────────────────────────────────┐
│                   Configuration Synchronization Flow                     │
└─────────────────────────────────────────────────────────────────────────┘

  Control Plane                    Data Plane
       │                                │
       │  Admin updates config          │
       │  via REST API                  │
       │                                │
       │  ┌───────────────┐             │
       │  │   CRUD Ops    │             │
       │  │   + Version   │             │
       │  │   Increment   │             │
       │  └───────┬───────┘             │
       │          │                     │
       │          ▼                     │
       │  ┌───────────────┐             │
       │  │    SQLite     │             │
       │  │  (Persistent) │             │
       │  └───────────────┘             │
       │                                │
       │         ┌─────────────────────┼─────────────────┐
       │         │                     │                 │
       │         │ Push                │ Poll            │ Pull
       │         │                     │                 │
       │         ▼                     │                 ▼
       │  ┌───────────────┐    ┌───────────────┐  ┌───────────────┐
       │  │ POST /sync    │    │  GET /poll    │  │ Config Store  │
       │  │ (immediate)   │    │  (long poll)  │  │ (in-memory)   │
       │  └───────────────┘    └───────────────┘  └───────────────┘
       │                                │
       │                                ▼
       │                      ┌───────────────────┐
       │                      │  Apply Changes     │
       │                      │  Update Pipeline   │
       │                      │  Executor          │
       │                      └───────────────────┘
       │                                │
       ▼                                ▼
```

---

## Configuration System

### Three-Layer Constraint Hierarchy

VERIDACTUS implements a hierarchical constraint configuration system:

```
┌─────────────────────────────────────────────────────────────────────────┐
│                    Configuration Hierarchy                               │
└─────────────────────────────────────────────────────────────────────────┘

  Layer 1: HTTP Request Headers (Most Flexible)
  ┌─────────────────────────────────────────────────────────────────────┐
  │  VERIDACTUS-Budget-Limit: 0.10                                      │
  │  VERIDACTUS-Budget-Strategy: hard_stop                              │
  │  VERIDACTUS-Privacy-Level: masked                                   │
  │  VERIDACTUS-Guardrails: G1,G2,G3                                   │
  │  VERIDACTUS-Instruction-Hierarchy: strict                          │
  │  VERIDACTUS-Compliance-Profile: EU_AI_ACT_GPAI                      │
  │  VERIDACTUS-Action: save-baseline                                   │
  └─────────────────────────────────────────────────────────────────────┘
                              │
                              ▼ (Overridden by)
  Layer 2: Request Body DSL
  ┌─────────────────────────────────────────────────────────────────────┐
  │  {                                                                     │
  │    "model": "glm-5.1",                                               │
  │    "veridactus_dsl": {                                               │
  │      "intents": { "budget_outcome": "cost_effective" },              │
  │      "constraints": {                                                │
  │        "budget": { "limit_usd": 0.05, "strategy": "hard_stop" },   │
  │        "privacy": { "level": "masked" },                            │
  │        "guardrails": { "levels": ["G1", "G2"], "strictness": "high" }│
  │      }                                                               │
  │    }                                                                 │
  │  }                                                                   │
  └─────────────────────────────────────────────────────────────────────┘
                              │
                              ▼ (Overridden by)
  Layer 3: Pipeline Preset (Admin Configured)
  ┌─────────────────────────────────────────────────────────────────────┐
  │  Pipeline "production-default"                                       │
  │  ┌──────────────────────────────────────────────────────────────┐  │
  │  │  Stage: pre_request                                           │  │
  │  │  ├─ BudgetGuardPlugin: { limit_usd: 0.10, strategy: "hard" }  │  │
  │  │  ├─ PiiDetectorPlugin: { action: "mask" }                    │  │
  │  │  └─ AuthValidatorPlugin: {}                                    │  │
  │  └──────────────────────────────────────────────────────────────┘  │
  └─────────────────────────────────────────────────────────────────────┘
                              │
                              ▼ (Overridden by)
  Layer 4: System Defaults
  ┌─────────────────────────────────────────────────────────────────────┐
  │  budget: { limit_usd: 1.0, strategy: "degrade" }                     │
  │  privacy: { level: "raw" }                                          │
  │  guardrails: { enabled: ["G1"] }                                    │
  └─────────────────────────────────────────────────────────────────────┘
```

---

## Security Model

### Authentication

| Component | Method | Description |
|-----------|--------|-------------|
| **Data Plane** | API Key | Bearer token in Authorization header |
| **Control Plane** | Admin Key | X-Admin-Key header |
| **Python Worker** | None | Internal service only |

### Authorization

- **Tenant Isolation**: API keys are scoped to tenants
- **Role-Based Access**: Admin keys have full access; API keys are limited to trace operations
- **Capability Negotiation**: Client/server negotiate supported capabilities

### Privacy Levels

| Level | Description | PII Handling |
|-------|-------------|--------------|
| `raw` | No privacy processing | Pass-through |
| `masked` | Mask PII in storage | Replace with `[REDACTED]` |
| `hash_only` | Store only hashes | SHA-256 of PII |
| `tee_private` | TEE-protected storage | Hardware isolation |

---

## Storage Architecture

### Supported Backends

| Backend | Use Case | Configuration |
|---------|----------|---------------|
| **PostgreSQL** | Production — trace persistence + CP business data | `DATABASE_URL` / `VERIDACTUS_STORE_BACKEND=postgres` |
| **In-Memory** | Development / testing | `VERIDACTUS_STORE_BACKEND=memory` |
| **SQLite** | Lightweight dev mode (CP only) | `STORE_BACKEND=sqlite` |

### Trace Storage Schema

```sql
CREATE TABLE traces (
    trace_id       TEXT PRIMARY KEY,
    tenant_id      TEXT NOT NULL,
    model          TEXT NOT NULL,
    request_hash   TEXT NOT NULL,
    response_hash  TEXT NOT NULL,
    state          TEXT NOT NULL,
    signature      TEXT,
    cost_usd       REAL,
    created_at     TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    metadata       JSONB
);

CREATE INDEX idx_traces_tenant ON traces(tenant_id);
CREATE INDEX idx_traces_created ON traces(created_at DESC);
```

---

## Next Steps

- [Plugin System](PLUGIN_SYSTEM.md) - Learn how to develop plugins
- [Deployment Guide](../deployment/DEPLOYMENT.md) - Deploy VERIDACTUS
- [API Reference](../api/OVERVIEW.md) - API documentation
