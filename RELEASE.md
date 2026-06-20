# VERIDACTUS Release Process

This document describes how VERIDACTUS releases are created, validated, and published. For version numbering policy, see [GOVERNANCE.md](GOVERNANCE.md).

---

## Versioning

VERIDACTUS follows [Semantic Versioning 2.0.0](https://semver.org/):

| Bump | When | Example |
|:---|:---|:---|
| **MAJOR** | Breaking protocol changes, incompatible API changes | `0.2.1` → `1.0.0` |
| **MINOR** | New features, non-breaking protocol extensions | `0.2.1` → `0.3.0` |
| **PATCH** | Bug fixes, security patches, docs | `0.2.1` → `0.2.2` |

For `0.x` versions (pre-1.0), MINOR bumps may include minor breaking changes per the [SemVer spec §4](https://semver.org/#spec-item-4).

---

## Release Artifacts

A VERIDACTUS release includes the following artifacts:

| Artifact | Location | Notes |
|:---|:---|:---|
| Source tarball | GitHub Releases | Auto-generated |
| `veridactus-core` binary (Linux x86_64) | GitHub Releases | `core/target/release/veridactus-core` |
| `veridactus-cp` binary (Linux x86_64) | GitHub Releases | `control-plane/cmd/server/veridactus-cp` |
| Docker images | GitHub Container Registry | `ghcr.io/veridactus/veridactus-core:${TAG}` |
| Helm charts | `deploy/helm/` | Published via GitHub Pages |
| OpenAPI spec | `veridactus/api/openapi.yaml` | Included in source |
| Conformance vectors | `veridactus/conformance/v${VERSION}/` | Included in source |

---

## Release Checklist

### 1. Code Freeze (T-7 days)

- [ ] Announce freeze on GitHub Discussions and `#announcements`
- [ ] All release-blocking PRs merged to `main`
- [ ] `make ci` passes on `main` branch
- [ ] Security audit (`cargo audit`, `npm audit`) passes
- [ ] All conformance tests pass

### 2. Release Candidate (T-3 days)

- [ ] Create branch `release/v${VERSION}`
- [ ] Bump version numbers in:
  - `core/Cargo.toml` → `version = "${VERSION}"`
  - `control-plane/go.mod` → (if Go version changes)
  - `veridactus-ui/package.json` → `"version": "${VERSION}"`
  - `veridactus/package.json` → `"version": "${VERSION}"`
  - `README.md` → update version badge
- [ ] Update `CHANGELOG.md`:
  - Move `[Unreleased]` to `[${VERSION}] - YYYY-MM-DD`
  - Add GitHub compare link at the bottom
- [ ] Tag RC: `git tag -a v${VERSION}-rc1 -m "Release candidate v${VERSION}-rc1"`
- [ ] Push tag and branch
- [ ] Verify CI passes on RC tag

### 3. RC Validation (T-2 days)

- [ ] Community testing period (minimum 48 hours)
- [ ] Deploy RC to staging environment
- [ ] Run full E2E test suite
- [ ] Validate OpenAPI spec: `make validate-openapi`
- [ ] Validate JSON schemas: `make validate-schemas`
- [ ] Conformance test suite passes against RC
- [ ] Address any blocking issues found

### 4. Final Release (T-0)

- [ ] Merge `release/v${VERSION}` into `main`
- [ ] Tag final release: `git tag -a v${VERSION} -m "VERIDACTUS v${VERSION}"`
- [ ] Sign tag: `git tag -s v${VERSION}` (maintainer GPG key required)
- [ ] Push tag to GitHub
- [ ] Create GitHub Release with release notes from CHANGELOG.md
- [ ] Attach binary artifacts to GitHub Release
- [ ] Push Docker images to `ghcr.io`
- [ ] Publish Helm chart update
- [ ] Update documentation site (Mintlify)
- [ ] Announce on:
  - GitHub Discussions
  - Project mailing list (if applicable)
  - CNCF announce list (if applicable)

### 5. Post-Release

- [ ] Bump versions to next dev version on `main`:
  - `core/Cargo.toml` → `version = "${NEXT}-dev"`
  - `veridactus-ui/package.json` → `"version": "${NEXT}"`
- [ ] Add new `[Unreleased]` section to `CHANGELOG.md`
- [ ] Retrospective: document lessons learned

---

## Emergency Patch Releases

For critical security vulnerabilities or severe bugs:

1. Create branch `hotfix/v${VERSION}` from the release tag
2. Apply the fix
3. Follow the Release Candidate process (compressed to 24 hours)
4. Publish with `PATCH` version bump

---

## Signing Policy

All release tags **must** be signed with a maintainer's GPG key. The project uses:

- **GPG key fingerprint**: Listed in `MAINTAINERS.md`
- **Signing command**: `git tag -s v${VERSION}`
- **Verification**: `git tag -v v${VERSION}`

Consumers can verify binaries with:
```bash
sha256sum veridactus-core > veridactus-core.sha256
# Signatures published alongside release artifacts
```

---

## Supported Versions

| Version | Status | Security Patches Until |
|:---|:---|:---|
| 0.2.1 | ✅ Active | Next release + 3 months |
| 0.2.0 | ❌ EOL | — |

See [SECURITY.md](SECURITY.md) for the full support policy.

---

*Process last updated: 2026-06-07*
