# Changelog

All notable changes to VERIDACTUS will be documented in this file.

## [0.2.1] - 2026-06-07

### Added
- L2A Merkle tree sampling verification proofs
- L2B NANOZK-style layered zero-knowledge proofs
- Governance DSL compilation integration into request flow
- `save-baseline`, `drift-test`, `audit-export` action implementations
- Global regex registry with OnceLock singletons (PII, injection, dangerous code patterns)
- GitHub Actions CI/CD pipeline (Rust + Go + TypeScript + E2E)
- OpenAPI 3.0 API specification
- Graceful shutdown (SIGTERM/SIGINT) for data plane
- AsyncWriteQueue shutdown with pending batch flush
- LRU eviction for InMemoryTraceStore (10K default capacity)
- X-Admin-Key authentication middleware for control plane
- UI admin key support via URL parameter / localStorage
- Budget awareness SSE events in streaming handler
- Constrained decoding integration in streaming handler
- Hash-only privacy level with SHA-256 content hashing
- Hook dispatch points (pre_execute, post_stream, on_failure)
- Degrade model fallback logic for budget thresholds
- Ed25519 delegation token validation in governance handler
- Fairness check and compliance mapping in governance mode

### Fixed
- S3 adapter block_on deadlock (replaced with direct .await)
- Software TEE fixed seed (replaced with OsRng random key)
- HybridTraceStore double-write for large traces
- Passthrough mode missing API key header for authenticated upstreams
- Default pipeline plan plugin name mismatch (budget→budget-guard)
- Idempotency key not reused as trace_id (causing 502 instead of 409)
- Go control plane migration error silent ignoring
- Go control plane API key plaintext logging
- CORS wildcard (*) restricted to configurable origin
- Hardcoded upstream IP replaced with environment variable
- server.rs model routes empty at startup (added default glm-5.1 route)

### Security
- Random API key generation via crypto/rand
- Sensitive credentials moved to environment variables
- Admin authentication middleware for management API
- VeridactusStreamHandler channel explicit close
- LRU memory store eviction prevents OOM

## [0.2.0] - 2026-05-12

### Added
- Initial release of VERIDACTUS protocol v0.2.0
- Rust data plane with OpenAI-compatible proxy
- Go control plane with REST API + SQLite
- React frontend with pipeline designer
- L0 hash chain proof generation
- 7 production governance plugins
- G1-G4 guardrail framework
- OWASP ASI Top 10 alignment
- GDPR right-to-erasure support
- Prometheus metrics export
