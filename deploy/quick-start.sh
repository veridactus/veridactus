#!/bin/bash
# ╔══════════════════════════════════════════════════════════════════════╗
# ║       VERIDACTUS — 一键生产部署脚本                                   ║
# ║  用法: ./quick-start.sh [core|full|observability|local]               ║
# ╚══════════════════════════════════════════════════════════════════════╝
set -euo pipefail

RED='\033[0;31m'; GREEN='\033[0;32m'; YELLOW='\033[1;33m'; BLUE='\033[0;34m'; NC='\033[0m'
MODE="${1:-core}"
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

log_info()  { echo -e "${GREEN}[VERIDACTUS]${NC} $1"; }
log_warn()  { echo -e "${YELLOW}[WARN]${NC} $1"; }
log_error() { echo -e "${RED}[ERROR]${NC} $1"; exit 1; }

check_deps() {
    log_info "检查依赖..."
    for cmd in docker openssl; do
        command -v "$cmd" >/dev/null 2>&1 || log_error "$cmd 未安装"
    done
    docker info >/dev/null 2>&1 || log_error "Docker 未运行"
    log_info "依赖检查通过 ✓"
}

gen_secrets() {
    log_info "生成安全密钥..."
    export JWT_SECRET="${JWT_SECRET:-$(openssl rand -base64 32)}"
    export VERIDACTUS_ADMIN_KEY="${VERIDACTUS_ADMIN_KEY:-$(openssl rand -hex 16)}"
    export VERIDACTUS_MASTER_KEY="${VERIDACTUS_MASTER_KEY:-$(openssl rand -base64 32)}"
    export PG_PASSWORD="${PG_PASSWORD:-$(openssl rand -hex 16)}"
    export MINIO_PASSWORD="${MINIO_PASSWORD:-$(openssl rand -hex 16)}"
    log_info "密钥生成完成 ✓"
}

deploy_core() {
    log_info "部署核心服务 (PG + Redis + MinIO + CP + DP + UI)..."
    cd "$SCRIPT_DIR"
    docker compose up -d postgres redis minio
    sleep 5
    docker compose up -d minio-init
    sleep 3
    docker compose up -d veridactus-cp veridactus-core veridactus-ui
    wait_healthy
}

deploy_full() {
    log_info "部署全部服务..."
    cd "$SCRIPT_DIR"
    docker compose --profile full up -d
    wait_healthy
}

deploy_observability() {
    log_info "部署核心 + 可观测性..."
    cd "$SCRIPT_DIR"
    docker compose up -d postgres redis minio
    sleep 5
    docker compose --profile observability up -d
    sleep 3
    docker compose up -d veridactus-cp veridactus-core veridactus-ui
    wait_healthy
}

wait_healthy() {
    log_info "等待服务就绪..."
    for i in $(seq 1 30); do
        if curl -sf http://localhost:8081/api/v1/health >/dev/null 2>&1; then
            log_info "Go CP :8081 ✓"
            break
        fi
        sleep 2
    done
    for i in $(seq 1 10); do
        if curl -sf http://localhost:8080/health >/dev/null 2>&1; then
            log_info "Rust DP :8080 ✓"
            break
        fi
        sleep 2
    done
}

show_summary() {
    echo ""
    echo -e "${BLUE}╔══════════════════════════════════════════════════════════════╗${NC}"
    echo -e "${BLUE}║              VERIDACTUS Deployed Successfully                ║${NC}"
    echo -e "${BLUE}╠══════════════════════════════════════════════════════════════╣${NC}"
    echo -e "${BLUE}║${NC}  Control Plane API:  ${GREEN}http://localhost:8081${NC}"
    echo -e "${BLUE}║${NC}  Data Plane Proxy:   ${GREEN}http://localhost:8080${NC}"
    echo -e "${BLUE}║${NC}  Web UI:             ${GREEN}http://localhost:3000${NC}"
    echo -e "${BLUE}║${NC}  MinIO Console:      ${GREEN}http://localhost:9001${NC}"
    if docker ps --filter "name=veridactus-grafana" --format '{{.Names}}' | grep -q grafana; then
        echo -e "${BLUE}║${NC}  Grafana:            ${GREEN}http://localhost:3001${NC}"
    fi
    if docker ps --filter "name=veridactus-jaeger" --format '{{.Names}}' | grep -q jaeger; then
        echo -e "${BLUE}║${NC}  Jaeger UI:          ${GREEN}http://localhost:16686${NC}"
    fi
    echo -e "${BLUE}╠══════════════════════════════════════════════════════════════╣${NC}"
    echo -e "${BLUE}║${NC}  Admin Key: ${YELLOW}${VERIDACTUS_ADMIN_KEY:-vd-admin-prod}${NC}"
    echo -e "${BLUE}╚══════════════════════════════════════════════════════════════╝${NC}"
    echo ""
    log_info "测试: curl http://localhost:8081/api/v1/health -H 'X-Admin-Key: ${VERIDACTUS_ADMIN_KEY:-vd-admin-prod}'"
}

# ── Main ──
check_deps
gen_secrets

case "$MODE" in
    core)           deploy_core ;;
    full)           deploy_full ;;
    observability)  deploy_observability ;;
    local)
        log_info "本地开发模式 — 仅启动基础设施"
        cd "$SCRIPT_DIR"
        docker compose up -d postgres redis minio minio-init
        log_info "基础设施就绪。请在各目录中手动启动应用服务。"
        ;;
    stop)
        cd "$SCRIPT_DIR"
        docker compose --profile full down -v
        log_info "所有服务已停止，卷已清理"
        ;;
    *)
        echo "Usage: $0 [core|full|observability|local|stop]"
        exit 1
        ;;
esac

[ "$MODE" != "stop" ] && [ "$MODE" != "local" ] && show_summary
