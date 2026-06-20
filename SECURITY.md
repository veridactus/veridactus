# Security Policy

## Supported Versions

| Version | Supported | Status |
|:---|:---|:---|
| 0.2.1 | ✅ | Current release |
| 0.1.x | ❌ | Deprecated |

## Reporting a Vulnerability

The VERIDACTUS project takes security seriously. We appreciate your efforts to responsibly disclose vulnerabilities.

### Process

1. **DO NOT** file a public GitHub issue for security vulnerabilities.
2. Send a detailed report to **tsc@veridactus.ai** with:
   - Description of the vulnerability
   - Steps to reproduce
   - Potential impact
   - Suggested fix (if available)
3. You will receive an acknowledgment within **48 hours**.
4. We will provide a timeline for the fix within **7 business days**.

### Disclosure Policy

- We follow a **90-day coordinated disclosure** timeline.
- After the fix is released, we will publish a security advisory.
- Credit will be given to the reporter (unless anonymity is requested).

## Security Design

VERIDACTUS implements a **defense-in-depth** architecture:

| Layer | Mechanism |
|:---|:---|
| Cryptographic | L0 hash chain + L2A Merkle sampling + L2B ZK proofs |
| Active Prevention | DFA-based constrained decoding (PII, credentials, dangerous code, agent hijack) |
| Privacy | 4-tier privacy model (raw/masked/hash_only/tee_private) |
| Guardrails | G1-G4 multi-level safety filters aligned with OWASP ASI Top 10 |
| Instruction Hierarchy | P0-P2 priority system with verified mode |
| Audit | RFC 8785 JCS canonicalization + SHA-256 audit trails |

## Security Best Practices for Deployers

1. 🔐 Use TLS 1.2+ for all external communications
2. 🔑 Store API keys in environment variables, never in config files
3. 🛡️ Restrict CORS to known origins in production
4. 📊 Enable Prometheus metrics for anomaly detection
5. 🔄 Rotate audit tokens regularly
6. 🗄️ Configure TTL-based trace expiration (GDPR compliance)
7. 🚫 Never expose the control plane API (port 8081) to public networks
8. 📝 Regularly run `cargo audit` and `npm audit` for dependency vulnerabilities

## Known Limitations

- L1 (TEE Attestation) requires hardware TEE support (Intel TDX / AMD SEV-SNP / NVIDIA CC)
- The `tee_private` privacy mode requires a TEE-enabled deployment
- ZK proof generation (L2B) adds 500-2000ms latency per request
- Budget metering is proxy-side estimation; final cost depends on upstream LLM billing

## Security Contacts

- Technical Steering Committee: **tsc@veridactus.ai**
- Security issues: **security@veridactus.ai**
- PGP Key: Available upon request
