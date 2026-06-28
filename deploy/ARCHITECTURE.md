# 🏗️ VERIDACTUS 部署架构文档

## 一、系统拓扑

```
                              ┌─────────────────────────────────┐
                              │         Internet / CDN           │
                              └──────────┬──────────────────────┘
                                         │
                              ┌──────────▼──────────────────────┐
                              │     Nginx Ingress Controller     │
                              │  api.veridactus.io  (Go CP)     │
                              │  proxy.veridactus.io (Rust DP)  │
                              │  app.veridactus.io  (React UI)  │
                              └──────────┬──────────────────────┘
                                         │
          ┌──────────────────────────────┼──────────────────────────────┐
          │                              │                              │
┌─────────▼─────────┐          ┌────────▼────────┐          ┌─────────▼─────────┐
│  Control Plane    │          │   Data Plane     │          │   Frontend (UI)   │
│  (Go)             │◄────────►│   (Rust)         │◄────────►│   (React/Nginx)   │
│  :8081            │  HTTP    │   :8080          │  SSE     │   :3000           │
│                   │  gRPC    │                  │  Proxy   │                   │
│ • Auth + JWT      │  (Phase2)│ • Chat Proxy     │          │ • Chat 沙箱       │
│ • OAuth (3端)     │          │ • 7 治理插件     │          │ • A/B 对比        │
│ • Casbin RBAC     │          │ • L0/L2A/L2B     │          │ • Holo-TraceVault │
│ • Virtual Key     │          │ • Budget Guard   │          │ • Web Crypto      │
│ • Platform Pool   │          │ • SSE Streaming  │          │ • 暗黑模式         │
│ • Stripe/支付     │          │ • Key Resolver   │          │                   │
└─────────┬─────────┘          └────────┬─────────┘          └───────────────────┘
          │                             │
          │       ┌─────────────────────┤
          │       │                     │
┌─────────▼───────▼───┐    ┌───────────▼──────────┐    ┌──────────────────┐
│    PostgreSQL 16    │    │      Redis 7          │    │   MinIO (S3)     │
│    (业务数据)        │    │   (实时/缓存/Stream)   │    │   (对象存储)      │
│                     │    │                       │    │                  │
│ • organizations     │    │ • 预算 Lua 扣减       │    │ • Raw Trace JSON │
│ • workspaces        │    │ • 限流令牌桶          │    │ • L2B ZK 证明    │
│ • users             │    │ • Stream 异步分发     │    │ • 备份归档       │
│ • virtual_keys      │    │ • Pipeline 热缓存     │    │                  │
│ • wallets/trans     │    │                       │    │                  │
└─────────────────────┘    └───────────┬───────────┘    └──────────────────┘
                                       │
                          ┌────────────▼────────────┐
                          │   Python Worker (:8001) │
                          │                          │
                          │ • 嵌入漂移检测           │
                          │ • C-SafeGen 安全评分     │
                          │ • PII 检测               │
                          │ • 合规报告 PDF           │
                          │ • Redis Stream 消费      │
                          └──────────────────────────┘

┌─────────────────────────────────────────────────────────────────────────┐
│                          可观测性层                                       │
│                                                                         │
│  ┌──────────────┐    ┌──────────────────┐    ┌──────────────────────┐   │
│  │  ClickHouse  │    │  Jaeger          │    │  Grafana             │   │
│  │  (OLAP)      │    │  (分布式追踪)     │    │  (统一仪表盘)         │   │
│  │              │    │                  │    │                      │   │
│  │ • audit_evts │    │ • OTel traces    │    │ • Prometheus 面板    │   │
│  │ • traces_agg │    │ • Span 可视化    │    │ • ClickHouse 面板    │   │
│  │ • MV 物化视图 │    │ • 耗时分析       │    │ • PII 热力图         │   │
│  └──────────────┘    └──────────────────┘    └──────────────────────┘   │
└─────────────────────────────────────────────────────────────────────────┘
```

## 二、数据流向

