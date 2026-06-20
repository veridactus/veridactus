# Contributing to VERIDACTUS

Thank you for your interest in contributing! VERIDACTUS follows a structured RFC process for substantive changes and welcomes bug fixes, documentation improvements, and feature enhancements.

## Code of Conduct

Please read and follow our [Code of Conduct](CODE_OF_CONDUCT.md).

## How to Contribute

### 1. Find or Create an Issue

- Check [existing issues](https://github.com/veridactus/veridactus/issues) before creating new ones
- Use the issue templates for bug reports, feature requests, or documentation
- For security vulnerabilities, follow the [Security Policy](SECURITY.md) — **do not file a public issue**

### 2. Development Setup

```bash
# Clone the monorepo
git clone https://github.com/veridactus/veridactus.git
cd veridactus

# Build everything
make build

# Start infrastructure (PostgreSQL, Redis, MinIO)
make infra

# Run all tests
make test

# Start all services locally
make dev
```

### 3. Make Changes

- Create a feature branch: `git checkout -b feature/your-feature-name`
- Follow the coding style for each component:
  - **Rust** (`core/`): `cargo fmt` + `cargo clippy`
  - **Go** (`control-plane/`): `gofmt` + `go vet`
  - **TypeScript** (`veridactus-ui/`): `npx tsc --noEmit` + `npx prettier --check .`
- Write tests for new functionality
- Update documentation if needed

### DCO (Developer Certificate of Origin)

All contributions must be signed off to certify that you have the right to submit the work under the project's open source license. **Every commit must include a `Signed-off-by:` line.**

```bash
# Sign off your commits automatically:
git commit -s -m "your commit message"

# Or add the sign-off manually to the commit message:
# Signed-off-by: Your Name <your.email@example.com>
```

By signing off, you acknowledge the [Developer Certificate of Origin](https://developercertificate.org/) (DCO v1.1):
- You created the contribution, or have the right to submit it
- You are not violating anyone's intellectual property rights
- The contribution is provided under the project's license (Apache 2.0)

### 4. Before Submitting a PR

```bash
make check   # Format + lint all components
make build   # Build all components
make test    # Run all tests
make e2e     # Run end-to-end tests
```

### 5. Pull Request Process

1. Fill in the PR template
2. Ensure all CI checks pass
3. Request review from a maintainer
4. Address review feedback
5. Squash merge after approval

## RFC Process (Substantive Changes)

For new features, protocol changes, or architectural changes, an RFC (Request for Comments) is required:

1. Create an RFC document in `veridactus/rfcs/` following the [RFC template](veridactus/rfcs/0000-template.md)
2. Open a Pull Request with the RFC
3. The RFC will be open for community feedback (14-30 days)
4. The Technical Steering Committee (TSC) reviews and votes
5. Approved RFCs are merged and scheduled for implementation

## Component Ownership

| Component | Language | Directory | Code Owners |
|:---|:---|:---|:---|
| Data Plane | Rust | `core/` | @veridactus/data-plane |
| Control Plane | Go | `control-plane/` | @veridactus/control-plane |
| Frontend | TypeScript/React | `veridactus-ui/` | @veridactus/ui |
| Protocol Spec | Markdown/MDX | `veridactus/docs/` | @veridactus/tsc |
| Python Worker | Python | `python-worker/` | @veridactus/worker |
| Helm Charts | YAML | `deploy/` | @veridactus/devops |

## Release Process

See [GOVERNANCE.md](GOVERNANCE.md) for the full release and versioning process.

- **PATCH** (0.2.1→0.2.2): Bug fixes, small improvements
- **MINOR** (0.2→0.3): New features, non-breaking changes
- **MAJOR** (0.x→1.0): Breaking changes

## Getting Help

- **GitHub Discussions**: For questions and general discussion
- **GitHub Issues**: For bugs and feature requests
- **Email**: tsc@veridactus.ai for governance or security matters
