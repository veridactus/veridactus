# Changelog

All notable changes to VERIDACTUS will be documented in this file.

## [0.3.0] - 2026-06-25

### Added — Multi-Tenant Foundation
- Organization → Workspace → User 三级多租户隔离
- PostgreSQL + SQLite 双存储后端
- GitHub OAuth 登录 + Email/Password 注册 + 手机验证码
- JWT 认证中间件 + 5 角色 RBAC 权限矩阵
- Virtual Key BYOK 双轨制 (AES-256-GCM 信封加密)
- Platform Unified LLM Master Key 池
- Wallet 钱包 + Transaction 交易记录
- Stripe Checkout 计费集成
- Redis Lua 原子预算扣减 + 流式熔断
- `/internal/resolve-key` Key 路由解析端点

### Added — Frontend Dual-Engine
- VERIDACTUS Chat 安全沙箱 (SSE 流式 + 模型选择器)
- 🛡️ 动态安全盾牌 (6 PII + 5 注入实时检测)
- ⚔️ A/B 双模型对比模式
- Developer Hub Playground (三栏布局 + X-Ray Panel)
- Holo-Trace Vault 全息证据金库
- 🔗 密码学自证 (Web Crypto API + JCS + 粒子爆发)
- 上帝视角分屏 (Raw/Sanitized 滚动百分比同步)
- Onboarding 引导页 (BYOK vs Platform 双卡片 + Lottie 动画)
- 审计指挥舱 (7 事件类型 + 风险分布)
- 品牌白标定制 (Logo/主题色/CSS Variables)

### Added — Enterprise & Production
- SSO 配置端点 (Okta/Azure/飞书/钉钉)
- 合规报告生成 (Merkle Root + ZK 证明 + 离线验证脚本)
- ClickHouse OLAP Schema (审计事件 + 聚合物化视图)
- Prometheus 指标导出 (28 metrics)
- Security audit script + Pre-commit hook
- E2E smoke test script
- OpenAPI 3.0 规范文档

### Changed
- 控制面从单文件重构为分层架构 (internal/store, auth, crypto, model)
- 前端路由重组为双引擎架构
- Python Worker 增强 drift-detection (余弦相似度)

### Security
- 移除所有硬编码 API Key
- AES-256-GCM 信封加密用于 LLM Key 存储
- 常数时间验证码比较防止时序攻击
- Public endpoints 正确绕过 AdminKey 中间件

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