```
┌─────────┐  HTTP    ┌──────────┐  HTTP/gRPC  ┌──────────┐  SSE    ┌────────┐
│  Client  │────────►│ Go CP    │────────────►│ Rust DP  │────────►│  User  │
│  (Web)   │  Auth   │ :8081    │  ResolveKey │ :8080    │  Stream │        │
└─────────┘          └────┬─────┘             └────┬─────┘          └────────┘
                          │                        │
              ┌───────────▼───────────┐  ┌─────────▼──────────┐
              │   PostgreSQL          │  │   Redis Stream      │
              │   (Users/Orgs/Keys)   │  │   (veridactus:tasks)│
              └───────────────────────┘  └─────────┬──────────┘
                                                   │ XREADGROUP
                                        ┌──────────▼──────────┐
                                        │  Python Worker      │
                                        │  (Drift/PII/PDF)     │
                                        └─────────────────────┘

每 10 Token 预算熔断:
  Rust DP ──EVAL──► Redis Lua ──DECRBY──► check ──EXCEEDED?──► cut SSE
                    workspace:{id}:budget

Trace 写入路径:
  Rust DP ──┬──► PG traces (元数据索引)
            ├──► ClickHouse audit_events + traces_agg (OLAP 查询)
            └──► MinIO raw JSON (完整 Trace + ZK 证明)
```

## 三、部署方案

### 方案 A: Docker Compose 核心服务

```bash
# 最小部署（PG + Redis + MinIO + Go CP + Rust DP + UI）
docker compose -f deploy/docker-compose.yml up -d

# 含完整可观测性
docker compose --profile observability up -d

# 含 Python Worker（异步任务处理）
docker compose --profile worker up -d

# 全部服务
docker compose --profile full up -d
```

### 方案 B: Kubernetes (Helm)

```bash
# 添加依赖仓库
helm repo add bitnami https://charts.bitnami.com/bitnami
helm repo add grafana https://grafana.github.io/helm-charts
helm repo update

# 核心部署
helm install veridactus ./deploy/helm/veridactus \
  --set controlPlane.secrets.JWT_SECRET=$(openssl rand -base64 32) \
  --set controlPlane.secrets.VERIDACTUS_ADMIN_KEY=$(openssl rand -hex 16) \
  --set controlPlane.secrets.VERIDACTUS_MASTER_KEY=$(openssl rand -base64 32) \
  --namespace veridactus --create-namespace

# 含可观测性
helm install veridactus ./deploy/helm/veridactus \
  -f deploy/helm/veridactus/values.yaml \
  --set clickhouse.enabled=true \
  --set grafana.enabled=true \
  --set minio.enabled=true \
  --namespace veridactus --create-namespace
```

### 方案 C: 本地开发

```bash
# 基础设施
docker compose -f deploy/docker-compose.yml up -d postgres redis minio minio-init

# 启动各服务（本地编译）
cd control-plane && go run ./cmd/server/ &
cd core && cargo run &
cd python-worker && python3 -m uvicorn app.main:app --host 0.0.0.0 --port 8001 &
cd veridactus-ui && npm run dev &
```

## 四、端口映射

| 服务 | 端口 | 协议 | 说明 |
|------|:----:|------|------|
| Go Control Plane | 8081 | HTTP | 多租户管理 API |
| Rust Data Plane | 8080 | HTTP/SSE | LLM 代理 + 治理 |
| React Frontend | 3000/80 | HTTP | Web UI (Dev/Prod) |
| Python Worker | 8001 | HTTP | 异步算法服务 |
| PostgreSQL | 5432 | TCP | 业务数据库 |
| Redis | 6379 | TCP | 缓存/限流/Stream |
| MinIO API | 9000 | HTTP | S3 对象存储 |
| MinIO Console | 9001 | HTTP | MinIO 管理界面 |
| ClickHouse HTTP | 8123 | HTTP | OLAP 查询 |
| ClickHouse Native | 9009 | TCP | 原生协议 |
| Jaeger UI | 16686 | HTTP | 调用链可视化 |
| Jaeger OTLP gRPC | 4317 | gRPC | OpenTelemetry |
| Grafana | 3001 | HTTP | 监控仪表盘 |
| Ollama | 11434 | HTTP | 本地 LLM |

## 五、环境变量

| 变量 | 必需 | 说明 |
|------|:--:|------|
| `JWT_SECRET` | ✅ | JWT 签名密钥 (>=32 bytes) |
| `VERIDACTUS_ADMIN_KEY` | ✅ | 管理 API 密钥 |
| `VERIDACTUS_MASTER_KEY` | ✅ | 信封加密主密钥 (base64, 32 bytes) |
| `PG_PASSWORD` | ✅ | PostgreSQL 密码 |
| `MINIO_PASSWORD` | ✅ | MinIO 密码 |
| `VERIDACTUS_ENV` | - | production/staging/development |
| `VERIDACTUS_KMS_TYPE` | - | env/aliyun/vault |
| `VERIDACTUS_PAYMENT_PROVIDER` | - | alipay/wechatpay/stripe |
| `CLICKHOUSE_URL` | - | ClickHouse 连接地址 |
| `RUST_LOG` | - | Rust 日志级别 |
