# VERIDACTUS 日志与排障指南

## 快速状态检查

```bash
# 一键检查所有服务状态（本地）
bash scripts/status.sh --local

# Docker 部署状态
bash scripts/status.sh --docker

# JSON 格式（CI/CD）
bash scripts/status.sh --local --json
```

## 启动流程

### 本地开发启动

```bash
# 1. 基础设施（Docker）
docker compose -f deploy/docker-compose.yml up -d postgres redis minio

# 2. 等基础设施就绪
until docker exec veridactus-postgres pg_isready -U veridactus && docker exec veridactus-redis redis-cli PING; do sleep 1; done

# 3. 启动应用（三终端）
cd control-plane && STORE_BACKEND=postgres PG_HOST=localhost ... go run ./cmd/server/
cd core && RUST_LOG=info cargo run
cd veridactus-ui && npm run dev
```

### Docker 一键启动

```bash
# 核心服务
docker compose -f deploy/docker-compose.yml up -d

# 含 Python Worker + ClickHouse + Jaeger + Grafana
docker compose -f deploy/docker-compose.yml --profile full up -d
```

## 日志位置

### Docker 部署

| 组件 | 日志命令 |
|------|---------|
| Go CP | `docker compose logs -f veridactus-cp` |
| Rust DP | `docker compose logs -f veridactus-core` |
| Python Worker | `docker compose logs -f veridactus-python-worker` |
| React UI | `docker compose logs -f veridactus-ui` |
| PostgreSQL | `docker compose logs -f veridactus-postgres` |
| Redis | `docker compose logs -f veridactus-redis` |
| MinIO | `docker compose logs -f veridactus-minio` |
| **全部** | `docker compose logs -f` |

### 本地部署

| 组件 | 默认输出 | 重定向示例 |
|------|---------|-----------|
| Go CP | stdout | `go run ./cmd/server/ > /tmp/veridactus-cp.log 2>&1 &` |
| Rust DP | stdout | `RUST_LOG=debug cargo run > /tmp/veridactus-dp.log 2>&1 &` |
| Python | stdout | `python -m uvicorn app.main:app > /tmp/veridactus-py.log 2>&1 &` |
| React | Vite dev server | `npm run dev > /tmp/veridactus-ui.log 2>&1 &` |

## 日志级别

| 组件 | 环境变量 | 默认值 | 可选值 |
|------|---------|--------|--------|
| Go CP | `LOG_LEVEL` | `info` | `debug`, `info`, `warn`, `error` |
| Rust DP | `RUST_LOG` | `info` | `debug`, `info`, `warn`, `error`, `veridactus_core=debug` |
| Python Worker | `WORKER_LOG_LEVEL` | `INFO` | `DEBUG`, `INFO`, `WARNING`, `ERROR` |

**提高详细程度**：

```bash
# Go CP 调试模式
LOG_LEVEL=debug go run ./cmd/server/

# Rust DP 详细日志
RUST_LOG=debug cargo run

# Python Worker 调试
WORKER_LOG_LEVEL=DEBUG python -m uvicorn app.main:app
```

## Go CP 日志格式

JSON 结构化格式，可直接被 Logstash/Fluentd/Loki 解析：

```json
{"timestamp":"2026-06-27T10:00:00Z","level":"info","message":"Control Plane started","port":"8081","store_backend":"postgres","component":"control-plane"}
```

关键字段：
- `timestamp`: ISO 8601 UTC
- `level`: debug / info / warn / error
- `message`: 人类可读消息
- `component`: 固定 `control-plane`
- 额外字段: `error`, `backend`, `port`, `mode` 等

## Rust DP 日志格式

`tracing-subscriber` 默认格式：

```
2026-06-27T10:00:00.123Z  INFO veridactus_core: VERIDACTUS Data Plane starting...
2026-06-27T10:00:01.456Z  INFO veridactus_core::store::backend: Store backend: Postgres
```

## 正常启动日志序列

### Go CP 成功启动

```
{"timestamp":"...","level":"info","message":"Storage initialized","backend":"postgres",...}
{"timestamp":"...","level":"info","message":"Control Plane started","port":"8081","store_backend":"postgres",...}
{"timestamp":"...","level":"info","message":"Data plane ready"}
```

