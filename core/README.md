# VERIDACTUS Data Plane (Rust)

[![License](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](../LICENSE)
[![Rust](https://img.shields.io/badge/Rust-1.75%2B-orange)](https://www.rust-lang.org/)

The VERIDACTUS Data Plane is an OpenAI-compatible HTTP/SSE proxy server that enforces governance policies, generates cryptographic proofs, and records auditable execution traces.

## Features

- **OpenAI-compatible API** — Drop-in replacement at `/v1/chat/completions`
- **Governance Pipeline** — 7 production plugins (BudgetGuard, PiiDetector, InputSanitizer, ResponseValidator, G1-G3 guardrails)
- **Cryptographic Proofs** — L0 hash chain → L2A Merkle sampling → L2B zero-knowledge proofs
- **Streaming Budget Control** — Real-time SSE budget enforcement with awareness events
- **Active Prevention** — DFA-based constrained decoding (PII, credentials, dangerous code)
- **Privacy Tiers** — Raw → Masked → Hash-Only → TEE-Private
- **Delegation Validation** — Composite attestation (Ed25519 + TEE + ZK)

## Quick Start

```bash
# Build
cargo build --release

# Run (requires upstream LLM configured)
VERIDACTUS_ADMIN_KEY="your-admin-key" cargo run --release

# Test
cargo test --lib

# Lint
cargo clippy --all-targets
cargo fmt --check
```

## Environment Variables

| Variable | Default | Description |
|:---|:---|:---|
| `VERIDACTUS_ADMIN_KEY` | auto-generated | Admin key for API authentication |
| `UPSTREAM_URL` | `http://localhost:8000` | Default upstream LLM base URL |
| `ZHIPU_API_KEY` | (from main.rs) | Zhipu AI API key |
| `CONTROL_PLANE_URL` | `http://localhost:8081` | Control plane URL for config sync |
| `RUST_LOG` | info | Logging level (trace/debug/info/warn/error) |

## Architecture

```
src/
├── http/           # Axum HTTP/SSE server (server, headers, streaming, error handling)
├── plugin/         # Governance plugins (7 production plugins + trait)
├── pipeline/       # Pipeline compiler + executor
├── crypto/         # JCS canonicalization, L0/L2A/L2B proofs, Merkle tree, ZK
├── types/          # Trace, Proof, Constraints, Error data types
├── store/          # Trace/config/budget/cache/object stores (Memory, PG, Redis, S3)
├── hooks/          # 9 semantic lifecycle hooks
├── governance_dsl/ # YAML policy DSL parser/compiler/validator
└── main.rs         # Entry point
```

## API Endpoints

| Endpoint | Method | Description |
|:---|:---|:---|
| `/health` | GET | Health check |
| `/models` | GET | List models |
| `/v1/chat/completions` | POST | OpenAI-compatible chat (with governance) |
| `/v1/traces` | GET | List/search traces |
| `/v1/traces/:id` | GET | Get single trace |
| `/v1/gdpr/delete` | POST | GDPR right-to-erasure |
| `/v1/compliance/report/:id` | GET | Compliance report |
| `/v1/prevention/stats` | GET | Prevention statistics |
| `/metrics` | GET | Prometheus metrics |
| `/.well-known/veridactus-extensions.json` | GET | Extension discovery |

## Proof Chain

Each request generates a layered proof chain:

| Level | Type | Description |
|:---|:---|:---|
| **L0** | Hash Chain | SHA-256 integrity via JCS canonicalization |
| **L2A** | Merkle Sampling | Random path verification (IMMACULATE framework) |
| **L2B** | Zero-Knowledge | NANOZK-style layered commitment proofs |

## Governance Plugins

| Plugin | Stage | Description |
|:---|:---|:---|
| `budget-guard` | pre_request | Micro-dollar budget control |
| `pii-detector` | pre_request | PII detection & masking |
| `input-sanitizer` | pre_request | Prompt injection defense |
| `g1-input-filter` | pre_request | OWASP input safety guard |
| `g2-output-filter` | post_response | Harmful content guard |
| `g3-semantic-guard` | post_response | Factual consistency guard |
| `response-validator` | post_response | Schema validation |
