# VERIDACTUS 全栈功能审计报告

> **审计日期**: 2026-06-26  
> **审计范围**: Go 控制面 / Rust 数据面 / Python Worker / React 前端  
> **审计方法**: 逐文件代码阅读 + 端到端 API 测试  
> **用途**: 供其他 AI / 审核人员交叉验证完整性

---

## 目录

1. [Go 控制面 (control-plane)](#1-go-控制面)
2. [Rust 数据面 (core)](#2-rust-数据面)
3. [Python Worker (python-worker)](#3-python-worker)
4. [React 前端 (veridactus-ui)](#4-react-前端)
5. [基础设施与部署](#5-基础设施与部署)
6. [已知限制 / 未实现项](#6-已知限制)

---

## 1. Go 控制面

**技术栈**: Go 1.24, net/http, lib/pq (PostgreSQL), mattn/go-sqlite3 (调试), go-redis/v9, casbin/v2  
**入口**: `control-plane/cmd/server/main.go`  
**端口**: 8081  
**存储后端**: PostgreSQL (生产默认), SQLite (开发调试, `STORE_BACKEND=sqlite`)

### 1.1 HTTP 端点完整清单

#### 公开端点（无需认证，AdminKey 中间件放行）

| 方法 | 路径 | Handler | 说明 |
|------|------|---------|------|
| GET | `/api/v1/health` | `handleHealth()` | 健康检查，返回 `{status,version,phase}` |
| GET | `/api/v1/config/poll` | `handleConfigPoll()` | 配置版本轮询（数据面使用） |

#### OAuth 认证端点（公开）

| 方法 | 路径 | 说明 |
|------|------|------|
| GET | `/api/v1/auth/login/github` | GitHub OAuth 登录入口 |
| GET | `/api/v1/auth/callback/github` | GitHub OAuth 回调 |
| GET | `/api/v1/auth/login/google` | Google OAuth 登录入口 |
| GET | `/api/v1/auth/callback/google` | Google OAuth 回调 |
| GET | `/api/v1/auth/login/wechat` | 微信扫码登录入口 |
| GET | `/api/v1/auth/callback/wechat` | 微信 OAuth 回调 |
| GET | `/api/v1/auth/wechat/callback-page` | 微信回调 HTML 页面（JWT localStorage 存储） |
| GET | `/api/v1/auth/wechat/status` | 微信扫码状态轮询 |
| POST | `/api/v1/auth/register` | 邮箱注册（RateLimitRegister 包裹） |
| POST | `/api/v1/auth/login` | 邮箱密码登录（RateLimitLogin 包裹） |
| POST | `/api/v1/auth/phone/send` | 发送手机验证码（RateLimitPhone 包裹） |
| POST | `/api/v1/auth/phone/verify` | 验证手机验证码 |
| POST | `/api/v1/auth/refresh` | 刷新 JWT Token |
| POST | `/api/v1/auth/logout` | 登出（吊销 Refresh Token） |
| POST | `/api/v1/auth/bind-phone` | 微信登录后绑定手机号 |

#### 受保护端点（JWT 中间件校验）

| 方法 | 路径 | 说明 |
|------|------|------|
| GET | `/api/v1/orgs` | 列出组织（平台管理员看全部，普通用户仅看自己所属） |
| GET | `/api/v1/orgs/{id}` | 获取组织详情（含显式所有权校验 `canAccessOrg`） |
| GET/POST | `/api/v1/workspaces` | 列出/创建工作空间 |
| GET | `/api/v1/workspaces/{id}` | 获取工作空间详情 |
| GET | `/api/v1/workspaces/members` | 列出工作空间成员 |
| GET/POST | `/api/v1/pipelines` | 列出/创建流水线 |
| GET | `/api/v1/pipelines/{id}` | 获取流水线详情（含显式所有权校验 `canAccessResource`） |
| GET | `/api/v1/plugins` | 列出插件 |
| GET | `/api/v1/policies` | 列出治理策略（按 workspace_id 过滤） |
| GET | `/api/v1/apikeys` | 列出 API 密钥 |
| GET | `/api/v1/apikeys/{id}` | 获取 API 密钥详情 |
| GET | `/api/v1/models` | 列出模型配置 |
| GET | `/api/v1/models/{id}` | 获取模型配置详情 |
| GET/POST | `/api/v1/virtual-keys` | 列出/创建虚拟密钥 |
| GET | `/api/v1/virtual-keys/{id}` | 获取虚拟密钥详情 |
| GET/POST | `/api/v1/wallets` | 获取钱包/充值 |
| GET/PUT | `/api/v1/settings` | 获取/更新工作空间设置 |
| GET | `/api/v1/traces` | 列出 Trace 记录 |

#### 管理端点（AdminKey 或 JWT）

| 方法 | 路径 | 说明 |
|------|------|------|
| GET/POST/DELETE | `/api/v1/platform/pool` | Platform LLM Pool 管理 |
| GET | `/api/v1/platform/models` | 列出可用平台模型 |
| POST | `/api/v1/billing/checkout` | Stripe Checkout（Stub 实现） |
| POST | `/api/v1/billing/webhook` | Stripe Webhook（Stub 实现） |
| GET/PUT | `/api/v1/enterprise/sso` | SSO 配置管理 |
| GET | `/api/v1/audit/events` | 审计事件列表 |
| POST | `/api/v1/compliance/reports` | 合规报告生成 |
| GET/PUT | `/api/v1/brand` | 品牌白标设置（Logo/主题色/名称） |

#### 内部端点（数据面调用，公开）

| 方法 | 路径 | 说明 |
|------|------|------|
| POST | `/internal/resolve-key` | 虚拟密钥解析 → 真实 Provider Key |
| POST | `/api/v1/traces/update` | 数据面回调更新 Trace 状态 |

### 1.2 中间件链

启动顺序（`main.go` 第 73-81 行）：
```
CORSMiddleware → JWTMiddleware → AdminKeyMiddleware → RequestLogger
```

| 中间件 | 文件 | 功能 |
|--------|------|------|
| `CORSMiddleware` | `middleware.go` | CORS 头、OPTIONS 预检 |
| `JWTMiddleware` | `middleware.go` | 从 `Authorization: Bearer <token>` 提取验证 JWT，注入 context |
| `AdminKeyMiddleware` | `middleware.go` | 公开路径放行；已有 JWT claims 则跳过；否则校验 `X-Admin-Key` |
| `RequestLogger` | `middleware.go` | 请求日志（方法/路径/用户） |
| `RequireRole` | `middleware.go` | 角色守卫中间件（已定义，部分 handler 使用） |

### 1.3 认证模块 (`internal/auth/`)

#### 1.3.1 OAuth Providers

| Provider | 文件 | 实现 | 状态 |
|----------|------|------|------|
| **GitHubProvider** | `oauth.go` | OAuth 2.0 完整流程（token 交换 + 用户信息 + 邮箱回退） | ✅ 生产就绪 |
| **GoogleProvider** | `oauth.go` | OpenID Connect（token 交换 + OIDC UserInfo） | ✅ 生产就绪 |
| **WeChatProvider** | `wechat.go` | 微信开放平台扫码登录 + 状态轮询 + 手机绑定 | ✅ 生产就绪 |

#### 1.3.2 JWT

| 功能 | 文件 | 详情 |
|------|------|------|
| `GenerateAccessToken` | `jwt.go` | HS256 签名，15 分钟 TTL，Claims: `user_id, email, org_id, workspace_id, role, plan` |
| `ValidateAccessToken` | `jwt.go` | 验证签名 + 过期检查 |
| `GenerateRefreshToken` | `jwt.go` | 30 天 TTL，存储于 `refresh_tokens` 表 |

#### 1.3.3 RBAC (Casbin)

| 角色 | 权限摘要 |
|------|----------|
| `platform_admin` | `*:*` 通配 |
| `org_admin` | 继承 workspace_admin + `org:*`, `workspace:*`, `member:*`, `billing:*`, `settings:*` |
| `workspace_admin` | 继承 developer + `pipeline:*`, `apikey:*`, `virtual_key:*`, `trace:*`, `member:read\|invite` |
| `developer` | `pipeline:read`, `plugin:read`, `model:read`, `chat:use`, `playground:use` |
| `auditor` | `trace:read\|export`, `compliance:read\|export`, `audit:read\|export` |

#### 1.3.4 安全功能

| 功能 | 文件 | 详情 |
|------|------|------|
| **Redis 限流** | `redis_limiter.go` | Lua 令牌桶，`rate:login:*` / `rate:register:*` / `rate:phone:*` keys |
| **内存限流（降级）** | `secure.go` | 内存 Token Bucket，Redis 不可用时自动降级 |
| **密码强度** | `secure.go` | ≥8 字符，大写+小写+数字+特殊字符，常见密码黑名单 |
| **账户锁定** | `secure.go` | 5 次失败锁定 15 分钟，基于 PG settings 表 |

### 1.4 加密模块 (`internal/crypto/`)

| 功能 | 文件 | 详情 |
|------|------|------|
| **信封加密** | `envelope.go` | AES-256-GCM，随机 DEK → 加密明文 → 主密钥加密 DEK |
| **KMS 接口** | `kms.go` | `KMSProvider` 接口 (GetMasterKey/IsAvailable)，EnvKMSProvider 实现 |
| **Refresh Token** | `envelope.go` | SHA-256 随机生成 |

> ⚠️ **KMS 硬编码已消除**: 所有代码路径通过 `InitMasterKey()` 获取主密钥。开发环境自动生成临时密钥（含 WARN 日志），生产环境必须设置 `VERIDACTUS_MASTER_KEY`。

### 1.5 短信提供商 (`internal/auth/sms_provider.go`)

| Provider | 状态 | 详情 |
|----------|------|------|
| **AliyunSMSProvider** | ✅ | 阿里云短信服务（HMAC-SHA1 签名），国内生产推荐 |
| **TwilioSMSProvider** | ✅ | Twilio REST API，国际场景 |

### 1.6 存储层 (`internal/store/`)

#### 接口 (StoreFacade)

`facade.go` 定义 **30+ 方法**，含组织/工作空间/用户/成员/流水线/插件/策略/API Key/虚拟密钥/钱包/交易/模型/设置/审计等完整 CRUD。

#### PostgreSQL 实现 (`postgres.go`)

**18 张表**（`getPostgresMigrations()` 定义）：

| 表名 | 隔离列 | 说明 |
|------|--------|------|
| `organizations` | — | 组织（plan, logo_url, primary_color, settings） |
| `workspaces` | `org_id` | 工作空间 |
| `users` | `org_id` | 用户（email, phone, auth_provider, password_hash） |
| `workspace_members` | `workspace_id, user_id` | 成员关系（role） |
| `refresh_tokens` | `user_id` | 刷新令牌 |
| `virtual_keys` | `workspace_id` | 虚拟密钥（provider_key_encrypted, type） |
| `wallets` | `workspace_id` | 钱包（balance_usd_micro） |
| `transactions` | `workspace_id, wallet_id` | 交易记录 |
| `pipelines` | `org_id, workspace_id` | 治理流水线 |
| `plugins` | `org_id, workspace_id` | 插件注册 |
| `policies` | `org_id, workspace_id` | 治理策略 |
| `apikeys` | `org_id, workspace_id` | API 密钥 |
| `models` | `org_id, workspace_id` | 模型配置 |
| `traces` | `org_id, workspace_id` | 执行轨迹 |
| `config_versions` | — | 配置版本号 |
| `data_plane_configs` | — | 数据面配置 |
| `settings` | `workspace_id` | 工作空间设置 |
| `invoices` | `workspace_id, org_id` | 发票/账单 |

**关键实现方法**:
- `ListOrganizationsByUser` — 通过 workspace_members JOIN 返回用户所属组织
- `nilSafeString` — 将空字符串转为 SQL NULL
- `listHelper[T]` — 泛型列表查询辅助函数
- 所有查询使用 `$N` 参数化防注入

#### SQLite 实现 (`sqlite.go`)

保留为开发/调试模式。与 PG 实现等价的接口，使用 `?` 占位符。

---

## 2. Rust 数据面

**技术栈**: Rust, Axum, tokio, redis-rs, aws-sdk-s3, sha2, utoipa  
**入口**: `core/src/main.rs`  
**端口**: 8080  
**版本**: 0.2.1

### 2.1 HTTP 端点

| 方法 | 路径 | 说明 |
|------|------|------|
| GET | `/health` | 健康检查 |
| POST | `/v1/chat/completions` | **核心端点**: OpenAI 兼容 Chat API，支持 SSE 流式 |
| GET | `/v1/traces` | 列出 Trace 记录 |
| GET | `/v1/traces/:id` | 获取 Trace 详情 |
| GET | `/v1/traces/:id/compliance` | 获取合规信息 |
| POST | `/v1/traces/:id/replay` | 重放 Trace |
| POST | `/v1/traces/:id/verify` | 验证 Trace 签名 |
| GET | `/v1/traces/:id/replay/branches` | 获取重放分支 |
| DELETE | `/v1/replay/branch/:id` | 删除重放分支 |
| DELETE | `/v1/gdpr/:request_id` | GDPR 删除请求 |
| GET | `/v1/gdpr` | GDPR 删除历史 |
| GET | `/v1/gdpr/:request_id/proof` | GDPR 删除证明 |
| GET | `/v1/extension-discovery` | 扩展发现端点 |
| GET | `/v1/metrics/realtime` | Prometheus 实时指标 |
| GET | `/v1/audit/log` | 审计日志 |
| GET | `/v1/prevention/stats` | 主动预防统计 |

### 2.2 7 大治理插件

| 插件 | 文件 | 类型 | 功能 |
|------|------|------|------|
| **BudgetGuardPlugin** | `production_plugins.rs` | Native | 预算检查/预留/释放，Lua 原子扣减 |
| **PiiDetectorPlugin** | `production_plugins.rs` | Native | PII 检测（email/phone/ssn/credit_card/api_key） |
| **InputSanitizerPlugin** | `production_plugins.rs` | Native | 输入净化/脱敏 |
| **ResponseValidatorPlugin** | `production_plugins.rs` | Native | 响应验证/安全评分 |
| **BudgetPlugin** | `governance.rs` | Native | 旧版预算守卫 |
| **AuthPlugin** | `governance.rs` | Native | 认证插件 |
| **SidecarPlugin** | `sidecar.rs` | HTTP→Python | 调用 Python Worker 执行算法 |

**插件类型**: Native (<10μs) / Wasm (50-200μs) / Sidecar (5-500ms) / Grpc (已废弃)

### 2.3 密码学证明链

| 级别 | 文件 | 算法 | 说明 |
|------|------|------|------|
| **L0** | `signature.rs` | SHA-256 → hex | JCS 规范化 + SHA-256 哈希 → 审计签名 |
| **L1** | `signature.rs` | Ed25519 | 非对称签名（预留接口） |
| **L2A** | `merkle.rs` | Merkle Tree | 批量 Trace 的 Merkle Root 证明 |
| **L2B** | `zk.rs` | NANOZK | 分层 ZK 证明（承诺+聚合根+witness+验证） |

**JCS 实现** (`jcs.rs`): 完整 RFC 8785 JSON Canonicalization Scheme — 键排序(BTreeMap)、数字规范化、UTF-8 转义。

### 2.4 流水线执行引擎 (`pipeline/executor.rs`)

**速度分层架构**:
1. **PreRequest** (热路径, <10μs): Rust Native 插件并行执行（FuturesUnordered）
2. **Streaming**: 同步处理每个 SSE chunk
3. **PostResponse**: 同步 + Redis Stream XADD 异步任务
4. **AsyncFinalize**: Redis Stream → Python Worker 消费

### 2.5 预算熔断 (`budget/stream_guard.rs`)

| 参数 | 值 |
|------|-----|
| `check_interval` | **每 10 个 Token 触发一次** |
| Redis Key | `workspace:{id}:budget` (总预算), `workspace:{id}:budget:daily` (日预算) |
| Lua 脚本 | `DECRBY` 扣减 + 日限额检查 + 双回滚 |
| SSE 事件 | `VERIDACTUS_BUDGET_EXCEEDED` |
| Trace 状态 | `BLOCKED` |

### 2.6 Redis 操作汇总

| 模块 | 数据类型 | Key 模式 | 操作 |
|------|----------|----------|------|
| `store/adapters/redis.rs` | String | `veridactus:budget:{tenant_id}` | GET/SET/DECRBYFLOAT/INCRBYFLOAT |
| `budget/stream_guard.rs` | String + Lua | `workspace:{id}:budget` | EVAL (Lua 原子扣减) |
| `dispatcher/redis_dispatch.rs` | Stream | `veridactus:tasks` | XADD |
| `middleware/rate_limit.rs` | Hash (预留) | — | 令牌桶 |

### 2.7 Prometheus 指标

| 指标名 | 类型 |
|--------|------|
| `veridactus_requests_total` | Counter |
| `veridactus_sse_connections_total` | Counter |
| `veridactus_tokens_consumed_total` | Counter |
| `veridactus_budget_exceeded_total` | Counter |
| `veridactus_safety_events_total` | Counter |
| `veridactus_pii_detections_total` | Counter |
| `veridactus_injection_blocks_total` | Counter |
| `veridactus_l0_signatures_total` | Counter |

### 2.8 API Key 管理 (`auth/keys.rs`)

- **ApiKeyManager**: HashMap<String, String> 内存存储，Ed25519 风格密钥生成
- 支持 `Bearer` 前缀自动剥离
- 环境变量 `VERIDACTUS_STATIC_API_KEYS` 注册静态密钥
- Admin Key 通过 `VERIDACTUS_ADMIN_KEY` 环境变量加载

### 2.9 约束评估 (`constraints/mod.rs`)

**5 种预算策略**:
- `HardLimit` — 硬性限制
- `SoftLimit` — 软限制（超过后降级）
- `PerToken` — 每 Token 扣减
- `PerRequest` — 每请求扣减
- `PreAllocated` — 预分配模式

**其他约束**: 隐私级别、指令层次（4 级）、Guardrails 严格度、合规 Profile

---

## 3. Python Worker

**技术栈**: FastAPI, uvicorn, redis (asyncio), httpx, pydantic  
**入口**: `python-worker/app/main.py`  
**端口**: 8001

### 3.1 HTTP 端点

| 方法 | 路径 | 说明 |
|------|------|------|
| GET | `/health` | 健康检查 |
| POST | `/api/v1/compute-guarantee` | C-SafeGen 经认证保证计算 |
| POST | `/api/v1/drift-detection` | 语义漂移检测 |
| POST | `/api/v1/pii-detection` | PII 检测 |
| GET | `/api/v1/pii-detection?text=...` | PII 检测（查询参数） |
| POST | `/plugin/execute` | V3 Sidecar 插件执行端点 |

### 3.2 Redis Stream 消费者

| 参数 | 值 |
|------|-----|
| Stream Key | `veridactus:tasks` |
| Consumer Group | `python-workers` |
| Consumer Name | `worker-1` |
| 消费模式 | XREADGROUP + COUNT 1 + BLOCK 5000ms + XACK 确认 |
| 回调地址 | `POST {CONTROL_PLANE_URL}/api/v1/traces/update` |

**处理的任务类型**: `embedding_drift`, `certified_guarantee`, `semantic_analysis`

### 3.3 算法实现

| 算法 | 函数 | 说明 |
|------|------|------|
| **Jaccard 相似度** | `detect_drift()` | 词集交集/并集 |
| **Cosine 余弦相似度** | `_compute_cosine_similarity()` | TF-IDF 加权，纯 Python 无依赖 |
| **综合相似度** | 0.3×Jaccard + 0.7×Cosine | 漂移阈值 0.7 |
| **C-SafeGen 安全评分** | `compute_guarantee()` | 4 维度（毒性/PII/偏见/幻觉）+ 共形校准 |

### 3.4 PII 检测模式

| 类型 | 正则 |
|------|------|
| email | `[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}` |
| phone | `1[3-9]\d{9}` |
| id_card | `[1-9]\d{5}(?:19\|20)\d{2}...` |
| ip_address | `\b\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3}\b` |

### 3.5 Sidecar 插件路由表

| 插件名 | 处理函数 | 行为 |
|--------|----------|------|
| `content-safety-scorer` | `_execute_content_safety()` | 关键词检测 → `block` / `flag` / `continue` |
| `toxicity-classifier` | `_execute_toxicity_classifier()` | 占位（可接入 detoxify 模型） |
| `bias-detector` | `_execute_bias_detector()` | 占位（可接入公平性模型） |

### 3.6 依赖

```
fastapi, uvicorn, httpx, redis==5.1.0, pydantic
```

---

## 4. React 前端

**技术栈**: React 18, TypeScript, Vite, TailwindCSS, Framer Motion, React Router v6  
**端口**: 3000

### 4.1 路由清单（来自 App.tsx）

| 路由 | 组件 | Auth | 说明 |
|------|------|------|------|
| `/login` | `LoginPage` | ❌ | 邮箱/手机/微信/GitHub/Google 登录 |
| `/bind-phone` | `PhoneBind` | ❌ | 微信后手机绑定 |
| `/onboarding` | `OnboardingPage` | ❌ | 新用户引导 |
| `/dashboard` | `Dashboard` | ✅ | 全局看板（健康分数/证明链/Trace） |
| `/chat` | `ChatPage` | ✅ | **Chat 沙箱** — 对话 + 安全盾牌 |
| `/playground` | `PlaygroundPage` | ✅ | **Developer Hub** — 三栏 Playground |
| `/vault` | `VaultPage` | ✅ | **Holo-Trace Vault** — Trace 列表 |
| `/vault/:traceId` | `VaultDetail` | ✅ | Trace 详情（分屏 + 密码学验证） |
| `/pipelines` | `Pipelines` | ✅ | Pipeline 列表 |
| `/pipelines/new` | `PipelineDesigner` | ✅ | 新建可视化流水线 |
| `/pipelines/design/:id?` | `PipelineDesigner` | ✅ | 编辑可视化流水线 |
| `/pipelines/edit/:id` | `PipelineEdit` | ✅ | 表单编辑流水线 |
| `/audit` | `AuditCenter` | ✅ | 审计中心（个人版） |
| `/audit-center` | `AuditorCommandCenter` | ✅ | 审计指挥舱（企业版） |
| `/plugins` | `Plugins` | ✅ | 插件管理 |
| `/api-keys` | `ApiKeys` | ✅ | API Key 管理 |
| `/models` | `Models` | ✅ | 模型配置 |
| `/settings` | `Settings` | ✅ | 工作空间设置 |
| `/brand` | `BrandSettings` | ✅ | 品牌白标设置（企业版） |
| `*` | 重定向 `/chat` | ✅ | 默认跳转 |

### 4.2 核心功能组件

#### Chat 沙箱 (`engines/chat/ChatPage.tsx`)

| 功能 | 状态 |
|------|------|
| 极简对话流 | ✅ |
| Model Selector 模型切换 | ✅ |
| 🛡️ 动态安全盾牌 (`SafetyShield.tsx`) — PII 检测 + 呼吸灯 | ✅ |
| SSE 流式输出 (`requestAnimationFrame` 优化零抖动) | ✅ |
| A/B 对比双模型并发 | ❌ 未实现 |

#### Holo-Trace Vault

| 功能 | 组件 | 状态 |
|------|------|------|
| Trace 列表看板（成本/安全徽章/归属） | `AuditCenter.tsx` | ✅ |
| 50/50 分屏（Raw/Sanitized） | `VaultDetail.tsx` | ✅ |
| 滚动条百分比同步联动 | `VaultDetail.tsx` | ✅ |
| `[REDACTED]` 删除线 + 红色微光 | `VaultDetail.tsx` | ✅ |
| **Web Crypto API 密码学自证** | `CryptoVerify.tsx` | ✅ |
| JCS 规范化 + SHA-256 → 后端签名比对 | `CryptoVerify.tsx` | ✅ |
| Framer Motion 绿色粒子爆发动效 | `CryptoVerify.tsx` | ✅ |
| "完全离线验证，无需信任服务器" | `CryptoVerify.tsx` | ✅ |

#### 全局 UI

| 功能 | 状态 |
|------|------|
| 暗黑/亮色主题切换 | ✅ |
| 双引擎侧边栏（Chat/DevHub/Vault） | ✅ |
| 企业/个人计划差异化菜单 | ✅ |
| 用户头像/名称/计划徽章/登出 (`UserHeader.tsx`) | ✅ |
| AuthGuard JWT 验证 + 路由保护 | ✅ |

### 4.3 API 调用层 (`api/index.ts`)

**数据面调用（→ :8080）**:
- `sendChatMessage()` → `POST /v1/chat/completions`
- `sendChatMessageStream()` → 同上 + SSE 流式解析
- `getTracesFromDataPlane()` → `GET /v1/traces`
- `getTraceDetail()` → `GET /v1/traces/:id`
- `verifyTraceSignature()` → `POST /v1/traces/:id/verify`
- `replayTrace()` → `POST /v1/traces/:id/replay`

**控制面调用（→ :8081）**:
- `login()` / `register()` / `refreshToken()` / `logout()`
- `getOrgs()` / `getWorkspaces()` / `getMembers()`
- `getPipelines()` / `createPipeline()` / `updatePipeline()` / `deletePipeline()`
- `getPlugins()` / `getPolicies()`
- `getApiKeys()` / `getModels()`
- `getVirtualKeys()` / `createVirtualKey()`
- `getWallet()` / `topUpWallet()`
- `getSettings()` / `updateSettings()`
- `getBrand()` / `updateBrand()`
- `getSSOConfig()` / `updateSSOConfig()`
- `getAuditEvents()` / `getAuditEventsDataPlane()`
- `getComplianceReport()` / `deleteDataPlaneConfig()`

### 4.4 暗黑模式

| 属性 | 值 |
|------|-----|
| 默认主题 | Dark (`:root` CSS 变量) |
| 切换方式 | `data-theme="light"` / `data-theme="dark"` |
| 主背景色 | `#0a0e27` (CSS 变量 `--bg-primary`) |
| Chat 背景 | `linear-gradient(180deg, #0B0F19 0%, #131633 100%)` |

---

## 5. 基础设施

### 5.1 Docker Compose 服务

| 服务 | 镜像 | 端口 | 配置 |
|------|------|------|------|
| PostgreSQL | postgres:16-alpine | 5432 | 持久化卷 |
| Redis | redis:7-alpine | 6379 | `appendonly yes; maxmemory 256mb` |
| MinIO | minio/minio | 9000/9001 | 3 bucket (veridactus/traces/backups) |

### 5.2 环境变量

| 变量 | 用途 | 默认值 |
|------|------|--------|
| `STORE_BACKEND` | 存储后端选择 | `postgres` |
| `PG_HOST/PORT/USER/PASS/DB_NAME/SSLMODE` | PostgreSQL 连接 | localhost/5432/veridactus/veridactus/veridactus/disable |
| `JWT_SECRET` | JWT 签名密钥 | 随机生成 |
| `VERIDACTUS_ADMIN_KEY` | 管理 API 密钥 | — |
| `VERIDACTUS_MASTER_KEY` | 信封加密主密钥 | 开发环境自动生成 |
| `VERIDACTUS_KMS_TYPE` | KMS 类型 (aliyun/vault) | env |
| `REDIS_HOST/PORT` | Redis 连接 | localhost/6379 |
| `GITHUB/GOOGLE/WECHAT_CLIENT_ID/SECRET/REDIRECT_URI` | OAuth 配置 | — |

---

## 6. 已知限制 / 待完成

### 6.1 未实现功能

| 功能 | AI-1.md 指令 | 当前状态 |
|------|-------------|----------|
| **gRPC 实际运行** | 3.3 | Proto 定义完成，实际通信仍走 HTTP REST |
| **企业 SSO 协议** | 2.2 | Okta/Azure/飞书/钉钉 仅配置占位，无 OIDC/SAML |
| **goth 库** | 2.2 | OAuth 为手工 HTTP 实现，未使用 goth |
| **GORM** | 1.2 | 使用原生 `database/sql`，未使用 GORM |
| **ClickHouse 集成** | 1.4 | Schema 已定义，无代码驱动连接 |
| **阿里云 KMS SDK** | 3.1 | 接口保留，未接入阿里云 KMS SDK |
| **Stripe 真实 SDK** | 4.1 | Stub 实现，返回模拟 session_id |
| **A/B 对比模式** | 6.3 | 前端未实现 |
| **OWASP 模板** | 9.3 | 未实现 |
| **合规 PDF 报告** | 10.2 | 端点存在，无 PDF 生成 |
| **cargo fuzz** | 6.4 | 完全缺失 |
| **Cypress/Playwright E2E** | 6.4 | 完全缺失 |
| **插件市场拖拽** | 9.2 | 未实现 |
| **CSS Variables 品牌注入** | 11.2 | 未实现 |
| **PII 热力图** | 10.1 | 未实现 |

### 6.2 技术债务

| 项 | 详情 |
|-----|------|
| `cmd/server/models.go` | 旧模型定义，部分与 `internal/model/types.go` 重复 |
| `internal/store/sqlite.go` | 保留但非默认后端，代码量 47KB |
| 权限检查 | `CheckPermission/RequireRole` 在大部分 handler 中未调用 |
| Pipeline 热缓存 | 使用 HTTP POST 推送而非 Redis Pub/Sub |
| Redis 预算扣减 | Go CP 侧使用 PG UPDATE 而非 Redis Lua |

---

> **审计完成时间**: 2026-06-26 20:32  
> **编译状态**: ✅ Go control-plane 编译通过  
> **运行状态**: ✅ Go CP :8081 (PG mode) + Rust DP :8080 + Python :8001 + Frontend :3000 全部正常
