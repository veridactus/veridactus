# API Documentation Structure

本目录包含 VERIDACTUS 各服务的 API 规范文档，均由代码自动生成。

## 目录结构

```
docs/
└── api/
    ├── data-plane/           # 数据面 API (Rust + Axum)
    │   ├── data-plane-v0.2.1.json   # OpenAPI 3.0 JSON 规范
    │   └── data-plane-v0.2.1.yaml   # OpenAPI 3.0 YAML 规范
    ├── control-plane/        # 控制面 API (Go + net/http)
    │   ├── docs.go          # Swagger 注解
    │   └── swagger.json      # Swagger 2.0 规范
    └── python-worker/        # Python Worker API (FastAPI)
        └── openapi.json      # OpenAPI 3.0 规范
```

## 生成方式

### 自动生成 (CI/CD)

API 文档通过 GitHub Actions 自动生成：

```bash
# 触发条件
- main/develop 分支有代码推送
- core/, control-plane/, python-worker/ 目录下 .rs/.go/.py 文件有变更

# 生成流程
1. 检出代码
2. 安装依赖 (Rust, Go, Python)
3. 运行各自的生成器
4. 上传产物到 Artifacts
5. 部署到 GitHub Pages
```

### 手动生成

#### 数据面 (Rust)

```bash
cd core
cargo run --bin generate-openapi -- --format both --output ../docs/api/data-plane
```

#### 控制面 (Go)

```bash
cd control-plane
go install github.com/swaggo/swag/cmd/swag@latest
swag init -g cmd/server/main.go -o ../docs/api/control-plane
```

#### Python Worker

```bash
cd python-worker
python -c "from app.main import app; ..."
```

## 查看文档

### 在线 Swagger UI

文档生成后可通过 GitHub Pages 访问：

```
https://veridactus.github.io/veridactus/api/
```

### 本地查看

```bash
# 安装 swagger-ui
docker run -p 8080:8080 \
  -e SWAGGER_JSON_URL=https://raw.githubusercontent.com/veridactus/veridactus/main/docs/api/data-plane/data-plane-v0.2.1.json \
  swaggerapi/swagger-ui
```

## 服务端点汇总

| 服务 | 端口 | 规范类型 | 文件位置 |
|------|------|---------|---------|
| 数据面 | 8080 | OpenAPI 3.0 | `docs/api/data-plane/` |
| 控制面 | 8081 | Swagger 2.0 | `docs/api/control-plane/` |
| Python Worker | 8002 | OpenAPI 3.0 | `docs/api/python-worker/` |

## 版本管理

- 每个服务的 API 规范包含版本号 (如 `v0.2.1`)
- 规范文件使用版本化命名
- 保留历史版本供兼容查询

## 贡献指南

如果需要修改 API 端点：

1. **Rust 数据面**: 在 `core/src/http/` 下修改处理器，添加 `#[utoipa::path]` 注解
2. **Go 控制面**: 在 `control-plane/cmd/server/` 下修改处理器，添加 Swagger 注释
3. **Python Worker**: 使用 FastAPI 路由装饰器，自动生成 OpenAPI

修改后提交 PR，CI/CD 将自动更新文档。
