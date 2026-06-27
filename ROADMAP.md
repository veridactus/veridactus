# VERIDACTUS Roadmap

This document outlines the planned development trajectory for the VERIDACTUS project. Priorities are driven by community feedback, security research, and the evolving AI governance landscape.

---

## v0.3.0 (Q3 2026) — Community & Production Readiness

**Goal**: First community-contributable release with production deployment tooling.

| Area | Deliverable | Status |
|:---|:---|:---|
| **Multi-Tenant Isolation** | Full workspace-level data isolation (traces, conversations, pipelines, models) | ✅ Done |
| **Dual-Engine Frontend** | Chat safety sandbox + Dev Hub playground + Holo-Trace Vault | ✅ Done |
| **Enterprise SSO** | Email/password registration with JWT auth, 5-role RBAC | ✅ Done |
| **Helm Charts** | Complete `deploy/helm/` with working templates + values for Kubernetes deployment | 📋 Planned |
| **Docker** | Multi-arch images (amd64 + arm64) published to `ghcr.io` | 📋 Planned |
| **Tests** | Go unit tests, TypeScript component tests, Python worker tests | 📋 Planned |
| **Docs** | Multi-language documentation site (zh-CN + en-US) | 📋 Planned |

---

## v0.4.0 (Q4 2026) — Advanced Governance

**Goal**: Production-grade governance features and ecosystem growth.

| Area | Deliverable | Status |
|:---|:---|:---|
| **RBAC** | 5-role access control (platform_admin/org_admin/workspace_admin/developer/auditor) | ✅ Done |
| **FinOps** | Micro-dollar billing with Stripe integration + wallet management | 🚧 In Progress |
| **L3 Proof** | Hardware TEE attestation (Intel TDX / AMD SEV-SNP) with remote verification | 🔬 Research |
| **gRPC Plugins** | External governance plugin SDK for custom safety filters | 📋 Planned |
| **Prometheus** | Native metrics exporter with Grafana dashboards | 📋 Planned |
| **SLSA L3** | Build provenance attestation and signed releases | 📋 Planned |

---

## v1.0.0 (H1 2027) — Stable Protocol

**Goal**: First stable release with long-term support commitments.

| Area | Deliverable | Status |
|:---|:---|:---|
| **Protocol Freeze** | VERIDACTUS Protocol v1.0 specification finalized and stable | 🔬 Drafting |
| **Backward Compatibility** | Guaranteed API stability for data plane and control plane | 📋 Planned |
| **LTS** | 18-month long-term support for v1.0.x | 📋 Planned |
| **Formal Verification** | TLA+ or Coq models for critical protocol properties | 🔬 Research |
| **Ecosystem** | 3+ independent implementations passing conformance | 📋 Planned |

---

## Ongoing Initiatives

These efforts span multiple releases and are continuously improved:

| Initiative | Description | Progress |
|:---|:---|:---|
| **Security Hardening** | Regular `cargo audit` + `npm audit`, fuzzing critical paths, third-party pentesting | 🟢 Active |
| **Documentation** | Improving API docs, adding architecture decision records (ADRs), onboarding guides | 🟡 In Progress |
| **Community Building** | Growing maintainer base, establishing SIGs, hosting community calls | 🟡 In Progress |
| **Performance** | Benchmarking pipeline throughput, optimizing S3/Redis store adapters | 🔵 Planned |
| **Spec Evolution** | RFC-driven protocol evolution through TSC governance | 🟢 Active |

---

## How to Influence the Roadmap

1. **Open an RFC** — Substantive feature proposals go through the [RFC process](https://github.com/veridactus/veridactus/tree/main/veridactus/rfcs)
2. **Join Discussions** — Share use cases and requirements on [GitHub Discussions](https://github.com/veridactus/veridactus/discussions)
3. **Vote** — React with 👍 on existing roadmap issues to signal demand
4. **Contribute** — Pick up a `help-wanted` issue and submit a PR

---

*Last updated: 2026-06-07*
