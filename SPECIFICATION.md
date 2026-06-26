# VERIDACTUS 产品全阶段实现规划 v1.0

> **文档定位**：未来 6 个月所有开发工作的最高指导准则
> **目标版本**：v0.2.1 → v1.0.0
> **技术栈**：Rust (数据面) + Go (控制面) + React/Vite (前端) + Python (增强计算)
> **面向用户**：个人开发者 + 中小企业 + 大型企业

---

## 目录

- [第〇章：总体架构设计](#第〇章总体架构设计)
- [第一阶段：多租户基础设施](#第一阶段多租户基础设施)
- [第二阶段：双轨制密钥与计费引擎](#第二阶段双轨制密钥与计费引擎)
- [第三阶段：前端双引擎重构](#第三阶段前端双引擎重构)
- [第四阶段：企业级管控与合规](#第四阶段企业级管控与合规)
- [第五阶段：生产化打磨](#第五阶段生产化打磨)
- [附录：接口契约定义](#附录接口契约定义)

---

## 第〇章：总体架构设计

### 0.1 目标架构全景图

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                            VERIDACTUS v1.0                                   │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  ┌──────────────────────────┐      ┌──────────────────────────────────────┐ │
│  │  React 前端 (:3000)       │      │  负载均衡 / API Gateway              │ │
│  │                          │      │  (nginx / traefik)                    │ │
│  │  ┌────────────────────┐  │      └────────────┬─────────────────────────┘ │
│  │  │ VERIDACTUS Chat   │  │                    │                           │
│  │  │ (安全沙箱对话)     │  │      ┌─────────────┼─────────────┐            │
│  │  └────────────────────┘  │      │             │             │            │
│  │  ┌────────────────────┐  │      ▼             ▼             ▼            │
│  │  │ Developer Hub     │  │  ┌────────┐  ┌──────────┐  ┌──────────┐      │
│  │  │ (全息调试台)       │──┼─▶│ Rust   │  │ Go       │  │ Python   │      │
│  │  └────────────────────┘  │  │ 数据面  │  │ 控制面    │  │ Worker   │      │
│  │  ┌────────────────────┐  │  │ :8080  │  │ :8081    │  │ :8001    │      │
│  │  │ Holo-Trace Vault  │  │  └───┬────┘  └────┬─────┘  └────┬─────┘      │
│  │  │ (全息证据金库)     │  │      │            │              │            │
│  │  └────────────────────┘  │      │     ┌──────┼──────┐       │            │
│  │  ┌────────────────────┐  │      │     │      │      │       │            │
│  │  │ Pipeline Studio   │  │      ▼     ▼      ▼      ▼       ▼            │
│  │  │ (治理流水线设计器)  │  │  ┌──────────────────────────────────────┐    │
│  │  └────────────────────┘  │  │         存储层 (异构混合)             │    │
│  │  ┌────────────────────┐  │  │                                      │    │
│  │  │ Admin Console     │  │  │ PostgreSQL  Redis    ClickHouse MinIO │    │
│  │  │ (管理控制台)       │  │  │ (业务数据) (缓存)   (OLAP)    (对象)  │    │
│  │  └────────────────────┘  │  └──────────────────────────────────────┘    │
│  └──────────────────────────┘                                               │
└─────────────────────────────────────────────────────────────────────────────┘
```

### 0.2 分层架构定义

```
┌──────────────────────────────────────────────────────────────────┐
│                        表示层 (Presentation)                      │
│  React SPA (Vite) + TailwindCSS + shadcn/ui + Zustand            │
│  路由: /chat | /playground | /vault | /studio | /admin           │
├──────────────────────────────────────────────────────────────────┤
│                        网关层 (Gateway)                           │
│  nginx/traefik: TLS终止, 路由分发, 限流, CORS                    │
├──────────────────────────────────────────────────────────────────┤
│                        应用层 (Application)                       │
│  ┌─────────────┐  ┌──────────────┐  ┌──────────────────┐         │
│  │ Rust 数据面  │  │ Go 控制面    │  │ Python Worker    │         │
│  │ :8080       │  │ :8081        │  │ :8001            │         │
│  │ AI代理+治理 │  │ 配置+用户+计费│  │ 增强计算         │         │
│  │             │  │              │  │ (异步冷路径)     │         │
│  └──────┬──────┘  └──────────────┘  └──────────────────┘         │
│         │ Redis Stream (非阻塞 XADD)                              │
│         └─────────────────────────►                               │
├──────────────────────────────────────────────────────────────────┤
│                        持久层 (Persistence)                       │
│  ┌─────────┐  ┌─────────┐  ┌──────────┐  ┌────────────────┐      │
│  │PostgreSQL│  │  Redis  │  │ClickHouse│  │    MinIO       │      │
│  │ 业务数据 │  │ 缓存/限流│  │ OLAP分析 │  │ 对象/证明存储  │      │
│  └─────────┘  └─────────┘  └──────────┘  └────────────────┘      │
└──────────────────────────────────────────────────────────────────┘

插件速度分层架构 (V3):
┌──────────────────────────────────────────────────────────────┐
│  🔥 热路径 PreRequest/Streaming — Rust Native (<10μs each)  │
│  ┌──────────────────────────────────────────────────────┐   │
│  │ BudgetGuard → PiiDetector → InputSanitizer → G1     │   │
│  │ FuturesUnordered 真并行, 任一 Block 即返回           │   │
│  └──────────────────────────────────────────────────────┘   │
├──────────────────────────────────────────────────────────────┤
│  🌡️ 温路径 PostResponse — Rust Native (同步)                 │
│  ┌──────────────────────────────────────────────────────┐   │
│  │ G2OutputFilter → ResponseValidator                   │   │
│  │ + Redis XADD (非阻塞, tokio::spawn dispatch)         │   │
│  └──────────────────────────────────────────────────────┘   │
├──────────────────────────────────────────────────────────────┤
│  ❄️ 冷路径 AsyncFinalize — Redis Stream → Python Worker      │
│  ┌──────────────────────────────────────────────────────┐   │
│  │ DriftDetection → C-SafeGen Score → SemanticAnalysis  │   │
│  │ 结果异步写回 Trace, 不阻塞 LLM 响应                   │   │
│  └──────────────────────────────────────────────────────┘   │
└──────────────────────────────────────────────────────────────┘
```

### 0.3 关键设计决策

| 决策 | 说明 | 替代方案 | 选择理由 |
|:---|:---|:---|:---|
| **前端保留 Vite+React** | 不迁移 Next.js | Next.js 14 App Router | 避免全量重写，Vite HMR 更快，SPA 已满足需求 |
| **Go 控制面扩能** | 控制面承担更多职责 | 拆分微服务 | 团队规模小，单服务够用但逻辑分层清晰 |
| **PostgreSQL 为主** | 业务数据全部入 PG | MySQL | PG 对 JSON/数组支持更好，适合配置型数据 |
| **SQLite 保留** | 作为单机开发/调试模式 | 彻底移除 | 降低开发门槛，CI 测试更简单 |
| **REST 内网通信** | 数据面↔控制面 用 HTTP/REST | gRPC | 当前阶段 gRPC 增加复杂度但不增加用户价值 |
| **Redis 预算扣减** | Lua 原子脚本 | PostgreSQL 行锁 | <1ms vs 5-20ms，对实时熔断场景至关重要 |
| **插件速度分层** | 热路径 Rust，冷路径 Redis→Python | 同步 HTTP 调用 Python | 避免 Python 阻塞 LLM 响应 (50-500ms) |
| **FuturesUnordered** | 插件真正并行执行 | 串行 for 循环 | PreRequest 多个独立插件同时执行 |
| **Python 异步消费** | Redis Stream consumer group | 每插件一个 gRPC 服务 | 单一进程托管多个插件，水平扩展 |

### 0.4 多租户数据隔离模型

```
Organization (组织)
  ├── id: UUID
  ├── name: string
  ├── plan: "free" | "pro" | "enterprise"
  │
  └── Workspace (工作空间) 1..N
        ├── id: UUID
        ├── org_id: FK → Organization
        ├── name: string
        │
        ├── User (用户) M..N (通过 WorkspaceMember)
        │     ├── id: UUID
        │     ├── email: string (unique)
        │     ├── auth_provider: "github" | "google" | "sso" | "email"
        │     ├── role: "platform_admin" | "org_admin" | "workspace_admin" | "developer" | "auditor"
        │
        ├── VirtualKey (虚拟密钥) 1..N
        │     ├── id: UUID
        │     ├── workspace_id: FK → Workspace
        │     ├── type: "byok" | "platform"
        │     ├── provider_key_encrypted: base64 (AES-256-GCM)
        │     ├── rate_limit_rpm: int
        │     └── status: "active" | "revoked"
        │
        ├── Wallet (钱包)
        │     ├── id: UUID
        │     ├── workspace_id: FK → Workspace
        │     ├── balance_usd_micro: bigint   -- 微美元 (1e-6 USD)
        │     └── overdraft_limit_usd_micro: bigint
        │
        ├── Pipeline (治理流水线)
        └── Trace (执行轨迹，关联到 workspace)
```

### 0.5 部署模式矩阵

| 模式 | 适用场景 | 存储 | 认证 | 计费 |
|:---|:---|:---|:---|:---|
| **单机开发模式** | 本地开发 | SQLite + 内存 Redis | AdminKey | 无 |
| **Docker Compose** | 小团队自托管 | PG + Redis + MinIO | 本地用户管理 | 无 |
| **Kubernetes 私有化** | 中大型企业 | PG + Redis Cluster + MinIO | LDAP/SAML SSO | 企业内部计费 |
| **SaaS 多租户** | 公共服务 | PG + Redis Cluster + ClickHouse + MinIO | OAuth + SSO | Stripe |

---

## 第一阶段：多租户基础设施

> **周期**：3 周 | **目标**：每个人都拥有独立的工作空间，数据完全隔离

### 1.1 数据库 Schema 重构

#### 1.1.1 新增表

```sql
-- ============================================
-- 组织表
-- ============================================
CREATE TABLE organizations (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name        VARCHAR(255) NOT NULL,
    slug        VARCHAR(64) NOT NULL UNIQUE,     -- URL 友好标识
    plan        VARCHAR(32) NOT NULL DEFAULT 'free', -- free|pro|enterprise
    logo_url    TEXT,
    primary_color VARCHAR(7) DEFAULT '#6c5ce7',
    settings    JSONB DEFAULT '{}',
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- ============================================
-- 工作空间表
-- ============================================
CREATE TABLE workspaces (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    org_id      UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    name        VARCHAR(255) NOT NULL,
    slug        VARCHAR(64) NOT NULL,
    description TEXT,
    settings    JSONB DEFAULT '{}',
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(org_id, slug)
);

-- ============================================
-- 用户表
-- ============================================
CREATE TABLE users (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email           VARCHAR(320) NOT NULL UNIQUE,
    display_name    VARCHAR(255),
    avatar_url      TEXT,
    auth_provider   VARCHAR(32) NOT NULL,           -- github|google|email|sso
    auth_provider_id VARCHAR(255),                   -- OAuth provider 的用户 ID
    password_hash   TEXT,                            -- 仅 email 注册时使用 (bcrypt)
    settings        JSONB DEFAULT '{}',
    last_login_at   TIMESTAMPTZ,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- ============================================
-- 工作空间成员表 (User ↔ Workspace M:N)
-- ============================================
CREATE TABLE workspace_members (
    id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    workspace_id UUID NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    user_id      UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    role         VARCHAR(32) NOT NULL DEFAULT 'developer',
                 -- platform_admin | org_admin | workspace_admin | developer | auditor
    invited_by   UUID REFERENCES users(id),
    invited_at   TIMESTAMPTZ,
    joined_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(workspace_id, user_id)
);

-- ============================================
-- JWT 刷新令牌表
-- ============================================
CREATE TABLE refresh_tokens (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id     UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token_hash  VARCHAR(64) NOT NULL UNIQUE,        -- SHA-256(token)
    expires_at  TIMESTAMPTZ NOT NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX idx_refresh_tokens_user ON refresh_tokens(user_id);
```

#### 1.1.2 改造现有表（迁移脚本）

```sql
-- 为所有业务表添加 org_id 和 workspace_id
ALTER TABLE apikeys ADD COLUMN org_id UUID REFERENCES organizations(id);
ALTER TABLE apikeys ADD COLUMN workspace_id UUID REFERENCES workspaces(id);
ALTER TABLE pipelines ADD COLUMN org_id UUID REFERENCES organizations(id);
ALTER TABLE pipelines ADD COLUMN workspace_id UUID REFERENCES workspaces(id);
ALTER TABLE models ADD COLUMN org_id UUID REFERENCES organizations(id);
ALTER TABLE models ADD COLUMN workspace_id UUID REFERENCES workspaces(id);
ALTER TABLE plugins ADD COLUMN org_id UUID REFERENCES organizations(id);
ALTER TABLE plugins ADD COLUMN workspace_id UUID REFERENCES workspaces(id);

-- 为已有数据迁移（如果存在）
-- UPDATE apikeys SET org_id = (SELECT id FROM organizations LIMIT 1),
--     workspace_id = (SELECT id FROM workspaces LIMIT 1)
-- WHERE org_id IS NULL;
```

#### 1.1.3 StoreFacade 接口抽象

```go
// control-plane/internal/store/facade.go
// 存储后端接口抽象，支持 PG 和 SQLite 双实现

type StoreFacade interface {
    // === 组织 ===
    CreateOrganization(ctx context.Context, org *Organization) error
    GetOrganization(ctx context.Context, id uuid.UUID) (*Organization, error)
    GetOrganizationBySlug(ctx context.Context, slug string) (*Organization, error)
    UpdateOrganization(ctx context.Context, id uuid.UUID, updates map[string]interface{}) error
    DeleteOrganization(ctx context.Context, id uuid.UUID) error

    // === 工作空间 ===
    CreateWorkspace(ctx context.Context, ws *Workspace) error
    GetWorkspace(ctx context.Context, id uuid.UUID) (*Workspace, error)
    ListWorkspaces(ctx context.Context, orgID uuid.UUID) ([]*Workspace, error)
    DeleteWorkspace(ctx context.Context, id uuid.UUID) error

    // === 用户 ===
    CreateUser(ctx context.Context, user *User) error
    GetUser(ctx context.Context, id uuid.UUID) (*User, error)
    GetUserByEmail(ctx context.Context, email string) (*User, error)
    GetUserByProvider(ctx context.Context, provider, providerID string) (*User, error)
    UpdateUser(ctx context.Context, id uuid.UUID, updates map[string]interface{}) error

    // === 成员 ===
    AddMember(ctx context.Context, member *WorkspaceMember) error
    ListMembers(ctx context.Context, workspaceID uuid.UUID) ([]*WorkspaceMember, error)
    UpdateMemberRole(ctx context.Context, memberID uuid.UUID, role string) error
    RemoveMember(ctx context.Context, workspaceID, userID uuid.UUID) error

    // === Pipeline (带租户过滤) ===
    ListPipelines(ctx context.Context, workspaceID uuid.UUID) ([]*Pipeline, error)
    GetPipeline(ctx context.Context, workspaceID uuid.UUID, id string) (*Pipeline, error)
    CreatePipeline(ctx context.Context, p *Pipeline) error
    UpdatePipeline(ctx context.Context, id string, p *Pipeline) error
    DeletePipeline(ctx context.Context, id string) error

    // === ApiKey (带租户过滤) ===
    ListApiKeys(ctx context.Context, workspaceID uuid.UUID) ([]*ApiKey, error)
    GetApiKey(ctx context.Context, workspaceID uuid.UUID, id string) (*ApiKey, error)
    CreateApiKey(ctx context.Context, k *ApiKey) error
    UpdateApiKey(ctx context.Context, k *ApiKey) error

    // === Model (带租户过滤) ===
    ListModels(ctx context.Context, workspaceID uuid.UUID) ([]*ModelConfig, error)
    GetModel(ctx context.Context, workspaceID uuid.UUID, id string) (*ModelConfig, error)
    CreateModel(ctx context.Context, m *ModelConfig) error
    UpdateModel(ctx context.Context, id string, m *ModelConfig) error
    DeleteModel(ctx context.Context, id string) error

    // === 迁移与健康 ===
    RunMigrations(ctx context.Context) error
    HealthCheck(ctx context.Context) error
}
```

### 1.2 认证与授权

#### 1.2.1 OAuth 认证流程

```
┌──────────┐     ┌──────────────┐     ┌──────────────┐     ┌──────────┐
│  Browser  │     │ React 前端   │     │ Go 控制面    │     │  GitHub  │
│           │     │              │     │              │     │  OAuth   │
└────┬─────┘     └──────┬───────┘     └──────┬───────┘     └────┬─────┘
     │                  │                    │                  │
     │ 1. Click Login   │                    │                  │
     │─────────────────▶│                    │                  │
     │                  │ 2. GET /auth/github│                  │
     │                  │───────────────────▶│                  │
     │                  │                    │ 3. Redirect      │
     │ 4. Redirect to GitHub                │─────────────────▶│
     │◀─────────────────────────────────────│                  │
     │                  │                    │                  │
     │ 5. Authorize     │                    │                  │
     │─────────────────────────────────────────────────────────▶│
     │                  │                    │                  │
     │ 6. Callback ?code=xxx                │                  │
     │─────────────────▶│                    │                  │
     │                  │ 7. POST /auth/github/callback?code=xxx│
     │                  │───────────────────▶│                  │
     │                  │                    │ 8. Exchange token│
     │                  │                    │─────────────────▶│
     │                  │                    │ 9. User info     │
     │                  │                    │◀─────────────────│
     │                  │                    │                  │
     │                  │                    │ 10. Create/Find  │
     │                  │                    │     User + Org   │
     │                  │                    │     + Workspace  │
     │                  │                    │                  │
     │                  │ 11. {access_token,│                  │
     │                  │      refresh_token}│                  │
     │                  │◀───────────────────│                  │
     │ 12. Redirect to /chat                │                  │
     │◀─────────────────│                    │                  │
```

#### 1.2.2 JWT 定义

```json
// Access Token Payload (有效期: 15分钟)
{
  "sub": "user-uuid-here",
  "email": "user@example.com",
  "name": "Display Name",
  "org_id": "org-uuid-here",
  "workspace_id": "workspace-uuid-here",
  "role": "workspace_admin",
  "permissions": [
    "pipeline:read",
    "pipeline:write",
    "apikey:manage",
    "trace:read",
    "billing:read"
  ],
  "iat": 1719331200,
  "exp": 1719332100,
  "iss": "veridactus"
}
```

#### 1.2.3 权限矩阵

```go
// 角色权限定义
var RolePermissions = map[string][]string{
    "platform_admin": {
        "*:*",  // 所有权限
    },
    "org_admin": {
        "org:*",
        "workspace:*",
        "member:*",
        "pipeline:*",
        "model:*",
        "apikey:*",
        "trace:*",
        "billing:*",
        "settings:*",
    },
    "workspace_admin": {
        "pipeline:*",
        "model:*",
        "apikey:*",
        "trace:*",
        "member:read",
        "member:invite",
        "settings:read",
        "settings:write",
        "billing:read",
    },
    "developer": {
        "pipeline:read",
        "model:read",
        "apikey:read",
        "apikey:create_own",
        "trace:read",
        "trace:write",
        "chat:use",
        "playground:use",
    },
    "auditor": {
        "trace:read",
        "trace:export",
        "compliance:read",
        "compliance:export",
        "audit:read",
    },
}
```

#### 1.2.4 数据面 JWT 验证

Rust 数据面接收请求时，验证 `Authorization: Bearer <jwt>` 并提取 workspace_id 用于隔离：

```rust
// core/src/auth/jwt.rs (新增)
pub struct VeridactusClaims {
    pub sub: String,           // user_id
    pub workspace_id: String,
    pub org_id: String,
    pub role: String,
    pub permissions: Vec<String>,
}

// Middleware: 每个请求验证 JWT + 从 Redis 获取 workspace 预算配置
pub async fn jwt_auth_middleware(
    req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let token = extract_bearer_token(&req)?;
    let claims = verify_jwt(&token, &jwk_set).await?;

    // 将 claims 注入 request extensions
    req.extensions_mut().insert(claims);
    next.run(req).await
}
```

### 1.3 Go 控制面重构

#### 1.3.1 新目录结构

```
control-plane/
├── cmd/server/
│   ├── main.go                  # 启动入口 (支持 --mode=pg|sqlite)
│   ├── router.go                # 路由注册 (按功能模块分组)
│   └── config.go                # 配置加载
├── internal/
│   ├── store/
│   │   ├── facade.go            # StoreFacade 接口定义
│   │   ├── postgres.go          # PostgreSQL 实现
│   │   ├── sqlite.go            # SQLite 实现 (开发/单机)
│   │   └── migrations/
│   │       ├── 001_initial.sql
│   │       ├── 002_multitenant.sql
│   │       └── migrate.go
│   ├── auth/
│   │   ├── jwt.go               # JWT 签发/验证
│   │   ├── oauth.go             # OAuth2 处理器 (GitHub/Google)
│   │   ├── middleware.go        # JWT 验证中间件
│   │   └── rbac.go             # 权限检查
│   ├── handler/
│   │   ├── auth.go              # 认证端点
│   │   ├── org.go               # 组织 CRUD
│   │   ├── workspace.go         # 工作空间 CRUD
│   │   ├── member.go            # 成员管理
│   │   ├── pipeline.go          # 流水线 CRUD
│   │   ├── apikey.go            # API Key CRUD
│   │   ├── model.go             # 模型 CRUD
│   │   ├── settings.go          # 设置
│   │   └── health.go            # 健康检查
│   ├── model/
│   │   ├── types.go             # 所有数据模型
│   │   └── request.go           # 请求/响应 DTO
│   └── config/
│       ├── config.go            # 配置结构体
│       └── push.go              # 配置推送到数据面
├── go.mod
├── go.sum
├── Dockerfile
└── README.md
```

#### 1.3.2 路由表

```go
// control-plane/cmd/server/router.go
func RegisterRoutes(mux *http.ServeMux, store store.StoreFacade) {
    // === 公开端点 (无需认证) ===
    mux.HandleFunc("/api/v1/health", handler.Health(store))

    // === 认证端点 (公开) ===
    mux.HandleFunc("/api/v1/auth/login/github", handler.OAuthLogin("github"))
    mux.HandleFunc("/api/v1/auth/callback/github", handler.OAuthCallback("github", store))
    mux.HandleFunc("/api/v1/auth/login/google", handler.OAuthLogin("google"))
    mux.HandleFunc("/api/v1/auth/callback/google", handler.OAuthCallback("google", store))
    mux.HandleFunc("/api/v1/auth/refresh", handler.RefreshToken(store))
    mux.HandleFunc("/api/v1/auth/logout", handler.Logout(store))

    // === 需要 JWT 认证的端点 ===
    auth := mux.Group(handler.JWTMiddleware)

    // 组织
    auth.HandleFunc("/api/v1/orgs", handler.ListOrgs(store))          // GET
    auth.HandleFunc("/api/v1/orgs/", handler.OrgByID(store))          // GET/PUT

    // 工作空间
    auth.HandleFunc("/api/v1/workspaces", handler.ListWorkspaces(store))     // GET/POST
    auth.HandleFunc("/api/v1/workspaces/", handler.WorkspaceByID(store))     // GET/PUT/DELETE

    // 成员
    auth.HandleFunc("/api/v1/workspaces/{wsId}/members", handler.Members(store))    // GET/POST
    auth.HandleFunc("/api/v1/workspaces/{wsId}/members/", handler.MemberByID(store)) // PUT/DELETE

    // 流水线 (按 workspace 隔离)
    auth.HandleFunc("/api/v1/pipelines", handler.ListPipelines(store))       // GET/POST
    auth.HandleFunc("/api/v1/pipelines/", handler.PipelineByID(store))       // GET/PUT/DELETE

    // API Key (按 workspace 隔离)
    auth.HandleFunc("/api/v1/apikeys", handler.ListApiKeys(store))           // GET/POST
    auth.HandleFunc("/api/v1/apikeys/", handler.ApiKeyByID(store))           // GET/PUT/DELETE

    // 模型
    auth.HandleFunc("/api/v1/models", handler.ListModels(store))             // GET/POST
    auth.HandleFunc("/api/v1/models/", handler.ModelByID(store))             // GET/PUT/DELETE

    // 设置
    auth.HandleFunc("/api/v1/settings", handler.GetSettings(store))          // GET
    auth.HandleFunc("/api/v1/settings", handler.UpdateSettings(store))       // POST

    // 数据面配置轮询
    mux.HandleFunc("/api/v1/config/poll", handler.ConfigPoll(store))         // GET
}
```

### 1.4 Rust 数据面适配

#### 1.4.1 租户上下文提取

```rust
// core/src/middleware/tenant.rs (新增)
pub struct TenantContext {
    pub user_id: Uuid,
    pub workspace_id: Uuid,
    pub org_id: Uuid,
    pub role: String,
}

// 从 JWT claims 或 VERIDACTUS-Tenant 头中提取
pub fn extract_tenant(req: &Request) -> Option<TenantContext> {
    // 优先从 JWT claims (SaaS 模式)
    if let Some(claims) = req.extensions().get::<VeridactusClaims>() {
        return Some(TenantContext {
            user_id: Uuid::parse_str(&claims.sub).ok()?,
            workspace_id: Uuid::parse_str(&claims.workspace_id).ok()?,
            org_id: Uuid::parse_str(&claims.org_id).ok()?,
            role: claims.role.clone(),
        });
    }
    // 回退: 从 HTTP 头 (自托管模式)
    if let Some(tenant_id) = req.headers().get("VERIDACTUS-Tenant") {
        // ...
    }
    None
}
```

#### 1.4.2 Trace 写入带租户标识

```rust
// 确保每个 Trace 写入时带上 workspace_id
trace.tenant_id = tenant_ctx.as_ref().map(|t| t.workspace_id.to_string());
```

### 1.5 第一阶段验证清单

| 验证项 | 测试方法 | 通过标准 |
|:---|:---|:---|
| **数据库迁移** | 执行 `migrate.go`，检查所有表结构 | 所有表存在、字段类型正确、索引创建 |
| **OAuth 登录流程** | 模拟 GitHub OAuth callback | 成功创建 User + Org + Workspace，返回有效 JWT |
| **JWT 验证** | 发送请求带合法/过期/篡改的 JWT | 合法:200, 过期:401, 篡改:401 |
| **权限隔离** | developer 角色尝试访问 `/api/v1/orgs/{other}` | 返回 403 Forbidden |
| **workspace 数据隔离** | workspace A 的 pipeline 列表不包含 workspace B 的数据 | GET pipelines 仅返回当前 workspace 的数据 |
| **SQLite 模式可用** | 设置 `--mode=sqlite` 启动 | 单机开发模式正常工作 |
| **Rust 数据面正确提取 tenant** | 发送带 JWT 的 chat completions 请求 | Trace 中 `tenant_id` 字段正确填充 |
| **所有现有测试仍然通过** | `cargo test --lib` + `go test ./...` | 185 passed + Go tests pass |
| **E2E 测试** | 完整用户旅程：注册→创建workspace→配置pipeline→发送请求→查看trace | 端到端可追溯 |

---

## 第二阶段：双轨制密钥与计费引擎

> **周期**：4 周 | **目标**：用户可以用自己的 Key（BYOK）或平台 Key 访问模型，费用实时可控

### 2.1 双轨制 Key 架构

#### 2.1.1 Virtual Key 模型

```sql
CREATE TABLE virtual_keys (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    workspace_id    UUID NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    name            VARCHAR(255) NOT NULL,
    key_prefix      VARCHAR(16) NOT NULL,           -- "vd-xxxx"
    key_hash        VARCHAR(64) NOT NULL UNIQUE,    -- SHA-256(完整key), 用于验证
    type            VARCHAR(16) NOT NULL,           -- "byok" | "platform"

    -- BYOK 专用
    provider_key_encrypted TEXT,                    -- AES-256-GCM 加密的真实 LLM Key
    provider_key_kms_id    VARCHAR(255),            -- KMS key ID (生产环境)

    -- 通用
    allowed_models  TEXT[] DEFAULT '{}',            -- 允许使用的模型列表
    rate_limit_rpm  INTEGER DEFAULT 60,             -- 每分钟最大请求数
    rate_limit_tpm  INTEGER DEFAULT 100000,         -- 每分钟最大 Token 数
    spend_limit_usd_micro BIGINT DEFAULT 0,         -- 日消费上限 (微美元), 0=无限
    status          VARCHAR(16) NOT NULL DEFAULT 'active', -- active|revoked|expired

    last_used_at    TIMESTAMPTZ,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_by      UUID NOT NULL REFERENCES users(id)
);
```

#### 2.1.2 Key 加密方案

```go
// internal/crypto/envelope.go
// 信封加密：数据密钥加密数据，主密钥加密数据密钥

type EncryptedKey struct {
    Ciphertext  []byte `json:"ciphertext"`   // AES-256-GCM 加密的 LLM Key
    DataKey     []byte `json:"data_key"`     // 加密后的数据密钥
    IV          []byte `json:"iv"`           // 初始化向量
    Version     int    `json:"version"`      // 加密方案版本
}

// 真实 Key 流转路径：
// 1. 用户在前端输入 LLM Key → HTTPS → Go 控制面
// 2. Go 控制面生成数据密钥 → AES-256-GCM 加密 LLM Key
// 3. 使用 KMS 主密钥加密数据密钥
// 4. 加密后的 LLM Key + 加密的数据密钥 → 存储到 PG
// 5. Rust 数据面请求时 → gRPC 调用 Go → Go 解密 → 返回明文 Key (仅内存)
```

#### 2.1.3 Key 路由解析流程

```
Rust 数据面接收请求
  │
  ├── 1. 提取 Virtual Key (从 Authorization 头: Bearer vd-xxxx)
  │
  ├── 2. HTTP POST → Go 控制面 /internal/resolve-key
  │        body: { "virtual_key_hash": "sha256(vd-xxxx)", "model": "gpt-4o" }
  │
  ├── 3. Go 控制面:
  │      ├── 验证 Virtual Key 有效性 (状态、限额、限流)
  │      ├── 检查模型是否在 allowed_models 中
  │      └── 解密对应 LLM Key
  │
  ├── 4. 响应:
  │      {
  │        "resolved": true,
  │        "provider": "openai",
  │        "provider_key": "sk-proj-... (明文, 仅内存)",  ← 传输层 TLS 加密
  │        "upstream_url": "https://api.openai.com",
  │        "rate_limit_remaining": 55,
  │        "budget_remaining_micro": 950000  // $0.95
  │      }
  │
  └── 5. Rust 使用返回的 Key 调用上游 LLM (不持久化, 响应用完后从内存清除)
```

### 2.2 FinOps 计费引擎

#### 2.2.1 钱包模型

```sql
CREATE TABLE wallets (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    workspace_id    UUID NOT NULL UNIQUE REFERENCES workspaces(id),
    balance_usd_micro       BIGINT NOT NULL DEFAULT 0,    -- 可用余额 (微美元)
    overdraft_limit_micro   BIGINT NOT NULL DEFAULT 0,    -- 透支额度
    last_credit_at  TIMESTAMPTZ,                           -- 最后充值时间
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE transactions (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    workspace_id    UUID NOT NULL REFERENCES workspaces(id),
    wallet_id       UUID NOT NULL REFERENCES wallets(id),
    type            VARCHAR(32) NOT NULL,      -- credit|debit|refund|correction
    amount_usd_micro BIGINT NOT NULL,          -- 金额 (正数=增加, 负数=减少)
    balance_after_micro BIGINT NOT NULL,       -- 交易后余额
    description     TEXT,
    trace_id        UUID,                       -- 关联的 Trace
    metadata        JSONB DEFAULT '{}',
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX idx_transactions_workspace ON transactions(workspace_id, created_at DESC);
```

#### 2.2.2 Redis 实时预算扣减

```lua
-- scripts/budget_decr.lua
-- KEYS[1]: workspace:{id}:budget  (当前预算剩余, 微美元)
-- KEYS[2]: workspace:{id}:budget:daily  (日预算已消耗)
-- ARGV[1]: 本次要扣减的金额 (微美元)
-- ARGV[2]: 日预算上限 (微美元, 0=无限)
-- ARGV[3]: 请求 ID (用于日志)

local remaining = redis.call('DECRBY', KEYS[1], ARGV[1])

if remaining < 0 then
    -- 余额不足, 回滚
    redis.call('INCRBY', KEYS[1], ARGV[1])
    return {0, "budget_exceeded", redis.call('GET', KEYS[1])}
end

-- 检查日预算
local daily_limit = tonumber(ARGV[2])
if daily_limit > 0 then
    local daily_spent = redis.call('INCRBY', KEYS[2], ARGV[1])
    if daily_spent > daily_limit then
        -- 日预算超限, 回滚
        redis.call('INCRBY', KEYS[1], ARGV[1])
        redis.call('DECRBY', KEYS[2], ARGV[1])
        return {0, "daily_limit_exceeded", tostring(daily_spent)}
    end
    -- 设置日预算 key 的 TTL (到当天结束)
    local now = redis.call('TIME')
    local seconds_today = now[1] % 86400
    redis.call('EXPIRE', KEYS[2], 86400 - seconds_today + 60)
end

return {1, "ok", tostring(remaining)}
```

#### 2.2.3 Rust 数据面流式熔断

```rust
// core/src/budget/stream_guard.rs (新增)
pub struct StreamBudgetGuard {
    redis: RedisClient,
    workspace_id: String,
    check_interval: usize,  // 每 N 个 token 检查一次
    tokens_since_check: usize,
    cost_per_token_micro: u64,
}

impl StreamBudgetGuard {
    pub async fn check_and_decr(&mut self) -> Result<BudgetStatus, BudgetError> {
        self.tokens_since_check += 1;
        if self.tokens_since_check < self.check_interval {
            return Ok(BudgetStatus::Ok);
        }

        let cost = self.tokens_since_check as u64 * self.cost_per_token_micro;
        let result: LuaResult = self.redis
            .eval("budget_decr.lua", &[budget_key, daily_key], &[cost, daily_limit, request_id])
            .await?;

        self.tokens_since_check = 0;

        match result.status {
            1 => Ok(BudgetStatus::Ok),
            0 => {
                // 立即切断 SSE 流
                Err(BudgetError::Exceeded(result.reason))
            }
            _ => Err(BudgetError::Internal),
        }
    }
}
```

### 2.3 第二阶段验证清单

| 验证项 | 测试方法 | 通过标准 |
|:---|:---|:---|
| **Virtual Key 创建** | 前端创建 BYOK 和 Platform 两种 Key | 正确加密存储，返回 key_prefix |
| **Key 验证** | 使用无效 Key 发送请求 | 返回 401 |
| **Key 吊销** | 吊销后使用原 Key 发送请求 | 返回 401 |
| **Key 路由解析** | Rust 数据面请求 `/internal/resolve-key` | 正确返回解密后的 LLM Key |
| **LLM Key 不落地** | 检查 Rust 内存和日志 | 无真实 LLM Key 在日志/持久层中出现 |
| **钱包充值** | 通过 API 增加余额 | 余额正确更新 |
| **预算扣减** | 发送请求消耗 Token | Redis 余额正确减少 |
| **余额不足熔断** | 消耗完预算后继续请求 | SSE 流被立即切断，返回 `[VERIDACTUS:BUDGET_EXCEEDED]` |
| **日预算限制** | 超过日限额后请求 | 返回 429 + `daily_limit_exceeded` |
| **Redis 故障降级** | 停止 Redis 后发送请求 | 服务降级但不断链 (使用内存计数兜底) |
| **并发预算扣减正确性** | 100 并发请求同时扣减 | Redis Lua 原子性保证无 race condition |

---

## 第三阶段：前端双引擎重构

> **周期**：4 周 | **目标**：用户 5 分钟内的 Wow Moment™

**技术栈保持不变**：Vite + React 18 + TypeScript + TailwindCSS + shadcn/ui + ReactFlow + Framer Motion + Zustand

### 3.1 前端架构重构

#### 3.1.1 新目录结构

```
veridactus-ui/src/
├── app/
│   ├── App.tsx                    # 根组件 (认证路由 + 主题提供者)
│   ├── router.tsx                 # 路由配置 (公开/认证/管理三种路由组)
│   └── providers.tsx              # 全局 Provider (Auth, Theme, i18n, Store)
├── engines/                       # 双引擎
│   ├── chat/                      # 引擎 A: VERIDACTUS Chat
│   │   ├── ChatPage.tsx           # 对话主页面
│   │   ├── ChatInput.tsx          # 输入框 + 动态安全盾牌
│   │   ├── ChatMessage.tsx        # 消息气泡
│   │   ├── ModelSelector.tsx      # 模型选择器 (悬浮)
│   │   ├── SafetyShield.tsx       # 🛡️ 动态安全盾牌组件
│   │   ├── ABCompare.tsx          # ⚔️ A/B 对比模式
│   │   └── ChatSidebar.tsx        # 对话历史侧边栏
│   ├── devhub/                    # 引擎 B: Developer Hub
│   │   ├── PlaygroundPage.tsx     # Playground 主页面
│   │   ├── PromptEditor.tsx       # 左侧 Prompt 编辑器
│   │   ├── XRayPanel.tsx          # 右侧 X 光面板
│   │   ├── StreamOutput.tsx       # 中间流式输出
│   │   └── TokenMonitor.tsx       # Token 消耗实时监控
│   └── vault/                     # Holo-Trace Vault
│       ├── VaultPage.tsx          # Trace 列表看板
│       ├── VaultDetail.tsx        # 上帝视角分屏
│       ├── CompareView.tsx        # Raw vs Sanitized 左右分屏
│       ├── CryptoVerify.tsx       # 🔗 密码学自证组件
│       └── VaultExport.tsx        # 批量导出
├── studio/                        # Pipeline Studio
│   ├── StudioPage.tsx             # 流水线设计器 (已有, 增强)
│   ├── Canvas.tsx                 # ReactFlow 画布
│   ├── PluginLibrary.tsx          # 插件库抽屉
│   ├── PropertyPanel.tsx          # 属性面板
│   └── TemplateSelector.tsx       # 一键模板
├── admin/                         # 管理控制台
│   ├── AdminPage.tsx              # 组织管理
│   ├── WorkspacePage.tsx          # 工作空间管理
│   ├── MemberPage.tsx             # 成员管理
│   ├── ApiKeyPage.tsx             # API Key 管理 (已有, 增强)
│   ├── ModelPage.tsx              # 模型管理 (已有, 增强)
│   ├── BillingPage.tsx            # 计费与账单
│   └── SettingsPage.tsx           # 设置 (已有, 增强)
├── auth/                          # 认证模块
│   ├── LoginPage.tsx              # 登录页 (GitHub/Google/Email)
│   ├── OnboardingPage.tsx         # 引导页 (BYOK vs 平台)
│   ├── AuthGuard.tsx              # 路由守卫
│   └── useAuth.ts                 # 认证 Hook
├── components/                    # 共享组件 (保持现有结构)
│   ├── ui/                        # 基础 UI (GlassCard, Button 等)
│   ├── viz/                       # 可视化 (CircularProgress, 等)
│   ├── atoms/                     # 原子组件 (Badge, Icon 等)
│   └── layout/                    # 布局 (Sidebar, StatusBar 等)
├── lib/                           # 工具库
│   ├── api/client.ts              # 统一 HTTP 客户端 (JWT + 错误处理)
│   ├── api/endpoints.ts           # API 端点定义
│   ├── crypto/                    # 密码学工具
│   │   ├── jcs.ts                 # JCS 规范化 (json-canonicalize)
│   │   └── verify.ts              # L0 签名验证 (Web Crypto API)
│   ├── hooks/                     # 通用 Hooks
│   │   ├── useSSE.ts              # SSE 流式 Hook
│   │   ├── useBudget.ts           # 预算实时查询
│   │   └── useWebSocket.ts        # WebSocket Hook (安全盾牌)
│   └── i18n/                      # 国际化
│       ├── index.ts
│       ├── zh.ts
│       └── en.ts
├── store/                         # Zustand 状态管理
│   ├── authStore.ts               # 认证状态
│   ├── workspaceStore.ts          # 当前工作空间
│   ├── chatStore.ts               # 对话状态
│   ├── budgetStore.ts             # 预算实时状态
│   └── uiStore.ts                 # UI 状态 (主题、侧边栏等)
└── types/                         # TypeScript 类型定义
    ├── models.ts                  # 数据模型
    ├── api.ts                     # API 请求/响应类型
    └── schema.ts                  # Zod 验证 Schema
```

#### 3.1.2 路由设计

```typescript
// app/router.tsx
const routes = [
  // === 公开路由 ===
  { path: '/login', element: <LoginPage /> },
  { path: '/auth/callback/:provider', element: <OAuthCallback /> },

  // === 认证路由 ===
  { path: '/onboarding', element: <OnboardingPage /> },         // 首次登录引导
  { path: '/chat', element: <ChatPage /> },                     // 引擎 A
  { path: '/playground', element: <PlaygroundPage /> },         // 引擎 B
  { path: '/vault', element: <VaultPage /> },                   // 金库列表
  { path: '/vault/:traceId', element: <VaultDetail /> },        // 金库详情

  // === 管理路由 (需对应角色) ===
  { path: '/studio', element: <StudioPage /> },                 // 现有
  { path: '/studio/:id', element: <StudioPage /> },             // 编辑
  { path: '/admin', element: <AdminPage /> },
  { path: '/admin/workspace', element: <WorkspacePage /> },
  { path: '/admin/members', element: <MemberPage /> },
  { path: '/admin/apikeys', element: <ApiKeyPage /> },
  { path: '/admin/models', element: <ModelPage /> },
  { path: '/admin/billing', element: <BillingPage /> },
  { path: '/admin/settings', element: <SettingsPage /> },

  // 默认重定向
  { path: '/', element: <Navigate to="/chat" /> },
  { path: '*', element: <Navigate to="/chat" /> },
];
```

### 3.2 引擎 A：VERIDACTUS Chat 实现规格

#### 3.2.1 组件树

```
ChatPage
├── ChatSidebar (左侧对话历史)
│   ├── NewChatButton
│   ├── SearchBar
│   └── ChatHistoryList
│        └── ChatHistoryItem (可重命名、删除)
├── ChatMain (中间对话区)
│   ├── ChatHeader
│   │   ├── ModelSelector      ← 悬浮，支持搜索过滤
│   │   └── ABCompareToggle   ← ⚔️ 对比模式开关
│   ├── ChatMessages
│   │   └── ChatMessage (每组 request+response)
│   │        ├── UserBubble
│   │        ├── AssistantBubble
│   │        │   ├── StreamingText (SSE)
│   │        │   ├── TokenCounter (实时跳动)
│   │        │   ├── BudgetBar (预算消耗进度条)
│   │        │   └── SafetyBadge (安全状态: 🟢🟡🔴)
│   │        └── ABCompare (A/B 模式下左右并排)
│   ├── ChatInput
│   │   ├── SafetyShield (🛡️ 动态盾牌)
│   │   ├── TextArea
│   │   ├── GuardrailSelector (快捷选择 G1-G4)
│   │   └── SendButton
│   └── BudgetAwarenessBanner (可选: 剩余预算提示)
└── ChatRightPanel (可选: 右侧上下文面板)
    └── TraceQuickView (当前对话的 Trace 摘要)
```

#### 3.2.2 动态安全盾牌实现

```typescript
// engines/chat/SafetyShield.tsx
// 行为：
//   - 输入框为空 → 灰色盾牌
//   - 输入安全文本 → 绿色盾牌 + "安全"
//   - 检测到 PII → 黄色盾牌 + 呼吸灯 + "已识别 PII"
//   - 检测到注入攻击 → 红色盾牌 + "检测到风险输入"

function SafetyShield({ text }: { text: string }) {
  const [status, setStatus] = useState<'idle' | 'safe' | 'warning' | 'danger'>('idle');
  const [tooltip, setTooltip] = useState('');

  useEffect(() => {
    // 防抖 300ms 检测 (避免每次按键都触发)
    const timer = setTimeout(() => {
      if (!text.trim()) {
        setStatus('idle');
        return;
      }
      // 前端正则预检 (快速, <1ms)
      const piiCount = countPiiMatches(text);
      const injectionScore = checkInjectionPatterns(text);

      if (injectionScore > 0.7) {
        setStatus('danger');
        setTooltip('高风险: 检测到潜在注入攻击');
      } else if (piiCount > 0) {
        setStatus('warning');
        setTooltip(`已识别 ${piiCount} 项敏感信息，发送时将自动掩码`);
      } else {
        setStatus('safe');
        setTooltip('输入安全');
      }
    }, 300);
    return () => clearTimeout(timer);
  }, [text]);

  return (
    <motion.div
      animate={{
        scale: status === 'warning' ? [1, 1.05, 1] : 1,
        boxShadow: status === 'warning'
          ? ['0 0 0px #fdcb6e', '0 0 20px #fdcb6e', '0 0 0px #fdcb6e']
          : 'none',
      }}
      transition={{ repeat: status === 'warning' ? Infinity : 0, duration: 2 }}
    >
      <ShieldIcon color={shieldColors[status]} />
      <span>{shieldLabels[status]}</span>
      {tooltip && <Tooltip>{tooltip}</Tooltip>}
    </motion.div>
  );
}
```

#### 3.2.3 A/B 对比模式

```typescript
// engines/chat/ABCompare.tsx
// 同一 prompt 并发请求两个模型，左右分屏展示
function ABCompare({ prompt, modelA, modelB }: ABProps) {
  const [streamA, setStreamA] = useState('');
  const [streamB, setStreamB] = useState('');

  useEffect(() => {
    // 使用 useSSE Hook 并发请求
    const controller = new AbortController();

    Promise.all([
      streamChatCompletion(modelA, prompt, controller.signal),
      streamChatCompletion(modelB, prompt, controller.signal),
    ]).then(([a, b]) => {
      // 使用 requestAnimationFrame 同步渲染
      const sync = (fn: () => void) => requestAnimationFrame(fn);
      sync(() => setStreamA(a));
      sync(() => setStreamB(b));
    });

    return () => controller.abort();
  }, [prompt, modelA, modelB]);

  return (
    <div className="flex gap-4 h-full">
      <div className="flex-1 border-r">
        <ModelLabel model={modelA} />
        <StreamingText text={streamA} />
      </div>
      <div className="flex-1">
        <ModelLabel model={modelB} />
        <StreamingText text={streamB} />
      </div>
    </div>
  );
}
```

### 3.3 引擎 B：Developer Hub 实现规格

#### 3.3.1 X 光面板

```typescript
// engines/devhub/XRayPanel.tsx
// 三栏布局的右侧面板，实时展示治理透视信息
function XRayPanel({ traceId, streamingText }: XRayProps) {
  return (
    <div className="flex flex-col gap-4 p-4">
      {/* 1. Raw vs Sanitized 差异视图 */}
      <RequestDiff raw={rawRequest} sanitized={sanitizedRequest} />

      {/* 2. Token 消耗速率 (实时跳动数字) */}
      <TokenRateMonitor tokens={currentTokens} elapsed={elapsed} />

      {/* 3. 预算剩余 (环形进度条) */}
      <CircularProgress
        value={budgetRemaining}
        max={budgetTotal}
        color={budgetRemaining > 0.2 ? '#00d4aa' : '#ff7675'}
      />

      {/* 4. Guardrail 触发记录 */}
      <GuardrailLog events={safetyEvents} />

      {/* 5. 约束冲突检测 */}
      <ConstraintConflictView conflicts={constraintConflicts} />

      {/* 6. 证明链状态 */}
      <ProofChainStatus levels={['L0', 'L2A', 'L2B']} />
    </div>
  );
}
```

### 3.4 Holo-Trace Vault 实现规格

#### 3.4.1 密码学自证组件

```typescript
// engines/vault/CryptoVerify.tsx
// 核心体验：点击按钮 → Web Crypto API 计算 → 比对 → 动效
import canonicalize from 'json-canonicalize';

async function CryptoVerify({ trace }: { trace: TraceDetail }) {
  const [verifying, setVerifying] = useState(false);
  const [result, setResult] = useState<'idle' | 'pass' | 'fail'>('idle');

  const handleVerify = async () => {
    setVerifying(true);
    try {
      // 1. 获取 Trace JSON → 剥离内部字段 → JCS 规范化
      const stripped = stripInternalFields(trace);
      const canonical = canonicalize(stripped);

      // 2. Web Crypto API 计算 SHA-256 (客户端)
      const encoder = new TextEncoder();
      const hashBuffer = await crypto.subtle.digest('SHA-256', encoder.encode(canonical));
      const hashArray = Array.from(new Uint8Array(hashBuffer));
      const hashHex = hashArray.map(b => b.toString(16).padStart(2, '0')).join('');

      // 3. 与后端 audit_signature 比对
      const match = hashHex.toLowerCase() === trace.proofs.audit_signature.toLowerCase();
      setResult(match ? 'pass' : 'fail');
    } catch (err) {
      setResult('fail');
    } finally {
      setVerifying(false);
    }
  };

  return (
    <div>
      <button onClick={handleVerify} disabled={verifying}>
        {verifying ? '验证中...' : '🔍 验证签名'}
      </button>

      {result === 'pass' && (
        <ParticleBurstEffect />  {/* 绿色粒子爆发全屏动效 */}
      )}
      {result === 'fail' && (
        <FailAlert message="签名不匹配，此记录可能已被篡改" />
      )}
    </div>
  );
}
```

### 3.5 前端全局状态管理

```typescript
// store/index.ts
interface RootStore {
  // 认证
  auth: {
    user: User | null;
    token: string | null;
    isLoading: boolean;
    login: (provider: string) => Promise<void>;
    logout: () => void;
  };

  // 工作空间
  workspace: {
    current: Workspace | null;
    workspaces: Workspace[];
    setCurrent: (ws: Workspace) => void;
  };

  // 对话
  chat: {
    conversations: Conversation[];
    activeId: string | null;
    sendMessage: (content: string, model: string) => Promise<void>;
  };

  // 预算 (实时)
  budget: {
    remaining: number;   // 微美元
    dailySpent: number;
    dailyLimit: number;
    refresh: () => Promise<void>;
  };

  // UI
  ui: {
    theme: 'dark' | 'light';
    sidebarCollapsed: boolean;
    toggleTheme: () => void;
    toggleSidebar: () => void;
  };
}
```

### 3.6 第三阶段验证清单

| 验证项 | 测试方法 | 通过标准 |
|:---|:---|:---|
| **OAuth 登录流程** | 点击 GitHub 登录 → 授权 → 回调 | 自动创建账号，跳转到 Onboarding |
| **Onboarding 引导** | 首次登录 → 选择 "我有 API Key" 或 "我需要" | 正确创建 BYOK 或 Platform Key |
| **Chat 发送消息** | 输入文本 → 点击发送 | SSE 流式输出正常，Token 计数跳动 |
| **安全盾牌动效** | 输入身份证号/邮箱 | 盾牌变黄 + 呼吸灯 + 提示文字 |
| **预算熔断展示** | 消耗完预算后发送 | 前端提示预算不足，输入框禁用 |
| **A/B 对比** | 开启对比模式 → 选择两个模型 → 发送 | 左右分屏同步输出 |
| **X 光面板** | Playground 发送请求 | 右侧实时显示请求/响应对比、Token 速率、预算 |
| **Trace 列表看板** | 进入 Vault 页面 | 表格显示成本、安全状态、归属标签 |
| **上帝视角分屏** | 点击 Trace 详情 | 左 Raw / 右 Sanitized，滚动百分比同步 |
| **密码学自证** | 点击"验证"按钮 | SHA-256 计算正确 → 绿色粒子爆发动效 |
| **密码学自证(篡改)** | 修改 Trace 后验证 | 签名不匹配 → 红色警告 |
| **暗黑/亮色切换** | 切换主题 | 所有组件正确渲染 |
| **i18n 中英文** | 切换语言 | 所有文本正确翻译 |
| **移动端响应式** | 缩小浏览器窗口至 375px | 布局不崩，关键功能可用 |
| **前端测试** | `vitest run` | 核心组件 + API 测试通过率 > 80% |

---

## 第四阶段：企业级管控与合规

> **周期**：3 周 | **目标**：CISO/CIO 的管控武器，支撑高客单价企业版销售

### 4.1 企业 SSO 集成

```go
// internal/auth/sso.go
type SSOProvider interface {
    GetAuthURL(state string) string
    ExchangeCode(code string) (*SSOUser, error)
    ValidateToken(token string) (*SSOUser, error)
}

// 支持 provider:
// - Okta (OIDC)
// - Azure AD (SAML/OIDC)
// - 飞书 (OIDC)
// - 钉钉 (OIDC)
// - 通用 SAML 2.0

type SSOConfig struct {
    Provider     string `json:"provider"`      // okta|azure|feishu|dingtalk|saml
    ClientID     string `json:"client_id"`
    ClientSecret string `json:"client_secret"` // KMS 加密存储
    IssuerURL    string `json:"issuer_url"`
    Domain       string `json:"domain"`        // 邮件域名映射 (auto-join)
    JITProvisioning bool `json:"jit_provisioning"` // Just-in-Time 自动创建用户
}
```

### 4.2 审计指挥舱

```sql
-- ClickHouse: 审计事件表 (OLAP 优化)
CREATE TABLE audit_events (
    event_id        UUID,
    org_id          UUID,
    workspace_id    UUID,
    event_type      LowCardinality(String),   -- pii_detected|injection_blocked|budget_exceeded|guardrail_triggered
    severity        LowCardinality(String),   -- low|medium|high|critical
    trace_id        UUID,
    user_id         UUID,
    model           String,
    cost_usd_micro  Int64,
    asi_risk_id     LowCardinality(String),   -- ASI01-ASI10
    metadata        String,                    -- JSON
    created_at      DateTime64(3)
) ENGINE = MergeTree()
PARTITION BY toYYYYMM(created_at)
ORDER BY (org_id, workspace_id, event_type, created_at);
```

```typescript
// admin/AuditCenter.tsx
// 审计指挥舱页面
function AuditCommandCenter() {
  return (
    <div className="grid grid-cols-2 gap-4">
      {/* 左上: 今日拦截次数趋势 (ECharts 折线图) */}
      <Card>
        <LineChart data={dailyBlocks} />
      </Card>

      {/* 右上: PII 泄露尝试热力图 (按 Workspace) */}
      <Card>
        <Heatmap data={piiByWorkspace} />
      </Card>

      {/* 左下: Guardrail 触发分布 (饼图) */}
      <Card>
        <PieChart data={guardrailDistribution} />
      </Card>

      {/* 右下: 高风险用户 Top 10 (排序表格) */}
      <Card>
        <Table data={topRiskyUsers} />
      </Card>

      {/* 底部: 一键合规报告生成 */}
      <ComplianceReportGenerator
        onGenerate={async (dateRange) => {
          // 异步生成 PDF 报告
          const jobId = await createComplianceReport(dateRange);
          // 轮询直到完成
          pollUntilComplete(jobId, downloadReport);
        }}
      />
    </div>
  );
}
```

### 4.3 合规报告 PDF 生成

```python
# python-worker/app/report.py (新增)
# 异步任务: 打包 Merkle Root + ZK 证明 → 生成签名 PDF
def generate_compliance_report(trace_ids: List[str], regulation: str) -> bytes:
    """
    生成合规报告 PDF:
    1. 从 ClickHouse 查询指定时间段的 Trace
    2. 聚合 L2A Merkle Root
    3. 包含 L2B ZK 证明的 base64 编码
    4. 生成带有 SHA-256 数字签名的 PDF
    5. 附带离线验证 Python 脚本
    """
    traces = fetch_traces_from_clickhouse(trace_ids)
    merkle_root = compute_aggregate_merkle(traces)
    zk_proofs = collect_zk_proofs(traces)

    pdf = build_pdf({
        'title': f'VERIDACTUS Compliance Report - {regulation}',
        'date_range': f'{traces[0].created_at} → {traces[-1].created_at}',
        'trace_count': len(traces),
        'merkle_root': merkle_root,
        'zk_proofs': zk_proofs,
        'compliance_articles': compliance_mappings[regulation],
        'signature': sign_report(merkle_root, zk_proofs),
    })

    # 同时生成验证脚本
    verify_script = generate_verify_script(merkle_root)
    return zip_pdf_and_script(pdf, verify_script)
```

### 4.4 白标定制

```typescript
// admin/BrandSettings.tsx
function BrandSettings({ org }: { org: Organization }) {
  const [logo, setLogo] = useState<File | null>(null);
  const [primaryColor, setPrimaryColor] = useState(org.primary_color || '#6c5ce7');
  const [logoUrl, setLogoUrl] = useState(org.logo_url || '');

  const handleSave = async () => {
    // 上传 Logo 到 MinIO
    let newLogoUrl = logoUrl;
    if (logo) {
      newLogoUrl = await uploadLogo(org.id, logo);
    }

    // 更新组织设置
    await updateOrg(org.id, {
      logo_url: newLogoUrl,
      primary_color: primaryColor,
    });

    // 动态注入 CSS Variables
    document.documentElement.style.setProperty('--brand-primary', primaryColor);
    document.documentElement.style.setProperty('--brand-logo', `url(${newLogoUrl})`);
  };

  // 预览
  return (
    <div>
      <ColorPicker value={primaryColor} onChange={setPrimaryColor} />
      <LogoUploader value={logo} onChange={setLogo} />
      <PreviewPanel color={primaryColor} logo={logoUrl} />
      <Button onClick={handleSave}>保存品牌设置</Button>
    </div>
  );
}
```

### 4.5 第四阶段验证清单

| 验证项 | 测试方法 | 通过标准 |
|:---|:---|:---|
| **Okta SSO 登录** | 配置 Okta → 企业用户 SSO 登录 | 自动创建/关联账号 |
| **飞书 SSO 登录** | 配置飞书 → SSO 登录 | 自动创建/关联账号 |
| **JIT Provisioning** | 首次 SSO 登录 | 自动创建 User + 加入默认 Workspace |
| **审计事件存储** | 触发安全事件 → 查询 ClickHouse | 事件正确写入，按 org/workspace 聚合正确 |
| **PII 热力图** | 多个 workspace 触发 PII 检测 | 热力图按 workspace 正确展示 |
| **合规报告 PDF** | 生成 EU AI Act 报告 | PDF 包含 Merkle Root + ZK 证明 + 数字签名 |
| **离线验证脚本** | 下载验证脚本 + PDF → 本地 Python 执行 | 脚本正确验证 PDF 签名 |
| **白标 Logo** | 上传 Logo + 设置颜色 | Chat UI 显示企业 Logo 和品牌色 |
| **品牌颜色注入** | 修改 primary_color → 刷新页面 | 所有组件颜色正确更新 |
| **权限矩阵完整** | 测试所有 5 种角色的 API 权限 | 每个端点返回正确的 200/403 |

---

## 第五阶段：生产化打磨

> **周期**：2 周 | **目标**：生产级可靠性、安全性和可观测性

### 5.1 可观测性

```yaml
# 指标导出
veridactus_requests_total{workspace_id,model,status}
veridactus_request_duration_seconds{workspace_id,model,quantile}
veridactus_tokens_consumed_total{workspace_id,model}
veridactus_budget_remaining{workspace_id}
veridactus_safety_events_total{event_type,severity}
veridactus_pii_detections_total{pii_type}
veridactus_l0_signature_verify_total{result}     # pass/fail
veridactus_redis_operations_duration_seconds{operation}
veridactus_postgres_query_duration_seconds{query}
```

```bash
# Grafana Dashboard 面板
1. 全局 QPS + 延迟 P50/P95/P99
2. 预算消耗速率 (实时)
3. 安全事件热力图
4. L0 签名验证成功率
5. Redis 操作延迟分布
6. 上游 LLM 错误率 (按模型)
```

### 5.2 安全加固

| 加固项 | 实现 | 验证方式 |
|:---|:---|:---|
| **secrets 扫描** | `.pre-commit-config.yaml` 集成 `detect-secrets` | CI 流水线自动扫描 |
| **依赖审计** | `cargo audit` + `npm audit` + `go mod tidy` | 每周自动运行 |
| **SQL 注入防护** | 全部使用参数化查询 (PostgreSQL `$1, $2`) | 代码审查 + 静态分析 |
| **XSS 防护** | React 默认转义 + CSP headers | OWASP ZAP 扫描 |
| **Rate Limiting** | nginx `limit_req_zone` + Go 控制面令牌桶 | 压测验证 |
| **TLS 1.3** | 所有外部通信强制 TLS 1.3+ | SSL Labs A+ 评分 |
| **secrets 加密** | LLM Key: AES-256-GCM + KMS | 数据库中无明文 Key |
| **审计日志** | 所有管理操作写入 audit_events | 不可删除 |

### 5.3 性能目标

| 指标 | 目标 | 验证方法 |
|:---|:---|:---|
| **P50 延迟** (不含 LLM) | < 5ms | `wrk` 压测 |
| **P99 延迟** (不含 LLM) | < 20ms | `wrk` 压测 |
| **并发连接数** | > 1000 SSE 连接 | `k6` 压测 |
| **内存占用** (Rust) | < 256MB (空闲), < 512MB (满载) | `docker stats` |
| **Redis 预算扣减** | < 1ms (P99) | 内部 metrics |
| **数据库查询** (PostgreSQL) | < 5ms (P95) | pg_stat_statements |
| **前端 FCP** | < 1.5s | Lighthouse |
| **前端 LCP** | < 2.5s | Lighthouse |
| **前端 TBT** | < 200ms | Lighthouse |

### 5.4 灾难恢复

| 场景 | 策略 | RPO | RTO |
|:---|:---|:---|:---|
| **PostgreSQL 故障** | 主从复制 + 自动 failover | < 1分钟 | < 5分钟 |
| **Redis 故障** | Sentinel 自动切换 | 0 (预算扣减降级到内存) | < 30秒 |
| **Rust 数据面崩溃** | K8s/Docker 自动重启 | 0 (无状态) | < 10秒 |
| **MinIO 故障** | 多节点纠删码 | 0 | < 1分钟 |
| **整个区域故障** | 跨区域部署 (Phase 5+) | 取决于复制延迟 | < 30分钟 |

### 5.5 第五阶段验证清单

| 验证项 | 测试方法 | 通过标准 |
|:---|:---|:---|
| **负载测试** | `k6` 模拟 1000 并发 SSE 连接 | 错误率 < 0.1% |
| **压力测试** | 逐步增加并发直到服务降级 | 优雅降级，不崩溃 |
| **Redis 故障恢复** | 手动停止 Redis → 发送请求 → 恢复 Redis | 请求在内存模式继续，恢复后无缝切换 |
| **PG 故障恢复** | 模拟主库故障 | 自动切换到从库，数据无丢失 |
| **安全扫描** | `cargo audit` + `npm audit` + OWASP ZAP | 0 个 high/critical 漏洞 |
| **secrets 泄露检查** | `git log -p \| grep -i 'sk-'` | 0 个匹配 |
| **前端性能审计** | Lighthouse 报告 | FCP < 1.5s, LCP < 2.5s |
| **全链路 E2E** | 注册 → 登录 → 创建 Key → 发送 Chat → 查看 Trace → 验证签名 | 完整链路无断点 |

---

## 附录：接口契约定义

### A.1 数据面 API (Rust, :8080)

```
# 受治理的聊天补全 (已有, 保持兼容)
POST   /v1/chat/completions
  Headers:
    Authorization: Bearer <jwt-or-virtual-key>
    VERIDACTUS-Budget-Limit: 0.10
    VERIDACTUS-Privacy-Level: masked
    VERIDACTUS-Guardrails: G1,G2,G3
    VERIDACTUS-Compliance-Profile: EU_AI_ACT_GPAI
    VERIDACTUS-Certified-Guarantee: C-SafeGen:0.01@0.99
  Body: OpenAI-compatible Chat Completions request
  Response: OpenAI-compatible (SSE if stream=true)

# 审计 Trace (已有)
GET    /v1/traces
GET    /v1/traces/:trace_id
POST   /v1/traces/:trace_id/replay
POST   /v1/traces/:trace_id/verify    # L0 签名验证
DELETE /v1/traces/:trace_id           # GDPR 删除

# 实时指标
GET    /v1/metrics/realtime

# 健康检查
GET    /health
```

### A.2 控制面 API (Go, :8081)

```
# === 认证 ===
GET    /api/v1/auth/login/{provider}           # provider: github|google
GET    /api/v1/auth/callback/{provider}?code=  # OAuth callback
POST   /api/v1/auth/refresh                     # { "refresh_token": "..." }
POST   /api/v1/auth/logout                      # 吊销 refresh_token

# === 组织 ===
GET    /api/v1/orgs                             # 列出用户所属组织
GET    /api/v1/orgs/{orgId}                     # 组织详情
PUT    /api/v1/orgs/{orgId}                     # 更新组织设置

# === 工作空间 ===
GET    /api/v1/workspaces                       # 列出当前组织的工作空间
POST   /api/v1/workspaces                       # { "name": "...", "org_id": "..." }
GET    /api/v1/workspaces/{wsId}                # 工作空间详情
PUT    /api/v1/workspaces/{wsId}                # 更新
DELETE /api/v1/workspaces/{wsId}                # 删除

# === 成员 ===
GET    /api/v1/workspaces/{wsId}/members        # 成员列表
POST   /api/v1/workspaces/{wsId}/members        # 邀请 { "email": "...", "role": "..." }
PUT    /api/v1/workspaces/{wsId}/members/{id}   # 修改角色
DELETE /api/v1/workspaces/{wsId}/members/{id}   # 移除

# === Virtual Key ===
GET    /api/v1/virtual-keys                     # 列表
POST   /api/v1/virtual-keys                     # 创建 { "name": "", "type": "byok", "provider_key": "sk-..." }
GET    /api/v1/virtual-keys/{keyId}             # 详情 (不返回 provider_key)
PUT    /api/v1/virtual-keys/{keyId}             # 更新限额
DELETE /api/v1/virtual-keys/{keyId}             # 吊销

# === 钱包 ===
GET    /api/v1/wallets/{wsId}                   # 钱包余额
GET    /api/v1/wallets/{wsId}/transactions      # 交易记录 (分页)
POST   /api/v1/wallets/{wsId}/topup             # 充值 (管理端)

# === 流水线 ===
GET    /api/v1/pipelines
POST   /api/v1/pipelines
GET    /api/v1/pipelines/{id}
PUT    /api/v1/pipelines/{id}
DELETE /api/v1/pipelines/{id}

# === 模型 ===
GET    /api/v1/models
POST   /api/v1/models
GET    /api/v1/models/{id}
PUT    /api/v1/models/{id}
DELETE /api/v1/models/{id}

# === 设置 ===
GET    /api/v1/settings                         # 获取所有设置
POST   /api/v1/settings                         # 更新设置 { "key": "value" }

# === 合规 ===
POST   /api/v1/compliance/reports               # 生成合规报告 (异步)
GET    /api/v1/compliance/reports/{jobId}        # 查询报告生成状态
GET    /api/v1/compliance/reports/{jobId}/download  # 下载报告

# === 内部 (仅供数据面调用) ===
POST   /internal/resolve-key                    # Key 路由解析
GET    /api/v1/config/poll                      # 配置长轮询 (已有)
```

### A.2.2 内部端点 (数据面)

```
POST   /internal/resolve-key              # Key 路由解析 (Rust→Go)
GET    /api/v1/config/poll                # 配置长轮询
```

### A.3 Python Worker API (:8001)

```
GET    /health                            # 健康检查
POST   /api/v1/compute-guarantee          # C-SafeGen 认证保证
POST   /api/v1/drift-detection            # 语义漂移检测 (余弦相似度)
POST   /api/v1/pii-detection              # PII 深度检测
GET    /api/v1/pii-detection?text=...     # PII 检测 (GET)
POST   /api/v1/compliance/report/generate # 合规报告生成
POST   /plugin/execute                    # V3: 统一插件执行端点
                                          # 请求: {"plugin":"name","stage":"pre_request","request":{...}}
                                          # 响应: {"action":"continue"|"block"|"flag"}
```

## 附录 B: 插件体系架构

### B.1 三层插件模型

| 类型 | 运行时 | 延迟 | 适用场景 |
|:---|:---|:---|:---|
| **Native** | Rust 内联编译 | <10μs | 预算/PII/注入检测 (热路径) |
| **WASM** | wasmtime 沙箱 | 50-200μs | 社区插件 (可选 feature `wasm-runtime`) |
| **Sidecar** | HTTP REST → Python | 5-500ms | ML 算法/深度分析 (温/冷路径) |

### B.2 WASM ABI (Guest 导出)

```rust
// Guest .wasm 文件必须导出以下函数:
fn on_request(ctx_ptr: i32, ctx_len: i32) -> i32   // 0=Continue 1=Block 2=Degrade 3=Flag
fn on_response(ctx_ptr: i32, ctx_len: i32) -> i32
fn on_async_finalize(ctx_ptr: i32, ctx_len: i32) -> i32
```

### B.3 Sidecar 协议

```
POST /plugin/execute
{
  "plugin": "content-safety-scorer",   // Python 插件名
  "stage": "pre_request",              // pre_request|streaming|post_response|async_finalize
  "request": { ... }                   // GovernancePlugin trait 上下文
}

响应:
{
  "action": "continue",                // continue|block|flag|degrade
  "score": 0.03,                       // 可选, 算法评估分数
  "reason": "..."                      // 可选
}
```

### B.4 添加新 Python 算法插件

```python
# python-worker/app/main.py
PYTHON_PLUGIN_ROUTER = {
    "content-safety-scorer": "_execute_content_safety",
    "toxicity-classifier": "_execute_toxicity_classifier",
    "bias-detector": "_execute_bias_detector",
    # 🔧 新增: 只需在这里注册
    "my-ml-model": "_execute_my_ml_model",
}

def _execute_my_ml_model(ctx: dict) -> dict:
    # 接入 transformers/numpy/scipy/sklearn 等任意 Python ML 库
    from transformers import pipeline
    classifier = pipeline("sentiment-analysis")
    result = classifier(ctx["request"].get("body", ""))[0]
    if result["label"] == "NEGATIVE" and result["score"] > 0.9:
        return {"action": "flag", "score": result["score"], "label": "negative"}
    return {"action": "continue"}
```

### B.5 添加新 WASM 插件

```bash
# 使用 Rust 编写 WASM 插件 (支持 C/C++/Zig/Rust 等任意语言)
cargo init --lib my-wasm-plugin
# 实现 on_request/on_response/on_async_finalize 导出
cargo build --target wasm32-unknown-unknown --release
# 将 my_wasm_plugin.wasm 部署到 plugins/ 目录
```

```
GET    /health
POST   /api/v1/compute-guarantee                # C-SafeGen 认证保证
POST   /api/v1/drift-detection                   # 语义漂移检测
POST   /api/v1/pii-detection                     # PII 深度检测
POST   /api/v1/compliance/report/generate        # PDF 报告生成 (Phase 4)
```

### A.4 错误响应格式

```json
// 所有 API 错误统一格式
{
  "error": {
    "code": "VERIDACTUS_WORKSPACE_NOT_FOUND",
    "message": "Workspace not found: 550e8400-e29b-41d4-a716-446655440000",
    "request_id": "req_abc123",
    "details": {
      "workspace_id": "550e8400-e29b-41d4-a716-446655440000"
    }
  }
}

// 错误代码规范: VERIDACTUS_<LAYER>_<ERROR>
// LAYER: AUTH | WORKSPACE | BUDGET | KEY | PIPELINE | MODEL | COMPLIANCE
```

---

## 附录 B：开发规范

### B.1 分支策略

```
main          ← 生产就绪代码
  └── develop  ← 日常开发集成
        ├── feat/multitenant-foundation  (Phase 1)
        ├── feat/key-billing             (Phase 2)
        ├── feat/frontend-dual-engine    (Phase 3)
        ├── feat/enterprise-compliance   (Phase 4)
        └── feat/production-hardening    (Phase 5)
```

### B.2 代码审查检查点

- [ ] API 端点是否在 OpenAPI 规范中定义？
- [ ] 所有数据库查询是否包含 `workspace_id` 过滤？
- [ ] JWT 权限检查是否在 handler 层执行？
- [ ] LLM Key 是否仅在内存中传输？
- [ ] 前端空 catch 块是否有 `console.warn` 日志？
- [ ] 新增代码是否有对应的单元测试？
- [ ] 数据库迁移是否可回滚？

### B.3 发布检查清单

- [ ] `cargo test --lib` 185+ tests pass
- [ ] `go test ./...` all pass
- [ ] `python -m pytest` all pass
- [ ] `vitest run` 80%+ coverage
- [ ] `cargo audit` 0 high/critical
- [ ] `npm audit` 0 high/critical (prod deps)
- [ ] E2E 测试: 注册→创建 Workspace→配置 Key→发送 Chat→查看 Trace→验证签名
- [ ] 数据库迁移脚本在 staging 环境测试通过
- [ ] CHANGELOG.md 更新

---

> **此文档版本**: v1.0
> **最后更新**: 2026-06-25
> **批准**: 待 TSC 审查
> **下一审查日期**: 每个 Phase 完成后
