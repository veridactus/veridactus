#!/bin/bash
# =============================================================================
# VERIDACTUS 一键部署脚本
# 快速启动完整的 VERIDACTUS 服务栈
#
# Usage:
#   ./deploy/quick-start.sh          # 标准部署
#   ./deploy/quick-start.sh --init   # 首次部署（初始化 MinIO buckets）
#   ./deploy/quick-start.sh --ollama # 包含本地 LLM
#   ./deploy/quick-start.sh --worker # 包含 Python Worker
#   ./deploy/quick-start.sh --all    # 全部服务
#
# Project: VERIDACTUS - Trusted AI Execution Governance
# License: Apache-2.0
# =============================================================================

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Script directory
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
COMPOSE_FILE="$SCRIPT_DIR/docker-compose.yml"

# Default flags
INIT_MODE=false
OLLAMA_MODE=false
WORKER_MODE=false
VERBOSE=false

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --init) INIT_MODE=true; shift ;;
        --ollama) OLLAMA_MODE=true; shift ;;
        --worker) WORKER_MODE=true; shift ;;
        --all) INIT_MODE=true; OLLAMA_MODE=true; WORKER_MODE=true; shift ;;
        --verbose) VERBOSE=true; shift ;;
        -h|--help) echo "Usage: $0 [--init] [--ollama] [--worker] [--all] [--verbose]"; exit 0 ;;
        *) echo "Unknown option: $1"; exit 1 ;;
    esac
done

echo ""
echo "═══════════════════════════════════════════════════════════════"
echo "  ${BLUE}VERIDACTUS 一键部署${NC}"
echo "═══════════════════════════════════════════════════════════════"
echo ""

# Check prerequisites
echo "${YELLOW}[1/6] 检查系统环境...${NC}"

# Check Docker
if ! command -v docker &> /dev/null; then
    echo "${RED}❌ Docker 未安装${NC}"
    echo "请先安装 Docker: https://docs.docker.com/get-docker/"
    exit 1
fi
echo "  ✅ Docker: $(docker --version)"

# Check Docker Compose
if ! docker compose version &> /dev/null; then
    echo "${RED}❌ Docker Compose 未安装${NC}"
    echo "请先安装 Docker Compose v2"
    exit 1
fi
echo "  ✅ Docker Compose: $(docker compose version)"

# Check ports availability
check_port() {
    local port=$1
    if lsof -Pi :$port -sTCP:LISTEN -t >/dev/null 2>&1; then
        echo "${RED}❌ 端口 $port 已被占用${NC}"
        return 1
    fi
    return 0
}

echo "  检查端口..."
PORTS="3000 8080 8081 6379 5432 9000 9001"
PORT_OK=true
for port in $PORTS; do
    if ! check_port $port; then
        PORT_OK=false
    fi
done

if [ "$PORT_OK" = false ]; then
    echo "${YELLOW}提示: 可使用 'docker compose down' 停止现有服务${NC}"
    read -p "是否继续部署？(y/N) " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        exit 1
    fi
fi

# Setup environment
echo ""
echo "${YELLOW}[2/6] 配置环境变量...${NC}"

ENV_FILE="$ROOT_DIR/.env"
ENV_EXAMPLE="$ROOT_DIR/.env.example"

if [ ! -f "$ENV_FILE" ]; then
    if [ -f "$ENV_EXAMPLE" ]; then
        echo "  复制 .env.example 到 .env"
        cp "$ENV_EXAMPLE" "$ENV_FILE"
        echo "${GREEN}  ✅ .env 文件已创建${NC}"
        echo "${YELLOW}  ⚠️  请编辑 .env 文件设置生产环境密钥！${NC}"
    else
        echo "${RED}❌ .env.example 文件不存在${NC}"
        exit 1
    fi
else
    echo "  ✅ .env 文件已存在"
fi

# Pull images
echo ""
echo "${YELLOW}[3/6] 拉取 Docker 镜像...${NC}"

IMAGES=(
    "redis:7-alpine"
    "postgres:16-alpine"
    "minio/minio:latest"
)