### Rust DP 成功启动

```
INFO veridactus_core: VERIDACTUS Data Plane starting...
INFO veridactus_core::store::backend: Store backend: Postgres (trace_storage) + S3 (artifacts)
INFO veridactus_core::store::backend: Redis connected
INFO veridactus_core::store::backend: ClickHouse connected
INFO veridactus_core: Server listening on 0.0.0.0:8080
INFO veridactus_core: VERIDACTUS Data Plane ready
```

## 常见故障排查

### 1. Go CP 启动失败

| 日志 | 原因 | 解决 |
|------|------|------|
| `JWT_SECRET must be set for non-development environments` | 生产环境未设置 JWT | `export JWT_SECRET=$(openssl rand -base64 32)` |
| `Casbin RBAC initialization failed` | Casbin 模型加载失败 | 检查 `internal/auth/rbac.go` 中的 `casbinModel` 字符串 |
| `Master key initialization failed` | 加密主密钥不可用 | `export VERIDACTUS_MASTER_KEY=$(openssl rand -base64 32)` |
| `store init failed` | 数据库连接失败 | 检查 PG_HOST/PG_PORT/PG_USER/PG_PASS/PG_DB_NAME |
| `Data plane not ready after 30 attempts` | Rust DP 未启动 | 先启动 Rust DP，或设置 `DATA_PLANE_URL` |

### 2. Rust DP 启动失败

| 日志 | 原因 | 解决 |
|------|------|------|
| `ZHIPU_API_KEY not set` | 未配置上游 API Key | 设置 `ZHIPU_API_KEY` 或通过 CP 推送模型配置 |
| `No model routes configured` | 无可用 LLM | 在控制面添加模型配置 |
| `Redis connection failed` | Redis 不可用 | 检查 Redis 是否运行 |
| `ClickHouse connection failed` | ClickHouse 不可用 | 不影响核心功能（warn 级别） |

### 3. 数据库连接问题

```bash
# 检查 PostgreSQL
docker exec veridactus-postgres pg_isready -U veridactus -d veridactus

# 查看 PG 日志
docker compose logs veridactus-postgres | tail -20

# 检查 PG 表
docker exec veridactus-postgres psql -U veridactus -d veridactus -c '\dt'
```

### 4. Redis 连接问题

```bash
# 检查 Redis
docker exec veridactus-redis redis-cli PING

# Redis 数据检查
docker exec veridactus-redis redis-cli KEYS "*"
docker exec veridactus-redis redis-cli INFO keyspace
```

### 5. LLM 调用失败

```bash
# 测试 Chat 端点
curl -X POST http://localhost:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{"model":"glm-4-flash","messages":[{"role":"user","content":"hello"}],"max_tokens":10}'

# 检查 Rust DP 日志中的上游调用错误
docker compose logs veridactus-core | grep -i "error\|upstream\|api.key"
```

### 6. 前端构建/访问问题

```bash
# TypeScript 编译检查
cd veridactus-ui && npx tsc --noEmit

# Vite 构建
cd veridactus-ui && npx vite build

# 前端访问
curl http://localhost:3000
```

## 日志聚合（可选）

### Loki + Promtail

```yaml
# 在 docker-compose.yml 添加
loki:
  image: grafana/loki:latest
  ports: ["3100:3100"]
promtail:
  image: grafana/promtail:latest
  volumes: [/var/lib/docker/containers:/var/lib/docker/containers:ro]
```

Grafana 数据源: `http://loki:3100`

### 本地文件持久化

```bash
# 在 docker-compose 中为每个 service 添加
logging:
  driver: "json-file"
  options:
    max-size: "10m"
    max-file: "3"
```

---

## 一键健康检查端点调用

```bash
curl -s http://localhost:8081/api/v1/health && echo " CP OK"
curl -s http://localhost:8080/health && echo " DP OK"
curl -s http://localhost:8001/health && echo " Worker OK"
curl -s -o /dev/null -w "%{http_code}" http://localhost:3000 && echo " UI OK"
docker exec veridactus-postgres pg_isready -U veridactus && echo " PG OK"
docker exec veridactus-redis redis-cli PING && echo " Redis OK"
```