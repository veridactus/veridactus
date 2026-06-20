# VERIDACTUS Project Governance

VERIDACTUS is governed by a Technical Steering Committee (TSC) under the Apache 2.0 license.

## Technical Steering Committee (TSC)

The TSC is responsible for:
- Protocol specification maintenance
- RFC review and approval
- Release management
- Ecosystem integration decisions
- Security vulnerability triage

### Current TSC Members

| Name | Role | GitHub |
|:---|:---|:---|
| William Lee | Protocol Architecture, Cryptographic Audit | @william-lee |
| *[Open]* | Conformance Testing | — |
| *[Open]* | Enterprise Adoption | — |

### TSC Decision Process

- Routine decisions: Lazy consensus (no objections within 72 hours)
- RFCs: Formal vote, decision within 14 business days
- Security-sensitive RFCs: Extended review (30-day minimum), supermajority (≥2/3)
- Release decisions: Simple majority

## Versioning Policy

Follows [Semantic Versioning 2.0.0](https://semver.org/):

| Bump | Scope | Examples |
|:---|:---|:---|
| PATCH | Bug fixes, docs, non-functional changes | `0.2.1 → 0.2.2` |
| MINOR | New features, new optional fields/headers | `0.2 → 0.3` |
| MAJOR | Breaking API changes, core schema changes | `0.x → 1.0` |

- `v0.x` core `required` fields are permanently frozen
- Breaking changes require a deprecation period of ≥1 MINOR version
- Protocol extensions use namespace isolation: `veridactus.ai/v{major}/{feature}`

## Release Process

1. **Code Freeze**: All features for the release are merged
2. **Release Candidate**: Tag `vX.Y.Z-rc1`, run full conformance suite
3. **Community Testing**: ≥7 days for minor, ≥14 days for major
4. **Final Release**: Tag `vX.Y.Z`, publish release notes
5. **Post-Release**: Update ecosystem adaptors, announce

## Conformance Certification

Implementations can self-certify using the public conformance test suite:

1. Run `conformance/run_tests.sh` in CI
2. Generate compliance report
3. Submit to ADOPTERS.md

Tiers:
- 🔵 **Core Compatible**: Schema + L0 proofs + headers
- 🟢 **Full Compatible**: Core + constraints + state machine + active prevention
- 🟡 **Extended Compatible**: Full + certified guarantees + agentic security + L2B ZK

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for the full contribution guide.

## License

All contributions are licensed under [Apache License 2.0](LICENSE).