for image in "${IMAGES[@]}"; do
    echo "  拉取 $image..."
    if [ "$VERBOSE" = true ]; then
        docker pull $image
    else
        docker pull $image 2>&1 | grep -E "(digest|status|error)" || true
    fi
done

# Check if VERIDACTUS images exist locally or need to be built
VERIDACTUS_IMAGES=(
    "veridactus/veridactus-core:main"
    "veridactus/veridactus-cp:main"
    "veridactus/veridactus-ui:main"
)

for image in "${VERIDACTUS_IMAGES[@]}"; do
    if ! docker image inspect $image &>/dev/null; then
        echo "${YELLOW}  ⚠️  $image 不存在，将尝试从 Docker Hub 拉取${NC}"
        docker pull $image 2>&1 || echo "${YELLOW}  如果拉取失败，请先构建镜像: make build${NC}"
    fi
done

echo "  ✅ 镜像准备完成"

# Build compose command
echo ""
echo "${YELLOW}[4/6] 启动服务...${NC}"

COMPOSE_CMD="docker compose -f $COMPOSE_FILE"

if [ "$INIT_MODE" = true ]; then
    COMPOSE_CMD="$COMPOSE_CMD --profile init"
fi

if [ "$OLLAMA_MODE" = true ]; then
    COMPOSE_CMD="$COMPOSE_CMD --profile ollama"
fi

if [ "$WORKER_MODE" = true ]; then
    COMPOSE_CMD="$COMPOSE_CMD --profile worker"
fi

echo "  执行: $COMPOSE_CMD up -d"

if [ "$VERBOSE" = true ]; then
    $COMPOSE_CMD up -d
else
    $COMPOSE_CMD up -d 2>&1 | tail -20
fi

# Wait for services
echo ""
echo "${YELLOW}[5/6] 等待服务健康...${NC}"

wait_for_health() {
    local service=$1
    local url=$2
    local max_wait=60
    local count=0
    
    echo "  等待 $service..."
    while [ $count -lt $max_wait ]; do
        if curl -sf "$url" >/dev/null 2>&1; then
            echo "    ✅ $service 健康"
            return 0
        fi
        sleep 2
        count=$((count + 2))
    done
    echo "    ⚠️  $service 响应超时 (${max_wait}s)"
    return 1
}

# Wait for infrastructure
sleep 5

wait_for_health "Redis" "http://localhost:6379" || true
wait_for_health "PostgreSQL" "localhost:5432" || true

# Wait for application services (longer timeout)
sleep 10

wait_for_health "Control Plane" "http://localhost:8081/api/v1/health" || true
wait_for_health "Data Plane" "http://localhost:8080/health" || true
wait_for_health "UI" "http://localhost:3000" || true

# Show status
echo ""
echo "${YELLOW}[6/6] 部署状态...${NC}"

docker compose -f $COMPOSE_FILE ps

# Final message
echo ""
echo "═══════════════════════════════════════════════════════════════"
echo "  ${GREEN}🎉 VERIDACTUS 部署完成！${NC}"
echo "═══════════════════════════════════════════════════════════════"
echo ""
echo "  ${BLUE}服务地址:${NC}"
echo "    • UI Dashboard:     http://localhost:3000"
echo "    • Data Plane API:   http://localhost:8080"
echo "    • Control Plane:    http://localhost:8081"
echo "    • MinIO Console:    http://localhost:9001"
echo ""
echo "  ${BLUE}健康检查:${NC}"
echo "    curl http://localhost:8080/health"
echo "    curl http://localhost:8081/api/v1/health"
echo ""
echo "  ${BLUE}常用命令:${NC}"
echo "    查看日志:   docker compose -f deploy/docker-compose.yml logs -f"
echo "    停止服务:   docker compose -f deploy/docker-compose.yml down"
echo "    重启服务:   docker compose -f deploy/docker-compose.yml restart"
echo "    清理数据:   docker compose -f deploy/docker-compose.yml down -v"
echo ""
echo "  ${YELLOW}⚠️  生产环境请修改 .env 文件中的密钥！${NC}"
echo ""
echo "═══════════════════════════════════════════════════════════════"