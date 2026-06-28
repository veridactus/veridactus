# VERIDACTUS Documentation

**Trusted AI Execution Governance Infrastructure**

VERIDACTUS is a cloud-native AI governance layer that sits between your application and any LLM, providing cryptographic audit trails, multi-tenant isolation, safety filtering, and cost control.

## Architecture Overview

```
                          ┌─────────────────┐
                          │  Client (curl,   │
                          │  SDK, Web UI)    │
                          └────────┬────────┘
                                   │
                    ┌──────────────┼──────────────┐
                    ▼              ▼              ▼
          ┌─────────────┐ ┌─────────────┐ ┌─────────────┐
          │ veridactus-ui│ │veridactus-cp│ │veridactus-  │
          │ React + Vite │ │ Go REST API │ │core (Rust)  │
          │ (:3000)      │ │ (:8081)     │ │ (:8080)     │
          └──────┬───────┘ └──────┬──────┘ └──────┬──────┘
                 │                │                │
                 └────────────────┼────────────────┘
                                  │
                          ┌───────┴───────┐
                          │  PostgreSQL   │
                          │ (unified)     │
                          └───────────────┘
```

## Key Features

| Category | Features |
|----------|----------|
| **Governance** | 7-plugin pipeline (Budget Guard, PII Detector, Input Sanitizer, Response Validator, G1/G2/G3 filters) |
| **Multi-Tenant** | Strict workspace-level data isolation for traces, conversations, pipelines, and models |
| **Cryptographic Proof** | L0 JCS+SHA-256 hash chain, L2A Merkle tree sampling, L2B ZK proofs |
| **Safety** | Dynamic PII detection shield, OWASP ASI Top 10-aligned filters, prompt injection prevention |
| **Audit** | Holo-Trace Vault with session-grouped browsing, cryptographic self-verification |
| **FinOps** | Streaming budget guard, micro-dollar cost tracking, wallet management |

## Documentation Structure

```
docs/
├── README.md                         # This file — overview
├── architecture/
│   ├── ARCHITECTURE.md               # System architecture & component design
│   └── PLUGIN_SYSTEM.md             # Plugin development guide
├── api/
│   ├── OVERVIEW.md                   # API endpoints overview
│   └── openapi.yaml                  # OpenAPI 3.0 specification
└── VERIDACTUS-架构与插件体系详解.md    # Chinese architecture deep-dive
```

## Quick Links

- [Main README](../README.md) — Project overview, quick start, business scenarios
- [Architecture Guide](architecture/ARCHITECTURE.md) — Component design & data flow
- [API Overview](api/OVERVIEW.md) — All REST endpoints
- [OpenAPI Spec](api/openapi.yaml) — Machine-readable API specification

## License

Apache License 2.0 — See [LICENSE](../LICENSE).
