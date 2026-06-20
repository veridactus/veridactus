# VERIDACTUS API 规范概览

本文档概述 VERIDACTUS 项目的 API 设计原则和规范结构。

## 服务架构

VERIDACTUS 采用微服务架构，包含以下服务：

```
┌─────────────────────────────────────────────────────────────────┐
│                         客户端                                  │
└─────────────────────────────────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────┐
│                    veridactus-ui (3000)                         │
│                    React + Vite 前端                            │
└─────────────────────────────────────────────────────────────────┘
                │                                    │
                ▼                                    ▼
┌───────────────────────────┐    ┌───────────────────────────────────┐
│   数据面 (8080)            │    │      控制面 (8081)                  │
│   Rust + Axum              │    │      Go + SQLite                   │
│                            │    │                                   │
│ • /v1/chat/completions     │    │ • /api/v1/pipelines               │
│ • /v1/traces/*             │    │ • /api/v1/plugins                 │
│ • /v1/replay/*             │    │ • /api/v1/models                  │
│ • /v1/gdpr/*              │    │ • /api/v1/apikeys                 │
│ • /v1/compliance/*        │    │ • /api/v1/policies                │
│ • /v1/metrics/*           │    │ • /api/v1/config/poll             │
│ • /health                  │    │ • /api/v1/health                  │
└───────────────────────────┘    └───────────────────────────────────┘
                │                                    │
                ▼                                    ▼
┌─────────────────────────────────────────────────────────────────┐
│                    python-worker (8002)                          │
│                    Python + FastAPI                              │
│                                                                │
│ • /pw/health                                                    │
│ • /pw/v1/pii/detect                                             │
│ • /pw/v1/drift/detect                                           │
│ • /pw/v1/privacy/analyze                                        │
└─────────────────────────────────────────────────────────────────┘
```

## API 版本控制

| 服务 | 当前版本 | 规范格式 | 生成方式 |
|------|---------|---------|---------|
| 数据面 | v0.2.1 | OpenAPI 3.0 | utoipa (Rust) |
| 控制面 | v0.2.1 | Swagger 2.0 | swaggo (Go) |
| Python Worker | v0.2.1 | OpenAPI 3.0 | FastAPI 自动 |

## 认证机制

### 数据面认证

数据面使用 **API Key** 认证：

```http
Authorization: Bearer <api_key>
```

**受保护的端点**：
- `/v1/traces/*` - 需要轨迹访问权限
- `/v1/chat/completions` - 需要有效配额
- `/v1/replay/*` - 需要重放权限
- `/v1/gdpr/*` - 需要管理员权限
- `/v1/metrics/*` - 需要监控权限

**公开端点**：
- `/health` - 健康检查
- `/metrics` - Prometheus 指标 (无需认证)

### 控制面认证

控制面使用 **Admin Key** 认证：

```http
X-Admin-Key: <admin_key>
```

所有 `/api/v1/*` 端点都需要有效的 Admin Key。

## 数据面 API 端点

### 聊天完成

```
POST /v1/chat/completions
```

OpenAI 兼容的聊天完成接口。

**请求体**：
```json
{
  "model": "gpt-4o",
  "messages": [
    {"role": "system", "content": "You are a helpful assistant."},
    {"role": "user", "content": "Hello!"}
  ],
  "max_tokens": 100,
  "temperature": 0.7,
  "stream": false
}
```

**响应头**：
```
VERIDACTUS-Trace-Id: <trace_id>
VERIDACTUS-Proof-Levels: L0,L2A
VERIDACTUS-Cost-Consumed: 0.00123
VERIDACTUS-Version: 0.2.1
```

### 轨迹管理

| 端点 | 方法 | 描述 |
|------|------|------|
| `/v1/traces` | GET | 列出轨迹 |
| `/v1/traces/{id}` | GET | 获取轨迹详情 |
| `/v1/traces/{id}/verify` | POST | 验证轨迹签名 |
| `/v1/traces/{id}/replay` | POST | 重放轨迹 |
| `/v1/traces/{id}/compliance` | GET | 获取合规报告 |

### GDPR 合规

| 端点 | 方法 | 描述 |
|------|------|------|
| `/v1/gdpr/delete` | POST | 提交删除请求 |
| `/v1/gdpr/deletion-proof/{id}` | GET | 获取删除证明 |
| `/v1/gdpr/deletion-history` | GET | 获取删除历史 |

### 监控

| 端点 | 方法 | 描述 |
|------|------|------|
| `/v1/metrics/realtime` | GET | 实时指标 |
| `/v1/prevention/stats` | GET | 防护统计 |
| `/v1/audit/log` | GET | 审计日志 |

## 控制面 API 端点

### 流水线管理

```
GET    /api/v1/pipelines          # 列出所有流水线
POST   /api/v1/pipelines          # 创建新流水线
GET    /api/v1/pipelines/{id}     # 获取流水线详情
PUT    /api/v1/pipelines/{id}     # 更新流水线
DELETE /api/v1/pipelines/{id}     # 删除流水线
```

### 插件管理

```
GET    /api/v1/plugins             # 列出已注册插件
POST   /api/v1/plugins            # 注册新插件
```

### 模型配置

```
GET    /api/v1/models             # 列出所有模型
POST   /api/v1/models             # 创建模型配置
GET    /api/v1/models/{id}        # 获取模型详情
PUT    /api/v1/models/{id}        # 更新模型配置
DELETE /api/v1/models/{id}        # 删除模型配置
```

### API 密钥管理

```
GET    /api/v1/apikeys            # 列出所有密钥
POST   /api/v1/apikeys            # 创建新密钥
GET    /api/v1/apikeys/{id}       # 获取密钥详情
PUT    /api/v1/apikeys/{id}       # 更新密钥
DELETE /api/v1/apikeys/{id}       # 吊销密钥
```

### 配置轮询

```
GET    /api/v1/config/poll         # 数据面轮询获取配置变更
```

## 错误响应格式

所有服务使用统一的错误响应格式：

```json
{
  "error": {
    "code": "VALIDATION_ERROR",
    "message": "Invalid request parameters",
    "hint": "Check the API documentation for required fields"
  }
}
```

### 错误代码

| 代码 | HTTP 状态码 | 描述 |
|------|-------------|------|
| `VALIDATION_ERROR` | 400 | 请求参数验证失败 |
| `UNAUTHORIZED` | 401 | 认证失败或缺失 |
| `FORBIDDEN` | 403 | 权限不足 |
| `NOT_FOUND` | 404 | 资源不存在 |
| `RATE_LIMITED` | 429 | 请求频率超限 |
| `BUDGET_EXCEEDED` | 429 | 预算额度用尽 |
| `INTERNAL_ERROR` | 500 | 服务器内部错误 |

## 速率限制

| 端点类型 | 限制 |
|---------|------|
| `/v1/chat/completions` | 100 请求/分钟 |
| `/v1/traces` (读) | 1000 请求/分钟 |
| `/api/v1/*` (控制面) | 100 请求/分钟 |

## SDK 和客户端

| 语言 | SDK | 状态 |
|------|-----|------|
| Python | `pip install veridactus` | 规划中 |
| JavaScript/TypeScript | 内置前端使用 | 已实现 |
| Go | `go get github.com/veridactus/sdk-go` | 规划中 |
| Rust | `cargo add veridactus-core` | 已实现 |

## 相关文档

- [数据面 OpenAPI 规范](./data-plane/)
- [控制面 Swagger 规范](./control-plane/)
- [Python Worker API](./python-worker/)
- [协议规范](../../veridactus/docs/specification/v0.2.1/)
