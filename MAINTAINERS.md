# VERIDACTUS Maintainers

This file lists the maintainers for each component of the VERIDACTUS project.

## Core Maintainers

### Data Plane (Rust) — `core/`
- @william-lee — Architecture, cryptographic proofs, pipeline engine
- *[Open]* — Storage adapters, performance optimization

### Control Plane (Go) — `control-plane/`
- @william-lee — REST API, SQLite persistence, configuration management
- *[Open]* — PostgreSQL migration, multi-tenant support

### Frontend (React/TypeScript) — `veridactus-ui/`
- *[Open]* — Dashboard, pipeline designer, audit center

### Protocol Specification — `veridactus/docs/`
- @william-lee — Core protocol, data model, cryptographic audit
- *[Open]* — Conformance testing, compliance mappings

### Python Worker — `python-worker/`
- *[Open]* — PII detection, async task processing

### DevOps — `deploy/`, `scripts/`, `.github/`
- *[Open]* — CI/CD, Helm charts, Docker images

## Emeritus Maintainers

*None yet*

## Becoming a Maintainer

1. Consistently contribute high-quality PRs to a component
2. Participate in code reviews and RFC discussions
3. Be nominated by an existing maintainer
4. Approved by the TSC

## Maintainer Responsibilities

- Review PRs in their area within 5 business days
- Respond to security issues promptly
- Follow the [Code of Conduct](CODE_OF_CONDUCT.md)
- Participate in TSC discussions for their component
